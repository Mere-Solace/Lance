use glam::{Mat4, Quat, Vec3};

/// Spatial transform with position, rotation, and scale.
pub struct Transform {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Transform {
    pub fn new(position: Vec3) -> Self {
        Self {
            position,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }

    pub fn matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.position)
    }
}

/// Index into the MeshStore resource.
#[derive(Clone, Copy)]
pub struct MeshHandle(pub usize);

/// RGB color applied to an entity for rendering.
pub struct Color(pub Vec3);
