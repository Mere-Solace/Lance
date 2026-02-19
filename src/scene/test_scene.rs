use glam::Vec3;
use hecs::{Entity, World};

use crate::renderer::MeshStore;
use crate::scene::prefabs::{
    spawn_directional_light, spawn_ground, spawn_physics_sphere, spawn_player, spawn_point_light,
    spawn_spot_light, spawn_static_box,
};

/// Build and populate the test scene.
/// Returns the mesh store (owns all GPU mesh data) and the player entity.
pub fn load_test_scene(world: &mut World) -> (MeshStore, Entity) {
    let mut meshes = MeshStore::new();

    spawn_ground(world, &mut meshes);

    spawn_physics_sphere(
        world,
        &mut meshes,
        Vec3::new(0.0, 2.0, -3.0),
        Vec3::new(0.8, 0.2, 0.15),
        0.5,
        Vec3::new(0.0, 5.0, 0.0),
    );

    // Grey boxes scattered around spawn
    let grey = Vec3::new(0.5, 0.5, 0.52);
    for &(x, z, h) in &[(6.0_f32, -4.0_f32, 2.0_f32), (-5.0, 3.0, 3.5), (3.0, 7.0, 1.5)] {
        spawn_static_box(
            world,
            &mut meshes,
            Vec3::new(x, h / 2.0, z),
            Vec3::new(2.5, h / 2.0, 3.5),
            grey,
        );
    }

    let player_entity = spawn_player(world, &mut meshes, Vec3::new(0.0, 10.0, 0.0));

    spawn_directional_light(
        world,
        Vec3::new(-0.5, -1.0, -0.3),
        Vec3::new(1.0, 0.95, 0.85),
        1.0,
    );
    spawn_point_light(world, Vec3::new(3.0, 3.0, 0.0), Vec3::new(1.0, 0.6, 0.2), 2.0, 15.0);
    spawn_point_light(world, Vec3::new(-4.0, 2.0, -3.0), Vec3::new(0.2, 0.4, 1.0), 1.5, 12.0);
    spawn_point_light(world, Vec3::new(0.0, 4.0, -8.0), Vec3::new(0.1, 0.9, 0.3), 1.8, 18.0);
    spawn_spot_light(
        world,
        Vec3::new(5.0, 6.0, 5.0),
        Vec3::new(0.0, -1.0, 0.0),
        Vec3::new(1.0, 0.9, 0.7),
        3.0,
        15.0,
        30.0,
        20.0,
    );

    (meshes, player_entity)
}
