mod camera;
mod components;
mod engine;
mod renderer;

use camera::Camera;
use components::{Color, Transform};
use engine::input::InputState;
use engine::time::FrameTimer;
use engine::window::GameWindow;
use glam::Vec3;
use hecs::World;
use renderer::mesh::{create_ground_plane, create_sphere};
use renderer::{MeshStore, Renderer};

fn main() {
    let sdl = sdl2::init().expect("Failed to init SDL2");
    let window = GameWindow::new(&sdl, "Lance Engine", 1280, 720);

    let mut renderer = Renderer::init();

    // Mesh storage — entities reference meshes by handle
    let mut meshes = MeshStore::new();
    let sphere_handle = meshes.add(create_sphere(1.0, 16, 32));
    let ground_handle = meshes.add(create_ground_plane(500.0));

    // ECS world — scene objects are entities with Transform, MeshHandle, Color
    let mut world = World::new();

    world.spawn((
        Transform::new(Vec3::ZERO),
        ground_handle,
        Color(Vec3::new(0.3, 0.6, 0.2)),
    ));

    world.spawn((
        Transform::new(Vec3::new(0.0, 2.0, 0.0)),
        sphere_handle,
        Color(Vec3::new(0.8, 0.2, 0.15)),
    ));

    sdl.mouse().set_relative_mouse_mode(true);

    let mut event_pump = sdl.event_pump().expect("Failed to get event pump");
    let mut input = InputState::new();
    let mut timer = FrameTimer::new();
    let mut camera = Camera::new();

    loop {
        timer.tick();
        input.update(&mut event_pump);

        if input.should_quit() {
            break;
        }

        camera.look(input.mouse_dx, input.mouse_dy);
        camera.move_wasd(&input, timer.dt);

        let view = camera.view_matrix();
        let proj = camera.projection_matrix(window.aspect_ratio());

        renderer.draw_scene(&world, &meshes, &view, &proj, camera.position);
        window.swap();
    }
}
