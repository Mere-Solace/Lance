use glam::{Mat4, Vec3};
use hecs::{Entity, World};

use crate::components::*;
use crate::renderer::mesh::{
    create_capsule, create_sphere, create_sword, create_tapered_box,
};
use crate::renderer::MeshStore;

// ---------------------------------------------------------------------------
// CharacterRig — private proportions table for spawn_player
// ---------------------------------------------------------------------------

/// All body proportions and joint offsets in one place.
/// Separates mesh dimensions from collider dimensions so hitbox ≠ visual is possible.
struct CharacterRig {
    // Torso (tapered box mesh + capsule collider)
    torso_top_w: f32,
    torso_top_d: f32,
    torso_bot_w: f32,
    torso_bot_d: f32,
    torso_height: f32,
    body_collider_radius: f32,
    body_collider_height: f32,

    // Head (sphere mesh; collider radius = head_mesh_radius * head_scale)
    head_mesh_radius: f32,
    head_scale: f32,

    // Limbs (same capsule dimensions for all segments; mesh == collider by default)
    limb_radius: f32,
    limb_height: f32,

    // Attachment points, relative to body (torso) center
    shoulder_x: f32,      // ± X offset for upper arms
    shoulder_y: f32,      // Y offset for upper arms
    shoulder_angle: f32,  // Z rotation, outward tilt (radians)
    hip_x: f32,           // ± X offset for upper legs
    hip_y: f32,           // Y offset for upper legs

    // Colors
    body_color: Vec3,
    head_color: Vec3,
    limb_color: Vec3,
}

impl CharacterRig {
    fn head_world_radius(&self) -> f32 {
        self.head_mesh_radius * self.head_scale
    }

    fn head_y(&self) -> f32 {
        self.torso_height / 2.0 + self.head_world_radius()
    }

    /// Y offset from a parent capsule center to its child (joint connection point).
    fn joint_y(&self) -> f32 {
        -(self.limb_height / 2.0 + self.limb_height / 2.0 + self.limb_radius)
    }
}

/// Default rig matching the current scene tuning.
fn default_rig() -> CharacterRig {
    CharacterRig {
        torso_top_w: 0.7,
        torso_top_d: 0.5,
        torso_bot_w: 0.35,
        torso_bot_d: 0.25,
        torso_height: 0.8,
        body_collider_radius: 0.3,
        body_collider_height: 2.4,

        head_mesh_radius: 0.8,
        head_scale: 0.3,

        limb_radius: 0.15,
        limb_height: 0.4,

        shoulder_x: 0.45,
        shoulder_y: 0.1,
        shoulder_angle: 0.14,
        hip_x: 0.2,
        hip_y: -0.6,

        body_color: Vec3::new(0.8, 0.2, 0.15),
        head_color: Vec3::new(0.7, 0.65, 0.6),
        limb_color: Vec3::new(0.5, 0.5, 0.6),
    }
}

// ---------------------------------------------------------------------------
// spawn_character — private helper used by spawn_player
// ---------------------------------------------------------------------------

