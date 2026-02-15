use glam::Vec3;
use hecs::World;

use crate::components::{Acceleration, CollisionEvent, GravityAffected, LocalTransform, Velocity};
use super::collision::collision_system;

const PHYSICS_DT: f32 = 1.0 / 60.0;
const GRAVITY: Vec3 = Vec3::new(0.0, -9.81, 0.0);

pub fn physics_system(world: &mut World, accumulator: &mut f32, frame_dt: f32) -> Vec<CollisionEvent> {
    *accumulator += frame_dt;
    let mut all_events = Vec::new();

    while *accumulator >= PHYSICS_DT {
        // 1. Integrate velocity + position
        for (_entity, (local, vel, accel, gravity)) in world
            .query_mut::<(
                &mut LocalTransform,
                &mut Velocity,
                Option<&Acceleration>,
                Option<&GravityAffected>,
            )>()
        {
            if gravity.is_some() {
                vel.0 += GRAVITY * PHYSICS_DT;
            }
            if let Some(accel) = accel {
                vel.0 += accel.0 * PHYSICS_DT;
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
