mod camera;
mod engine;
mod renderer;

use camera::Camera;
use engine::input::InputState;
use engine::time::FrameTimer;
use engine::window::GameWindow;
use renderer::Renderer;

fn main() {
    let sdl = sdl2::init().expect("Failed to init SDL2");
    let window = GameWindow::new(&sdl, "Lance Engine", 1280, 720);

    let mut renderer = Renderer::init();

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

        renderer.draw_scene(&view, &proj, camera.position);
        window.swap();
    }
}
