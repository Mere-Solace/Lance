pub mod mesh;
pub mod shader;

use glam::{Mat4, Vec3};
use hecs::World;
use mesh::Mesh;
use shader::ShaderProgram;

use crate::components::{Checkerboard, Color, GlobalTransform, Hidden, MeshHandle};

const VERT_SRC: &str = include_str!("../../shaders/cel.vert");
const FRAG_SRC: &str = include_str!("../../shaders/cel.frag");

const FOG_COLOR: Vec3 = Vec3::new(0.1, 0.1, 0.15);
const LIGHT_DIR: Vec3 = Vec3::new(-0.5, -1.0, -0.3);

/// Holds all loaded meshes. Entities reference meshes by MeshHandle index.
pub struct MeshStore {
    meshes: Vec<Mesh>,
}

impl MeshStore {
    pub fn new() -> Self {
        Self { meshes: Vec::new() }
    }

    pub fn add(&mut self, mesh: Mesh) -> MeshHandle {
        let handle = MeshHandle(self.meshes.len());
        self.meshes.push(mesh);
        handle
    }

    pub fn get(&self, handle: MeshHandle) -> &Mesh {
        &self.meshes[handle.0]
    }
}

pub struct Renderer {
    shader: ShaderProgram,
}

impl Renderer {
    pub fn init() -> Self {
        unsafe {
            gl::Enable(gl::DEPTH_TEST);
            gl::ClearColor(FOG_COLOR.x, FOG_COLOR.y, FOG_COLOR.z, 1.0);
        }

        let shader =
            ShaderProgram::from_sources(VERT_SRC, FRAG_SRC).expect("Failed to compile shaders");

        Self { shader }
    }

    pub fn draw_scene(
        &mut self,
        world: &World,
        meshes: &MeshStore,
        view: &Mat4,
        proj: &Mat4,
        camera_pos: Vec3,
    ) {
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

        for (_entity, (global_transform, mesh_handle, color, checker, hidden)) in
            world.query::<(&GlobalTransform, &MeshHandle, &Color, Option<&Checkerboard>, Option<&Hidden>)>().iter()
        {
            if hidden.is_some() {
                continue;
            }
            self.shader.set_mat4("u_model", &global_transform.0);
            self.shader.set_vec3("u_object_color", color.0);
            if let Some(checker) = checker {
                self.shader.set_int("u_checkerboard", 1);
                self.shader.set_vec3("u_object_color_2", checker.0);
            } else {
                self.shader.set_int("u_checkerboard", 0);
            }
            meshes.get(*mesh_handle).draw();
        }
    }
}
