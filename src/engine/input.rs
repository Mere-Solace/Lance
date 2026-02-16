use sdl2::event::Event;
use sdl2::keyboard::Scancode;
use sdl2::mouse::MouseButton;
use sdl2::EventPump;
use std::collections::HashSet;

#[allow(dead_code)]
pub enum InputEvent {
    KeyPressed(Scancode),
    KeyReleased(Scancode),
    MouseButtonPressed(MouseButton),
    MouseButtonReleased(MouseButton),
    MouseMotion { dx: f32, dy: f32 },
    Quit,
}

pub struct InputState {
    pub keys: HashSet<Scancode>,
    pub mouse_buttons: HashSet<MouseButton>,
    pub mouse_dx: f32,
    pub mouse_dy: f32,
    pub events: Vec<InputEvent>,
}

impl InputState {
    pub fn new() -> Self {
        Self {
            keys: HashSet::new(),
            mouse_buttons: HashSet::new(),
            mouse_dx: 0.0,
            mouse_dy: 0.0,
            events: Vec::new(),
        }
    }

    pub fn update(&mut self, event_pump: &mut EventPump) {
        self.mouse_dx = 0.0;
        self.mouse_dy = 0.0;
        self.events.clear();

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => {
                    self.events.push(InputEvent::Quit);
                }
                Event::KeyDown {
                    scancode: Some(sc), ..
                } => {
                    if self.keys.insert(sc) {
                        self.events.push(InputEvent::KeyPressed(sc));
                    }
                }
                Event::KeyUp {
                    scancode: Some(sc), ..
                } => {
                    self.keys.remove(&sc);
                    self.events.push(InputEvent::KeyReleased(sc));
                }
                Event::MouseButtonDown { mouse_btn, .. } => {
                    if self.mouse_buttons.insert(mouse_btn) {
                        self.events.push(InputEvent::MouseButtonPressed(mouse_btn));
                    }
                }
                Event::MouseButtonUp { mouse_btn, .. } => {
                    self.mouse_buttons.remove(&mouse_btn);
                    self.events.push(InputEvent::MouseButtonReleased(mouse_btn));
                }
                Event::MouseMotion { xrel, yrel, .. } => {
                    let dx = xrel as f32;
                    let dy = yrel as f32;
                    self.mouse_dx += dx;
                    self.mouse_dy += dy;
                    self.events.push(InputEvent::MouseMotion { dx, dy });
                }
                _ => {}
            }
        }
    }

    pub fn is_key_held(&self, sc: Scancode) -> bool {
        self.keys.contains(&sc)
    }

    pub fn is_mouse_button_held(&self, btn: MouseButton) -> bool {
        self.mouse_buttons.contains(&btn)
    }

    pub fn should_quit(&self) -> bool {
        self.events
            .iter()
            .any(|e| matches!(e, InputEvent::Quit))
    }
}
