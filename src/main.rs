mod camera;
mod components;
mod engine;
mod recording;
mod renderer;
mod systems;
mod ui;

use camera::{Camera, CameraMode};
use clap::Parser;
use components::{
    add_child, CharacterBody, Checkerboard, Children, Collider, Color, DirectionalLight, Drag,
    Friction, GlobalTransform, GrabState, Grabbable, GravityAffected, Grounded, Held, Hidden,
    LocalTransform, Mass, Player, PointLight, PreviousPosition, Restitution, SpotLight, Static,
    SwordPosition, SwordState, Velocity,
};
use engine::input::{InputEvent, InputState};
use engine::time::FrameTimer;
use engine::window::GameWindow;
use glam::{Mat4, Vec3};
use hecs::{Entity, World};
use renderer::mesh::{create_capsule, create_ground_plane, create_sphere, create_sword, create_tapered_box};
use renderer::{MeshStore, Renderer};
use sdl2::keyboard::Scancode;
use systems::{grab_throw_system, grounded_system, physics_system, player_movement_system, transform_propagation_system};
use ui::{GameState, PauseAction, PauseMenu, TextRenderer};

#[derive(Parser)]
#[command(name = "lance", about = "Lance Engine")]
struct Args {
    /// Record 5 seconds of video to demos/demo.mp4
    #[arg(long)]
    record: bool,
}

/// Defines all body proportions and joint offsets for a character in one place.
struct CharacterRig {
    // Body (tapered torso box + capsule collider)
    torso_top_w: f32,
    torso_top_d: f32,
    torso_bot_w: f32,
    torso_bot_d: f32,
    torso_height: f32,
    body_collider_radius: f32,
    body_collider_height: f32,

    // Head (sphere)
    head_mesh_radius: f32,
    head_scale: f32,

    // Limb capsule dimensions
    limb_radius: f32,
    limb_height: f32,

    // Attachment points (relative to body center)
    shoulder_x: f32,
    shoulder_y: f32,
    shoulder_angle: f32,
    hip_x: f32,
    hip_y: f32,

    // Colors
    body_color: Vec3,
    head_color: Vec3,
    limb_color: Vec3,
}

impl CharacterRig {
    fn head_world_radius(&self) -> f32 {
        self.head_mesh_radius * self.head_scale
    }

    fn head_y(&self) -> f32 {
        self.torso_height / 2.0 + self.head_world_radius()
    }

    /// Y offset to place a child capsule's center below a parent capsule's center,
    /// so the child's top hemisphere overlaps the parent's bottom hemisphere at the joint.
    fn joint_y(&self) -> f32 {
        -(self.limb_height / 2.0 + self.limb_height / 2.0 + self.limb_radius)
    }
}

