use glam::{Mat4, Vec3};

use crate::camera::Camera;
use crate::ui::text::TextRenderer;

const HUD_SCALE: f32 = 2.0;
const HUD_MARGIN: f32 = 8.0;
// 8px glyph height * scale + 4px padding
const LINE_HEIGHT: f32 = 8.0 * HUD_SCALE + 4.0;
const HUD_COLOR: Vec3 = Vec3::new(1.0, 1.0, 0.0);

const FPS_SAMPLES: usize = 60;

pub struct DebugHud {
    visible: bool,
    fps_ring: [f32; FPS_SAMPLES],
    fps_index: usize,
    fps_count: usize,
}

impl DebugHud {
    pub fn new() -> Self {
        Self {
            visible: false,
            fps_ring: [0.0; FPS_SAMPLES],
            fps_index: 0,
            fps_count: 0,
        }
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Push a frame delta into the rolling FPS buffer. Call every frame when visible.
    pub fn update(&mut self, dt: f32) {
        self.fps_ring[self.fps_index] = dt;
        self.fps_index = (self.fps_index + 1) % FPS_SAMPLES;
        if self.fps_count < FPS_SAMPLES {
            self.fps_count += 1;
        }
    }

    /// Render HUD lines at the top-left of the screen. Caller must set up the
    /// orthographic projection and GL blend state before calling.
    pub fn draw(&self, text_renderer: &mut TextRenderer, camera: &Camera, projection: &Mat4) {
        let fps = if self.fps_count == 0 {
            0.0
        } else {
            let sum: f32 = self.fps_ring[..self.fps_count].iter().sum();
            self.fps_count as f32 / sum
        };

        let pos = camera.position;
        let yaw = camera.yaw.to_degrees();
        let pitch = camera.pitch.to_degrees();

        let x = HUD_MARGIN;
        let y = HUD_MARGIN;

        let line0 = format!("FPS: {:.0}", fps);
        let line1 = format!("Pos: {:.2} {:.2} {:.2}", pos.x, pos.y, pos.z);
        let line2 = format!("Yaw: {:.1}  Pitch: {:.1}", yaw, pitch);

        text_renderer.draw_text(&line0, x, y, HUD_SCALE, HUD_COLOR, projection);
        text_renderer.draw_text(&line1, x, y + LINE_HEIGHT, HUD_SCALE, HUD_COLOR, projection);
        text_renderer.draw_text(
            &line2,
            x,
            y + LINE_HEIGHT * 2.0,
            HUD_SCALE,
            HUD_COLOR,
            projection,
        );
    }
}
