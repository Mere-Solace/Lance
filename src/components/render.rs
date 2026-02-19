use glam::Vec3;

/// Index into the MeshStore resource.
#[derive(Clone, Copy)]
pub struct MeshHandle(pub usize);

/// RGB color applied to an entity for rendering.
pub struct Color(pub Vec3);

/// Checkerboard pattern using primary Color and this secondary color.
pub struct Checkerboard(pub Vec3);

/// Marker: entity is hidden from rendering but still participates in physics/collision.
pub struct Hidden;