/// Spawn all character body parts (head, arms, legs, sword) as children of `player_entity`.
/// Body parts are visual-only — no colliders. The root entity's capsule collider handles all physics.
/// Returns a `CharacterBody` struct referencing all spawned entities.
fn spawn_character(
    world: &mut World,
    player_entity: Entity,
    head_handle: components::MeshHandle,
    upper_arm_handle: components::MeshHandle,
    forearm_handle: components::MeshHandle,
    upper_leg_handle: components::MeshHandle,
    lower_leg_handle: components::MeshHandle,
    sword_handle: components::MeshHandle,
    rig: &CharacterRig,
) -> CharacterBody {
    use glam::Quat;
    use std::f32::consts::FRAC_PI_2;
    use std::f32::consts::FRAC_PI_6;

    // Head — sphere at top of torso
    let mut head_tr = LocalTransform::new(Vec3::new(0.0, rig.head_y(), 0.1));
    head_tr.scale = Vec3::splat(rig.head_scale);
    let head = world.spawn((
        head_tr,
        GlobalTransform(Mat4::IDENTITY),
        head_handle,
        Color(rig.head_color),
    ));
    add_child(world, player_entity, head);

    // --- Arms (2-segment: upper arm + forearm) ---

    // Left upper arm — positioned at shoulder (+X = left)
    let mut left_upper_arm_t = LocalTransform::new(Vec3::new(rig.shoulder_x, rig.shoulder_y, 0.0));
    left_upper_arm_t.rotation = Quat::from_rotation_z(rig.shoulder_angle);
    let left_upper_arm = world.spawn((
        left_upper_arm_t,
        GlobalTransform(Mat4::IDENTITY),
        upper_arm_handle,
        Color(rig.body_color),
    ));
    add_child(world, player_entity, left_upper_arm);

    // Left forearm — child of left upper arm
    let left_forearm = world.spawn((
        LocalTransform::new(Vec3::new(0.0, rig.joint_y(), 0.0)),
        GlobalTransform(Mat4::IDENTITY),
        forearm_handle,
        Color(rig.limb_color),
    ));
    add_child(world, left_upper_arm, left_forearm);

    // Right upper arm — mirror of left (-X = right)
    let mut right_upper_arm_t = LocalTransform::new(Vec3::new(-rig.shoulder_x, rig.shoulder_y, 0.0));
    right_upper_arm_t.rotation = Quat::from_rotation_z(-rig.shoulder_angle);
    let right_upper_arm = world.spawn((
        right_upper_arm_t,
        GlobalTransform(Mat4::IDENTITY),
        upper_arm_handle,
        Color(rig.body_color),
    ));
    add_child(world, player_entity, right_upper_arm);

    // Right forearm — child of right upper arm
    let right_forearm = world.spawn((
        LocalTransform::new(Vec3::new(0.0, rig.joint_y(), 0.0)),
        GlobalTransform(Mat4::IDENTITY),
        forearm_handle,
        Color(rig.limb_color),
    ));
    add_child(world, right_upper_arm, right_forearm);

    // --- Legs (2-segment: upper leg + lower leg) ---

    // Left upper leg (+X = left)
    let left_upper_leg = world.spawn((
        LocalTransform::new(Vec3::new(rig.hip_x, rig.hip_y, 0.0)),
        GlobalTransform(Mat4::IDENTITY),
        upper_leg_handle,
        Color(rig.body_color),
    ));
    add_child(world, player_entity, left_upper_leg);

    // Left lower leg — child of left upper leg
    let left_lower_leg = world.spawn((
        LocalTransform::new(Vec3::new(0.0, rig.joint_y(), 0.0)),
        GlobalTransform(Mat4::IDENTITY),
        lower_leg_handle,
        Color(rig.limb_color),
    ));
    add_child(world, left_upper_leg, left_lower_leg);

    // Right upper leg (-X = right)
    let right_upper_leg = world.spawn((
        LocalTransform::new(Vec3::new(-rig.hip_x, rig.hip_y, 0.0)),
        GlobalTransform(Mat4::IDENTITY),
        upper_leg_handle,
        Color(rig.body_color),
    ));
    add_child(world, player_entity, right_upper_leg);

    // Right lower leg — child of right upper leg
    let right_lower_leg = world.spawn((
        LocalTransform::new(Vec3::new(0.0, rig.joint_y(), 0.0)),
        GlobalTransform(Mat4::IDENTITY),
        lower_leg_handle,
        Color(rig.limb_color),
    ));
    add_child(world, right_upper_leg, right_lower_leg);

    // --- Sword — starts sheathed at the hip ---
    let sheathed_pos = Vec3::new(0.25, 0.0, 0.4);
    let sheathed_rot = Quat::from_rotation_y(FRAC_PI_2);
    let sheathed_rot = Quat::from_rotation_x(2.0 * FRAC_PI_2 + 2.0 * FRAC_PI_6) * sheathed_rot;

    let wielded_pos = Vec3::new(-0.55, -0.5, 0.3);
    let wielded_rot = Quat::from_rotation_y(FRAC_PI_2);
    let wielded_rot = Quat::from_rotation_x(FRAC_PI_2-0.1) * wielded_rot;

    let mut sword_t = LocalTransform::new(sheathed_pos);
    sword_t.rotation = sheathed_rot;
    sword_t.scale = Vec3::splat(3.0);

    let sword_entity = world.spawn((
        sword_t,
        GlobalTransform(Mat4::IDENTITY),
        sword_handle,
        Color(Vec3::new(0.75, 0.75, 0.8)),
        SwordState {
            position: SwordPosition::Sheathed,
            sheathed_pos,
            sheathed_rot,
            wielded_pos,
            wielded_rot,
        },
    ));
    add_child(world, player_entity, sword_entity);

    CharacterBody {
        head,
        left_upper_arm,
        left_forearm,
        right_upper_arm,
        right_forearm,
        left_upper_leg,
        left_lower_leg,
        right_upper_leg,
        right_lower_leg,
        sword: sword_entity,
    }
}

