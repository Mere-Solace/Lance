use glam::Vec3;
use hecs::World;

use crate::components::{Acceleration, CollisionEvent, Drag, GravityAffected, Held, LocalTransform, Velocity};
use super::collision::collision_system;

const PHYSICS_DT: f32 = 1.0 / 60.0;
const GRAVITY: Vec3 = Vec3::new(0.0, -9.81, 0.0);

pub fn physics_system(world: &mut World, accumulator: &mut f32, frame_dt: f32) -> Vec<CollisionEvent> {
    *accumulator += frame_dt;
    let mut all_events = Vec::new();

    while *accumulator >= PHYSICS_DT {
        // 1. Integrate velocity + position
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

        // 2. Detect & resolve collisions
        let events = collision_system(world);
        all_events.extend(events);

        *accumulator -= PHYSICS_DT;
    }

    all_events
}
