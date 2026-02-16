pub mod mesh;
pub mod shader;

use gl::types::*;
use glam::{Mat4, Vec3};
use hecs::World;
use mesh::Mesh;
use shader::ShaderProgram;

use crate::components::{
    Checkerboard, Color, DirectionalLight, GlobalTransform, Hidden, LocalTransform, MeshHandle,
    PointLight, SpotLight,
};

const VERT_SRC: &str = include_str!("../../shaders/cel.vert");
const FRAG_SRC: &str = include_str!("../../shaders/cel.frag");
const SHADOW_VERT_SRC: &str = include_str!("../../shaders/shadow.vert");
const SHADOW_FRAG_SRC: &str = include_str!("../../shaders/shadow.frag");

const FOG_COLOR: Vec3 = Vec3::new(0.1, 0.1, 0.15);

const MAX_POINT_LIGHTS: usize = 8;
const MAX_SPOT_LIGHTS: usize = 4;

/// Shadow map framebuffer object.
struct ShadowMap {
    fbo: GLuint,
    texture: GLuint,
    resolution: u32,
}

impl ShadowMap {
    fn new(resolution: u32) -> Self {
        let mut fbo: GLuint = 0;
        let mut texture: GLuint = 0;

        unsafe {
            gl::GenFramebuffers(1, &mut fbo);
            gl::GenTextures(1, &mut texture);

            gl::BindTexture(gl::TEXTURE_2D, texture);
            gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::DEPTH_COMPONENT as i32,
                resolution as i32,
                resolution as i32,
                0,
                gl::DEPTH_COMPONENT,
                gl::FLOAT,
                std::ptr::null(),
            );
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
            gl::TexParameteri(
                gl::TEXTURE_2D,
                gl::TEXTURE_WRAP_S,
                gl::CLAMP_TO_BORDER as i32,
            );
            gl::TexParameteri(
                gl::TEXTURE_2D,
                gl::TEXTURE_WRAP_T,
                gl::CLAMP_TO_BORDER as i32,
            );
            let border_color: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
            gl::TexParameterfv(
                gl::TEXTURE_2D,
                gl::TEXTURE_BORDER_COLOR,
                border_color.as_ptr(),
            );

            gl::BindFramebuffer(gl::FRAMEBUFFER, fbo);
            gl::FramebufferTexture2D(
                gl::FRAMEBUFFER,
                gl::DEPTH_ATTACHMENT,
                gl::TEXTURE_2D,
                texture,
                0,
            );
            gl::DrawBuffer(gl::NONE);
            gl::ReadBuffer(gl::NONE);
            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
        }

        Self {
            fbo,
            texture,
            resolution,
        }
    }
}

impl Drop for ShadowMap {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteFramebuffers(1, &self.fbo);
            gl::DeleteTextures(1, &self.texture);
        }
    }
}

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
    shadow_shader: ShaderProgram,
    shadow_map: ShadowMap,
    viewport_size: (i32, i32),
}

impl Renderer {
    pub fn init() -> Self {
        unsafe {
            gl::Enable(gl::DEPTH_TEST);
            gl::ClearColor(FOG_COLOR.x, FOG_COLOR.y, FOG_COLOR.z, 1.0);
        }

        let shader =
            ShaderProgram::from_sources(VERT_SRC, FRAG_SRC).expect("Failed to compile cel shaders");
        let shadow_shader = ShaderProgram::from_sources(SHADOW_VERT_SRC, SHADOW_FRAG_SRC)
            .expect("Failed to compile shadow shaders");

        // Default shadow map at 2048 — will be recreated if a DirectionalLight requests different
        let shadow_map = ShadowMap::new(2048);

        // Query current viewport for restore after shadow pass
        let mut viewport = [0i32; 4];
        unsafe {
            gl::GetIntegerv(gl::VIEWPORT, viewport.as_mut_ptr());
        }

        Self {
            shader,
            shadow_shader,
            shadow_map,
            viewport_size: (viewport[2], viewport[3]),
        }
    }

    /// Compute a light-space (view * projection) matrix for a directional shadow caster
    /// centered around the given focus position.
    fn light_space_matrix(dir: Vec3, extent: f32, focus: Vec3) -> Mat4 {
        let light_dir = dir.normalize();
        let light_pos = focus - light_dir * extent;
        let light_view = Mat4::look_at_rh(light_pos, focus, Vec3::Y);
        let light_proj = Mat4::orthographic_rh_gl(
            -extent, extent, -extent, extent, 0.1, extent * 2.5,
        );
        light_proj * light_view
    }

