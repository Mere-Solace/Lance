use glam::Vec3;
use hecs::{Entity, World};

use crate::components::{Acceleration, CollisionEvent, Drag, GravityAffected, Held, LocalTransform, PreviousPosition, Velocity};
use super::collision::collision_system;

const PHYSICS_DT: f32 = 1.0 / 60.0;
const GRAVITY: Vec3 = Vec3::new(0.0, -9.81, 0.0);

/// Returns all collision events, the interpolation alpha (0..1), and the number of physics
/// ticks that ran this frame.
///
/// alpha = remaining_accumulator / PHYSICS_DT — used to lerp between previous and current
/// physics positions in the transform propagation system.
///
/// The tick count is used by `grounded_system` to skip clearing the Grounded marker on
/// frames where no physics ticks ran (high framerate case), preventing false Falling
/// transitions when the render rate exceeds the fixed physics rate.
pub fn physics_system(world: &mut World, accumulator: &mut f32, frame_dt: f32) -> (Vec<CollisionEvent>, f32, usize) {
    *accumulator += frame_dt;
    let mut all_events = Vec::new();
    let mut ticks = 0usize;

    while *accumulator >= PHYSICS_DT {
        ticks += 1;
        // Snapshot previous positions for render interpolation.
        // Collect first (drops the borrow), then insert/update.
        let prev_snapshots: Vec<(Entity, Vec3)> = world
            .query::<&LocalTransform>()
            .with::<&Velocity>()
            .without::<&Held>()
            .iter()
            .map(|(e, lt)| (e, lt.position))
            .collect();

        // insert_one replaces the component if it already exists.
        for (entity, pos) in prev_snapshots {
            let _ = world.insert_one(entity, PreviousPosition(pos));
        }

        // Integrate velocity + position
        for (_entity, (local, vel, accel, gravity, drag, held)) in world
            .query_mut::<(
                &mut LocalTransform,
                &mut Velocity,
                Option<&Acceleration>,
                Option<&GravityAffected>,
                Option<&Drag>,
                Option<&Held>,
            )>()
        {
            if held.is_some() {
                continue;
            }
            if gravity.is_some() {
                vel.0 += GRAVITY * PHYSICS_DT;
            }
            if let Some(accel) = accel {
                vel.0 += accel.0 * PHYSICS_DT;
            }
            // Apply drag: vel *= (1 - drag * dt)
            if let Some(drag) = drag {
                let damping = (1.0 - drag.0 * PHYSICS_DT).max(0.0);
                vel.0 *= damping;
            }
            // Semi-implicit Euler: update velocity first, then position
            local.position += vel.0 * PHYSICS_DT;
        }

        // Detect & resolve collisions
        let events = collision_system(world);
        all_events.extend(events);

        *accumulator -= PHYSICS_DT;
    }

    // alpha: how far into the next physics step this render frame falls.
    // Used to interpolate entity positions for smooth rendering.
    let alpha = *accumulator / PHYSICS_DT;
    (all_events, alpha, ticks)
}
