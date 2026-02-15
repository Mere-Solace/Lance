use glam::Vec3;
use hecs::World;
use sdl2::keyboard::Scancode;

use crate::camera::Camera;
use crate::components::{CollisionEvent, Grounded, Player, Velocity};
use crate::engine::input::InputState;

const PLAYER_MOVE_SPEED: f32 = 6.0;
const JUMP_IMPULSE: f32 = 7.0;

pub fn player_movement_system(world: &mut World, input: &InputState, camera: &Camera) {
    let yaw_rad = camera.yaw.to_radians();
    let forward = Vec3::new(yaw_rad.cos(), 0.0, yaw_rad.sin()).normalize();
    let right = forward.cross(Vec3::Y).normalize();

    let mut move_dir = Vec3::ZERO;
    if input.is_key_held(Scancode::W) {
        move_dir += forward;
    }
    if input.is_key_held(Scancode::S) {
        move_dir -= forward;
    }
    if input.is_key_held(Scancode::A) {
        move_dir -= right;
    }
    if input.is_key_held(Scancode::D) {
        move_dir += right;
    }

    let horizontal = if move_dir.length_squared() > 0.0 {
        move_dir.normalize() * PLAYER_MOVE_SPEED
    } else {
        Vec3::ZERO
    };

    for (_entity, (vel, _player, grounded)) in
        world.query_mut::<(&mut Velocity, &Player, Option<&Grounded>)>()
    {
        vel.0.x = horizontal.x;
        vel.0.z = horizontal.z;

        if grounded.is_some() && input.is_key_held(Scancode::Space) {
            vel.0.y = JUMP_IMPULSE;
        }
    }
}

pub fn grounded_system(world: &mut World, events: &[CollisionEvent]) {
    // Remove Grounded from all player entities
    let players: Vec<_> = world
        .query_mut::<(&Player, &Grounded)>()
        .into_iter()
        .map(|(e, _)| e)
        .collect();
    for entity in players {
        let _ = world.remove_one::<Grounded>(entity);
    }

    // Check collision events for ground contacts
    for event in events {
        // Normal points A→B. Push direction for A is -normal, for B is +normal.
        let a_is_player = world.get::<&Player>(event.entity_a).is_ok();
        let b_is_player = world.get::<&Player>(event.entity_b).is_ok();

        if a_is_player {
            let push_dir = -event.contact_normal;
            if push_dir.dot(Vec3::Y) > 0.7 {
                let _ = world.insert_one(event.entity_a, Grounded);
            }
        }
        if b_is_player {
            let push_dir = event.contact_normal;
            if push_dir.dot(Vec3::Y) > 0.7 {
                let _ = world.insert_one(event.entity_b, Grounded);
            }
        }
    }
}
