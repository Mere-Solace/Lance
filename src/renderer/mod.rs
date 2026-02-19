pub mod mesh;
pub mod shader;

use gl::types::*;
use glam::{Mat4, Vec3, Vec4};
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

/// Number of shadow cascade slices.
const NUM_CASCADES: usize = 3;

/// Camera-space depth split points (positive, metres).
/// Cascade i covers [CASCADE_SPLITS[i], CASCADE_SPLITS[i+1]).
const CASCADE_SPLITS: [f32; 4] = [0.1, 8.0, 25.0, 80.0];

/// How far behind each cascade to extend the light frustum to capture shadow casters.
const SHADOW_CASTER_REACH: f32 = 150.0;

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

        Self { fbo, texture, resolution }
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
    /// One shadow map per cascade.
    shadow_maps: Vec<ShadowMap>,
    /// Cached resolution to detect changes.
    shadow_resolution: u32,
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

        let shadow_resolution = 2048;
        let shadow_maps = (0..NUM_CASCADES).map(|_| ShadowMap::new(shadow_resolution)).collect();

        let mut viewport = [0i32; 4];
        unsafe {
            gl::GetIntegerv(gl::VIEWPORT, viewport.as_mut_ptr());
        }

        Self {
            shader,
            shadow_shader,
            shadow_maps,
            shadow_resolution,
            viewport_size: (viewport[2], viewport[3]),
        }
    }

    /// Compute a tight light-space VP matrix for cascade slice [near_dist, far_dist].
    ///
    /// Unprojects the 8 NDC corners of the cascade slice to world space, finds the minimal
    /// bounding sphere, and builds an orthographic shadow frustum around it. The sphere-based
    /// approach is rotation-invariant, preventing shadow shimmer when the camera rotates.
    fn cascade_matrix(
        light_dir: Vec3,
        view: &Mat4,
        proj: &Mat4,
        near_dist: f32,
        far_dist: f32,
    ) -> Mat4 {
        // Map camera-space depths to NDC z using the projection matrix.
        // For GL right-handed perspective: NDC_z = (P22 * z_view + P32) / (-z_view)
        // where z_view = -dist (negative, in front of camera).
        let p22 = proj.col(2).z; // -(far+near)/(far-near)
        let p32 = proj.col(3).z; // -2*far*near/(far-near)
        let ndc_z = |dist: f32| (p22 * (-dist) + p32) / dist;

        // Unproject the 8 corners of the cascade frustum slice to world space.
        let inv_vp = (*proj * *view).inverse();
        let mut corners = [Vec3::ZERO; 8];
        let nzs = [ndc_z(near_dist), ndc_z(far_dist)];
        let mut k = 0;
        for &nz in &nzs {
            for &nx in &[-1.0f32, 1.0] {
                for &ny in &[-1.0f32, 1.0] {
                    let h = inv_vp * Vec4::new(nx, ny, nz, 1.0);
                    corners[k] = h.truncate() / h.w;
                    k += 1;
                }
            }
        }

        // Bounding sphere of the 8 corners. Rounding radius up to the nearest whole unit
        // prevents sub-texel shadow shimmer as the camera translates.
        let centroid = corners.iter().fold(Vec3::ZERO, |a, &c| a + c) / 8.0;
        let radius_raw = corners.iter().map(|&c| (c - centroid).length()).fold(0.0f32, f32::max);
        let radius = (radius_raw + 1.0).ceil();

        // Position the shadow camera behind the scene along the light direction.
        let ld = light_dir.normalize();
        let up = if ld.y.abs() < 0.99 { Vec3::Y } else { Vec3::X };
        let eye = centroid - ld * SHADOW_CASTER_REACH;
        let light_view = Mat4::look_at_rh(eye, centroid, up);

        // Square orthographic frustum sized to the bounding sphere radius.
        let light_proj = Mat4::orthographic_rh_gl(
            -radius,
            radius,
            -radius,
            radius,
            0.1,
            SHADOW_CASTER_REACH + radius * 2.0,
        );

        light_proj * light_view
    }

    /// Extract the 6 Gribb-Hartmann frustum planes from a combined VP matrix.
    /// A point P is inside if dot(plane, P) >= 0 (unnormalised).
    fn frustum_planes(vp: &Mat4) -> [Vec4; 6] {
        // Extract matrix rows (each row is a dot product over homogeneous coords).
        let row = |i: usize| Vec4::new(vp.col(0)[i], vp.col(1)[i], vp.col(2)[i], vp.col(3)[i]);
        let r0 = row(0);
        let r1 = row(1);
        let r2 = row(2);
        let r3 = row(3);
        [
            r3 + r0, // left
            r3 - r0, // right
            r3 + r1, // bottom
            r3 - r1, // top
            r3 + r2, // near
            r3 - r2, // far
        ]
    }

    /// Returns true if a sphere (world-space center + radius) is fully outside any frustum plane.
    fn sphere_outside_frustum(center: Vec3, radius: f32, planes: &[Vec4; 6]) -> bool {
        for &p in planes {
            let dist = p.x * center.x + p.y * center.y + p.z * center.z + p.w;
            let len = Vec3::new(p.x, p.y, p.z).length();
            if dist < -radius * len {
                return true;
            }
        }
        false
    }

    /// Approximate world-space bounding sphere from a GlobalTransform matrix.
    /// Position = matrix translation; radius = max column scale × 2 (conservative).
    fn approx_bounding_sphere(gt: &GlobalTransform) -> (Vec3, f32) {
        let pos = gt.0.col(3).truncate();
        let sx = gt.0.col(0).truncate().length();
        let sy = gt.0.col(1).truncate().length();
        let sz = gt.0.col(2).truncate().length();
        let radius = sx.max(sy).max(sz) * 2.0;
        (pos, radius.max(0.5))
    }

    pub fn draw_scene(
        &mut self,
        world: &World,
        meshes: &MeshStore,
        view: &Mat4,
        proj: &Mat4,
        camera_pos: Vec3,
    ) {
        // Update cached viewport size.
        let mut viewport = [0i32; 4];
        unsafe {
            gl::GetIntegerv(gl::VIEWPORT, viewport.as_mut_ptr());
        }
        self.viewport_size = (viewport[2], viewport[3]);

        // --- Find directional light ---
        let mut dir_light_dir = Vec3::new(-0.5, -1.0, -0.3);
        let mut dir_light_color = Vec3::ONE;
        let mut dir_light_intensity: f32 = 1.0;
        let mut shadows_enabled = false;
        let mut shadow_resolution = self.shadow_resolution;

        for (_e, (dl,)) in world.query::<(&DirectionalLight,)>().iter() {
            dir_light_dir = dl.direction;
            dir_light_color = dl.color;
            dir_light_intensity = dl.intensity;
            shadow_resolution = dl.shadow_resolution;
            shadows_enabled = true;
            break; // first directional light only
        }

        // Recreate shadow maps if resolution changed.
        if shadow_resolution != self.shadow_resolution {
            self.shadow_maps =
                (0..NUM_CASCADES).map(|_| ShadowMap::new(shadow_resolution)).collect();
            self.shadow_resolution = shadow_resolution;
        }

        // Compute per-cascade light-space VP matrices.
        let mut cascade_matrices = [Mat4::IDENTITY; NUM_CASCADES];
        if shadows_enabled {
            for i in 0..NUM_CASCADES {
                cascade_matrices[i] = Self::cascade_matrix(
                    dir_light_dir,
                    view,
                    proj,
                    CASCADE_SPLITS[i],
                    CASCADE_SPLITS[i + 1],
                );
            }
        }

        // ============ PASS 1: Shadow maps (one per cascade) ============
        if shadows_enabled {
            unsafe {
                gl::Viewport(0, 0, self.shadow_resolution as i32, self.shadow_resolution as i32);
                gl::CullFace(gl::FRONT);
                gl::Enable(gl::CULL_FACE);
            }

            self.shadow_shader.bind();

            for c in 0..NUM_CASCADES {
                unsafe {
                    gl::BindFramebuffer(gl::FRAMEBUFFER, self.shadow_maps[c].fbo);
                    gl::Clear(gl::DEPTH_BUFFER_BIT);
                }

                self.shadow_shader.set_mat4("u_light_space", &cascade_matrices[c]);

                let planes = Self::frustum_planes(&cascade_matrices[c]);

                for (_entity, (gt, mesh_handle, hidden)) in
                    world.query::<(&GlobalTransform, &MeshHandle, Option<&Hidden>)>().iter()
                {
                    if hidden.is_some() {
                        continue;
                    }

                    // Frustum cull: skip entities outside this cascade's light frustum.
                    let (pos, radius) = Self::approx_bounding_sphere(gt);
                    if Self::sphere_outside_frustum(pos, radius, &planes) {
                        continue;
                    }

                    self.shadow_shader.set_mat4("u_model", &gt.0);
                    meshes.get(*mesh_handle).draw();
                }
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

        // Upload cascade light-space matrices
        for i in 0..NUM_CASCADES {
            self.shader
                .set_mat4(&format!("u_cascade_light_space[{}]", i), &cascade_matrices[i]);
        }

        // Bind cascade shadow maps to texture units 0–2
        unsafe {
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, self.shadow_maps[0].texture);
            gl::ActiveTexture(gl::TEXTURE1);
            gl::BindTexture(gl::TEXTURE_2D, self.shadow_maps[1].texture);
            gl::ActiveTexture(gl::TEXTURE2);
            gl::BindTexture(gl::TEXTURE_2D, self.shadow_maps[2].texture);
        }
        self.shader.set_int("u_shadow_map_0", 0);
        self.shader.set_int("u_shadow_map_1", 1);
        self.shader.set_int("u_shadow_map_2", 2);

        // Cascade split thresholds (camera depth at cascade boundaries)
        self.shader.set_float("u_cascade_splits[0]", CASCADE_SPLITS[1]);
        self.shader.set_float("u_cascade_splits[1]", CASCADE_SPLITS[2]);

        // --- Upload point lights ---
        let mut point_count = 0usize;
        for (_e, (lt, pl)) in world.query::<(&LocalTransform, &PointLight)>().iter() {
            if point_count >= MAX_POINT_LIGHTS {
                break;
            }
            self.shader.set_vec3(&format!("u_point_light_pos[{}]", point_count), lt.position);
            self.shader
                .set_vec3(&format!("u_point_light_color[{}]", point_count), pl.color);
            self.shader
                .set_float(&format!("u_point_light_intensity[{}]", point_count), pl.intensity);
            self.shader
                .set_float(&format!("u_point_light_constant[{}]", point_count), pl.constant);
            self.shader
                .set_float(&format!("u_point_light_linear[{}]", point_count), pl.linear);
            self.shader
                .set_float(&format!("u_point_light_quadratic[{}]", point_count), pl.quadratic);
            point_count += 1;
        }
        self.shader.set_int("u_num_point_lights", point_count as i32);

        // --- Upload spot lights ---
        let mut spot_count = 0usize;
        for (_e, (lt, sl)) in world.query::<(&LocalTransform, &SpotLight)>().iter() {
            if spot_count >= MAX_SPOT_LIGHTS {
                break;
            }
            self.shader.set_vec3(&format!("u_spot_light_pos[{}]", spot_count), lt.position);
            self.shader
                .set_vec3(&format!("u_spot_light_dir[{}]", spot_count), sl.direction);
            self.shader
                .set_vec3(&format!("u_spot_light_color[{}]", spot_count), sl.color);
            self.shader
                .set_float(&format!("u_spot_light_intensity[{}]", spot_count), sl.intensity);
            self.shader.set_float(
                &format!("u_spot_light_inner_cone[{}]", spot_count),
                sl.inner_cone,
            );
            self.shader.set_float(
                &format!("u_spot_light_outer_cone[{}]", spot_count),
                sl.outer_cone,
            );
            self.shader
                .set_float(&format!("u_spot_light_constant[{}]", spot_count), sl.constant);
            self.shader
                .set_float(&format!("u_spot_light_linear[{}]", spot_count), sl.linear);
            self.shader.set_float(
                &format!("u_spot_light_quadratic[{}]", spot_count),
                sl.quadratic,
            );
            spot_count += 1;
        }
        self.shader.set_int("u_num_spot_lights", spot_count as i32);

        // --- Draw entities ---
        for (_entity, (gt, mesh_handle, color, checker, hidden)) in world
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
            self.shader.set_mat4("u_model", &gt.0);
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
