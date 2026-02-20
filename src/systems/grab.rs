use glam::{Quat, Vec3};
use hecs::World;
use sdl2::keyboard::Scancode;
use sdl2::mouse::MouseButton;

use crate::camera::Camera;
use crate::components::{
    add_child, remove_child, Collider, GlobalTransform, GrabState, Grabbable, Held, LocalTransform,
    NoSelfCollision, Player, Static, Velocity,
};
use crate::engine::input::{InputEvent, InputState};

use super::collision::query_collisions_at;
use super::raycast::raycast_grabbable;

const GRAB_DISTANCE: f32 = 5.0;
const HOLD_RESOLVE_ITERS: usize = 3;
const HOLD_PUSH_IMPULSE: f32 = 3.0;
const HOLD_OFFSET: Vec3 = Vec3::new(0.0, 0.5, 1.5);
const HOLD_LERP_SPEED: f32 = 10.0;
const MIN_THROW_FORCE: f32 = 5.0;
const MAX_THROW_FORCE: f32 = 20.0;
const MAX_WIND_UP_TIME: f32 = 0.75;
const WIND_UP_MOVE_SLOWDOWN: f32 = 0.3;
const VELOCITY_SMOOTHING: f32 = 15.0;
const HELD_VELOCITY_DAMPER: f32 = 0.25;
const DROP_VELOCITY_DAMPER: f32 = 0.05;
const CHEST_HEIGHT: f32 = 0.5;
const PITCH_ROTATION_LERP_SPEED: f32 = 12.0;
/// Lateral (XZ) displacement from `world_target` beyond which the held object is considered
/// wall-blocked. Used to trigger the yaw-lock mechanic.
const BLOCK_DETECT_THRESHOLD: f32 = 0.05;
/// Extra degrees the player may rotate in the blocked direction after a wall block is first
/// detected, before the yaw is frozen. The rubber band snaps after this much give.
const YAW_GRACE_DEGREES: f32 = 10.0;

/// Build the entity skip list for hold collision queries: held object, player root, all body parts.
fn build_hold_skip_list(
    world: &World,
    held_entity: hecs::Entity,
    player_entity: hecs::Entity,
) -> Vec<hecs::Entity> {
    let mut skip = vec![held_entity, player_entity];
    for (entity, nsc) in world.query::<&NoSelfCollision>().iter() {
        if nsc.0 == player_entity {
            skip.push(entity);
        }
    }
    skip
}

/// Resolve a held object's world position against world colliders using `skip` as the exclusion list.
/// Dynamic objects that overlap receive a push impulse.
fn resolve_held_pos(
    world: &mut World,
    collider: &Collider,
    world_target: Vec3,
    skip: &[hecs::Entity],
) -> Vec3 {
    let mut pos = world_target;
    for _ in 0..HOLD_RESOLVE_ITERS {
        let overlaps = query_collisions_at(world, collider, pos, skip);
        if overlaps.is_empty() {
            break;
        }
        for (push, depth, other, is_dynamic) in overlaps {
            pos += push * depth;
            if is_dynamic {
                if let Ok(mut vel) = world.get::<&mut Velocity>(other) {
                    vel.0 -= push * HOLD_PUSH_IMPULSE;
                }
            }
        }
    }
    pos
}

