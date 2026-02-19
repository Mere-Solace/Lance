use glam::{Quat, Vec3};
use hecs::World;
use sdl2::keyboard::Scancode;

use crate::camera::Camera;
use crate::components::{
    CollisionEvent, Grounded, LocalTransform, Parent, Player, PlayerFsm, PlayerState, Velocity,
};
use crate::engine::input::InputState;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const PLAYER_WALK_SPEED: f32 = 6.0;
const PLAYER_RUN_SPEED: f32 = 10.0;
const JUMP_IMPULSE: f32 = 7.0;

// Stub durations for states not yet triggerable from input.
// These keep the match exhaustive and ready for the issues that add them.
const DASH_DURATION: f32 = 0.2;
const LANDING_DURATION: f32 = 0.05; // short — just enough for a skid; no animation yet
const SHEATHE_DURATION: f32 = 0.3;

// Air control — reduced max speed + acceleration-based steering (not instant override)
const AIR_CONTROL_SPEED: f32 = 4.0;  // max speed achievable through air input
const AIR_ACCELERATION: f32 = 10.0;  // m/s² added per second toward desired direction

// ---------------------------------------------------------------------------
// PlayerState transition logic
// ---------------------------------------------------------------------------

/// Context passed to [`PlayerState::next`] each frame.
pub struct PlayerCtx<'a> {
    pub input: &'a InputState,
    pub grounded: bool,
    pub velocity: Vec3,
    pub dt: f32,
}

impl PlayerState {
    /// Advance timers that live inside timed state variants.
    /// Called every frame before evaluating per-state transitions.
    pub fn tick_timers(&mut self, dt: f32) {
        match self {
            Self::Dashing { timer, cooldown_remaining, .. } => {
                *timer += dt;
                *cooldown_remaining = (*cooldown_remaining - dt).max(0.0);
            }
            Self::Landing { timer }     => *timer += dt,
            Self::Sheathing { timer }   => *timer += dt,
            Self::Unsheathing { timer } => *timer += dt,
            _ => {}
        }
    }

    /// Return the next state if a transition should fire, or `None` to stay.
    ///
    /// Each match arm covers all transitions **out** of one source state.
    /// Global transitions (jump, walk-off-edge) are checked separately in
    /// [`check_global_transitions`] and evaluated first.
    pub fn next(&self, ctx: &PlayerCtx) -> Option<PlayerState> {
        let moving = ctx.input.is_key_held(Scancode::W)
            || ctx.input.is_key_held(Scancode::A)
            || ctx.input.is_key_held(Scancode::S)
            || ctx.input.is_key_held(Scancode::D);

        let sprinting = ctx.input.is_key_held(Scancode::LShift);

        match self {
            Self::Idle => {
                if moving { Some(Self::Walking) } else { None }
            }

            Self::Walking => {
                if !moving        { Some(Self::Idle) }
                else if sprinting  { Some(Self::Running) }
                else               { None }
            }

            Self::Running => {
                if !moving        { Some(Self::Idle) }
                else if !sprinting { Some(Self::Walking) }
                else               { None }
            }

            // Dash ends when its internal timer expires.
            Self::Dashing { timer, .. } => {
                if *timer >= DASH_DURATION { Some(Self::Falling) }
                else { None }
            }

            // Jump-to-fall: velocity turned non-positive, or key released early.
            Self::Jumping { has_released_jump } => {
                let key_up = !ctx.input.is_key_held(Scancode::Space);
                if ctx.velocity.y <= 0.0 || (key_up && !*has_released_jump) {
                    Some(Self::Falling)
                } else {
                    None
                }
            }

            // Fall ends on ground contact.
            Self::Falling => {
                if ctx.grounded { Some(Self::Landing { timer: 0.0 }) }
                else            { None }
            }

            // Landing recovery ends when timer expires.
            Self::Landing { timer } => {
                if *timer >= LANDING_DURATION { Some(Self::Idle) }
                else                          { None }
            }

            // Sword transitions end when timer expires.
            Self::Sheathing { timer } | Self::Unsheathing { timer } => {
                if *timer >= SHEATHE_DURATION { Some(Self::Idle) }
                else                          { None }
            }
        }
    }

    /// Whether this state is airborne (player has partial air-steering control
    /// but not direct velocity override). Checked by `player_movement_system`.
    pub fn is_airborne(&self) -> bool {
        matches!(self, Self::Jumping { .. } | Self::Falling)
    }

