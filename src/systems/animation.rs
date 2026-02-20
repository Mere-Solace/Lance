use glam::{Quat, Vec3};
use hecs::{Entity, World};

use crate::components::{
    AnimationState, BonePose, CharacterBody, LocalTransform, PlayerFsm, PlayerState, Velocity,
};

// ---------------------------------------------------------------------------
// Rest pose — matches the initial bone rotations set in spawn_character
// ---------------------------------------------------------------------------

const SHOULDER_ANGLE: f32 = 0.14; // radians; must match CharacterRig::shoulder_angle

fn rest_pose() -> BonePose {
    BonePose {
        head_rot: Quat::IDENTITY,
        left_upper_arm_rot: Quat::from_rotation_z(SHOULDER_ANGLE),
        left_forearm_rot: Quat::IDENTITY,
        right_upper_arm_rot: Quat::from_rotation_z(-SHOULDER_ANGLE),
        right_forearm_rot: Quat::IDENTITY,
        left_upper_leg_rot: Quat::IDENTITY,
        left_lower_leg_rot: Quat::IDENTITY,
        right_upper_leg_rot: Quat::IDENTITY,
        right_lower_leg_rot: Quat::IDENTITY,
    }
}

// ---------------------------------------------------------------------------
// Pose computation — one function per state group
// ---------------------------------------------------------------------------

fn pose_idle(phase: f32) -> BonePose {
    let rest = rest_pose();
    // Subtle breathing: arms sway slightly forward/back
    let sway = phase.sin() * 0.04;
    BonePose {
        left_upper_arm_rot: rest.left_upper_arm_rot * Quat::from_rotation_x(-sway),
        right_upper_arm_rot: rest.right_upper_arm_rot * Quat::from_rotation_x(sway),
        ..rest
    }
}

fn pose_locomotion(phase: f32, running: bool) -> BonePose {
    let rest = rest_pose();
    let leg_amp = if running { 0.55 } else { 0.38 };
    let arm_amp = leg_amp * 0.45;
    let s = phase.sin();
    // Knee bends on the back-swing only
    let left_knee = (-s).max(0.0) * leg_amp * 0.7;
    let right_knee = s.max(0.0) * leg_amp * 0.7;
    BonePose {
        head_rot: Quat::from_rotation_z(phase.cos() * 0.025),
        left_upper_arm_rot: rest.left_upper_arm_rot * Quat::from_rotation_x(-arm_amp * s),
        left_forearm_rot: Quat::from_rotation_x(arm_amp * 0.25),
        right_upper_arm_rot: rest.right_upper_arm_rot * Quat::from_rotation_x(arm_amp * s),
        right_forearm_rot: Quat::from_rotation_x(arm_amp * 0.25),
        left_upper_leg_rot: Quat::from_rotation_x(leg_amp * s),
        left_lower_leg_rot: Quat::from_rotation_x(-left_knee),
        right_upper_leg_rot: Quat::from_rotation_x(-leg_amp * s),
        right_lower_leg_rot: Quat::from_rotation_x(-right_knee),
    }
}

fn pose_jumping() -> BonePose {
    let rest = rest_pose();
    BonePose {
        left_upper_arm_rot: rest.left_upper_arm_rot * Quat::from_rotation_x(-0.35),
        right_upper_arm_rot: rest.right_upper_arm_rot * Quat::from_rotation_x(-0.35),
        left_forearm_rot: Quat::from_rotation_x(-0.25),
        right_forearm_rot: Quat::from_rotation_x(-0.25),
        left_upper_leg_rot: Quat::from_rotation_x(-0.25),
        right_upper_leg_rot: Quat::from_rotation_x(-0.25),
        left_lower_leg_rot: Quat::from_rotation_x(-0.35),
        right_lower_leg_rot: Quat::from_rotation_x(-0.35),
        ..rest
    }
}

fn pose_falling() -> BonePose {
    let rest = rest_pose();
    BonePose {
        left_upper_arm_rot: rest.left_upper_arm_rot * Quat::from_rotation_x(-0.2),
        right_upper_arm_rot: rest.right_upper_arm_rot * Quat::from_rotation_x(-0.2),
        left_forearm_rot: Quat::from_rotation_x(0.15),
        right_forearm_rot: Quat::from_rotation_x(0.15),
        left_upper_leg_rot: Quat::from_rotation_x(0.12),
        right_upper_leg_rot: Quat::from_rotation_x(0.12),
        left_lower_leg_rot: Quat::from_rotation_x(-0.12),
        right_lower_leg_rot: Quat::from_rotation_x(-0.12),
        ..rest
    }
}

fn pose_landing(timer: f32) -> BonePose {
    let rest = rest_pose();
    const LAND_DUR: f32 = 0.05;
    let t = (timer / LAND_DUR).min(1.0);
    // Crouch on impact, spring back linearly
    let crouch = (1.0 - t) * 0.38;
    BonePose {
        left_upper_leg_rot: Quat::from_rotation_x(crouch),
        right_upper_leg_rot: Quat::from_rotation_x(crouch),
        left_lower_leg_rot: Quat::from_rotation_x(-crouch * 1.4),
        right_lower_leg_rot: Quat::from_rotation_x(-crouch * 1.4),
        ..rest
    }
}