fn main() {
    let args = Args::parse();
    let sdl = sdl2::init().expect("Failed to init SDL2");
    let window = GameWindow::new(&sdl, "Lance Engine", 1280, 720);

    let mut renderer = Renderer::init();
    let mut text_renderer = TextRenderer::new();
    let mut pause_menu = PauseMenu::new();
    let mut game_state = GameState::Running;

    let rig = CharacterRig {
        torso_top_w: 0.7,
        torso_top_d: 0.5,
        torso_bot_w: 0.35,
        torso_bot_d: 0.25,
        torso_height: 0.8,
        body_collider_radius: 0.3,
        body_collider_height: 2.4,

        head_mesh_radius: 0.8,
        head_scale: 0.3,

        limb_radius: 0.15,
        limb_height: 0.4,

        shoulder_x: 0.45,
        shoulder_y: 0.1,
        shoulder_angle: 0.14,
        hip_x: 0.2, //0.17,
        hip_y: -0.6, //-0.35,

        body_color: Vec3::new(0.8, 0.2, 0.15),
        head_color: Vec3::new(0.7, 0.65, 0.6),
        limb_color: Vec3::new(0.5, 0.5, 0.6),
    };

    // Mesh storage — entities reference meshes by handle
    let mut meshes = MeshStore::new();
    let sphere_handle = meshes.add(create_sphere(1.0, 16, 32));
    let ground_handle = meshes.add(create_ground_plane(500.0));
    let torso_handle = meshes.add(create_tapered_box(
        rig.torso_top_w, rig.torso_top_d,
        rig.torso_bot_w, rig.torso_bot_d,
        rig.torso_height,
    ));
    let upper_arm_handle = meshes.add(create_capsule(rig.limb_radius, rig.limb_height, 8, 8));
    let forearm_handle = meshes.add(create_capsule(rig.limb_radius, rig.limb_height, 8, 8));
    let upper_leg_handle = meshes.add(create_capsule(rig.limb_radius, rig.limb_height, 8, 8));
    let lower_leg_handle = meshes.add(create_capsule(rig.limb_radius, rig.limb_height, 8, 8));
    let head_handle = meshes.add(create_sphere(rig.head_mesh_radius, 8, 8));
    let sword_handle = meshes.add(create_sword());

    

    // ECS world — scene objects are entities with LocalTransform, GlobalTransform, MeshHandle, Color
    let mut world = World::new();

    world.spawn((
        LocalTransform::new(Vec3::ZERO),
        GlobalTransform(Mat4::IDENTITY),
        ground_handle,
        Color(Vec3::new(0.3, 0.6, 0.2)),
        Checkerboard(Vec3::new(0.22, 0.48, 0.15)),
        Collider::Plane {
            normal: Vec3::Y,
            offset: 0.0,
        },
        Static,
    ));

    let mut sphere_transform = LocalTransform::new(Vec3::new(0.0, 2.0, -3.0));
    sphere_transform.scale = Vec3::splat(0.5);

    let red_sphere = world.spawn((
        sphere_transform,
        GlobalTransform(Mat4::IDENTITY),
        sphere_handle,
        Color(Vec3::new(0.8, 0.2, 0.15)),
        Velocity(Vec3::new(0.0, 5.0, 0.0)),
        Mass(1.0),
        GravityAffected,
        Collider::Sphere { radius: 0.5 },
        Restitution(0.3),
        Friction(0.7),
        Drag(0.5),
        Grabbable,
    ));

    // Test child: small blue sphere offset to the right of the red sphere
    let mut child_transform = LocalTransform::new(Vec3::new(0.75, 0.0, 0.0));
    child_transform.scale = Vec3::splat(0.4);
    let child_sphere = world.spawn((
        child_transform,
        GlobalTransform(Mat4::IDENTITY),
        sphere_handle,
        Mass(1.0),
        GravityAffected,
        Color(Vec3::new(0.2, 0.4, 0.9)),
    ));

    add_child(&mut world, red_sphere, child_sphere);

    // Grey boxes scattered around spawn — 5 wide × 7 deep, varying heights
    let grey = Vec3::new(0.5, 0.5, 0.52);
    for &(x, z, h) in &[(6.0_f32, -4.0_f32, 2.0_f32), (-5.0, 3.0, 3.5), (3.0, 7.0, 1.5)] {
        let box_handle = meshes.add(create_tapered_box(5.0, 7.0, 5.0, 7.0, h));
        let mut bt = LocalTransform::new(Vec3::new(x, h / 2.0, z));
        bt.scale = Vec3::ONE;
        world.spawn((
            bt,
            GlobalTransform(Mat4::IDENTITY),
            box_handle,
            Color(grey),
            Collider::Box { half_extents: Vec3::new(2.5, h / 2.0, 3.5) },
            Static,
            Restitution(0.0),
            Friction(0.8),
        ));
    }

    // Player entity — capsule body with physics
    let mut player_transform = LocalTransform::new(Vec3::new(0.0, 10.0, 0.0));
    player_transform.scale = Vec3::splat(1.0);
    let player_entity = world.spawn((
        player_transform,
        GlobalTransform(Mat4::IDENTITY),
        torso_handle,
        Color(rig.body_color),
        Velocity(Vec3::ZERO),
        Mass(80.0),
        GravityAffected,
        Collider::Capsule {
            radius: rig.body_collider_radius,
            height: rig.body_collider_height,
        },
        Restitution(0.0),
        Friction(0.8),
        Player,
        Grounded,
        GrabState::new(),
    ));

    // Character body — head, 2-segment arms, 2-segment legs, and sword as children of the player
    let character_body = spawn_character(
        &mut world,
        player_entity,
        head_handle,
        upper_arm_handle,
        forearm_handle,
        upper_leg_handle,
        lower_leg_handle,
        sword_handle,
        &rig,
    );
    world.insert_one(player_entity, character_body).unwrap();

    // --- Light entities ---

    // Directional light (sun) with shadow mapping
    world.spawn((DirectionalLight {
        direction: Vec3::new(-0.5, -1.0, -0.3),
        color: Vec3::new(1.0, 0.95, 0.85),
        intensity: 1.0,
        shadow_resolution: 2048,
        shadow_extent: 40.0,
    },));

    // Warm point light near the red sphere
    world.spawn((
        LocalTransform::new(Vec3::new(3.0, 3.0, 0.0)),
        PointLight::new(Vec3::new(1.0, 0.6, 0.2), 2.0, 15.0),
    ));

    // Cool blue point light on the other side
    world.spawn((
        LocalTransform::new(Vec3::new(-4.0, 2.0, -3.0)),
        PointLight::new(Vec3::new(0.2, 0.4, 1.0), 1.5, 12.0),
    ));

    // Green point light farther out
    world.spawn((
        LocalTransform::new(Vec3::new(0.0, 4.0, -8.0)),
        PointLight::new(Vec3::new(0.1, 0.9, 0.3), 1.8, 18.0),
    ));

    // Spot light shining down like a street lamp
    world.spawn((
        LocalTransform::new(Vec3::new(5.0, 6.0, 5.0)),
        SpotLight::new(
            Vec3::new(0.0, -1.0, 0.0),    // pointing down
            Vec3::new(1.0, 0.9, 0.7),     // warm white
            3.0,                            // intensity
            15.0,                           // inner cone degrees
            30.0,                           // outer cone degrees
            20.0,                           // radius
        ),
    ));

    let mut recorder = if args.record {
        let (w, h) = window.size();
        Some(recording::Recorder::new(w, h, "demos/demo.mp4"))
    } else {
        None
    };
    let mut record_elapsed: f32 = 0.0;
    let mut record_frame_debt: f32 = 0.0;
    const RECORD_DURATION: f32 = 5.0;
    const RECORD_FRAME_INTERVAL: f32 = 1.0 / 60.0;

    sdl.mouse().set_relative_mouse_mode(true);

    let mut event_pump = sdl.event_pump().expect("Failed to get event pump");
    let mut input = InputState::new();
    let mut timer = FrameTimer::new();
    let mut camera = Camera::new();
    let mut physics_accum: f32 = 0.0;

    loop {
        timer.tick();
        input.update(&mut event_pump);

        if input.should_quit() {
            break;
        }

        // Handle Escape toggle between Running and Paused
        let mut just_paused = false;
        for event in &input.events {
            if let InputEvent::KeyPressed(Scancode::Escape) = event {
                if game_state == GameState::Running {
                    game_state = GameState::Paused;
                    pause_menu.reset_selection();
                    sdl.mouse().set_relative_mouse_mode(false);
                    just_paused = true;
                }
            }
        }

        // Physics interpolation alpha, set each frame by physics_system.
        // 1.0 when paused (render current state without interpolation).
        let mut alpha: f32 = 1.0;

        // Route input based on game state
        match game_state {
            GameState::Paused => {
                // Skip input on the frame we just entered pause (same Escape event would resume)
                let action = if just_paused {
                    PauseAction::None
                } else {
                    pause_menu.handle_input(&input.events)
                };
                match action {
                    PauseAction::Resume => {
                        game_state = GameState::Running;
                        sdl.mouse().set_relative_mouse_mode(true);
                    }
                    PauseAction::Quit => break,
                    PauseAction::None => {}
                }
            }
            GameState::Running => {
                // F1 toggles fly/player mode, Z toggles first/third person
                for event in &input.events {
                    match event {
                        InputEvent::KeyPressed(Scancode::F1) => camera.toggle_mode(),
                        InputEvent::KeyPressed(Scancode::Z) => {
                            camera.toggle_perspective();
                            // Collect player + children entity IDs
                            let mut to_toggle = vec![player_entity];
                            if let Ok(children) = world.get::<&Children>(player_entity) {
                                to_toggle.extend(children.0.iter().copied());
                            }
                            // Hide/show player body in first/third person
                            // Skip held objects and sword (always visible)
                            for entity in to_toggle {
                                if world.get::<&Held>(entity).is_ok() {
                                    continue;
                                }
                                if world.get::<&SwordState>(entity).is_ok() {
                                    continue;
                                }
                                if camera.is_third_person() {
                                    let _ = world.remove_one::<Hidden>(entity);
                                } else {
                                    let _ = world.insert_one(entity, Hidden);
                                }
                            }
                        }
                        InputEvent::KeyPressed(Scancode::F) => {
                            // Toggle sword between sheathed and wielded
                            for (_e, (sword, lt)) in
                                world.query_mut::<(&mut SwordState, &mut LocalTransform)>()
                            {
                                match sword.position {
                                    SwordPosition::Sheathed => {
                                        sword.position = SwordPosition::Wielded;
                                        lt.position = sword.wielded_pos;
                                        lt.rotation = sword.wielded_rot;
                                    }
                                    SwordPosition::Wielded => {
                                        sword.position = SwordPosition::Sheathed;
                                        lt.position = sword.sheathed_pos;
                                        lt.rotation = sword.sheathed_rot;
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }

                camera.look(input.mouse_dx, input.mouse_dy);

                // Grab/throw must run before player movement to produce speed multiplier
                let speed_mult = if camera.mode == CameraMode::Player {
                    grab_throw_system(&mut world, &input, &camera, timer.dt)
                } else {
                    1.0
                };

                match camera.mode {
                    CameraMode::Player => {
                        player_movement_system(&mut world, &input, &camera, speed_mult);
                    }
                    CameraMode::Fly => {
                        camera.move_wasd(&input, timer.dt);
                    }
                }

                let (collision_events, frame_alpha) = physics_system(&mut world, &mut physics_accum, timer.dt);
                alpha = frame_alpha;
                grounded_system(&mut world, &collision_events);

                if camera.mode == CameraMode::Player {
                    // Use interpolated player position so the camera follows
                    // smoothly between fixed physics ticks.
                    let player_pos = match (
                        world.get::<&LocalTransform>(player_entity),
                        world.get::<&PreviousPosition>(player_entity),
                    ) {
                        (Ok(local), Ok(prev)) => prev.0.lerp(local.position, frame_alpha),
                        (Ok(local), _) => local.position,
                        _ => glam::Vec3::ZERO,
                    };
                    camera.follow_player(player_pos, 0.7, 0.3);
                }
            }
        }

        // Propagate transforms before rendering (always, even when paused).
        // alpha interpolates entity positions between fixed physics steps.
        transform_propagation_system(&mut world, alpha);

        let view = camera.view_matrix();
        let proj = camera.projection_matrix(window.aspect_ratio());

        renderer.draw_scene(&world, &meshes, &view, &proj, camera.position);

        // UI pass — render on top of the scene
        if game_state == GameState::Paused {
            let (w, h) = window.size();
            let ui_proj = Mat4::orthographic_rh_gl(0.0, w as f32, h as f32, 0.0, -1.0, 1.0);

            unsafe {
                gl::Disable(gl::DEPTH_TEST);
                gl::Enable(gl::BLEND);
                gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
            }

            pause_menu.draw(&mut text_renderer, w as f32, h as f32, &ui_proj);

            unsafe {
                gl::Disable(gl::BLEND);
                gl::Enable(gl::DEPTH_TEST);
            }
        }

        if let Some(ref mut rec) = recorder {
            record_elapsed += timer.dt;
            record_frame_debt += timer.dt;
            while record_frame_debt >= RECORD_FRAME_INTERVAL {
                rec.capture_frame();
                record_frame_debt -= RECORD_FRAME_INTERVAL;
            }
            if record_elapsed >= RECORD_DURATION {
                recorder.take().unwrap().finish();
                break;
            }
        }

        window.swap();
    }
}
