use glam::{Quat, Vec3};
use hecs::World;
use sdl2::keyboard::Scancode;
use sdl2::mouse::MouseButton;

use crate::camera::Camera;
use crate::components::{
    add_child, remove_child, GlobalTransform, GrabState, Grabbable, Held, LocalTransform, Player,
    Static, Velocity,
};
use crate::engine::input::{InputEvent, InputState};

use super::raycast::raycast_grabbable;

const GRAB_DISTANCE: f32 = 5.0;
const HOLD_OFFSET: Vec3 = Vec3::new(0.0, 0.5, 1.5);
const HOLD_LERP_SPEED: f32 = 10.0;
const MIN_THROW_FORCE: f32 = 5.0;
const MAX_THROW_FORCE: f32 = 20.0;
const MAX_WIND_UP_TIME: f32 = 0.75;
const WIND_UP_MOVE_SLOWDOWN: f32 = 0.3;
const VELOCITY_SMOOTHING: f32 = 15.0;
const HELD_VELOCITY_DAMPER: f32 = 0.25;
const PITCH_ROTATION_LERP_SPEED: f32 = 12.0;

/// Grab/throw system. Returns movement speed multiplier (1.0 normal, 0.3 during wind-up).
pub fn grab_throw_system(
    world: &mut World,
    input: &InputState,
    camera: &Camera,
    dt: f32,
) -> f32 {
    // Get player's GrabState and entity
    let player_entity = {
        let mut found = None;
        for (entity, (_player, _grab)) in world.query::<(&Player, &GrabState)>().iter() {
            found = Some(entity);
            break;
        }
        match found {
            Some(e) => e,
            None => return 1.0,
        }
    };

    // Check for right-click pressed event (grab trigger: Alt + RightClick)
    let right_click_pressed = input.events.iter().any(|e| {
        matches!(e, InputEvent::MouseButtonPressed(MouseButton::Right))
    });
    let left_click_released = input.events.iter().any(|e| {
        matches!(e, InputEvent::MouseButtonReleased(MouseButton::Left))
    });
    let alt_held = input.is_key_held(Scancode::LAlt) || input.is_key_held(Scancode::RAlt);
    let right_held = input.is_mouse_button_held(MouseButton::Right);
    let left_held = input.is_mouse_button_held(MouseButton::Left);

    // Read current grab state
    let (held_entity, is_winding, wind_up_time, held_rotation, held_velocity) = {
        let grab = world.get::<&GrabState>(player_entity).unwrap();
        (grab.held_entity, grab.is_winding, grab.wind_up_time, grab.held_rotation, grab.held_velocity)
    };

    match held_entity {
        None => {
            // Not holding — check for grab attempt
            if right_click_pressed && alt_held {
                if let Some(hit) = raycast_grabbable(world, camera.position, camera.front(), GRAB_DISTANCE) {
                    // Don't grab static entities
                    if world.get::<&Static>(hit.entity).is_ok() {
                        return 1.0;
                    }
                    // Don't grab non-Grabbable (redundant since raycast filters, but safe)
                    if world.get::<&Grabbable>(hit.entity).is_err() {
                        return 1.0;
                    }

                    // Read player's world position and rotation for coordinate conversion
                    let (player_pos, player_yaw) = {
                        let lt = world.get::<&LocalTransform>(player_entity).unwrap();
                        (lt.position, lt.rotation)
                    };

                    // Read held entity's world position and rotation
                    let (held_world_pos, held_world_rot) = {
                        let lt = world.get::<&LocalTransform>(hit.entity).unwrap();
                        (lt.position, lt.rotation)
                    };

                    // Compute local offset relative to player
                    let world_offset = held_world_pos - player_pos;
                    let inv_yaw = player_yaw.inverse();
                    let local_offset = inv_yaw * world_offset;

                    // Re-parent held entity under player
                    add_child(world, player_entity, hit.entity);

                    // Set local transform relative to player
                    let local_rot = inv_yaw * held_world_rot;
                    if let Ok(mut lt) = world.get::<&mut LocalTransform>(hit.entity) {
                        lt.position = local_offset;
                        lt.rotation = local_rot;
                    }

                    // Mark as held, store the local rotation to keep it stable
                    let _ = world.insert_one(hit.entity, Held);
                    let mut grab = world.get::<&mut GrabState>(player_entity).unwrap();
                    grab.held_entity = Some(hit.entity);
                    grab.held_rotation = local_rot;
                    grab.wind_up_time = 0.0;
                    grab.is_winding = false;
                    grab.prev_world_pos = held_world_pos;
                    grab.held_velocity = Vec3::ZERO;
                }
            }
            1.0
        }
        Some(held) => {
            // Safety: check entity still exists
            if !world.contains(held) {
                let mut grab = world.get::<&mut GrabState>(player_entity).unwrap();
                grab.held_entity = None;
                grab.wind_up_time = 0.0;
                grab.is_winding = false;
                return 1.0;
            }

            // Drop when either Alt OR right-click is released (and not winding)
            let should_drop = (!alt_held || !right_held) && !is_winding;

            if should_drop {
                // Read world transform from GlobalTransform before un-parenting
                let (world_pos, world_rot) = extract_world_transform(world, held);

                // Un-parent from player
                remove_child(world, player_entity, held);

                // Restore world-space position and rotation
                if let Ok(mut lt) = world.get::<&mut LocalTransform>(held) {
                    lt.position = world_pos;
                    lt.rotation = world_rot;
                }
                let _ = world.remove_one::<Held>(held);
                if let Ok(mut vel) = world.get::<&mut Velocity>(held) {
                    vel.0 = held_velocity;
                }
                let mut grab = world.get::<&mut GrabState>(player_entity).unwrap();
                grab.held_entity = None;
                grab.wind_up_time = 0.0;
                grab.is_winding = false;
                grab.held_velocity = Vec3::ZERO;
                return 1.0;
            }

            // Compute pitch rotation from camera and apply to hold offset + rotation
            let pitch_quat = Quat::from_rotation_x(-camera.pitch.to_radians());
            let target_pos = pitch_quat * HOLD_OFFSET;
            let target_rot = pitch_quat * held_rotation;

            // Lerp local position and rotation toward pitch-adjusted targets
            if let Ok(mut lt) = world.get::<&mut LocalTransform>(held) {
                let pos_diff = target_pos - lt.position;
                lt.position += pos_diff * (HOLD_LERP_SPEED * dt).min(1.0);
                lt.rotation = lt.rotation.slerp(target_rot, (PITCH_ROTATION_LERP_SPEED * dt).min(1.0));
            }
            // Zero velocity while held
            if let Ok(mut vel) = world.get::<&mut Velocity>(held) {
                vel.0 = Vec3::ZERO;
            }

            // Track world-space velocity of the held object
            {
                let (current_world_pos, _) = extract_world_transform(world, held);
                let mut grab = world.get::<&mut GrabState>(player_entity).unwrap();
                if dt > 0.0 {
                    let frame_vel = (current_world_pos - grab.prev_world_pos) / dt;
                    // Exponential smoothing to avoid jitter
                    let smoothing = (VELOCITY_SMOOTHING * dt).min(1.0);
                    grab.held_velocity = grab.held_velocity.lerp(frame_vel, smoothing);
                }
                grab.prev_world_pos = current_world_pos;
            }

            // Wind-up with left click
            if left_held {
                let mut grab = world.get::<&mut GrabState>(player_entity).unwrap();
                grab.is_winding = true;
                grab.wind_up_time = (grab.wind_up_time + dt).min(MAX_WIND_UP_TIME);
                return WIND_UP_MOVE_SLOWDOWN;
            }

            // Throw on left click release while winding
            if left_click_released && is_winding {
                let throw_t = (wind_up_time / MAX_WIND_UP_TIME).clamp(0.0, 1.0);
                let force = MIN_THROW_FORCE + (MAX_THROW_FORCE - MIN_THROW_FORCE) * throw_t;
                let throw_vel = camera.front() * force + HELD_VELOCITY_DAMPER * held_velocity;

                // Read world transform from GlobalTransform before un-parenting
                let (world_pos, world_rot) = extract_world_transform(world, held);

                // Un-parent from player
                remove_child(world, player_entity, held);

                // Restore world-space position and rotation, apply throw velocity
                if let Ok(mut lt) = world.get::<&mut LocalTransform>(held) {
                    lt.position = world_pos;
                    lt.rotation = world_rot;
                }
                let _ = world.remove_one::<Held>(held);
                if let Ok(mut vel) = world.get::<&mut Velocity>(held) {
                    vel.0 = throw_vel;
                }
                let mut grab = world.get::<&mut GrabState>(player_entity).unwrap();
                grab.held_entity = None;
                grab.wind_up_time = 0.0;
                grab.is_winding = false;
                grab.held_velocity = Vec3::ZERO;
                return 1.0;
            }

            // Still holding, not winding
            1.0
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
