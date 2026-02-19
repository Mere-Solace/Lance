use glam::Vec3;
use hecs::{Entity, World};

use crate::components::{Collider, CollisionEvent, Friction, GlobalTransform, Held, LocalTransform, NoSelfCollision, Parent, Restitution, Static, Velocity};

struct ColliderEntry {
    entity: Entity,
    position: Vec3,
    collider_kind: ColliderKind,
    body_owner: Option<Entity>,
}

enum ColliderKind {
    Sphere { radius: f32 },
    Capsule { radius: f32, half_height: f32 },
    Plane { normal: Vec3, offset: f32 },
    Box { half_extents: Vec3 },
}

/// Closest point on an AABB (centered at `box_pos` with `half` extents) to point `p`.
fn closest_point_on_aabb(box_pos: Vec3, half: Vec3, p: Vec3) -> Vec3 {
    let local = p - box_pos;
    Vec3::new(
        local.x.clamp(-half.x, half.x),
        local.y.clamp(-half.y, half.y),
        local.z.clamp(-half.z, half.z),
    ) + box_pos
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

        // --- Box collisions ---

        // Box(A) vs Plane(B): project half-extents onto plane normal
        (ColliderKind::Box { half_extents }, ColliderKind::Plane { normal, offset }) => {
            let projected_radius =
                half_extents.x * normal.x.abs() +
                half_extents.y * normal.y.abs() +
                half_extents.z * normal.z.abs();
            let center_dist = a.position.dot(*normal) - offset;
            let penetration = projected_radius - center_dist;
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
        // Plane(A) vs Box(B): canonicalize so box=entity_a, plane=entity_b
        (ColliderKind::Plane { normal, offset }, ColliderKind::Box { half_extents }) => {
            let projected_radius =
                half_extents.x * normal.x.abs() +
                half_extents.y * normal.y.abs() +
                half_extents.z * normal.z.abs();
            let center_dist = b.position.dot(*normal) - offset;
            let penetration = projected_radius - center_dist;
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

        // Box(A) vs Sphere(B): clamp sphere center to box, check distance
        (ColliderKind::Box { half_extents }, ColliderKind::Sphere { radius }) => {
            let closest = closest_point_on_aabb(a.position, *half_extents, b.position);
            let diff = b.position - closest;
            let dist = diff.length();
            // Check if sphere center is inside the box
            let local = b.position - a.position;
            let inside = local.x.abs() <= half_extents.x
                && local.y.abs() <= half_extents.y
                && local.z.abs() <= half_extents.z;
            if inside {
                // Sphere center inside box — find shortest axis to push out
                let dx = half_extents.x - local.x.abs();
                let dy = half_extents.y - local.y.abs();
                let dz = half_extents.z - local.z.abs();
                let (pen, normal) = if dx <= dy && dx <= dz {
                    (dx + radius, Vec3::X * local.x.signum())
                } else if dy <= dz {
                    (dy + radius, Vec3::Y * local.y.signum())
                } else {
                    (dz + radius, Vec3::Z * local.z.signum())
                };
                Some(CollisionEvent {
                    entity_a: a.entity,
                    entity_b: b.entity,
                    contact_normal: normal,
                    penetration_depth: pen,
                })
            } else if dist < *radius {
                let normal = if dist > 1e-6 { diff / dist } else { Vec3::Y };
                Some(CollisionEvent {
                    entity_a: a.entity,
                    entity_b: b.entity,
                    contact_normal: normal,
                    penetration_depth: radius - dist,
                })
            } else {
                None
            }
        }
        // Sphere(A) vs Box(B): swap and negate normal
        (ColliderKind::Sphere { radius }, ColliderKind::Box { half_extents }) => {
            let closest = closest_point_on_aabb(b.position, *half_extents, a.position);
            let diff = a.position - closest;
            let dist = diff.length();
            let local = a.position - b.position;
            let inside = local.x.abs() <= half_extents.x
                && local.y.abs() <= half_extents.y
                && local.z.abs() <= half_extents.z;
            if inside {
                let dx = half_extents.x - local.x.abs();
                let dy = half_extents.y - local.y.abs();
                let dz = half_extents.z - local.z.abs();
                let (pen, axis_normal) = if dx <= dy && dx <= dz {
                    (dx + radius, Vec3::X * local.x.signum())
                } else if dy <= dz {
                    (dy + radius, Vec3::Y * local.y.signum())
                } else {
                    (dz + radius, Vec3::Z * local.z.signum())
                };
                // Normal points A→B, so negate (axis_normal points sphere outward from box)
                Some(CollisionEvent {
                    entity_a: a.entity,
                    entity_b: b.entity,
                    contact_normal: -axis_normal,
                    penetration_depth: pen,
                })
            } else if dist < *radius {
                // Normal from A toward B: -(diff/dist) since diff = A - closest_on_B
                let normal = if dist > 1e-6 { -diff / dist } else { Vec3::Y };
                Some(CollisionEvent {
                    entity_a: a.entity,
                    entity_b: b.entity,
                    contact_normal: normal,
                    penetration_depth: radius - dist,
                })
            } else {
                None
            }
        }

        // Box(A) vs Capsule(B): closest point on capsule segment, then treat as box-vs-sphere
        (ColliderKind::Box { half_extents }, ColliderKind::Capsule { radius: cr, half_height }) => {
            let top = b.position + Vec3::Y * *half_height;
            let bottom = b.position - Vec3::Y * *half_height;
            // Find the point on the capsule segment closest to the box center
            let seg_closest = closest_point_on_segment(bottom, top, a.position);
            // Now test box vs sphere centered at seg_closest with radius cr
            let closest = closest_point_on_aabb(a.position, *half_extents, seg_closest);
            let diff = seg_closest - closest;
            let dist = diff.length();
            let local = seg_closest - a.position;
            let inside = local.x.abs() <= half_extents.x
                && local.y.abs() <= half_extents.y
                && local.z.abs() <= half_extents.z;
            if inside {
                let dx = half_extents.x - local.x.abs();
                let dy = half_extents.y - local.y.abs();
                let dz = half_extents.z - local.z.abs();
                let (pen, normal) = if dx <= dy && dx <= dz {
                    (dx + cr, Vec3::X * local.x.signum())
                } else if dy <= dz {
                    (dy + cr, Vec3::Y * local.y.signum())
                } else {
                    (dz + cr, Vec3::Z * local.z.signum())
                };
                Some(CollisionEvent {
                    entity_a: a.entity,
                    entity_b: b.entity,
                    contact_normal: normal,
                    penetration_depth: pen,
                })
            } else if dist < *cr {
                let normal = if dist > 1e-6 { diff / dist } else { Vec3::Y };
                Some(CollisionEvent {
                    entity_a: a.entity,
                    entity_b: b.entity,
                    contact_normal: normal,
                    penetration_depth: cr - dist,
                })
            } else {
                None
            }
        }
        // Capsule(A) vs Box(B): swap
        (ColliderKind::Capsule { radius: cr, half_height }, ColliderKind::Box { half_extents }) => {
            let top = a.position + Vec3::Y * *half_height;
            let bottom = a.position - Vec3::Y * *half_height;
            let seg_closest = closest_point_on_segment(bottom, top, b.position);
            let closest = closest_point_on_aabb(b.position, *half_extents, seg_closest);
            let diff = seg_closest - closest;
            let dist = diff.length();
            let local = seg_closest - b.position;
            let inside = local.x.abs() <= half_extents.x
                && local.y.abs() <= half_extents.y
                && local.z.abs() <= half_extents.z;
            if inside {
                let dx = half_extents.x - local.x.abs();
                let dy = half_extents.y - local.y.abs();
                let dz = half_extents.z - local.z.abs();
                let (pen, axis_normal) = if dx <= dy && dx <= dz {
                    (dx + cr, Vec3::X * local.x.signum())
                } else if dy <= dz {
                    (dy + cr, Vec3::Y * local.y.signum())
                } else {
                    (dz + cr, Vec3::Z * local.z.signum())
                };
                // Normal points A→B: capsule segment is "A-side", box is "B-side"
                // axis_normal points capsule outward from box, so negate for A→B
                Some(CollisionEvent {
                    entity_a: a.entity,
                    entity_b: b.entity,
                    contact_normal: -axis_normal,
                    penetration_depth: pen,
                })
            } else if dist < *cr {
                // diff = seg_closest - closest_on_box, points from box toward capsule
                // Normal A→B means from capsule toward box = -diff
                let normal = if dist > 1e-6 { -diff / dist } else { Vec3::Y };
                Some(CollisionEvent {
                    entity_a: a.entity,
                    entity_b: b.entity,
                    contact_normal: normal,
                    penetration_depth: cr - dist,
                })
            } else {
                None
            }
        }

        // Box(A) vs Box(B): AABB overlap (SAT on 3 axes)
        (ColliderKind::Box { half_extents: ha }, ColliderKind::Box { half_extents: hb }) => {
            let d = b.position - a.position;
            let overlap_x = (ha.x + hb.x) - d.x.abs();
            let overlap_y = (ha.y + hb.y) - d.y.abs();
            let overlap_z = (ha.z + hb.z) - d.z.abs();
            if overlap_x > 0.0 && overlap_y > 0.0 && overlap_z > 0.0 {
                // Minimum penetration axis
                let (penetration, normal) = if overlap_x <= overlap_y && overlap_x <= overlap_z {
                    (overlap_x, Vec3::X * d.x.signum())
                } else if overlap_y <= overlap_z {
                    (overlap_y, Vec3::Y * d.y.signum())
                } else {
                    (overlap_z, Vec3::Z * d.z.signum())
                };
                let normal = if normal.length_squared() < 1e-6 { Vec3::Y } else { normal };
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

/// Walk up the Parent chain to find the root entity that owns physics (Velocity, LocalTransform).
fn find_physics_root(world: &World, entity: Entity) -> Entity {
    let mut current = entity;
    while let Ok(parent) = world.get::<&Parent>(current) {
        current = parent.0;
    }
    current
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
        .query_mut::<(&GlobalTransform, &Collider, Option<&Held>, Option<&NoSelfCollision>)>()
        .into_iter()
        .filter(|(_entity, (_global, _collider, held, _nsc))| held.is_none())
        .map(|(entity, (global, collider, _held, nsc))| {
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
                Collider::Box { half_extents } => ColliderKind::Box {
                    half_extents: *half_extents,
                },
            };
            ColliderEntry {
                entity,
                position: global.0.w_axis.truncate(),
                collider_kind: kind,
                body_owner: nsc.map(|n| n.0),
            }
        })
        .collect();

    // Broadphase: brute force O(n²)
    let mut events = Vec::new();
    for i in 0..entries.len() {
        for j in (i + 1)..entries.len() {
            // Skip self-collision between body parts of the same character
            if let (Some(owner_a), Some(owner_b)) = (entries[i].body_owner, entries[j].body_owner) {
                if owner_a == owner_b {
                    continue;
                }
            }
            if let Some(event) = test_pair(&entries[i], &entries[j]) {
                events.push(event);
            }
        }
    }

    // Response — normal points from A to B in all cases
    for event in &events {
        let root_a = find_physics_root(world, event.entity_a);
        let root_b = find_physics_root(world, event.entity_b);
        let a_static = world.get::<&Static>(root_a).is_ok();
        let b_static = world.get::<&Static>(root_b).is_ok();

        if a_static && b_static {
            continue;
        }

        let restitution_a = world
            .get::<&Restitution>(root_a)
            .map(|r| r.0)
            .unwrap_or(DEFAULT_RESTITUTION);
        let restitution_b = world
            .get::<&Restitution>(root_b)
            .map(|r| r.0)
            .unwrap_or(DEFAULT_RESTITUTION);
        let e = (restitution_a + restitution_b) * 0.5;

        let friction_a = world
            .get::<&Friction>(root_a)
            .map(|f| f.0)
            .unwrap_or(DEFAULT_FRICTION);
        let friction_b = world
            .get::<&Friction>(root_b)
            .map(|f| f.0)
            .unwrap_or(DEFAULT_FRICTION);
        let mu = (friction_a + friction_b) * 0.5;

        let n = event.contact_normal;
        let depth = event.penetration_depth;

        if a_static {
            // A is static, B is dynamic — push B's root away from A (along +normal)
            let phys_b = find_physics_root(world, event.entity_b);
            if let Ok(mut local) = world.get::<&mut LocalTransform>(phys_b) {
                local.position += n * depth;
            }
            if let Ok(mut vel) = world.get::<&mut Velocity>(phys_b) {
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
            // B is static, A is dynamic — push A's root away from B (along -normal)
            let phys_a = find_physics_root(world, event.entity_a);
            if let Ok(mut local) = world.get::<&mut LocalTransform>(phys_a) {
                local.position -= n * depth;
            }
            if let Ok(mut vel) = world.get::<&mut Velocity>(phys_a) {
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
            // Both dynamic — split push 50/50, redirect to physics roots
            let phys_a = find_physics_root(world, event.entity_a);
            let phys_b = find_physics_root(world, event.entity_b);

            if let Ok(mut local) = world.get::<&mut LocalTransform>(phys_a) {
                local.position -= n * (depth * 0.5);
            }
            if let Ok(mut local) = world.get::<&mut LocalTransform>(phys_b) {
                local.position += n * (depth * 0.5);
            }

            let vel_a = world.get::<&Velocity>(phys_a).map(|v| v.0).unwrap_or(Vec3::ZERO);
            let vel_b = world.get::<&Velocity>(phys_b).map(|v| v.0).unwrap_or(Vec3::ZERO);
            let relative_vel = vel_a - vel_b;
            let vel_along_n = relative_vel.dot(n);

            // Positive = A approaching B
            if vel_along_n > 0.0 {
                let impulse = if vel_along_n < REST_VELOCITY_THRESHOLD {
                    vel_along_n * 0.5
                } else {
                    (1.0 + e) * vel_along_n * 0.5
                };
                if let Ok(mut vel) = world.get::<&mut Velocity>(phys_a) {
                    vel.0 -= impulse * n;
                    apply_friction(&mut vel.0, n, mu, impulse);
                }
                if let Ok(mut vel) = world.get::<&mut Velocity>(phys_b) {
                    vel.0 += impulse * n;
                    apply_friction(&mut vel.0, n, mu, impulse);
                }
            }
        }
    }

    events
}