    /// Horizontal move speed for grounded states.
    /// - `Some(speed)` → directly set horizontal velocity to this speed.
    /// - `None`        → leave velocity untouched (airborne OR locked states).
    ///
    /// Call `is_airborne()` first; if true, use air-control path instead.
    pub fn move_speed(&self) -> Option<f32> {
        match self {
            Self::Idle    => Some(0.0),
            Self::Walking => Some(PLAYER_WALK_SPEED),
            Self::Running => Some(PLAYER_RUN_SPEED),
            // Airborne: handled by is_airborne() path — should not reach here.
            Self::Jumping { .. } | Self::Falling => None,
            // Locked states (Dashing, Landing, Sheathing, Unsheathing):
            // leave velocity alone so momentum carries through the state.
            _ => None,
        }
    }

    /// Whether jump input is accepted in this state.
    /// Landing is included so a buffered jump (Space held through landing)
    /// fires on the first frame of ground contact.
    pub fn can_jump(&self) -> bool {
        matches!(self, Self::Idle | Self::Walking | Self::Running | Self::Landing { .. })
    }
}

// ---------------------------------------------------------------------------
// Global transitions (any-state rules, checked before per-state logic)
// ---------------------------------------------------------------------------

/// Returns a transition that should fire regardless of current state, or `None`.
/// Jump and walk-off-edge are global because they can fire from multiple states.
fn check_global_transitions(
    state: &PlayerState,
    input: &InputState,
    grounded: bool,
) -> Option<PlayerState> {
    // Jump: from any grounded state that permits it.
    // Using is_key_held (not just KeyPressed) so holding Space through a fall
    // immediately re-triggers the jump on landing — a simple jump buffer.
    if grounded && state.can_jump() && input.is_key_held(Scancode::Space) {
        return Some(PlayerState::Jumping { has_released_jump: false });
    }

    // Walked off an edge: was in a ground-locomotion state but ground was lost.
    if !grounded
        && matches!(state, PlayerState::Idle | PlayerState::Walking | PlayerState::Running)
    {
        return Some(PlayerState::Falling);
    }

    None
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Drive player FSM transitions. Runs **before** `player_movement_system`.
///
/// Timing note: `fsm.tick(dt)` is called **first** each frame so that the
/// `just_entered` flag stays `true` for the entire frame a transition fires,
/// allowing downstream systems (movement, animation) to react on the same frame.
pub fn player_state_system(world: &mut World, input: &InputState, dt: f32) {
    for (_e, (fsm, grounded, vel)) in
        world.query_mut::<(&mut PlayerFsm, Option<&Grounded>, &mut Velocity)>()
    {
        let is_grounded = grounded.is_some();
        let velocity = vel.0;

        // 1. Advance elapsed timer and clear last frame's just_entered flag.
        fsm.tick(dt);

        // 2. Global transitions (jump, walk-off-edge) take priority.
        let global_next = check_global_transitions(&fsm.state, input, is_grounded);

        if let Some(next) = global_next {
            // Apply jump impulse here so movement_system never needs to.
            if matches!(next, PlayerState::Jumping { .. }) {
                vel.0.y = JUMP_IMPULSE;
            }
            fsm.go(next);
        } else {
            // 3. Advance intra-state timers, then check per-state transitions.
            fsm.state.tick_timers(dt);
            let ctx = PlayerCtx { input, grounded: is_grounded, velocity, dt };
            if let Some(next) = fsm.state.next(&ctx) {
                fsm.go(next);
            }
        }

        #[cfg(debug_assertions)]
        if fsm.just_entered() {
            let label = match &fsm.state {
                PlayerState::Idle              => "Idle",
                PlayerState::Walking           => "Walking",
                PlayerState::Running           => "Running",
                PlayerState::Dashing { .. }    => "Dashing",
                PlayerState::Jumping { .. }    => "Jumping",
                PlayerState::Falling           => "Falling",
                PlayerState::Landing { .. }    => "Landing",
                PlayerState::Sheathing { .. }  => "Sheathing",
                PlayerState::Unsheathing { .. } => "Unsheathing",
            };
            println!("[player_state] → {}", label);
        }
    }
}

/// Apply movement based on the current FSM state.
/// Jump velocity is already applied by `player_state_system`.
///
/// Three movement modes:
/// - **Ground** (Idle/Walking/Running): directly set horizontal velocity.
/// - **Air** (Jumping/Falling): acceleration-based steering at reduced speed;
///   no input = velocity untouched (no air braking).
/// - **Locked** (Landing/Dashing/Sheathing): leave velocity alone so momentum
///   carries through the state naturally.
pub fn player_movement_system(
    world: &mut World,
    input: &InputState,
    camera: &Camera,
    speed_multiplier: f32,
    dt: f32,
) {
    let yaw_rad = camera.yaw.to_radians();
    let forward = Vec3::new(yaw_rad.cos(), 0.0, yaw_rad.sin()).normalize();
    let right = forward.cross(Vec3::Y).normalize();

    // Build input direction once outside the loop.
    let mut move_dir = Vec3::ZERO;
    if input.is_key_held(Scancode::W) { move_dir += forward; }
    if input.is_key_held(Scancode::S) { move_dir -= forward; }
    if input.is_key_held(Scancode::A) { move_dir -= right; }
    if input.is_key_held(Scancode::D) { move_dir += right; }
    let has_input = move_dir.length_squared() > 0.0;
    let move_dir_norm = if has_input { move_dir.normalize() } else { Vec3::ZERO };

    for (_entity, (local, vel, _player, fsm)) in
        world.query_mut::<(&mut LocalTransform, &mut Velocity, &Player, &PlayerFsm)>()
    {
        // Rotate the player mesh to face camera yaw, unless free-look is active
        // (alt-look: camera pans freely, character facing stays fixed).
        if !camera.free_look {
            local.rotation = Quat::from_rotation_y(-yaw_rad + std::f32::consts::FRAC_PI_2);
        }

        if fsm.state.is_airborne() {
            // Air control: nudge velocity toward desired direction.
            // No input = velocity preserved (no air friction from player).
            if has_input {
                let desired_x = move_dir_norm.x * AIR_CONTROL_SPEED * speed_multiplier;
                let desired_z = move_dir_norm.z * AIR_CONTROL_SPEED * speed_multiplier;
                let diff_x = desired_x - vel.0.x;
                let diff_z = desired_z - vel.0.z;
                let dist = (diff_x * diff_x + diff_z * diff_z).sqrt();
                if dist > 0.0 {
                    let step = (AIR_ACCELERATION * dt).min(dist);
                    vel.0.x += diff_x / dist * step;
                    vel.0.z += diff_z / dist * step;
                }
            }
        } else if let Some(speed) = fsm.state.move_speed() {
            // Ground: directly override horizontal velocity.
            let horizontal = move_dir_norm * speed * speed_multiplier;
            vel.0.x = horizontal.x;
            vel.0.z = horizontal.z;
        }
        // else Locked (Landing, Dashing, Sheathing, etc.): leave velocity alone.
    }
}

// ---------------------------------------------------------------------------
// Grounded detection
// ---------------------------------------------------------------------------

/// Walk up the Parent chain to find the root entity.
fn find_root(world: &World, entity: hecs::Entity) -> hecs::Entity {
    let mut current = entity;
    while let Ok(parent) = world.get::<&Parent>(current) {
        current = parent.0;
    }
    current
}

/// `physics_ticks` is the number of fixed steps that ran this render frame.
/// When it is zero (render framerate > physics rate), no collision events were
/// generated, so we must NOT clear Grounded — contacts from last tick are still
/// valid. Clearing it would trigger a spurious Falling transition every other
/// frame on hardware faster than 60fps.
pub fn grounded_system(world: &mut World, events: &[CollisionEvent], physics_ticks: usize) {
    if physics_ticks == 0 {
        return;
    }

    // A physics tick ran — clear and rebuild from this tick's contacts.
    let players: Vec<_> = world
        .query_mut::<(&Player, &Grounded)>()
        .into_iter()
        .map(|(e, _)| e)
        .collect();
    for entity in players {
        let _ = world.remove_one::<Grounded>(entity);
    }

    // Re-add Grounded for any upward ground-contact collision this frame.
    for event in events {
        let root_a = find_root(world, event.entity_a);
        let root_b = find_root(world, event.entity_b);

        let a_is_player = world.get::<&Player>(root_a).is_ok();
        let b_is_player = world.get::<&Player>(root_b).is_ok();

        if a_is_player && (-event.contact_normal).dot(Vec3::Y) > 0.7 {
            let _ = world.insert_one(root_a, Grounded);
        }
        if b_is_player && event.contact_normal.dot(Vec3::Y) > 0.7 {
            let _ = world.insert_one(root_b, Grounded);
        }
    }
}
