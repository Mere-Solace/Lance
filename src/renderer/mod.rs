pub mod mesh;
pub mod shader;

use glam::{Mat4, Vec3};
use mesh::{create_ground_plane, create_sphere, Mesh};
use shader::ShaderProgram;

const VERT_SRC: &str = include_str!("../../shaders/cel.vert");
const FRAG_SRC: &str = include_str!("../../shaders/cel.frag");

const FOG_COLOR: Vec3 = Vec3::new(0.1, 0.1, 0.15);
const LIGHT_DIR: Vec3 = Vec3::new(-0.5, -1.0, -0.3);

pub struct Renderer {
    shader: ShaderProgram,
    sphere: Mesh,
    ground: Mesh,
}

impl Renderer {
    pub fn init() -> Self {
        unsafe {
            gl::Enable(gl::DEPTH_TEST);
            gl::ClearColor(FOG_COLOR.x, FOG_COLOR.y, FOG_COLOR.z, 1.0);
        }

        let shader =
            ShaderProgram::from_sources(VERT_SRC, FRAG_SRC).expect("Failed to compile shaders");
        let sphere = create_sphere(1.0, 16, 32);
        let ground = create_ground_plane(500.0);

        Self {
            shader,
            sphere,
            ground,
        }
    }

    pub fn draw_scene(&mut self, view: &Mat4, proj: &Mat4, camera_pos: Vec3) {
        unsafe {
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
        }

        self.shader.bind();
        self.shader.set_mat4("u_view", view);
        self.shader.set_mat4("u_projection", proj);
        self.shader.set_vec3("u_light_dir", LIGHT_DIR);
        self.shader.set_vec3("u_camera_pos", camera_pos);
        self.shader.set_vec3("u_ambient_color", Vec3::new(0.15, 0.15, 0.15));
        self.shader.set_vec3("u_fog_color", FOG_COLOR);
        self.shader.set_float("u_fog_start", 50.0);
        self.shader.set_float("u_fog_end", 300.0);

        // Ground plane - green
        let model = Mat4::IDENTITY;
        self.shader.set_mat4("u_model", &model);
        self.shader.set_vec3("u_object_color", Vec3::new(0.3, 0.6, 0.2));
        self.ground.draw();

        // Sphere - red, floating at (0, 2, 0)
        let model = Mat4::from_translation(Vec3::new(0.0, 2.0, 0.0));
        self.shader.set_mat4("u_model", &model);
        self.shader.set_vec3("u_object_color", Vec3::new(0.8, 0.2, 0.15));
        self.sphere.draw();
    }
}