fn spawn_character(
    world: &mut World,
    player_entity: Entity,
    head_handle: MeshHandle,
    upper_arm_handle: MeshHandle,
    forearm_handle: MeshHandle,
    upper_leg_handle: MeshHandle,
    lower_leg_handle: MeshHandle,
    sword_handle: MeshHandle,
    rig: &CharacterRig,
) -> CharacterBody {
    use glam::Quat;
    use std::f32::consts::FRAC_PI_2;
    use std::f32::consts::FRAC_PI_6;

    // Head — sphere at top of torso
    let mut head_tr = LocalTransform::new(Vec3::new(0.0, rig.head_y(), 0.1));
    head_tr.scale = Vec3::splat(rig.head_scale);
    let head = world.spawn((
        head_tr,
        GlobalTransform(Mat4::IDENTITY),
        head_handle,
        Color(rig.head_color),
    ));
    add_child(world, player_entity, head);

    // --- Arms (2-segment: upper arm + forearm) ---

    // Left upper arm — positioned at shoulder (+X = left)
    let mut left_upper_arm_t = LocalTransform::new(Vec3::new(rig.shoulder_x, rig.shoulder_y, 0.0));
    left_upper_arm_t.rotation = Quat::from_rotation_z(rig.shoulder_angle);
    let left_upper_arm = world.spawn((
        left_upper_arm_t,
        GlobalTransform(Mat4::IDENTITY),
        upper_arm_handle,
        Color(rig.body_color),
    ));
    add_child(world, player_entity, left_upper_arm);

    // Left forearm — child of left upper arm
    let left_forearm = world.spawn((
        LocalTransform::new(Vec3::new(0.0, rig.joint_y(), 0.0)),
        GlobalTransform(Mat4::IDENTITY),
        forearm_handle,
        Color(rig.limb_color),
    ));
    add_child(world, left_upper_arm, left_forearm);

    // Right upper arm — mirror of left (-X = right)
    let mut right_upper_arm_t = LocalTransform::new(Vec3::new(-rig.shoulder_x, rig.shoulder_y, 0.0));
    right_upper_arm_t.rotation = Quat::from_rotation_z(-rig.shoulder_angle);
    let right_upper_arm = world.spawn((
        right_upper_arm_t,
        GlobalTransform(Mat4::IDENTITY),
        upper_arm_handle,
        Color(rig.body_color),
    ));
    add_child(world, player_entity, right_upper_arm);

    // Right forearm — child of right upper arm
    let right_forearm = world.spawn((
        LocalTransform::new(Vec3::new(0.0, rig.joint_y(), 0.0)),
        GlobalTransform(Mat4::IDENTITY),
        forearm_handle,
        Color(rig.limb_color),
    ));
    add_child(world, right_upper_arm, right_forearm);

    // --- Legs (2-segment: upper leg + lower leg) ---

    // Left upper leg (+X = left)
    let left_upper_leg = world.spawn((
        LocalTransform::new(Vec3::new(rig.hip_x, rig.hip_y, 0.0)),
        GlobalTransform(Mat4::IDENTITY),
        upper_leg_handle,
        Color(rig.body_color),
    ));
    add_child(world, player_entity, left_upper_leg);

    // Left lower leg — child of left upper leg
    let left_lower_leg = world.spawn((
        LocalTransform::new(Vec3::new(0.0, rig.joint_y(), 0.0)),
        GlobalTransform(Mat4::IDENTITY),
        lower_leg_handle,
        Color(rig.limb_color),
    ));
    add_child(world, left_upper_leg, left_lower_leg);

    // Right upper leg (-X = right)
    let right_upper_leg = world.spawn((
        LocalTransform::new(Vec3::new(-rig.hip_x, rig.hip_y, 0.0)),
        GlobalTransform(Mat4::IDENTITY),
        upper_leg_handle,
        Color(rig.body_color),
    ));
    add_child(world, player_entity, right_upper_leg);

    // Right lower leg — child of right upper leg
    let right_lower_leg = world.spawn((
        LocalTransform::new(Vec3::new(0.0, rig.joint_y(), 0.0)),
        GlobalTransform(Mat4::IDENTITY),
        lower_leg_handle,
        Color(rig.limb_color),
    ));
    add_child(world, right_upper_leg, right_lower_leg);

    // --- Sword — starts sheathed at the hip ---
    let sheathed_pos = Vec3::new(0.25, 0.0, 0.4);
    let sheathed_rot = Quat::from_rotation_y(FRAC_PI_2);
    let sheathed_rot = Quat::from_rotation_x(2.0 * FRAC_PI_2 + 2.0 * FRAC_PI_6) * sheathed_rot;

    let wielded_pos = Vec3::new(-0.55, -0.5, 0.3);
    let wielded_rot = Quat::from_rotation_y(FRAC_PI_2);
    let wielded_rot = Quat::from_rotation_x(FRAC_PI_2 - 0.1) * wielded_rot;

    let mut sword_t = LocalTransform::new(sheathed_pos);
    sword_t.rotation = sheathed_rot;
    sword_t.scale = Vec3::splat(3.0);

    let sword_entity = world.spawn((
        sword_t,
        GlobalTransform(Mat4::IDENTITY),
        sword_handle,
        Color(Vec3::new(0.75, 0.75, 0.8)),
        SwordState {
            position: SwordPosition::Sheathed,
            sheathed_pos,
            sheathed_rot,
            wielded_pos,
            wielded_rot,
        },
    ));
    add_child(world, player_entity, sword_entity);

    CharacterBody {
        head,
        left_upper_arm,
        left_forearm,
        right_upper_arm,
        right_forearm,
        left_upper_leg,
        left_lower_leg,
        right_upper_leg,
        right_lower_leg,
        sword: sword_entity,
    }
}

