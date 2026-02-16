use glam::Vec3;
use hecs::{Entity, World};

use crate::components::{Collider, CollisionEvent, Friction, Held, LocalTransform, Restitution, Static, Velocity};

struct ColliderEntry {
    entity: Entity,
    position: Vec3,
    collider_kind: ColliderKind,
}

enum ColliderKind {
    Sphere { radius: f32 },
    Capsule { radius: f32, half_height: f32 },
    Plane { normal: Vec3, offset: f32 },
}

fn closest_point_on_segment(a: Vec3, b: Vec3, p: Vec3) -> Vec3 {
    let ab = b - a;
    let len_sq = ab.length_squared();
    if len_sq < 1e-12 {
        return a;
    }
    let t = ((p - a).dot(ab) / len_sq).clamp(0.0, 1.0);
    a + ab * t
}

/// All returned normals point from entity_a toward entity_b.
fn test_pair(a: &ColliderEntry, b: &ColliderEntry) -> Option<CollisionEvent> {
    match (&a.collider_kind, &b.collider_kind) {
        // Sphere(A) vs Plane(B): normal points from sphere toward plane = -plane_normal
        (ColliderKind::Sphere { radius }, ColliderKind::Plane { normal, offset }) => {
            let dist = a.position.dot(*normal) - offset;
            let penetration = radius - dist;
            if penetration > 0.0 {
                Some(CollisionEvent {
                    entity_a: a.entity,
                    entity_b: b.entity,
                    contact_normal: -*normal,
                    penetration_depth: penetration,
                })
            } else {
                None
            }
        }
        // Plane(A) vs Sphere(B): canonicalize so sphere=entity_a, plane=entity_b
        (ColliderKind::Plane { normal, offset }, ColliderKind::Sphere { radius }) => {
            let dist = b.position.dot(*normal) - offset;
            let penetration = radius - dist;
            if penetration > 0.0 {
                Some(CollisionEvent {
                    entity_a: b.entity,
                    entity_b: a.entity,
                    contact_normal: -*normal,
                    penetration_depth: penetration,
                })
            } else {
                None
            }
        }

        // Sphere(A) vs Sphere(B): normal = (B - A).normalize()
        (ColliderKind::Sphere { radius: r1 }, ColliderKind::Sphere { radius: r2 }) => {
            let diff = b.position - a.position;
            let dist = diff.length();
            let penetration = (r1 + r2) - dist;
            if penetration > 0.0 {
                let normal = if dist > 1e-6 { diff / dist } else { Vec3::Y };
                Some(CollisionEvent {
                    entity_a: a.entity,
                    entity_b: b.entity,
                    contact_normal: normal,
                    penetration_depth: penetration,
                })
            } else {
                None
            }
        }

        // Capsule(A) vs Plane(B): normal = -plane_normal (toward plane)
        (ColliderKind::Capsule { radius, half_height }, ColliderKind::Plane { normal, offset }) => {
            let top = a.position + Vec3::Y * *half_height;
            let bottom = a.position - Vec3::Y * *half_height;
            let dist_top = top.dot(*normal) - offset;
            let dist_bottom = bottom.dot(*normal) - offset;
            let min_dist = dist_top.min(dist_bottom);
            let penetration = radius - min_dist;
            if penetration > 0.0 {
                Some(CollisionEvent {
                    entity_a: a.entity,
                    entity_b: b.entity,
                    contact_normal: -*normal,
                    penetration_depth: penetration,
                })
            } else {
                None
            }
        }
        // Plane(A) vs Capsule(B): canonicalize so capsule=entity_a, plane=entity_b
        (ColliderKind::Plane { normal, offset }, ColliderKind::Capsule { radius, half_height }) => {
            let top = b.position + Vec3::Y * *half_height;
            let bottom = b.position - Vec3::Y * *half_height;
            let dist_top = top.dot(*normal) - offset;
            let dist_bottom = bottom.dot(*normal) - offset;
            let min_dist = dist_top.min(dist_bottom);
            let penetration = radius - min_dist;
            if penetration > 0.0 {
                Some(CollisionEvent {
                    entity_a: b.entity,
                    entity_b: a.entity,
                    contact_normal: -*normal,
                    penetration_depth: penetration,
                })
            } else {
                None
            }
        }

        // Capsule(A) vs Sphere(B): normal from A's closest point toward B
        (ColliderKind::Capsule { radius: cr, half_height }, ColliderKind::Sphere { radius: sr }) => {
            let top = a.position + Vec3::Y * *half_height;
            let bottom = a.position - Vec3::Y * *half_height;
            let closest = closest_point_on_segment(bottom, top, b.position);
            let diff = b.position - closest;
            let dist = diff.length();
            let penetration = (cr + sr) - dist;
            if penetration > 0.0 {
                let normal = if dist > 1e-6 { diff / dist } else { Vec3::Y };
                Some(CollisionEvent {
                    entity_a: a.entity,
                    entity_b: b.entity,
                    contact_normal: normal,
                    penetration_depth: penetration,
                })
            } else {
                None
            }
        }
        // Sphere(A) vs Capsule(B): normal from A toward B's closest point
        (ColliderKind::Sphere { radius: sr }, ColliderKind::Capsule { radius: cr, half_height }) => {
            let top = b.position + Vec3::Y * *half_height;
            let bottom = b.position - Vec3::Y * *half_height;
            let closest = closest_point_on_segment(bottom, top, a.position);
            let diff = closest - a.position;
            let dist = diff.length();
            let penetration = (cr + sr) - dist;
            if penetration > 0.0 {
                let normal = if dist > 1e-6 { diff / dist } else { Vec3::Y };
                Some(CollisionEvent {
                    entity_a: a.entity,
                    entity_b: b.entity,
                    contact_normal: normal,
                    penetration_depth: penetration,
                })
            } else {
                None
            }
        }

        // Plane vs Plane, Capsule vs Capsule — skip for now
        _ => None,
    }
}

