use glam::{Mat4, Quat, Vec3};
use hecs::{Entity, World};

/// Spatial transform with position, rotation, and scale (local space).
pub struct LocalTransform {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl LocalTransform {
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

/// Computed world-space transform matrix, updated by the propagation system.
pub struct GlobalTransform(pub Mat4);

/// Points to the parent entity in the transform hierarchy.
#[allow(dead_code)]
pub struct Parent(pub Entity);

/// Lists child entities in the transform hierarchy.
pub struct Children(pub Vec<Entity>);

/// Attach `child` under `parent` in the transform hierarchy.
pub fn add_child(world: &mut World, parent: Entity, child: Entity) {
    let has_children = world.get::<&Children>(parent).is_ok();
    if has_children {
        let mut children = world.get::<&mut Children>(parent).unwrap();
        if !children.0.contains(&child) {
            children.0.push(child);
        }
    } else {
        world.insert_one(parent, Children(vec![child])).unwrap();
    }

    let _ = world.insert_one(child, Parent(parent));
}

/// Detach `child` from `parent` in the transform hierarchy.
#[allow(dead_code)]
pub fn remove_child(world: &mut World, parent: Entity, child: Entity) {
    if let Ok(mut children) = world.get::<&mut Children>(parent) {
        children.0.retain(|&e| e != child);
    }
    let _ = world.remove_one::<Parent>(child);
}

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

/// Index into the MeshStore resource.
#[derive(Clone, Copy)]
pub struct MeshHandle(pub usize);

/// RGB color applied to an entity for rendering.
pub struct Color(pub Vec3);

/// Marker: this entity is the player.
pub struct Player;

/// Marker: entity is touching the ground (set each physics frame).
pub struct Grounded;
