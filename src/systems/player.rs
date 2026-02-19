use glam::{Quat, Vec3};
use hecs::World;
use sdl2::keyboard::Scancode;

use crate::camera::Camera;
use crate::components::{
    CollisionEvent, Grounded, LocalTransform, Parent, Player, PlayerFsm, PlayerState, Velocity,
};
use crate::engine::input::{InputEvent, InputState};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const PLAYER_WALK_SPEED: f32 = 6.0;
const PLAYER_RUN_SPEED: f32 = 10.0;
const JUMP_IMPULSE: f32 = 7.0;

// Stub durations for states not yet triggerable from input.
// These keep the match exhaustive and ready for the issues that add them.
const DASH_DURATION: f32 = 0.2;
const LANDING_DURATION: f32 = 0.15;
const SHEATHE_DURATION: f32 = 0.3;

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

    /// Horizontal move speed for this state.
    /// `None` = airborne — don't override horizontal velocity; let physics run.
    pub fn move_speed(&self) -> Option<f32> {
        match self {
            Self::Idle        => Some(0.0),
            Self::Walking     => Some(PLAYER_WALK_SPEED),
            Self::Running     => Some(PLAYER_RUN_SPEED),
            Self::Jumping { .. } | Self::Falling => None,
            // Dashing / Landing / Sheathing / Unsheathing: player-locked, no input movement.
            _ => Some(0.0),
        }
    }

    /// Whether jump input is accepted in this state.
    pub fn can_jump(&self) -> bool {
        matches!(self, Self::Idle | Self::Walking | Self::Running)
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
    // Jump: from any grounded state that permits it, on the frame Space is pressed.
    if grounded && state.can_jump() {
        for event in &input.events {
            if let InputEvent::KeyPressed(Scancode::Space) = event {
                return Some(PlayerState::Jumping { has_released_jump: false });
            }
        }
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
/// Reads `PlayerFsm` for speed; jump velocity is already applied by `player_state_system`.
pub fn player_movement_system(
    world: &mut World,
    input: &InputState,
    camera: &Camera,
    speed_multiplier: f32,
) {
    let yaw_rad = camera.yaw.to_radians();
    let forward = Vec3::new(yaw_rad.cos(), 0.0, yaw_rad.sin()).normalize();
    let right = forward.cross(Vec3::Y).normalize();

    for (_entity, (local, vel, _player, fsm)) in
        world.query_mut::<(&mut LocalTransform, &mut Velocity, &Player, &PlayerFsm)>()
    {
        // Always rotate the player mesh to face camera yaw.
        local.rotation = Quat::from_rotation_y(-yaw_rad + std::f32::consts::FRAC_PI_2);

        // Horizontal velocity only when the state grants a non-None speed.
        if let Some(speed) = fsm.state.move_speed() {
            let mut move_dir = Vec3::ZERO;
            if input.is_key_held(Scancode::W) { move_dir += forward; }
            if input.is_key_held(Scancode::S) { move_dir -= forward; }
            if input.is_key_held(Scancode::A) { move_dir -= right; }
            if input.is_key_held(Scancode::D) { move_dir += right; }

            let horizontal = if move_dir.length_squared() > 0.0 {
                move_dir.normalize() * speed * speed_multiplier
            } else {
                Vec3::ZERO
            };
            vel.0.x = horizontal.x;
            vel.0.z = horizontal.z;
        }
        // Airborne states (None speed): leave horizontal velocity untouched.
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
