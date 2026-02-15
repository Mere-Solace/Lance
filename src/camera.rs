use glam::{Mat4, Vec3};
use sdl2::keyboard::Scancode;

use crate::engine::input::InputState;

pub struct Camera {
    pub position: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub speed: f32,
    pub sensitivity: f32,
    pub fov: f32,
}

impl Camera {
    pub fn new() -> Self {
        Self {
            position: Vec3::new(0.0, 0.0, 3.0),
            yaw: -90.0_f32,
            pitch: 0.0,
            speed: 5.0,
            sensitivity: 0.1,
            fov: 45.0,
        }
    }

    pub fn look(&mut self, mouse_dx: f32, mouse_dy: f32) {
        self.yaw += mouse_dx * self.sensitivity;
        self.pitch -= mouse_dy * self.sensitivity;
        self.pitch = self.pitch.clamp(-89.0, 89.0);
    }

    pub fn move_wasd(&mut self, input: &InputState, dt: f32) {
        let front = self.front();
        let right = front.cross(Vec3::Y).normalize();
        let velocity = self.speed * dt;

        if input.is_key_held(Scancode::W) {
            self.position += front * velocity;
        }
        if input.is_key_held(Scancode::S) {
            self.position -= front * velocity;
        }
        if input.is_key_held(Scancode::A) {
            self.position -= right * velocity;
        }
        if input.is_key_held(Scancode::D) {
            self.position += right * velocity;
        }
    }

    pub fn front(&self) -> Vec3 {
        let yaw_rad = self.yaw.to_radians();
        let pitch_rad = self.pitch.to_radians();
        Vec3::new(
            yaw_rad.cos() * pitch_rad.cos(),
            pitch_rad.sin(),
            yaw_rad.sin() * pitch_rad.cos(),
        )
        .normalize()
    }

    pub fn view_matrix(&self) -> Mat4 {
        let front = self.front();
        Mat4::look_at_rh(self.position, self.position + front, Vec3::Y)
    }

    pub fn projection_matrix(&self, aspect: f32) -> Mat4 {
        Mat4::perspective_rh_gl(self.fov.to_radians(), aspect, 0.1, 100.0)
    }
}