/// Grab/throw system.
///
/// Returns `(speed_multiplier, yaw_lock)`:
/// - `speed_multiplier`: 1.0 normally, [`WIND_UP_MOVE_SLOWDOWN`] during throw wind-up.
/// - `yaw_lock`: `Some((clamp_yaw, block_dir))` when the held object is laterally blocked by a
///   wall and the player is trying to rotate into it. `block_dir` is `+1.0` (blocked turning
///   right) or `-1.0` (blocked turning left). `None` when rotation is free.
/// - `move_block`: horizontal unit vector pointing *away* from the blocking wall. The caller
///   should remove any movement component pointing toward the wall (negative dot with this vector).
///   `None` when the held object is unblocked.
///
/// The caller (`app.rs`) applies the yaw clamp and passes `move_block` to `player_movement_system`.
pub fn grab_throw_system(
    world: &mut World,
    input: &InputState,
    camera: &Camera,
    dt: f32,
) -> (f32, Option<(f32, f32)>, Option<Vec3>) {
    // Get player entity.
    let player_entity = {
        let mut found = None;
        for (entity, (_player, _grab)) in world.query::<(&Player, &GrabState)>().iter() {
            found = Some(entity);
            break;
        }
        match found {
            Some(e) => e,
            None => return (1.0, None, None),
        }
    };

    let right_click_pressed = input.events.iter().any(|e| {
        matches!(e, InputEvent::MouseButtonPressed(MouseButton::Right))
    });
    let left_click_released = input.events.iter().any(|e| {
        matches!(e, InputEvent::MouseButtonReleased(MouseButton::Left))
    });
    let alt_held = input.is_key_held(Scancode::LAlt) || input.is_key_held(Scancode::RAlt);
    let right_held = input.is_mouse_button_held(MouseButton::Right);
    let left_held = input.is_mouse_button_held(MouseButton::Left);

    let (held_entity, is_winding, wind_up_time, held_rotation, held_velocity) = {
        let grab = world.get::<&GrabState>(player_entity).unwrap();
        (grab.held_entity, grab.is_winding, grab.wind_up_time, grab.held_rotation, grab.held_velocity)
    };

    match held_entity {
        None => {
            // Not holding — check for grab attempt.
            if right_click_pressed && alt_held {
                let chest_pos = {
                    let lt = world.get::<&LocalTransform>(player_entity).unwrap();
                    lt.position + Vec3::Y * CHEST_HEIGHT
                };
                if let Some(hit) = raycast_grabbable(world, chest_pos, camera.front(), GRAB_DISTANCE) {
                    if world.get::<&Static>(hit.entity).is_ok() {
                        return (1.0, None, None);
                    }
                    if world.get::<&Grabbable>(hit.entity).is_err() {
                        return (1.0, None, None);
                    }

                    let (player_pos, player_yaw) = {
                        let lt = world.get::<&LocalTransform>(player_entity).unwrap();
                        (lt.position, lt.rotation)
                    };
                    let (held_world_pos, held_world_rot) = {
                        let lt = world.get::<&LocalTransform>(hit.entity).unwrap();
                        (lt.position, lt.rotation)
                    };

                    let world_offset = held_world_pos - player_pos;
                    let inv_yaw = player_yaw.inverse();
                    let local_offset = inv_yaw * world_offset;

                    add_child(world, player_entity, hit.entity);

                    let local_rot = inv_yaw * held_world_rot;
                    if let Ok(mut lt) = world.get::<&mut LocalTransform>(hit.entity) {
                        lt.position = local_offset;
                        lt.rotation = local_rot;
                    }

                    // NoSelfCollision lets collision_system treat the object as a kinematic
                    // obstacle that blocks the player's capsule while ignoring limbs/head.
                    let _ = world.insert_one(hit.entity, Held);
                    let _ = world.insert_one(hit.entity, NoSelfCollision(player_entity));
                    let mut grab = world.get::<&mut GrabState>(player_entity).unwrap();
                    grab.held_entity = Some(hit.entity);
                    grab.held_rotation = local_rot;
                    grab.wind_up_time = 0.0;
                    grab.is_winding = false;
                    grab.prev_world_pos = held_world_pos;
                    grab.held_velocity = Vec3::ZERO;
                }
            }
            (1.0, None, None)
        }
        Some(held) => {
            // Safety: check entity still exists.
            if !world.contains(held) {
                let mut grab = world.get::<&mut GrabState>(player_entity).unwrap();
                grab.held_entity = None;
                grab.wind_up_time = 0.0;
                grab.is_winding = false;
                grab.yaw_lock = None;
                return (1.0, None, None);
            }

            // Drop when either Alt OR right-click is released (and not winding).
            let should_drop = (!alt_held || !right_held) && !is_winding;
            if should_drop {
                let (world_pos, world_rot) = extract_world_transform(world, held);
                remove_child(world, player_entity, held);
                if let Ok(mut lt) = world.get::<&mut LocalTransform>(held) {
                    lt.position = world_pos;
                    lt.rotation = world_rot;
                }
                let _ = world.remove_one::<Held>(held);
                let _ = world.remove_one::<NoSelfCollision>(held);
                if let Ok(mut vel) = world.get::<&mut Velocity>(held) {
                    vel.0 = held_velocity * DROP_VELOCITY_DAMPER;
                }
                let mut grab = world.get::<&mut GrabState>(player_entity).unwrap();
                grab.held_entity = None;
                grab.wind_up_time = 0.0;
                grab.is_winding = false;
                grab.held_velocity = Vec3::ZERO;
                grab.yaw_lock = None;
                return (1.0, None, None);
            }

            // Compute pitch rotation from camera and apply to hold offset + rotation.
            let pitch_quat = Quat::from_rotation_x(-camera.pitch.to_radians());
            let target_pos = pitch_quat * HOLD_OFFSET;
            let target_rot = pitch_quat * held_rotation;

            // Resolve held object against world geometry in world space, then convert back to local.
            let (player_pos, player_yaw) = {
                let lt = world.get::<&LocalTransform>(player_entity).unwrap();
                (lt.position, lt.rotation)
            };
            let world_target = player_pos + player_yaw * target_pos;
            let collider_copy: Option<Collider> = world.get::<&Collider>(held).ok().map(|c| match &*c {
                Collider::Sphere { radius } => Collider::Sphere { radius: *radius },
                Collider::Capsule { radius, height } => Collider::Capsule { radius: *radius, height: *height },
                Collider::Plane { normal, offset } => Collider::Plane { normal: *normal, offset: *offset },
                Collider::Box { half_extents } => Collider::Box { half_extents: *half_extents },
            });
            let skip = build_hold_skip_list(world, held, player_entity);

            // Resolve against geometry. Lateral (XZ) displacement between the resolved position
            // and world_target indicates a wall is blocking the object. When that happens and the
            // player is rotating into the wall, start (or maintain) a yaw lock: the player gets
            // YAW_GRACE_DEGREES of further rotation, then yaw freezes until the ball is freed.
            let (effective_target, new_yaw_lock, new_move_block) = if let Some(ref coll) = collider_copy {
                let resolved = resolve_held_pos(world, coll, world_target, &skip);

                let disp = resolved - world_target;
                let horiz = Vec3::new(disp.x, 0.0, disp.z);
                let lateral_disp = horiz.length();

                let prev_lock = world.get::<&GrabState>(player_entity).ok().and_then(|g| g.yaw_lock);

                let new_lock = if lateral_disp > BLOCK_DETECT_THRESHOLD && input.mouse_dx != 0.0 {
                    let dir = input.mouse_dx.signum();
                    match prev_lock {
                        // Already locked in the same direction: keep the existing clamp unchanged
                        // so the grace window doesn't reset every frame.
                        Some((ly, pd)) if pd == dir => Some((ly, pd)),
                        // New block, or player reversed and hit again: fresh grace window.
                        _ => Some((camera.yaw + dir * YAW_GRACE_DEGREES, dir)),
                    }
                } else if lateral_disp > BLOCK_DETECT_THRESHOLD {
                    // Still blocked but not rotating: preserve any existing lock.
                    prev_lock
                } else {
                    None // unblocked: release yaw lock
                };

                // Movement block: unit vector pointing away from the wall. player_movement_system
                // zeroes out movement components pointing toward the wall (negative dot).
                let move_block = if lateral_disp > BLOCK_DETECT_THRESHOLD {
                    Some(horiz / lateral_disp) // already nonzero — lateral_disp > threshold
                } else {
                    None
                };

                (player_yaw.inverse() * (resolved - player_pos), new_lock, move_block)
            } else {
                (target_pos, None, None)
            };

            // Lerp local position and rotation toward collision-resolved targets.
            if let Ok(mut lt) = world.get::<&mut LocalTransform>(held) {
                let pos_diff = effective_target - lt.position;
                lt.position += pos_diff * (HOLD_LERP_SPEED * dt).min(1.0);
                lt.rotation = lt.rotation.slerp(target_rot, (PITCH_ROTATION_LERP_SPEED * dt).min(1.0));
            }
            // Zero velocity while held.
            if let Ok(mut vel) = world.get::<&mut Velocity>(held) {
                vel.0 = Vec3::ZERO;
            }

            // Track world-space velocity and persist the updated yaw lock.
            {
                let (current_world_pos, _) = extract_world_transform(world, held);
                let mut grab = world.get::<&mut GrabState>(player_entity).unwrap();
                if dt > 0.0 {
                    let frame_vel = (current_world_pos - grab.prev_world_pos) / dt;
                    let smoothing = (VELOCITY_SMOOTHING * dt).min(1.0);
                    grab.held_velocity = grab.held_velocity.lerp(frame_vel, smoothing);
                }
                grab.prev_world_pos = current_world_pos;
                grab.yaw_lock = new_yaw_lock;
            }

            // Wind-up with left click.
            if left_held {
                let mut grab = world.get::<&mut GrabState>(player_entity).unwrap();
                grab.is_winding = true;
                grab.wind_up_time = (grab.wind_up_time + dt).min(MAX_WIND_UP_TIME);
                return (WIND_UP_MOVE_SLOWDOWN, new_yaw_lock, new_move_block);
            }

            // Throw on left click release while winding.
            if left_click_released && is_winding {
                let throw_t = (wind_up_time / MAX_WIND_UP_TIME).clamp(0.0, 1.0);
                let force = MIN_THROW_FORCE + (MAX_THROW_FORCE - MIN_THROW_FORCE) * throw_t;
                let throw_vel = camera.front() * force + HELD_VELOCITY_DAMPER * held_velocity;

                let (world_pos, world_rot) = extract_world_transform(world, held);
                remove_child(world, player_entity, held);
                if let Ok(mut lt) = world.get::<&mut LocalTransform>(held) {
                    lt.position = world_pos;
                    lt.rotation = world_rot;
                }
                let _ = world.remove_one::<Held>(held);
                let _ = world.remove_one::<NoSelfCollision>(held);
                if let Ok(mut vel) = world.get::<&mut Velocity>(held) {
                    vel.0 = throw_vel;
                }
                let mut grab = world.get::<&mut GrabState>(player_entity).unwrap();
                grab.held_entity = None;
                grab.wind_up_time = 0.0;
                grab.is_winding = false;
                grab.held_velocity = Vec3::ZERO;
                grab.yaw_lock = None;
                return (1.0, None, None);
            }

            // Still holding, not winding.
            (1.0, new_yaw_lock, new_move_block)
        }
    }
}

/// Extract world-space position and rotation from an entity's GlobalTransform.
fn extract_world_transform(world: &World, entity: hecs::Entity) -> (Vec3, Quat) {
    world
        .get::<&GlobalTransform>(entity)
        .map(|gt| {
            let (_scale, rot, pos) = gt.0.to_scale_rotation_translation();
            (pos, rot)
        })
        .unwrap_or_else(|_| {
            world
                .get::<&LocalTransform>(entity)
                .map(|lt| (lt.position, lt.rotation))
                .unwrap_or((Vec3::ZERO, Quat::IDENTITY))
        })
}
