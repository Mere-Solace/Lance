use glam::Vec3;
use hecs::{Entity, World};

use crate::components::{Collider, GlobalTransform, Grabbable, Static};

#[allow(dead_code)]
pub struct RaycastHit {
    pub entity: Entity,
    pub distance: f32,
    pub point: Vec3,
}

/// Cast a ray against all Grabbable entities, returning the nearest hit within max_distance.
pub fn raycast_grabbable(
    world: &World,
    origin: Vec3,
    direction: Vec3,
    max_distance: f32,
) -> Option<RaycastHit> {
    let dir = direction.normalize();
    let mut best: Option<RaycastHit> = None;

    for (entity, (_grabbable, global, collider)) in
        world.query::<(&Grabbable, &GlobalTransform, &Collider)>().iter()
    {
        let center = Vec3::new(global.0.w_axis.x, global.0.w_axis.y, global.0.w_axis.z);

        let t = match collider {
            Collider::Sphere { radius } => ray_sphere_intersection(origin, dir, center, *radius),
            Collider::Capsule { radius, height } => {
                ray_capsule_intersection(origin, dir, center, *radius, *height)
            }
            Collider::Box { half_extents } => {
                ray_aabb_intersection(origin, dir, center, *half_extents)
            }
            Collider::Plane { .. } => None,
        };

        if let Some(t) = t {
            if t > 0.0 && t <= max_distance {
                let is_closer = best.as_ref().map_or(true, |b| t < b.distance);
                if is_closer {
                    best = Some(RaycastHit {
                        entity,
                        distance: t,
                        point: origin + dir * t,
                    });
                }
            }
        }
    }

    best
}

/// Cast a ray against all Static geometry, returning the nearest hit distance within max_distance.
/// Used for camera wall-clip occlusion queries.
pub fn raycast_static(
    world: &World,
    origin: Vec3,
    direction: Vec3,
    max_distance: f32,
) -> Option<f32> {
    let dir = direction.normalize();
    let mut best: Option<f32> = None;

    for (_, (_, collider, global)) in
        world.query::<(&Static, &Collider, &GlobalTransform)>().iter()
    {
        let center = Vec3::new(global.0.w_axis.x, global.0.w_axis.y, global.0.w_axis.z);

        let t = match collider {
            Collider::Sphere { radius } => ray_sphere_intersection(origin, dir, center, *radius),
            Collider::Capsule { radius, height } => {
                ray_capsule_intersection(origin, dir, center, *radius, *height)
            }
            Collider::Box { half_extents } => {
                ray_aabb_intersection(origin, dir, center, *half_extents)
            }
            // Plane colliders are infinite floors — skip them for camera occlusion.
            Collider::Plane { .. } => None,
        };

        if let Some(t) = t {
            if t > 0.0 && t <= max_distance {
                let is_closer = best.map_or(true, |b| t < b);
                if is_closer {
                    best = Some(t);
                }
            }
        }
    }

    best
}

fn ray_sphere_intersection(origin: Vec3, dir: Vec3, center: Vec3, radius: f32) -> Option<f32> {
    let oc = origin - center;
    let a = dir.dot(dir);
    let b = 2.0 * oc.dot(dir);
    let c = oc.dot(oc) - radius * radius;
    let discriminant = b * b - 4.0 * a * c;

    if discriminant < 0.0 {
        return None;
    }

    let sqrt_d = discriminant.sqrt();
    let t1 = (-b - sqrt_d) / (2.0 * a);
    let t2 = (-b + sqrt_d) / (2.0 * a);

    if t1 > 0.0 {
        Some(t1)
    } else if t2 > 0.0 {
        Some(t2)
    } else {
        None
    }
}

fn ray_capsule_intersection(
    origin: Vec3,
    dir: Vec3,
    center: Vec3,
    radius: f32,
    height: f32,
) -> Option<f32> {
    let half_h = height * 0.5;
    let top = center + Vec3::Y * half_h;
    let bottom = center - Vec3::Y * half_h;

    // Test both hemisphere centers as spheres (approximation suitable for grab detection)
    let t_top = ray_sphere_intersection(origin, dir, top, radius);
    let t_bottom = ray_sphere_intersection(origin, dir, bottom, radius);
    // Also test center sphere for the cylindrical body
    let t_center = ray_sphere_intersection(origin, dir, center, radius);

    [t_top, t_bottom, t_center]
        .iter()
        .filter_map(|t| *t)
        .filter(|t| *t > 0.0)
        .reduce(f32::min)
}

fn ray_aabb_intersection(origin: Vec3, dir: Vec3, center: Vec3, half: Vec3) -> Option<f32> {
    let min = center - half;
    let max = center + half;
    let inv_dir = Vec3::new(1.0 / dir.x, 1.0 / dir.y, 1.0 / dir.z);

    let t1 = (min.x - origin.x) * inv_dir.x;
    let t2 = (max.x - origin.x) * inv_dir.x;
    let t3 = (min.y - origin.y) * inv_dir.y;
    let t4 = (max.y - origin.y) * inv_dir.y;
    let t5 = (min.z - origin.z) * inv_dir.z;
    let t6 = (max.z - origin.z) * inv_dir.z;

    let tmin = t1.min(t2).max(t3.min(t4)).max(t5.min(t6));
    let tmax = t1.max(t2).min(t3.max(t4)).min(t5.max(t6));

    if tmax < 0.0 || tmin > tmax {
        return None;
    }
    // If tmin < 0, ray starts inside the box — return tmax
    Some(if tmin < 0.0 { tmax } else { tmin })
}
