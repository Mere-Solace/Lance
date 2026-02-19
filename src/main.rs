mod app;
mod camera;
mod components;
mod engine;
mod fsm;
mod recording;
mod renderer;
mod scene;
mod systems;
mod ui;

use app::GameApp;
use clap::Parser;
use engine::window::GameWindow;
use hecs::World;
use scene::test_scene::load_test_scene;

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

    let mut world = World::new();
    let (meshes, player_entity) = load_test_scene(&mut world);

    let mut app = GameApp::new(world, meshes, player_entity, args.record, &window);
    app.run(&sdl, &window);
}