    pub fn draw_scene(
        &mut self,
        world: &World,
        meshes: &MeshStore,
        view: &Mat4,
        proj: &Mat4,
        camera_pos: Vec3,
    ) {
        // Update viewport size
        let mut viewport = [0i32; 4];
        unsafe {
            gl::GetIntegerv(gl::VIEWPORT, viewport.as_mut_ptr());
        }
        self.viewport_size = (viewport[2], viewport[3]);

        // --- Find directional light ---
        let mut dir_light_dir = Vec3::new(-0.5, -1.0, -0.3);
        let mut dir_light_color = Vec3::ONE;
        let mut dir_light_intensity: f32 = 1.0;
        let mut shadow_extent: f32 = 40.0;
        let mut shadows_enabled = false;

        for (_e, (dl,)) in world.query::<(&DirectionalLight,)>().iter() {
            dir_light_dir = dl.direction;
            dir_light_color = dl.color;
            dir_light_intensity = dl.intensity;
            shadow_extent = dl.shadow_extent;
            shadows_enabled = true;

            // Recreate shadow map if resolution changed
            if dl.shadow_resolution != self.shadow_map.resolution {
                self.shadow_map = ShadowMap::new(dl.shadow_resolution);
            }
            break; // Use first directional light only
        }

        let light_space = Self::light_space_matrix(dir_light_dir, shadow_extent, camera_pos);

        // ============ PASS 1: Shadow map ============
        if shadows_enabled {
            unsafe {
                gl::Viewport(
                    0,
                    0,
                    self.shadow_map.resolution as i32,
                    self.shadow_map.resolution as i32,
                );
                gl::BindFramebuffer(gl::FRAMEBUFFER, self.shadow_map.fbo);
                gl::Clear(gl::DEPTH_BUFFER_BIT);
                // Reduce shadow acne with front-face culling during shadow pass
                gl::CullFace(gl::FRONT);
                gl::Enable(gl::CULL_FACE);
            }

            self.shadow_shader.bind();
            self.shadow_shader.set_mat4("u_light_space", &light_space);

            for (_entity, (global_transform, mesh_handle, hidden)) in world
                .query::<(&GlobalTransform, &MeshHandle, Option<&Hidden>)>()
                .iter()
            {
                if hidden.is_some() {
                    continue;
                }
                self.shadow_shader
                    .set_mat4("u_model", &global_transform.0);
                meshes.get(*mesh_handle).draw();
            }

            unsafe {
                gl::Disable(gl::CULL_FACE);
                gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
                gl::Viewport(0, 0, self.viewport_size.0, self.viewport_size.1);
            }
        }

        // ============ PASS 2: Scene rendering ============
        unsafe {
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
        }

        self.shader.bind();
        self.shader.set_mat4("u_view", view);
        self.shader.set_mat4("u_projection", proj);
        self.shader.set_mat4("u_light_space", &light_space);
        self.shader.set_vec3("u_camera_pos", camera_pos);
        self.shader.set_vec3("u_ambient_color", Vec3::new(0.15, 0.15, 0.15));
        self.shader.set_vec3("u_fog_color", FOG_COLOR);
        self.shader.set_float("u_fog_start", 50.0);
        self.shader.set_float("u_fog_end", 300.0);

        // Directional light uniforms
        self.shader.set_vec3("u_dir_light_dir", dir_light_dir);
        self.shader.set_vec3("u_dir_light_color", dir_light_color);
        self.shader.set_float("u_dir_light_intensity", dir_light_intensity);
        self.shader.set_int("u_shadows_enabled", if shadows_enabled { 1 } else { 0 });

        // Bind shadow map to texture unit 0
        unsafe {
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, self.shadow_map.texture);
        }
        self.shader.set_int("u_shadow_map", 0);

        // --- Upload point lights ---
        let mut point_count = 0usize;
        for (_e, (lt, pl)) in world.query::<(&LocalTransform, &PointLight)>().iter() {
            if point_count >= MAX_POINT_LIGHTS {
                break;
            }
            self.shader.set_vec3(
                &format!("u_point_light_pos[{}]", point_count),
                lt.position,
            );
            self.shader.set_vec3(
                &format!("u_point_light_color[{}]", point_count),
                pl.color,
            );
            self.shader.set_float(
                &format!("u_point_light_intensity[{}]", point_count),
                pl.intensity,
            );
            self.shader.set_float(
                &format!("u_point_light_constant[{}]", point_count),
                pl.constant,
            );
            self.shader.set_float(
                &format!("u_point_light_linear[{}]", point_count),
                pl.linear,
            );
            self.shader.set_float(
                &format!("u_point_light_quadratic[{}]", point_count),
                pl.quadratic,
            );
            point_count += 1;
        }
        self.shader.set_int("u_num_point_lights", point_count as i32);

        // --- Upload spot lights ---
        let mut spot_count = 0usize;
        for (_e, (lt, sl)) in world.query::<(&LocalTransform, &SpotLight)>().iter() {
            if spot_count >= MAX_SPOT_LIGHTS {
                break;
            }
            self.shader.set_vec3(
                &format!("u_spot_light_pos[{}]", spot_count),
                lt.position,
            );
            self.shader.set_vec3(
                &format!("u_spot_light_dir[{}]", spot_count),
                sl.direction,
            );
            self.shader.set_vec3(
                &format!("u_spot_light_color[{}]", spot_count),
                sl.color,
            );
            self.shader.set_float(
                &format!("u_spot_light_intensity[{}]", spot_count),
                sl.intensity,
            );
            self.shader.set_float(
                &format!("u_spot_light_inner_cone[{}]", spot_count),
                sl.inner_cone,
            );
            self.shader.set_float(
                &format!("u_spot_light_outer_cone[{}]", spot_count),
                sl.outer_cone,
            );
            self.shader.set_float(
                &format!("u_spot_light_constant[{}]", spot_count),
                sl.constant,
            );
            self.shader.set_float(
                &format!("u_spot_light_linear[{}]", spot_count),
                sl.linear,
            );
            self.shader.set_float(
                &format!("u_spot_light_quadratic[{}]", spot_count),
                sl.quadratic,
            );
            spot_count += 1;
        }
        self.shader.set_int("u_num_spot_lights", spot_count as i32);

        // --- Draw entities ---
        for (_entity, (global_transform, mesh_handle, color, checker, hidden)) in world
            .query::<(
                &GlobalTransform,
                &MeshHandle,
                &Color,
                Option<&Checkerboard>,
                Option<&Hidden>,
            )>()
            .iter()
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