// ---------------------------------------------------------------------------
// Public prefab factories
// ---------------------------------------------------------------------------

/// Spawn the ground as a thick box so the top face is captured in the shadow map,
/// giving correct contact shadows for objects resting on it. Top face sits at Y=0.
///
/// Uses a unit mesh (1×1×1) with large scale so that `approx_bounding_sphere`
/// in the renderer reads the correct size from the transform columns.  Without
/// this, the floor (scale=1 on a 1000-unit mesh) gets a bounding sphere of
/// radius=2 and is frustum-culled out of shadow cascades as soon as the camera
/// moves away from the world origin.
pub fn spawn_ground(world: &mut World, meshes: &mut MeshStore) -> Entity {
    const HALF_EXTENT: f32 = 500.0;
    const THICKNESS: f32 = 2.0;
    // Unit box: Y from -0.5 to +0.5 in local space.
    let ground_handle = meshes.add(create_tapered_box(1.0, 1.0, 1.0, 1.0, 1.0));
    // Scale so the box covers [-HALF_EXTENT, HALF_EXTENT] in X/Z and [0, -THICKNESS] in Y.
    let mut ground_t = LocalTransform::new(Vec3::new(0.0, -THICKNESS / 2.0, 0.0));
    ground_t.scale = Vec3::new(HALF_EXTENT * 2.0, THICKNESS, HALF_EXTENT * 2.0);
    world.spawn((
        ground_t,
        GlobalTransform(Mat4::IDENTITY),
        ground_handle,
        Color(Vec3::new(0.3, 0.6, 0.2)),
        Checkerboard(Vec3::new(0.22, 0.48, 0.15)),
        Collider::Plane { normal: Vec3::Y, offset: 0.0 },
        Static,
    ))
}

/// Spawn a dynamic sphere with physics and a child blue satellite sphere.
/// Returns the root sphere entity. The child is attached automatically.
pub fn spawn_physics_sphere(
    world: &mut World,
    meshes: &mut MeshStore,
    pos: Vec3,
    color: Vec3,
    collider_radius: f32,
    initial_vel: Vec3,
) -> Entity {
    let mesh_scale = collider_radius; // mesh was built at radius 1.0
    let sphere_handle = meshes.add(create_sphere(1.0, 16, 32));

    let mut sphere_t = LocalTransform::new(pos);
    sphere_t.scale = Vec3::splat(mesh_scale);

    let root = world.spawn((
        sphere_t,
        GlobalTransform(Mat4::IDENTITY),
        sphere_handle,
        Color(color),
        Velocity(initial_vel),
        Mass(1.0),
        GravityAffected,
        Collider::Sphere { radius: collider_radius },
        Restitution(0.3),
        Friction(0.7),
        Drag(0.5),
        Grabbable,
    ));

    // Blue satellite child sphere
    let mut child_t = LocalTransform::new(Vec3::new(0.75, 0.0, 0.0));
    child_t.scale = Vec3::splat(0.4);
    let child = world.spawn((
        child_t,
        GlobalTransform(Mat4::IDENTITY),
        sphere_handle,
        Mass(1.0),
        GravityAffected,
        Color(Vec3::new(0.2, 0.4, 0.9)),
    ));
    add_child(world, root, child);

    root
}