fn pose_dashing(timer: f32) -> BonePose {
    let rest = rest_pose();
    const DASH_DUR: f32 = 0.2;
    let t = (timer / DASH_DUR).min(1.0);
    // Arms trail on burst, return to neutral
    let trail = (1.0 - t) * 0.5;
    BonePose {
        left_upper_arm_rot: rest.left_upper_arm_rot * Quat::from_rotation_x(trail),
        right_upper_arm_rot: rest.right_upper_arm_rot * Quat::from_rotation_x(trail),
        left_forearm_rot: Quat::from_rotation_x(trail * 0.4),
        right_forearm_rot: Quat::from_rotation_x(trail * 0.4),
        ..rest
    }
}

fn pose_sheathing(timer: f32) -> BonePose {
    let rest = rest_pose();
    const SHEATHE_DUR: f32 = 0.3;
    let t = (timer / SHEATHE_DUR).min(1.0);
    // Right arm sweeps down toward hip/back
    let arm = Quat::from_rotation_x(t * 0.7) * Quat::from_rotation_z(t * -0.25);
    BonePose {
        right_upper_arm_rot: rest.right_upper_arm_rot * arm,
        right_forearm_rot: Quat::from_rotation_x(t * 0.4),
        ..rest
    }
}

fn pose_unsheathing(timer: f32) -> BonePose {
    let rest = rest_pose();
    const SHEATHE_DUR: f32 = 0.3;
    let t = (timer / SHEATHE_DUR).min(1.0);
    // Right arm draws up from hip/back to ready
    let arm = Quat::from_rotation_x((1.0 - t) * 0.7) * Quat::from_rotation_z((1.0 - t) * -0.25);
    BonePose {
        right_upper_arm_rot: rest.right_upper_arm_rot * arm,
        right_forearm_rot: Quat::from_rotation_x((1.0 - t) * 0.4),
        ..rest
    }
}

fn compute_target(state: &PlayerState, phase: f32) -> BonePose {
    match state {
        PlayerState::Idle => pose_idle(phase),
        PlayerState::Walking => pose_locomotion(phase, false),
        PlayerState::Running => pose_locomotion(phase, true),
        PlayerState::Jumping { .. } => pose_jumping(),
        PlayerState::Falling => pose_falling(),
        PlayerState::Landing { timer } => pose_landing(*timer),
        PlayerState::Dashing { timer, .. } => pose_dashing(*timer),
        PlayerState::Sheathing { timer } => pose_sheathing(*timer),
        PlayerState::Unsheathing { timer } => pose_unsheathing(*timer),
    }
}

// ---------------------------------------------------------------------------
// Blend helpers
// ---------------------------------------------------------------------------

fn slerp_pose(a: &BonePose, b: &BonePose, t: f32) -> BonePose {
    BonePose {
        head_rot: a.head_rot.slerp(b.head_rot, t),
        left_upper_arm_rot: a.left_upper_arm_rot.slerp(b.left_upper_arm_rot, t),
        left_forearm_rot: a.left_forearm_rot.slerp(b.left_forearm_rot, t),
        right_upper_arm_rot: a.right_upper_arm_rot.slerp(b.right_upper_arm_rot, t),
        right_forearm_rot: a.right_forearm_rot.slerp(b.right_forearm_rot, t),
        left_upper_leg_rot: a.left_upper_leg_rot.slerp(b.left_upper_leg_rot, t),
        left_lower_leg_rot: a.left_lower_leg_rot.slerp(b.left_lower_leg_rot, t),
        right_upper_leg_rot: a.right_upper_leg_rot.slerp(b.right_upper_leg_rot, t),
        right_lower_leg_rot: a.right_lower_leg_rot.slerp(b.right_lower_leg_rot, t),
    }
}

fn snapshot_bones(world: &World, body: &CharacterBody) -> BonePose {
    let rot = |e: Entity| {
        world
            .get::<&LocalTransform>(e)
            .map(|lt| lt.rotation)
            .unwrap_or(Quat::IDENTITY)
    };
    BonePose {
        head_rot: rot(body.head),
        left_upper_arm_rot: rot(body.left_upper_arm),
        left_forearm_rot: rot(body.left_forearm),
        right_upper_arm_rot: rot(body.right_upper_arm),
        right_forearm_rot: rot(body.right_forearm),
        left_upper_leg_rot: rot(body.left_upper_leg),
        left_lower_leg_rot: rot(body.left_lower_leg),
        right_upper_leg_rot: rot(body.right_upper_leg),
        right_lower_leg_rot: rot(body.right_lower_leg),
    }
}

