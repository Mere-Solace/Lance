use glam::Vec3;
use hecs::Entity;

/// Linear velocity in world space.
pub struct Velocity(pub Vec3);

/// Per-entity acceleration (accumulated forces / mass).
pub struct Acceleration(pub Vec3);

/// Entity mass in kilograms.
#[allow(dead_code)]
pub struct Mass(pub f32);

/// Marker: entity is affected by gravity.
pub struct GravityAffected;

/// Collision shape attached to an entity.
#[allow(dead_code)]
pub enum Collider {
    Sphere { radius: f32 },
    Capsule { radius: f32, height: f32 },
    Plane { normal: Vec3, offset: f32 },
    Box { half_extents: Vec3 },
}

/// Marker: entity is immovable (infinite mass for collision response).
pub struct Static;

/// Restitution coefficient (bounciness). 0.0 = no bounce, 1.0 = perfect bounce.
pub struct Restitution(pub f32);

/// Surface friction coefficient. Higher values = more friction. 0.0 = ice, 1.0 = rubber.
/// Combined between contact pairs by averaging.
pub struct Friction(pub f32);

/// Velocity damping factor (air resistance / drag). Applied as vel *= (1 - drag * dt) each step.
/// 0.0 = no drag, higher values = faster deceleration.
pub struct Drag(pub f32);

/// Collision contact produced by the detection phase.
pub struct CollisionEvent {
    pub entity_a: Entity,
    pub entity_b: Entity,
    pub contact_normal: Vec3,
    pub penetration_depth: f32,
}

/// Marker: entity is touching the ground (set each physics frame).
pub struct Grounded;

/// Previous physics-step position, stored for render interpolation.
/// Updated at the start of each physics step; used by transform propagation
/// to lerp between prev and current position by the accumulator alpha.
pub struct PreviousPosition(pub Vec3);
