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
    /// Accumulates real time between 1-second FPS refreshes.
    fps_timer: f32,
    /// Last computed SMA FPS, updated once per second.
    displayed_fps: f32,
}

impl DebugHud {
    pub fn new() -> Self {
        Self {
            visible: false,
            fps_ring: [0.0; FPS_SAMPLES],
            fps_index: 0,
            fps_count: 0,
            fps_timer: 0.0,
            displayed_fps: 0.0,
        }
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Push a frame delta into the rolling buffer and refresh the displayed FPS
    /// once per second. Call every frame when visible.
    pub fn update(&mut self, dt: f32) {
        self.fps_ring[self.fps_index] = dt;
        self.fps_index = (self.fps_index + 1) % FPS_SAMPLES;
        if self.fps_count < FPS_SAMPLES {
            self.fps_count += 1;
        }

        self.fps_timer += dt;
        if self.fps_timer >= 1.0 {
            self.fps_timer = 0.0;
            if self.fps_count > 0 {
                let sum: f32 = self.fps_ring[..self.fps_count].iter().sum();
                self.displayed_fps = self.fps_count as f32 / sum;
            }
        }
    }

    /// Render HUD lines at the top-left of the screen.
    ///
    /// `pos` — world position to display. In Player mode pass the player body
    /// position; in Fly mode pass `camera.position`.
    ///
    /// Caller must set up the orthographic projection and GL blend state.
    pub fn draw(&self, text_renderer: &mut TextRenderer, pos: Vec3, camera: &Camera, projection: &Mat4) {
        // Yaw: 0 = +X axis, counterclockwise increases, wraps [0, 360).
        // camera.yaw is stored in degrees; negate so CCW (left turn) increases.
        let yaw = (-camera.yaw).rem_euclid(360.0);
        // Pitch is stored in degrees and already clamped to [-89, 89] by camera.look().
        let pitch = camera.pitch;

        let x = HUD_MARGIN;
        let y = HUD_MARGIN;

        let line0 = format!("FPS: {:.0}", self.displayed_fps);
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
