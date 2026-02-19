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
use renderer::Renderer;
use scene::test_scene::load_test_scene;
use sdl2::keyboard::Scancode;
use systems::{
    collision_system, grab_throw_system, grounded_system, physics_step, player_movement_system,
    player_state_system, transform_propagation_system, PHYSICS_DT,
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

    let mut world = World::new();
    let (meshes, player_entity) = load_test_scene(&mut world);

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
                        player_movement_system(&mut world, &input, &camera, speed_mult, timer.dt);
                    }
                    CameraMode::Fly => {
                        camera.move_wasd(&input, timer.dt);
                    }
                }

                let mut collision_events = Vec::new();
                let mut physics_ticks = 0usize;
                physics_accum += timer.dt;
                while physics_accum >= PHYSICS_DT {
                    physics_ticks += 1;
                    physics_step(&mut world);
                    collision_events.extend(collision_system(&mut world));
                    physics_accum -= PHYSICS_DT;
                }
                alpha = physics_accum / PHYSICS_DT;
                grounded_system(&mut world, &collision_events, physics_ticks);

                if camera.mode == CameraMode::Player {
                    // Use interpolated player position so the camera follows
                    // smoothly between fixed physics ticks.
                    let player_pos = match (
                        world.get::<&LocalTransform>(player_entity),
                        world.get::<&PreviousPosition>(player_entity),
                    ) {
                        (Ok(local), Ok(prev)) => prev.0.lerp(local.position, alpha),
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
