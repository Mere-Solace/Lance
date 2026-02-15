use std::collections::VecDeque;

use glam::Mat4;
use hecs::{Entity, World};

use crate::components::{Children, GlobalTransform, LocalTransform, Parent};

/// Propagates LocalTransform down the hierarchy via BFS.
/// Roots (entities with LocalTransform but no Parent) compute GlobalTransform
/// from their own LocalTransform. Children inherit parent's GlobalTransform
/// multiplied by their own LocalTransform.
pub fn transform_propagation_system(world: &mut World) {
    let mut queue: VecDeque<(Entity, Mat4)> = VecDeque::new();

    // Phase 1: update roots and seed BFS with their children
    let roots: Vec<(Entity, Mat4)> = world
        .query::<&LocalTransform>()
        .without::<&Parent>()
        .iter()
        .map(|(entity, local)| (entity, local.matrix()))
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