fn apply_pose(world: &mut World, body: &BodyEntities, pose: &BonePose) {
    macro_rules! set_rot {
        ($e:expr, $r:expr) => {
            if let Ok(mut lt) = world.get::<&mut LocalTransform>($e) {
                lt.rotation = $r;
            }
        };
    }
    set_rot!(body.head, pose.head_rot);
    set_rot!(body.left_upper_arm, pose.left_upper_arm_rot);
    set_rot!(body.left_forearm, pose.left_forearm_rot);
    set_rot!(body.right_upper_arm, pose.right_upper_arm_rot);
    set_rot!(body.right_forearm, pose.right_forearm_rot);
    set_rot!(body.left_upper_leg, pose.left_upper_leg_rot);
    set_rot!(body.left_lower_leg, pose.left_lower_leg_rot);
    set_rot!(body.right_upper_leg, pose.right_upper_leg_rot);
    set_rot!(body.right_lower_leg, pose.right_lower_leg_rot);
}

// Plain-data copy of CharacterBody entity handles — lets us release the world
// borrow from the query before calling get::<&mut ...>() per entity.
struct BodyEntities {
    head: Entity,
    left_upper_arm: Entity,
    left_forearm: Entity,
    right_upper_arm: Entity,
    right_forearm: Entity,
    left_upper_leg: Entity,
    left_lower_leg: Entity,
    right_upper_leg: Entity,
    right_lower_leg: Entity,
}

impl From<&CharacterBody> for BodyEntities {
    fn from(b: &CharacterBody) -> Self {
        Self {
            head: b.head,
            left_upper_arm: b.left_upper_arm,
            left_forearm: b.left_forearm,
            right_upper_arm: b.right_upper_arm,
            right_forearm: b.right_forearm,
            left_upper_leg: b.left_upper_leg,
            left_lower_leg: b.left_lower_leg,
            right_upper_leg: b.right_upper_leg,
            right_lower_leg: b.right_lower_leg,
        }
    }
}

// ---------------------------------------------------------------------------
// System
// ---------------------------------------------------------------------------

/// Reads `PlayerFsm` + `Velocity`, writes `LocalTransform::rotation` on
/// character bone entities. Runs after `player_state_system` and before
/// `transform_propagation_system`.
pub fn animation_system(world: &mut World, dt: f32) {
    // --- Phase 1: collect data (shared borrows; query released after collect) ---
    struct FrameData {
        entity: Entity,
        state_changed: bool,
        state: PlayerState,
        horiz_speed: f32,
        phase: f32,
        blend: f32,
        blend_speed: f32,
        blend_from: Option<BonePose>,
        bones: BodyEntities,
    }

    let players: Vec<FrameData> = world
        .query::<(&PlayerFsm, &Velocity, &CharacterBody, &AnimationState)>()
        .iter()
        .map(|(e, (fsm, vel, body, anim))| {
            let horiz = Vec3::new(vel.0.x, 0.0, vel.0.z).length();
            FrameData {
                entity: e,
                state_changed: fsm.just_entered(),
                state: fsm.state.clone(),
                horiz_speed: horiz,
                phase: anim.phase,
                blend: anim.blend,
                blend_speed: anim.blend_speed,
                blend_from: anim.blend_from,
                bones: BodyEntities::from(body),
            }
        })
        .collect();

    // --- Phase 2: compute and apply (mutable access per entity) ---
    for pd in players {
        // On state transition: snapshot current bone rotations, reset phase.
        let (blend_from, blend, phase_start) = if pd.state_changed {
            let snap = {
                // Temporarily borrow CharacterBody to snapshot bone rotations.
                let body = world.get::<&CharacterBody>(pd.entity).unwrap();
                snapshot_bones(world, &*body)
            };
            (Some(snap), 0.0f32, 0.0f32)
        } else {
            (pd.blend_from, pd.blend, pd.phase)
        };

        // Advance blend toward 1.0.
        let blend = (blend + pd.blend_speed * dt).min(1.0);

        // Advance phase at a rate appropriate for the current state.
        let phase = phase_start
            + match &pd.state {
                PlayerState::Walking | PlayerState::Running => {
                    // Scale stride frequency with horizontal speed.
                    pd.horiz_speed * 1.6 * dt
                }
                PlayerState::Idle => {
                    // Slow breathing oscillation (~0.3 Hz).
                    std::f32::consts::TAU * 0.3 * dt
                }
                // Timed one-shot states: don't accumulate; timer drives animation directly.
                _ => 0.0,
            };

        // Compute the target pose for this frame.
        let target = compute_target(&pd.state, phase);

        // Crossfade from snapshot toward target.
        let final_pose = match blend_from {
            Some(ref from) if blend < 1.0 => slerp_pose(from, &target, blend),
            _ => target,
        };

        // Write updated AnimationState back to the player entity.
        {
            let mut anim = world.get::<&mut AnimationState>(pd.entity).unwrap();
            anim.phase = phase;
            anim.blend = blend;
            anim.blend_from = blend_from;
        }

        // Apply final bone rotations.
        apply_pose(world, &pd.bones, &final_pose);
    }
}
