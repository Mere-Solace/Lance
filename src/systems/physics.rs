use glam::Vec3;
use hecs::{Entity, World};

use crate::components::{Acceleration, Drag, GravityAffected, Held, LocalTransform, PreviousPosition, Velocity};

pub const PHYSICS_DT: f32 = 1.0 / 60.0;
const GRAVITY: Vec3 = Vec3::new(0.0, -9.81, 0.0);

/// Integrates one fixed physics step (PHYSICS_DT seconds) for all dynamic entities.
///
/// Snapshots previous positions for render interpolation, applies gravity, acceleration,
/// drag, and semi-implicit Euler integration.  Does NOT run collision detection — the
/// caller is responsible for calling `collision_system` after each `physics_step` and for
/// managing the fixed-timestep accumulator.
pub fn physics_step(world: &mut World) {
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
}