/// Spawn a static box (axis-aligned). `pos` is the world-space center.
/// Mesh and collider use the same half-extents, but collider can differ
/// from mesh if needed — the mesh is always a straight box (no taper).
pub fn spawn_static_box(
    world: &mut World,
    meshes: &mut MeshStore,
    pos: Vec3,
    half_extents: Vec3,
    color: Vec3,
) -> Entity {
    // create_tapered_box takes full dimensions; half_extents * 2 = full size
    let box_handle = meshes.add(create_tapered_box(
        half_extents.x * 2.0, half_extents.z * 2.0,
        half_extents.x * 2.0, half_extents.z * 2.0,
        half_extents.y * 2.0,
    ));
    world.spawn((
        LocalTransform::new(pos),
        GlobalTransform(Mat4::IDENTITY),
        box_handle,
        Color(color),
        Collider::Box { half_extents },
        Static,
        Restitution(0.0),
        Friction(0.8),
    ))
}

/// Spawn the player entity with full character body (torso, head, arms, legs, sword).
/// Returns the player entity. The CharacterBody component is also inserted onto it.
pub fn spawn_player(world: &mut World, meshes: &mut MeshStore, pos: Vec3) -> Entity {
    let rig = default_rig();

    let torso_handle = meshes.add(create_tapered_box(
        rig.torso_top_w, rig.torso_top_d,
        rig.torso_bot_w, rig.torso_bot_d,
        rig.torso_height,
    ));
    let upper_arm_handle = meshes.add(create_capsule(rig.limb_radius, rig.limb_height, 8, 8));
    let forearm_handle   = meshes.add(create_capsule(rig.limb_radius, rig.limb_height, 8, 8));
    let upper_leg_handle = meshes.add(create_capsule(rig.limb_radius, rig.limb_height, 8, 8));
    let lower_leg_handle = meshes.add(create_capsule(rig.limb_radius, rig.limb_height, 8, 8));
    let head_handle      = meshes.add(create_sphere(rig.head_mesh_radius, 8, 8));
    let sword_handle     = meshes.add(create_sword());

    let mut player_t = LocalTransform::new(pos);
    player_t.scale = Vec3::splat(1.0);

    let player_entity = world.spawn((
        player_t,
        GlobalTransform(Mat4::IDENTITY),
        torso_handle,
        Color(rig.body_color),
        Velocity(Vec3::ZERO),
        Mass(80.0),
        GravityAffected,
        Collider::Capsule {
            radius: rig.body_collider_radius,
            height: rig.body_collider_height,
        },
        Restitution(0.0),
        Friction(0.8),
        Player,
        GrabState::new(),
        // Player spawns airborne (pos.y = 10); starts in Falling so the FSM
        // is correct immediately without a dummy Grounded → Falling transition.
        PlayerFsm::new(PlayerState::Falling),
    ));

    let body = spawn_character(
        world,
        player_entity,
        head_handle,
        upper_arm_handle,
        forearm_handle,
        upper_leg_handle,
        lower_leg_handle,
        sword_handle,
        &rig,
    );
    world.insert_one(player_entity, body).unwrap();

    player_entity
}

/// Spawn a directional light (sun-like, no position).
pub fn spawn_directional_light(
    world: &mut World,
    direction: Vec3,
    color: Vec3,
    intensity: f32,
) -> Entity {
    world.spawn((DirectionalLight {
        direction,
        color,
        intensity,
        shadow_resolution: 2048,
    },))
}

/// Spawn a point light at `pos`.
pub fn spawn_point_light(
    world: &mut World,
    pos: Vec3,
    color: Vec3,
    intensity: f32,
    radius: f32,
) -> Entity {
    world.spawn((
        LocalTransform::new(pos),
        PointLight::new(color, intensity, radius),
    ))
}

/// Spawn a spot light at `pos` pointing in `direction`.
pub fn spawn_spot_light(
    world: &mut World,
    pos: Vec3,
    direction: Vec3,
    color: Vec3,
    intensity: f32,
    inner_deg: f32,
    outer_deg: f32,
    radius: f32,
) -> Entity {
    world.spawn((
        LocalTransform::new(pos),
        SpotLight::new(direction, color, intensity, inner_deg, outer_deg, radius),
    ))
}
