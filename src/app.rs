use crate::camera::{Camera, CameraMode};
use crate::components::{Children, Held, Hidden, LocalTransform, PreviousPosition, SwordPosition, SwordState};
use crate::engine::input::{InputEvent, InputState};
use crate::engine::time::FrameTimer;
use crate::engine::window::GameWindow;
use crate::recording;
use crate::renderer::{MeshStore, Renderer};
use crate::systems::{
    collision_system, grab_throw_system, grounded_system, physics_step, player_movement_system,
    player_state_system, raycast_static, transform_propagation_system, PHYSICS_DT,
};
use crate::ui::{DebugHud, GameState, PauseAction, PauseMenu, TextRenderer};
use glam::{Mat4, Vec3};
use hecs::{Entity, World};
use sdl2::keyboard::Scancode;
use sdl2::Sdl;

pub struct GameApp {
    world: World,
    meshes: MeshStore,
    player_entity: Entity,
    camera: Camera,
    renderer: Renderer,
    text_renderer: TextRenderer,
    pause_menu: PauseMenu,
    debug_hud: DebugHud,
    game_state: GameState,
    physics_accum: f32,
    recorder: Option<recording::Recorder>,
    record_elapsed: f32,
    record_frame_debt: f32,
}

impl GameApp {
    pub fn new(
        world: World,
        meshes: MeshStore,
        player_entity: Entity,
        record: bool,
        window: &GameWindow,
    ) -> Self {
        let recorder = if record {
            let (w, h) = window.size();
            Some(recording::Recorder::new(w, h, "demos/demo.mp4"))
        } else {
            None
        };

        Self {
            world,
            meshes,
            player_entity,
            camera: Camera::new(),
            renderer: Renderer::init(),
            text_renderer: TextRenderer::new(),
            pause_menu: PauseMenu::new(),
            debug_hud: DebugHud::new(),
            game_state: GameState::Running,
            physics_accum: 0.0,
            recorder,
            record_elapsed: 0.0,
            record_frame_debt: 0.0,
        }
    }

