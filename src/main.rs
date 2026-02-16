mod camera;
mod components;
mod engine;
mod recording;
mod renderer;
mod systems;

use camera::{Camera, CameraMode};
use clap::Parser;
use components::{
    add_child, Checkerboard, Children, Collider, Color, Drag, Friction, GlobalTransform, GrabState,
    Grabbable, GravityAffected, Grounded, Held, Hidden, LocalTransform, Mass, Player, Restitution,
    Static, SwordPosition, SwordState, Velocity,
};
use engine::input::{InputEvent, InputState};
use engine::time::FrameTimer;
use engine::window::GameWindow;
use glam::{Mat4, Vec3};
use hecs::World;
use renderer::mesh::{create_capsule, create_ground_plane, create_sphere, create_sword};
use renderer::{MeshStore, Renderer};
use sdl2::keyboard::Scancode;
use systems::{grab_throw_system, grounded_system, physics_system, player_movement_system, transform_propagation_system};

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

    // Mesh storage — entities reference meshes by handle
    let mut meshes = MeshStore::new();
    let sphere_handle = meshes.add(create_sphere(1.0, 16, 32));
    let ground_handle = meshes.add(create_ground_plane(500.0));
    let capsule_handle = meshes.add(create_capsule(0.3, 1.0, 16, 16));
    let arm_handle = meshes.add(create_capsule(0.08, 0.5, 8, 8));
    let sword_handle = meshes.add(create_sword());

    // ECS world — scene objects are entities with LocalTransform, GlobalTransform, MeshHandle, Color
    let mut world = World::new();

    world.spawn((
        LocalTransform::new(Vec3::ZERO),
        GlobalTransform(Mat4::IDENTITY),
        ground_handle,
        Color(Vec3::new(0.3, 0.6, 0.2)),
        Checkerboard(Vec3::new(0.22, 0.48, 0.15)),
        Collider::Plane {
            normal: Vec3::Y,
            offset: 0.0,
        },
        Static,
    ));

    let mut sphere_transform = LocalTransform::new(Vec3::new(0.0, 2.0, 0.0));
    sphere_transform.scale = Vec3::splat(0.5);

    let red_sphere = world.spawn((
        sphere_transform,
        GlobalTransform(Mat4::IDENTITY),
        sphere_handle,
        Color(Vec3::new(0.8, 0.2, 0.15)),
        Velocity(Vec3::new(0.0, 5.0, 0.0)),
        Mass(1.0),
        GravityAffected,
        Collider::Sphere { radius: 0.5 },
        Restitution(0.3),
        Friction(0.7),
        Drag(0.5),
        Grabbable,
    ));

    // Test child: small blue sphere offset to the right of the red sphere
    let mut child_transform = LocalTransform::new(Vec3::new(0.75, 0.0, 0.0));
    child_transform.scale = Vec3::splat(0.4);
    let child_sphere = world.spawn((
        child_transform,
        GlobalTransform(Mat4::IDENTITY),
        sphere_handle,
        Color(Vec3::new(0.2, 0.4, 0.9)),
    ));

    add_child(&mut world, red_sphere, child_sphere);

    // Player entity — capsule body with physics
    let player_entity = world.spawn((
        LocalTransform::new(Vec3::new(0.0, 2.0, 5.0)),
        GlobalTransform(Mat4::IDENTITY),
        capsule_handle,
        Color(Vec3::new(0.6, 0.6, 0.7)),
        Velocity(Vec3::ZERO),
        Mass(80.0),
        GravityAffected,
        Collider::Capsule {
            radius: 0.3,
            height: 1.0,
        },
        Restitution(0.0),
        Friction(0.8),
        Player,
        Grounded,
        GrabState::new(),
    ));

    // Arm capsules — children of the player, positioned at shoulders
    {
        use glam::Quat;
        let mut left_arm_t = LocalTransform::new(Vec3::new(-0.25, 0.2, 0.4));
        left_arm_t.rotation = Quat::from_rotation_z(0.15);
        let left_arm = world.spawn((
            left_arm_t,
            GlobalTransform(Mat4::IDENTITY),
            arm_handle,
            Color(Vec3::new(0.6, 0.6, 0.7)),
        ));
        add_child(&mut world, player_entity, left_arm);

        let mut right_arm_t = LocalTransform::new(Vec3::new(0.25, 0.2, 0.4));
        right_arm_t.rotation = Quat::from_rotation_z(-0.15);
        let right_arm = world.spawn((
            right_arm_t,
            GlobalTransform(Mat4::IDENTITY),
            arm_handle,
            Color(Vec3::new(0.6, 0.6, 0.7)),
        ));
        add_child(&mut world, player_entity, right_arm);
    }

    // Sword — child of the player, starts sheathed at the hip
    {
        use glam::Quat;
        use std::f32::consts::FRAC_PI_6;
        use std::f32::consts::FRAC_PI_2;

        // Sheathed
        let sheathed_pos = Vec3::new(0.25, 0.2, 0.0);
        let sheathed_rot = Quat::from_rotation_y(FRAC_PI_2);
        let sheathed_rot = Quat::from_rotation_x(2.0 * FRAC_PI_2 + FRAC_PI_6) * sheathed_rot;

        let wielded_pos = Vec3::new(-0.25, 0.1, 0.6);
        let wielded_rot = Quat::from_rotation_y(FRAC_PI_2);

        let mut sword_t = LocalTransform::new(sheathed_pos);
        sword_t.rotation = sheathed_rot;
        sword_t.scale = Vec3::splat(3.0);

        let sword_entity = world.spawn((
            sword_t,
            GlobalTransform(Mat4::IDENTITY),
            sword_handle,
            Color(Vec3::new(0.75, 0.75, 0.8)),
            SwordState {
                position: SwordPosition::Sheathed,
                sheathed_pos,
                sheathed_rot,
                wielded_pos,
                wielded_rot,
            },
        ));
        add_child(&mut world, player_entity, sword_entity);
    }

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
                player_movement_system(&mut world, &input, &camera, speed_mult);
            }
            CameraMode::Fly => {
                camera.move_wasd(&input, timer.dt);
            }
        }

        let collision_events = physics_system(&mut world, &mut physics_accum, timer.dt);
        grounded_system(&mut world, &collision_events);

        if camera.mode == CameraMode::Player {
            if let Ok(local) = world.get::<&LocalTransform>(player_entity) {
                camera.follow_player(local.position, 0.7, 0.3);
            }
        }

        // Propagate transforms before rendering
        transform_propagation_system(&mut world);

        let view = camera.view_matrix();
        let proj = camera.projection_matrix(window.aspect_ratio());

        renderer.draw_scene(&world, &meshes, &view, &proj, camera.position);

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
