use glam::{Quat, Vec3};
use hecs::Entity;

use crate::fsm::StateMachine;

// ---------------------------------------------------------------------------
// Animation components
// ---------------------------------------------------------------------------

/// A snapshot of all bone orientations used for crossfade blending.
#[derive(Clone, Copy)]
pub struct BonePose {
    pub head_rot: Quat,
    pub left_upper_arm_rot: Quat,
    pub left_forearm_rot: Quat,
    pub right_upper_arm_rot: Quat,
    pub right_forearm_rot: Quat,
    pub left_upper_leg_rot: Quat,
    pub left_lower_leg_rot: Quat,
    pub right_upper_leg_rot: Quat,
    pub right_lower_leg_rot: Quat,
}

/// Attached to the player entity. Drives procedural animation of character bones.
pub struct AnimationState {
    /// Phase accumulator for cyclic animations (walk/run). Resets on state change.
    pub phase: f32,
    /// Blend factor: 0.0 = blend_from pose, 1.0 = current target pose.
    pub blend: f32,
    /// Speed at which blend approaches 1.0 (per second).
    pub blend_speed: f32,
    /// Snapshot of bone rotations at the start of the last state transition.
    pub blend_from: Option<BonePose>,
}

impl AnimationState {
    pub fn new() -> Self {
        Self {
            phase: 0.0,
            blend: 1.0,
            blend_speed: 8.0,
            blend_from: None,
        }
    }
}

/// Marker: this entity is the player.
pub struct Player;

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
    /// Yaw lock: `Some((clamp_yaw, block_dir))` when the held object is laterally blocked by
    /// a wall. `block_dir` is `+1.0` (blocked turning right) or `-1.0` (blocked turning left).
    /// `app.rs` reads this each frame to clamp `camera.yaw` and `camera.body_yaw`.
    pub yaw_lock: Option<(f32, f32)>,
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
            yaw_lock: None,
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

// ---------------------------------------------------------------------------
// Player state machine
// ---------------------------------------------------------------------------

/// All discrete states the player can be in.
///
/// Transition logic lives in `impl PlayerState` in `src/systems/player.rs`
/// (where it has access to input and physics context) rather than here so
/// that this file stays pure data.
#[derive(Clone)]
pub enum PlayerState {
    /// Standing still, no movement input.
    Idle,
    /// Moving at walk speed.
    Walking,
    /// Sprint key held while moving.
    Running,
    /// Brief directional burst. Timer counts up; burst ends when it exceeds
    /// `DASH_DURATION`. `cooldown_remaining` counts down after each dash.
    Dashing {
        direction: Vec3,
        timer: f32,
        cooldown_remaining: f32,
    },
    /// Ascending after jump input. `has_released_jump` tracks whether the
    /// player let go of the jump key (for variable-height jump cut).
    Jumping { has_released_jump: bool },
    /// Airborne and descending (or walked off an edge).
    Falling,
    /// Brief recovery animation on ground contact. Timer counts up.
    Landing { timer: f32 },
    /// Sword transition: sheathing. Timer counts up.
    Sheathing { timer: f32 },
    /// Sword transition: unsheathing. Timer counts up.
    Unsheathing { timer: f32 },
}

/// FSM component attached to the player entity.
pub type PlayerFsm = StateMachine<PlayerState>;

// ---------------------------------------------------------------------------

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
