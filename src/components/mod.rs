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

/// Index into the MeshStore resource.
#[derive(Clone, Copy)]
pub struct MeshHandle(pub usize);

/// RGB color applied to an entity for rendering.
pub struct Color(pub Vec3);

/// Marker: this entity is the player.
pub struct Player;

/// Marker: entity is touching the ground (set each physics frame).
pub struct Grounded;

/// Checkerboard pattern using primary Color and this secondary color.
pub struct Checkerboard(pub Vec3);

/// Marker: entity is hidden from rendering but still participates in physics/collision.
pub struct Hidden;

/// Previous physics-step position, stored for render interpolation.
/// Updated at the start of each physics step; used by transform propagation
/// to lerp between prev and current position by the accumulator alpha.
pub struct PreviousPosition(pub Vec3);

/// Marker: entities with the same owner Entity skip collision with each other.
/// Attach to all body parts of a character (torso, head, limbs) with the root entity as owner.
#[derive(Clone, Copy)]
pub struct NoSelfCollision(pub Entity);

/// Marker: entity can be grabbed by the player.
pub struct Grabbable;

/// Marker: entity is currently held (skip physics/collision).
pub struct Held;

/// State for the grab/throw system, attached to the player entity.
pub struct GrabState {
    pub held_entity: Option<Entity>,
    pub wind_up_time: f32,
    pub is_winding: bool,
    /// Player-local rotation of the held entity (rotates with player via parenting).
    pub held_rotation: Quat,
    /// Previous frame's world position of the held entity (for velocity tracking).
    pub prev_world_pos: Vec3,
    /// Smoothed world-space velocity of the held entity.
    pub held_velocity: Vec3,
}

impl GrabState {
    pub fn new() -> Self {
        Self {
            held_entity: None,
            wind_up_time: 0.0,
            is_winding: false,
            held_rotation: Quat::IDENTITY,
            prev_world_pos: Vec3::ZERO,
            held_velocity: Vec3::ZERO,
        }
    }
}

/// Whether the sword is sheathed at the hip or wielded in hand.
#[derive(Clone, Copy, PartialEq)]
pub enum SwordPosition {
    Sheathed,
    Wielded,
}

/// State for the sword entity, attached to the sword child of the player.
pub struct SwordState {
    pub position: SwordPosition,
    pub sheathed_pos: Vec3,
    pub sheathed_rot: Quat,
    pub wielded_pos: Vec3,
    pub wielded_rot: Quat,
}

/// Tracks the limb entities that make up the player's character body.
/// Attached to the player entity for direct access to limbs.
pub struct CharacterBody {
    pub head: Entity,
    pub left_upper_arm: Entity,
    pub left_forearm: Entity,
    pub right_upper_arm: Entity,
    pub right_forearm: Entity,
    pub left_upper_leg: Entity,
    pub left_lower_leg: Entity,
    pub right_upper_leg: Entity,
    pub right_lower_leg: Entity,
    pub sword: Entity,
}

/// Directional light component (sun-like). Casts shadows via shadow mapping.
pub struct DirectionalLight {
    pub direction: Vec3,
    pub color: Vec3,
    pub intensity: f32,
    /// Shadow map resolution (width = height).
    pub shadow_resolution: u32,
    /// Half-extent of the orthographic shadow volume.
    pub shadow_extent: f32,
}

/// Point light component with distance attenuation.
pub struct PointLight {
    pub color: Vec3,
    pub intensity: f32,
    pub radius: f32,
    pub constant: f32,
    pub linear: f32,
    pub quadratic: f32,
}

impl PointLight {
    pub fn new(color: Vec3, intensity: f32, radius: f32) -> Self {
        Self {
            color,
            intensity,
            radius,
            constant: 1.0,
            linear: 4.5 / radius,
            quadratic: 75.0 / (radius * radius),
        }
    }
}

/// Spot light component with cone angle and falloff.
pub struct SpotLight {
    pub direction: Vec3,
    pub color: Vec3,
    pub intensity: f32,
    pub inner_cone: f32,
    pub outer_cone: f32,
    pub radius: f32,
    pub constant: f32,
    pub linear: f32,
    pub quadratic: f32,
}

impl SpotLight {
    pub fn new(direction: Vec3, color: Vec3, intensity: f32, inner_deg: f32, outer_deg: f32, radius: f32) -> Self {
        Self {
            direction: direction.normalize(),
            color,
            intensity,
            inner_cone: inner_deg.to_radians().cos(),
            outer_cone: outer_deg.to_radians().cos(),
            radius,
            constant: 1.0,
            linear: 4.5 / radius,
            quadratic: 75.0 / (radius * radius),
        }
    }
}
