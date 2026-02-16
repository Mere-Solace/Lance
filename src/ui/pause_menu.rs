use gl::types::*;
use glam::{Mat4, Vec3};
use std::mem;

use crate::engine::input::InputEvent;
use crate::renderer::shader::ShaderProgram;
use crate::ui::text::TextRenderer;
use sdl2::keyboard::Scancode;

const QUAD_VERT_SRC: &str = include_str!("../../shaders/quad.vert");
const QUAD_FRAG_SRC: &str = include_str!("../../shaders/quad.frag");

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum GameState {
    Running,
    Paused,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PauseAction {
    None,
    Resume,
    Quit,
}

const MENU_ITEMS: &[&str] = &["Resume", "Quit"];

pub struct PauseMenu {
    shader: ShaderProgram,
    vao: GLuint,
    vbo: GLuint,
    selected: usize,
}

impl PauseMenu {
    pub fn new() -> Self {
        let shader = ShaderProgram::from_sources(QUAD_VERT_SRC, QUAD_FRAG_SRC)
            .expect("Failed to compile quad shaders");

        let mut vao: GLuint = 0;
        let mut vbo: GLuint = 0;

        unsafe {
            gl::GenVertexArrays(1, &mut vao);
            gl::GenBuffers(1, &mut vbo);

            gl::BindVertexArray(vao);
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
            // Enough for a single fullscreen quad (6 vertices * 2 floats)
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (12 * mem::size_of::<f32>()) as GLsizeiptr,
                std::ptr::null(),
                gl::DYNAMIC_DRAW,
            );

            let stride = (2 * mem::size_of::<f32>()) as GLsizei;
            gl::EnableVertexAttribArray(0);
            gl::VertexAttribPointer(0, 2, gl::FLOAT, gl::FALSE, stride, std::ptr::null());

            gl::BindVertexArray(0);
        }

        Self {
            shader,
            vao,
            vbo,
            selected: 0,
        }
    }

    pub fn handle_input(&mut self, events: &[InputEvent]) -> PauseAction {
        for event in events {
            match event {
                InputEvent::KeyPressed(Scancode::Up | Scancode::W) => {
                    if self.selected > 0 {
                        self.selected -= 1;
                    } else {
                        self.selected = MENU_ITEMS.len() - 1;
                    }
                }
                InputEvent::KeyPressed(Scancode::Down | Scancode::S) => {
                    self.selected = (self.selected + 1) % MENU_ITEMS.len();
                }
                InputEvent::KeyPressed(Scancode::Return | Scancode::KpEnter) => {
                    return match self.selected {
                        0 => PauseAction::Resume,
                        1 => PauseAction::Quit,
                        _ => PauseAction::None,
                    };
                }
                InputEvent::KeyPressed(Scancode::Escape) => {
                    return PauseAction::Resume;
                }
                _ => {}
            }
        }
        PauseAction::None
    }

    pub fn draw(
        &mut self,
        text_renderer: &mut TextRenderer,
        width: f32,
        height: f32,
        projection: &Mat4,
    ) {
        // Draw semi-transparent dark overlay
        self.draw_quad(0.0, 0.0, width, height, [0.0, 0.0, 0.0, 0.6], projection);

        let title_scale = 4.0;
        let item_scale = 2.5;
        let title = "PAUSED";
        let title_w = text_renderer.measure_text(title, title_scale);
        let title_x = (width - title_w) / 2.0;
        let title_y = height * 0.30;

        text_renderer.draw_text(title, title_x, title_y, title_scale, Vec3::ONE, projection);

        let item_start_y = height * 0.48;
        let item_spacing = 40.0;

        for (i, item) in MENU_ITEMS.iter().enumerate() {
            let item_w = text_renderer.measure_text(item, item_scale);
            let item_x = (width - item_w) / 2.0;
            let item_y = item_start_y + i as f32 * item_spacing;

            let color = if i == self.selected {
                Vec3::new(1.0, 0.9, 0.2) // yellow for selected
            } else {
                Vec3::new(0.6, 0.6, 0.6) // gray for unselected
            };

            // Draw selection indicator
            if i == self.selected {
                let arrow = ">";
                let arrow_w = text_renderer.measure_text(arrow, item_scale);
                text_renderer.draw_text(
                    arrow,
                    item_x - arrow_w - 8.0,
                    item_y,
                    item_scale,
                    color,
                    projection,
                );
            }

            text_renderer.draw_text(item, item_x, item_y, item_scale, color, projection);
        }
    }

    fn draw_quad(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        color: [f32; 4],
        projection: &Mat4,
    ) {
        #[rustfmt::skip]
        let vertices: [f32; 12] = [
            x,     y,
            x + w, y,
            x + w, y + h,
            x,     y,
            x + w, y + h,
            x,     y + h,
        ];

        unsafe {
            self.shader.bind();
            self.shader.set_mat4("u_projection", projection);
            self.shader
                .set_vec4("u_color", color);

            gl::BindVertexArray(self.vao);
            gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo);
            gl::BufferSubData(
                gl::ARRAY_BUFFER,
                0,
                mem::size_of_val(&vertices) as GLsizeiptr,
                vertices.as_ptr() as *const _,
            );

            gl::DrawArrays(gl::TRIANGLES, 0, 6);
            gl::BindVertexArray(0);
        }
    }

    pub fn reset_selection(&mut self) {
        self.selected = 0;
    }
}

impl Drop for PauseMenu {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteVertexArrays(1, &self.vao);
            gl::DeleteBuffers(1, &self.vbo);
        }
    }
}
