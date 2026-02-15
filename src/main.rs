mod camera;
mod engine;
mod renderer;

use camera::Camera;
use engine::input::InputState;
use engine::time::FrameTimer;
use engine::window::GameWindow;

fn main() {
    let sdl = sdl2::init().expect("Failed to init SDL2");
    let window = GameWindow::new(&sdl, "Lance Engine", 1280, 720);

    renderer::init();

    sdl.mouse().set_relative_mouse_mode(true);

    let mut event_pump = sdl.event_pump().expect("Failed to get event pump");
    let mut input = InputState::new();
    let mut timer = FrameTimer::new();
    let mut camera = Camera::new();

    loop {
        timer.tick();
        input.update(&mut event_pump);

        if input.quit {
            break;
        }

        camera.look(input.mouse_dx, input.mouse_dy);
        camera.move_wasd(&input, timer.dt);

        let _view = camera.view_matrix();
        let _proj = camera.projection_matrix(window.aspect_ratio());

        renderer::begin_frame();
        window.swap();
    }
}