const REST_VELOCITY_THRESHOLD: f32 = 0.5;
const DEFAULT_RESTITUTION: f32 = 0.3;
const DEFAULT_FRICTION: f32 = 0.5;
const PHYSICS_DT: f32 = 1.0 / 60.0;

/// Apply Coulomb friction: reduce tangential velocity proportional to normal impulse.
/// Clamps so friction never reverses the sliding direction.
fn apply_friction(vel: &mut Vec3, normal: Vec3, mu: f32, normal_impulse: f32) {
    let tangent_vel = *vel - vel.dot(normal) * normal;
    let tangent_speed = tangent_vel.length();
    if tangent_speed < 1e-6 {
        return;
    }
    let tangent_dir = tangent_vel / tangent_speed;
    // Friction impulse magnitude, clamped to not exceed tangential speed
    let friction_impulse = (mu * normal_impulse * PHYSICS_DT).min(tangent_speed);
    *vel -= tangent_dir * friction_impulse;
}

/// Detect collisions and apply impulse-based response.
/// contact_normal convention: always points from entity_a toward entity_b.
/// - To push A out of B: move A along -normal
/// - To push B out of A: move B along +normal
pub fn collision_system(world: &mut World) -> Vec<CollisionEvent> {
    // Gather all collider entries
    let entries: Vec<ColliderEntry> = world
        .query_mut::<(&LocalTransform, &Collider, Option<&Held>)>()
        .into_iter()
        .filter(|(_entity, (_local, _collider, held))| held.is_none())
        .map(|(entity, (local, collider, _held))| {
            let kind = match collider {
                Collider::Sphere { radius } => ColliderKind::Sphere { radius: *radius },
                Collider::Capsule { radius, height } => ColliderKind::Capsule {
                    radius: *radius,
                    half_height: height * 0.5,
                },
                Collider::Plane { normal, offset } => ColliderKind::Plane {
                    normal: *normal,
                    offset: *offset,
                },
            };
            ColliderEntry {
                entity,
                position: local.position,
                collider_kind: kind,
            }
        })
        .collect();

    // Broadphase: brute force O(n²)
    let mut events = Vec::new();
    for i in 0..entries.len() {
        for j in (i + 1)..entries.len() {
            if let Some(event) = test_pair(&entries[i], &entries[j]) {
                events.push(event);
            }
        }
    }

    // Response — normal points from A to B in all cases
    for event in &events {
        let a_static = world.get::<&Static>(event.entity_a).is_ok();
        let b_static = world.get::<&Static>(event.entity_b).is_ok();

        if a_static && b_static {
            continue;
        }

        let restitution_a = world
            .get::<&Restitution>(event.entity_a)
            .map(|r| r.0)
            .unwrap_or(DEFAULT_RESTITUTION);
        let restitution_b = world
            .get::<&Restitution>(event.entity_b)
            .map(|r| r.0)
            .unwrap_or(DEFAULT_RESTITUTION);
        let e = (restitution_a + restitution_b) * 0.5;

        let friction_a = world
            .get::<&Friction>(event.entity_a)
            .map(|f| f.0)
            .unwrap_or(DEFAULT_FRICTION);
        let friction_b = world
            .get::<&Friction>(event.entity_b)
            .map(|f| f.0)
            .unwrap_or(DEFAULT_FRICTION);
        let mu = (friction_a + friction_b) * 0.5;

        let n = event.contact_normal;
        let depth = event.penetration_depth;

        if a_static {
            // A is static, B is dynamic — push B away from A (along +normal)
            if let Ok(mut local) = world.get::<&mut LocalTransform>(event.entity_b) {
                local.position += n * depth;
            }
            if let Ok(mut vel) = world.get::<&mut Velocity>(event.entity_b) {
                let vel_along_n = vel.0.dot(n);
                // Negative = B moving toward A (into collision)
                if vel_along_n < 0.0 {
                    let normal_impulse = if vel_along_n.abs() < REST_VELOCITY_THRESHOLD {
                        vel.0 -= vel_along_n * n;
                        vel_along_n.abs()
                    } else {
                        vel.0 -= (1.0 + e) * vel_along_n * n;
                        (1.0 + e) * vel_along_n.abs()
                    };

                    // Coulomb friction: reduce tangential velocity
                    apply_friction(&mut vel.0, n, mu, normal_impulse);
                }
            }
        } else if b_static {
            // B is static, A is dynamic — push A away from B (along -normal)
            if let Ok(mut local) = world.get::<&mut LocalTransform>(event.entity_a) {
                local.position -= n * depth;
            }
            if let Ok(mut vel) = world.get::<&mut Velocity>(event.entity_a) {
                let vel_along_n = vel.0.dot(n);
                // Positive = A moving toward B (into collision)
                if vel_along_n > 0.0 {
                    let normal_impulse = if vel_along_n < REST_VELOCITY_THRESHOLD {
                        vel.0 -= vel_along_n * n;
                        vel_along_n
                    } else {
                        vel.0 -= (1.0 + e) * vel_along_n * n;
                        (1.0 + e) * vel_along_n
                    };

                    // Coulomb friction: reduce tangential velocity
                    apply_friction(&mut vel.0, n, mu, normal_impulse);
                }
            }
        } else {
            // Both dynamic — split push 50/50
            if let Ok(mut local) = world.get::<&mut LocalTransform>(event.entity_a) {
                local.position -= n * (depth * 0.5);
            }
            if let Ok(mut local) = world.get::<&mut LocalTransform>(event.entity_b) {
                local.position += n * (depth * 0.5);
            }

            let vel_a = world.get::<&Velocity>(event.entity_a).map(|v| v.0).unwrap_or(Vec3::ZERO);
            let vel_b = world.get::<&Velocity>(event.entity_b).map(|v| v.0).unwrap_or(Vec3::ZERO);
            let relative_vel = vel_a - vel_b;
            let vel_along_n = relative_vel.dot(n);

            // Positive = A approaching B
            if vel_along_n > 0.0 {
                let impulse = if vel_along_n < REST_VELOCITY_THRESHOLD {
                    vel_along_n * 0.5
                } else {
                    (1.0 + e) * vel_along_n * 0.5
                };
                if let Ok(mut vel) = world.get::<&mut Velocity>(event.entity_a) {
                    vel.0 -= impulse * n;
                    apply_friction(&mut vel.0, n, mu, impulse);
                }
                if let Ok(mut vel) = world.get::<&mut Velocity>(event.entity_b) {
                    vel.0 += impulse * n;
                    apply_friction(&mut vel.0, n, mu, impulse);
                }
            }
        }
    }

    events
}
