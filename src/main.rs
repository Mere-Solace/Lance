mod camera;
mod components;
mod engine;
mod renderer;
mod systems;

use camera::{Camera, CameraMode};
use components::{
    add_child, Collider, Color, Drag, Friction, GlobalTransform, GravityAffected, Grounded,
    LocalTransform, Mass, Player, Restitution, Static, Velocity,
};
use engine::input::{InputEvent, InputState};
use engine::time::FrameTimer;
use engine::window::GameWindow;
use glam::{Mat4, Vec3};
use hecs::World;
use renderer::mesh::{create_capsule, create_ground_plane, create_sphere};
use renderer::{MeshStore, Renderer};
use sdl2::keyboard::Scancode;
use systems::{grounded_system, physics_system, player_movement_system, transform_propagation_system};

fn main() {
    let sdl = sdl2::init().expect("Failed to init SDL2");
    let window = GameWindow::new(&sdl, "Lance Engine", 1280, 720);

    let mut renderer = Renderer::init();

    // Mesh storage — entities reference meshes by handle
    let mut meshes = MeshStore::new();
    let sphere_handle = meshes.add(create_sphere(1.0, 16, 32));
    let ground_handle = meshes.add(create_ground_plane(500.0));
    let capsule_handle = meshes.add(create_capsule(0.3, 1.0, 16, 16));

    // ECS world — scene objects are entities with LocalTransform, GlobalTransform, MeshHandle, Color
    let mut world = World::new();

    world.spawn((
        LocalTransform::new(Vec3::ZERO),
        GlobalTransform(Mat4::IDENTITY),
        ground_handle,
        Color(Vec3::new(0.3, 0.6, 0.2)),
        Collider::Plane {
            normal: Vec3::Y,
            offset: 0.0,
        },
        Static,
    ));

    let red_sphere = world.spawn((
        LocalTransform::new(Vec3::new(0.0, 2.0, 0.0)),
        GlobalTransform(Mat4::IDENTITY),
        sphere_handle,
        Color(Vec3::new(0.8, 0.2, 0.15)),
        Velocity(Vec3::new(0.0, 5.0, 0.0)),
        Mass(1.0),
        GravityAffected,
        Collider::Sphere { radius: 1.0 },
        Restitution(0.3),
        Friction(0.5),
        Drag(0.5),
    ));

    // Test child: small blue sphere offset to the right of the red sphere
    let mut child_transform = LocalTransform::new(Vec3::new(2.5, 0.0, 0.0));
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
        LocalTransform::new(Vec3::new(0.0, 2.0, -5.0)),
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
    ));

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
                InputEvent::KeyPressed(Scancode::Z) => camera.toggle_perspective(),
                _ => {}
            }
        }

        camera.look(input.mouse_dx, input.mouse_dy);

        match camera.mode {
            CameraMode::Player => {
                player_movement_system(&mut world, &input, &camera);
            }
            CameraMode::Fly => {
                camera.move_wasd(&input, timer.dt);
            }
        }

        let collision_events = physics_system(&mut world, &mut physics_accum, timer.dt);
        grounded_system(&mut world, &collision_events);

        if camera.mode == CameraMode::Player {
            if let Ok(local) = world.get::<&LocalTransform>(player_entity) {
                camera.follow_player(local.position, 0.7);
            }
        }

        // Propagate transforms before rendering
        transform_propagation_system(&mut world);

        let view = camera.view_matrix();
        let proj = camera.projection_matrix(window.aspect_ratio());

        renderer.draw_scene(&world, &meshes, &view, &proj, camera.position);
        window.swap();
    }
}
