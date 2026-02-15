use sdl2::event::Event;
use sdl2::keyboard::Scancode;
use sdl2::EventPump;
use std::collections::HashSet;

pub struct InputState {
    pub keys: HashSet<Scancode>,
    pub mouse_dx: f32,
    pub mouse_dy: f32,
    pub quit: bool,
}

impl InputState {
    pub fn new() -> Self {
        Self {
            keys: HashSet::new(),
            mouse_dx: 0.0,
            mouse_dy: 0.0,
            quit: false,
        }
    }

    pub fn update(&mut self, event_pump: &mut EventPump) {
        self.mouse_dx = 0.0;
        self.mouse_dy = 0.0;

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => self.quit = true,
                Event::KeyDown {
                    scancode: Some(Scancode::Escape),
                    ..
                } => self.quit = true,
                Event::KeyDown {
                    scancode: Some(sc), ..
                } => {
                    self.keys.insert(sc);
                }
                Event::KeyUp {
                    scancode: Some(sc), ..
                } => {
                    self.keys.remove(&sc);
                }
                Event::MouseMotion { xrel, yrel, .. } => {
                    self.mouse_dx += xrel as f32;
                    self.mouse_dy += yrel as f32;
                }
                _ => {}
            }
        }
    }

    pub fn is_key_held(&self, sc: Scancode) -> bool {
        self.keys.contains(&sc)
    }
}
