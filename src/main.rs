mod camera;
mod components;
mod engine;
mod renderer;
mod systems;

use camera::Camera;
use components::{add_child, Color, GlobalTransform, LocalTransform};
use engine::input::InputState;
use engine::time::FrameTimer;
use engine::window::GameWindow;
use glam::{Mat4, Vec3};
use hecs::World;
use renderer::mesh::{create_ground_plane, create_sphere};
use renderer::{MeshStore, Renderer};
use systems::transform_propagation_system;

fn main() {
    let sdl = sdl2::init().expect("Failed to init SDL2");
    let window = GameWindow::new(&sdl, "Lance Engine", 1280, 720);

    let mut renderer = Renderer::init();

    // Mesh storage — entities reference meshes by handle
    let mut meshes = MeshStore::new();
    let sphere_handle = meshes.add(create_sphere(1.0, 16, 32));
    let ground_handle = meshes.add(create_ground_plane(500.0));

    // ECS world — scene objects are entities with LocalTransform, GlobalTransform, MeshHandle, Color
    let mut world = World::new();

    world.spawn((
        LocalTransform::new(Vec3::ZERO),
        GlobalTransform(Mat4::IDENTITY),
        ground_handle,
        Color(Vec3::new(0.3, 0.6, 0.2)),
    ));

    let red_sphere = world.spawn((
        LocalTransform::new(Vec3::new(0.0, 2.0, 0.0)),
        GlobalTransform(Mat4::IDENTITY),
        sphere_handle,
        Color(Vec3::new(0.8, 0.2, 0.15)),
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

    sdl.mouse().set_relative_mouse_mode(true);

    let mut event_pump = sdl.event_pump().expect("Failed to get event pump");
    let mut input = InputState::new();
    let mut timer = FrameTimer::new();
    let mut camera = Camera::new();
    let mut time_accum: f32 = 0.0;

    loop {
        timer.tick();
        input.update(&mut event_pump);

        if input.should_quit() {
            break;
        }

        camera.look(input.mouse_dx, input.mouse_dy);
        camera.move_wasd(&input, timer.dt);

        // Animate red sphere in a gentle circle for visual testing
        time_accum += timer.dt;
        if let Ok(mut local) = world.get::<&mut LocalTransform>(red_sphere) {
            let t = time_accum * 0.5; // slow rotation
            local.position.x = 4.0 * t.cos();
            local.position.z = 4.0 * t.sin();
        }

        // Propagate transforms before rendering
        transform_propagation_system(&mut world);

        let view = camera.view_matrix();
        let proj = camera.projection_matrix(window.aspect_ratio());

        renderer.draw_scene(&world, &meshes, &view, &proj, camera.position);
        window.swap();
    }
}
