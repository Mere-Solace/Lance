mod camera;
mod components;
mod engine;
mod fsm;
mod recording;
mod renderer;
mod scene;
mod systems;
mod ui;

use camera::{Camera, CameraMode};
use clap::Parser;
use components::{
    Children, Held, Hidden, LocalTransform, PreviousPosition, SwordPosition, SwordState,
};
use engine::input::{InputEvent, InputState};
use engine::time::FrameTimer;
use engine::window::GameWindow;
use glam::{Mat4, Vec3};
use hecs::World;
use renderer::{MeshStore, Renderer};
use scene::prefabs::{
    spawn_directional_light, spawn_ground, spawn_physics_sphere, spawn_player, spawn_point_light,
    spawn_spot_light, spawn_static_box,
};
use sdl2::keyboard::Scancode;
use systems::{
    grab_throw_system, grounded_system, physics_system, player_movement_system,
    player_state_system, transform_propagation_system,
};
use ui::{GameState, PauseAction, PauseMenu, TextRenderer};

#[derive(Parser)]
#[command(name = "lance", about = "Lance Engine")]
struct Args {
    /// Record 5 seconds of video to demos/demo.mp4
    #[arg(long)]
    record: bool,
}

fn main() {
    let args = Args::parse();
    let sdl = sdl2::init().expect("Failed to init SDL2");
    let window = GameWindow::new(&sdl, "Lance Engine", 1280, 720);

    let mut renderer = Renderer::init();
    let mut text_renderer = TextRenderer::new();
    let mut pause_menu = PauseMenu::new();
    let mut game_state = GameState::Running;

    let mut meshes = MeshStore::new();
    let mut world = World::new();

    // --- Scene setup ---
    spawn_ground(&mut world, &mut meshes);

    spawn_physics_sphere(
        &mut world,
        &mut meshes,
        Vec3::new(0.0, 2.0, -3.0),
        Vec3::new(0.8, 0.2, 0.15),
        0.5,
        Vec3::new(0.0, 5.0, 0.0),
    );

    // Grey boxes scattered around spawn
    let grey = Vec3::new(0.5, 0.5, 0.52);
    for &(x, z, h) in &[(6.0_f32, -4.0_f32, 2.0_f32), (-5.0, 3.0, 3.5), (3.0, 7.0, 1.5)] {
        spawn_static_box(
            &mut world,
            &mut meshes,
            Vec3::new(x, h / 2.0, z),
            Vec3::new(2.5, h / 2.0, 3.5),
            grey,
        );
    }

    let player_entity = spawn_player(&mut world, &mut meshes, Vec3::new(0.0, 10.0, 0.0));

    spawn_directional_light(
        &mut world,
        Vec3::new(-0.5, -1.0, -0.3),
        Vec3::new(1.0, 0.95, 0.85),
        1.0,
    );
    spawn_point_light(&mut world, Vec3::new(3.0, 3.0, 0.0), Vec3::new(1.0, 0.6, 0.2), 2.0, 15.0);
    spawn_point_light(&mut world, Vec3::new(-4.0, 2.0, -3.0), Vec3::new(0.2, 0.4, 1.0), 1.5, 12.0);
    spawn_point_light(&mut world, Vec3::new(0.0, 4.0, -8.0), Vec3::new(0.1, 0.9, 0.3), 1.8, 18.0);
    spawn_spot_light(
        &mut world,
        Vec3::new(5.0, 6.0, 5.0),
        Vec3::new(0.0, -1.0, 0.0),
        Vec3::new(1.0, 0.9, 0.7),
        3.0,
        15.0,
        30.0,
        20.0,
    );

    let mut recorder = if args.record {
        let (w, h) = window.size();
        Some(recording::Recorder::new(w, h, "demos/demo.mp4"))
    } else {
        None
    };
    let mut record_elapsed: f32 = 0.0;
    let mut record_frame_debt: f32 = 0.0;
    const RECORD_DURATION: f32 = 5.0;
    const RECORD_FRAME_INTERVAL: f32 = 1.0 / 60.0;

    sdl.mouse().set_relative_mouse_mode(true);

    let mut event_pump = sdl.event_pump().expect("Failed to get event pump");
    let mut input = InputState::new();
    let mut timer = FrameTimer::new();
    let mut camera = Camera::new();
    let mut physics_accum: f32 = 0.0;

    loop {
        timer.tick();
        input.update(&mut event_pump);

        if input.should_quit() {
            break;
        }

        // Handle Escape toggle between Running and Paused
        let mut just_paused = false;
        for event in &input.events {
            if let InputEvent::KeyPressed(Scancode::Escape) = event {
                if game_state == GameState::Running {
                    game_state = GameState::Paused;
                    pause_menu.reset_selection();
                    sdl.mouse().set_relative_mouse_mode(false);
                    just_paused = true;
                }
            }
        }

        // Physics interpolation alpha, set each frame by physics_system.
        // 1.0 when paused (render current state without interpolation).
        let mut alpha: f32 = 1.0;

        // Route input based on game state
        match game_state {
            GameState::Paused => {
                // Skip input on the frame we just entered pause (same Escape event would resume)
                let action = if just_paused {
                    PauseAction::None
                } else {
                    pause_menu.handle_input(&input.events)
                };
                match action {
                    PauseAction::Resume => {
                        game_state = GameState::Running;
                        sdl.mouse().set_relative_mouse_mode(true);
                    }
                    PauseAction::Quit => break,
                    PauseAction::None => {}
                }
            }
            GameState::Running => {
                // F1 toggles fly/player mode, Z toggles first/third person
                for event in &input.events {
                    match event {
                        InputEvent::KeyPressed(Scancode::F1) => camera.toggle_mode(),
                        InputEvent::KeyPressed(Scancode::Z) => {
                            camera.toggle_perspective();
                            // Collect player + children entity IDs
                            let mut to_toggle = vec![player_entity];
                            if let Ok(children) = world.get::<&Children>(player_entity) {
                                to_toggle.extend(children.0.iter().copied());
                            }
                            // Hide/show player body in first/third person
                            // Skip held objects and sword (always visible)
                            for entity in to_toggle {
                                if world.get::<&Held>(entity).is_ok() {
                                    continue;
                                }
                                if world.get::<&SwordState>(entity).is_ok() {
                                    continue;
                                }
                                if camera.is_third_person() {
                                    let _ = world.remove_one::<Hidden>(entity);
                                } else {
                                    let _ = world.insert_one(entity, Hidden);
                                }
                            }
                        }
                        InputEvent::KeyPressed(Scancode::F) => {
                            // Toggle sword between sheathed and wielded
                            for (_e, (sword, lt)) in
                                world.query_mut::<(&mut SwordState, &mut LocalTransform)>()
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

                camera.look(input.mouse_dx, input.mouse_dy);

                // Grab/throw must run before player movement to produce speed multiplier
                let speed_mult = if camera.mode == CameraMode::Player {
                    grab_throw_system(&mut world, &input, &camera, timer.dt)
                } else {
                    1.0
                };

                match camera.mode {
                    CameraMode::Player => {
                        player_state_system(&mut world, &input, timer.dt);
                        player_movement_system(&mut world, &input, &camera, speed_mult);
                    }
                    CameraMode::Fly => {
                        camera.move_wasd(&input, timer.dt);
                    }
                }

                let (collision_events, frame_alpha, physics_ticks) =
                    physics_system(&mut world, &mut physics_accum, timer.dt);
                alpha = frame_alpha;
                grounded_system(&mut world, &collision_events, physics_ticks);

                if camera.mode == CameraMode::Player {
                    // Use interpolated player position so the camera follows
                    // smoothly between fixed physics ticks.
                    let player_pos = match (
                        world.get::<&LocalTransform>(player_entity),
                        world.get::<&PreviousPosition>(player_entity),
                    ) {
                        (Ok(local), Ok(prev)) => prev.0.lerp(local.position, frame_alpha),
                        (Ok(local), _) => local.position,
                        _ => Vec3::ZERO,
                    };
                    camera.follow_player(player_pos, 0.7, 0.3);
                }
            }
        }

        // Propagate transforms before rendering (always, even when paused).
        // alpha interpolates entity positions between fixed physics steps.
        transform_propagation_system(&mut world, alpha);

        let view = camera.view_matrix();
        let proj = camera.projection_matrix(window.aspect_ratio());

        renderer.draw_scene(&world, &meshes, &view, &proj, camera.position);

        // UI pass — render on top of the scene
        if game_state == GameState::Paused {
            let (w, h) = window.size();
            let ui_proj = Mat4::orthographic_rh_gl(0.0, w as f32, h as f32, 0.0, -1.0, 1.0);

            unsafe {
                gl::Disable(gl::DEPTH_TEST);
                gl::Enable(gl::BLEND);
                gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
            }

            pause_menu.draw(&mut text_renderer, w as f32, h as f32, &ui_proj);

            unsafe {
                gl::Disable(gl::BLEND);
                gl::Enable(gl::DEPTH_TEST);
            }
        }

        if let Some(ref mut rec) = recorder {
            record_elapsed += timer.dt;
            record_frame_debt += timer.dt;
            while record_frame_debt >= RECORD_FRAME_INTERVAL {
                rec.capture_frame();
                record_frame_debt -= RECORD_FRAME_INTERVAL;
            }
            if record_elapsed >= RECORD_DURATION {
                recorder.take().unwrap().finish();
                break;
            }
        }

        window.swap();
    }
}
