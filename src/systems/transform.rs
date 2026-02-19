use std::collections::VecDeque;

use glam::Mat4;
use hecs::{Entity, World};

use crate::components::{Children, GlobalTransform, LocalTransform, Parent, PreviousPosition};

/// Propagates LocalTransform down the hierarchy via BFS.
/// Roots (entities with LocalTransform but no Parent) compute GlobalTransform
/// from their own LocalTransform. Children inherit parent's GlobalTransform
/// multiplied by their own LocalTransform.
///
/// `alpha` is the render interpolation factor (0..1): how far into the current
/// physics step this render frame falls. Root physics entities with a
/// `PreviousPosition` component have their translation lerped between the
/// previous and current physics position, eliminating fixed-timestep jitter.
pub fn transform_propagation_system(world: &mut World, alpha: f32) {
    let mut queue: VecDeque<(Entity, Mat4)> = VecDeque::new();

    // Phase 1: update roots and seed BFS with their children.
    // Query LocalTransform + optional PreviousPosition together so the borrow
    // is released before we write GlobalTransform.
    let roots: Vec<(Entity, Mat4)> = world
        .query::<(&LocalTransform, Option<&PreviousPosition>)>()
        .without::<&Parent>()
        .iter()
        .map(|(entity, (local, prev))| {
            let mat = if let Some(prev) = prev {
                // Lerp translation between previous and current physics state.
                let interp_pos = prev.0.lerp(local.position, alpha);
                Mat4::from_scale_rotation_translation(local.scale, local.rotation, interp_pos)
            } else {
                local.matrix()
            };
            (entity, mat)
        })
        .collect();

    for (entity, global_mat) in &roots {
        if let Ok(mut gt) = world.get::<&mut GlobalTransform>(*entity) {
            gt.0 = *global_mat;
        }
        if let Ok(children) = world.get::<&Children>(*entity) {
            for &child in &children.0 {
                queue.push_back((child, *global_mat));
            }
        }
    }

    // Phase 2: BFS propagation
    while let Some((entity, parent_global)) = queue.pop_front() {
        let child_global = if let Ok(local) = world.get::<&LocalTransform>(entity) {
            parent_global * local.matrix()
        } else {
            parent_global
        };

        if let Ok(mut gt) = world.get::<&mut GlobalTransform>(entity) {
            gt.0 = child_global;
        }

        if let Ok(children) = world.get::<&Children>(entity) {
            for &child in &children.0 {
                queue.push_back((child, child_global));
            }
        }
    }
}