    pub fn run(&mut self, sdl: &Sdl, window: &GameWindow) {
        sdl.mouse().set_relative_mouse_mode(true);
        let mut event_pump = sdl.event_pump().expect("Failed to get event pump");
        let mut input = InputState::new();
        let mut timer = FrameTimer::new();

        'main: loop {
            timer.tick();
            input.update(&mut event_pump);

            if input.should_quit() {
                break;
            }

            // Handle Escape toggle between Running and Paused
            let mut just_paused = false;
            for event in &input.events {
                if let InputEvent::KeyPressed(Scancode::Escape) = event {
                    if self.game_state == GameState::Running {
                        self.game_state = GameState::Paused;
                        self.pause_menu.reset_selection();
                        sdl.mouse().set_relative_mouse_mode(false);
                        just_paused = true;
                    }
                }
            }

            // Physics interpolation alpha — 1.0 when paused (no interpolation).
            let mut alpha: f32 = 1.0;

            match self.game_state {
                GameState::Paused => {
                    // Skip input on the frame we just entered pause (same Escape event would resume)
                    if !just_paused {
                        match self.handle_paused_input(&input) {
                            PauseAction::Resume => {
                                self.game_state = GameState::Running;
                                sdl.mouse().set_relative_mouse_mode(true);
                            }
                            PauseAction::Quit => break 'main,
                            PauseAction::None => {}
                        }
                    }
                }
                GameState::Running => {
                    alpha = self.update_systems(&input, timer.dt);
                    if self.debug_hud.is_visible() {
                        self.debug_hud.update(timer.dt);
                    }
                }
            }

            // Propagate transforms before rendering (always, even when paused).
            transform_propagation_system(&mut self.world, alpha);
            self.render(window);

            if self.tick_recorder(timer.dt) {
                break;
            }

            window.swap();
        }
    }

    fn handle_running_input(&mut self, input: &InputState) {
        for event in &input.events {
            match event {
                InputEvent::KeyPressed(Scancode::F1) => self.camera.toggle_mode(),
                InputEvent::KeyPressed(Scancode::F3) => self.debug_hud.toggle(),
                InputEvent::KeyPressed(Scancode::Z) => {
                    self.camera.toggle_perspective();
                    let mut to_toggle = vec![self.player_entity];
                    if let Ok(children) = self.world.get::<&Children>(self.player_entity) {
                        to_toggle.extend(children.0.iter().copied());
                    }
                    let is_third_person = self.camera.is_third_person();
                    for entity in to_toggle {
                        if self.world.get::<&Held>(entity).is_ok() {
                            continue;
                        }
                        if self.world.get::<&SwordState>(entity).is_ok() {
                            continue;
                        }
                        if is_third_person {
                            let _ = self.world.remove_one::<Hidden>(entity);
                        } else {
                            let _ = self.world.insert_one(entity, Hidden);
                        }
                    }
                }
                InputEvent::KeyPressed(Scancode::F) => {
                    for (_e, (sword, lt)) in
                        self.world.query_mut::<(&mut SwordState, &mut LocalTransform)>()
                    {
                        match sword.position {
                            SwordPosition::Sheathed => {
                                sword.position = SwordPosition::Wielded;
                                lt.position = sword.wielded_pos;
                                lt.rotation = sword.wielded_rot;
                            }
                            SwordPosition::Wielded => {
                                sword.position = SwordPosition::Sheathed;
                                lt.position = sword.sheathed_pos;
                                lt.rotation = sword.sheathed_rot;
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        // Free-look (hold C): camera pans without rotating the character.
        // On release, snap camera yaw back to the body's facing direction.
        let was_free_look = self.camera.free_look;
        self.camera.free_look = input.is_key_held(Scancode::C);
        if was_free_look && !self.camera.free_look {
            self.camera.yaw = self.camera.body_yaw;
        }

        // Scroll wheel zoom.
        if input.scroll_dy != 0.0 {
            self.camera.apply_zoom(input.scroll_dy);
        }

        self.camera.look(input.mouse_dx, input.mouse_dy);

        // Keep body_yaw in sync with camera.yaw every frame we are NOT in free-look.
        if !self.camera.free_look {
            self.camera.body_yaw = self.camera.yaw;
        }
    }

    fn handle_paused_input(&mut self, input: &InputState) -> PauseAction {
        self.pause_menu.handle_input(&input.events)
    }

    fn update_systems(&mut self, input: &InputState, dt: f32) -> f32 {
        self.handle_running_input(input);

        // Grab/throw must run before player movement to produce speed multiplier
        let speed_mult = if self.camera.mode == CameraMode::Player {
            let camera = &self.camera;
            grab_throw_system(&mut self.world, input, camera, dt)
        } else {
            1.0
        };

        match self.camera.mode {
            CameraMode::Player => {
                player_state_system(&mut self.world, input, dt);
                let camera = &self.camera;
                player_movement_system(&mut self.world, input, camera, speed_mult, dt);
            }
            CameraMode::Fly => {
                self.camera.move_wasd(input, dt);
            }
        }

        let mut collision_events = Vec::new();
        let mut physics_ticks = 0usize;
        self.physics_accum += dt;
        while self.physics_accum >= PHYSICS_DT {
            physics_ticks += 1;
            physics_step(&mut self.world);
            collision_events.extend(collision_system(&mut self.world));
            self.physics_accum -= PHYSICS_DT;
        }
        let alpha = self.physics_accum / PHYSICS_DT;
        grounded_system(&mut self.world, &collision_events, physics_ticks);

        if self.camera.mode == CameraMode::Player {
            // Use interpolated player position so the camera follows
            // smoothly between fixed physics ticks.
            let player_pos = match (
                self.world.get::<&LocalTransform>(self.player_entity),
                self.world.get::<&PreviousPosition>(self.player_entity),
            ) {
                (Ok(local), Ok(prev)) => prev.0.lerp(local.position, alpha),
                (Ok(local), _) => local.position,
                _ => Vec3::ZERO,
            };
            // Compute desired camera position, raycast for wall occlusion, apply.
            let (eye, desired) = self.camera.desired_follow_pos(player_pos, 0.7, 0.3);
            let ray_to_desired = desired - eye;
            let max_dist = ray_to_desired.length();
            let hit_dist = if max_dist > 1e-6 && self.camera.is_third_person() {
                raycast_static(&self.world, eye, ray_to_desired / max_dist, max_dist)
            } else {
                None
            };
            self.camera.apply_occlusion(eye, desired, hit_dist, dt);
        }

        alpha
    }

    fn render(&mut self, window: &GameWindow) {
        let view = self.camera.view_matrix();
        let proj = self.camera.projection_matrix(window.aspect_ratio());

        self.renderer
            .draw_scene(&self.world, &self.meshes, &view, &proj, self.camera.position);

        // UI pass — render on top of the scene
        if self.game_state == GameState::Paused {
            let (w, h) = window.size();
            let ui_proj = Mat4::orthographic_rh_gl(0.0, w as f32, h as f32, 0.0, -1.0, 1.0);

            unsafe {
                gl::Disable(gl::DEPTH_TEST);
                gl::Enable(gl::BLEND);
                gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
            }

            self.pause_menu
                .draw(&mut self.text_renderer, w as f32, h as f32, &ui_proj);

            unsafe {
                gl::Disable(gl::BLEND);
                gl::Enable(gl::DEPTH_TEST);
            }
        }

        // Debug HUD — always on top, independent of game state
        if self.debug_hud.is_visible() {
            let (w, h) = window.size();
            let ui_proj = Mat4::orthographic_rh_gl(0.0, w as f32, h as f32, 0.0, -1.0, 1.0);

            // In Player mode show the player body position, not the orbiting camera.
            let hud_pos = if self.camera.mode == CameraMode::Player {
                self.world
                    .get::<&LocalTransform>(self.player_entity)
                    .map(|t| t.position)
                    .unwrap_or(self.camera.position)
            } else {
                self.camera.position
            };

            unsafe {
                gl::Disable(gl::DEPTH_TEST);
                gl::Enable(gl::BLEND);
                gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
            }

            self.debug_hud
                .draw(&mut self.text_renderer, hud_pos, &self.camera, &ui_proj);

            unsafe {
                gl::Disable(gl::BLEND);
                gl::Enable(gl::DEPTH_TEST);
            }
        }
    }

    fn tick_recorder(&mut self, dt: f32) -> bool {
        const RECORD_FRAME_INTERVAL: f32 = 1.0 / 60.0;
        const RECORD_DURATION: f32 = 5.0;
        if let Some(ref mut rec) = self.recorder {
            self.record_elapsed += dt;
            self.record_frame_debt += dt;
            while self.record_frame_debt >= RECORD_FRAME_INTERVAL {
                rec.capture_frame();
                self.record_frame_debt -= RECORD_FRAME_INTERVAL;
            }
            if self.record_elapsed < RECORD_DURATION {
                return false;
            }
        } else {
            return false;
        }
        self.recorder.take().unwrap().finish();
        true
    }
}
