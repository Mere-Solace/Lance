use glam::{Mat4, Vec3};
use sdl2::keyboard::Scancode;

use crate::engine::input::InputState;

#[derive(PartialEq, Eq)]
pub enum CameraMode {
    Player,
    Fly,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Perspective {
    FirstPerson,
    ThirdPersonBack,
    ThirdPersonFront,
}

/// Default arm lengths (distance from player eye to camera).
const DEFAULT_ARM_BACK: f32 = 3.0;
const DEFAULT_ARM_FRONT: f32 = 5.0;

/// Zoom clamp ranges for third-person arm length.
const ARM_MIN: f32 = 1.0;
const ARM_MAX: f32 = 8.0;

/// First-person FOV zoom range (degrees).
const FOV_MIN: f32 = 20.0;
const FOV_MAX: f32 = 70.0;

/// Clearance between camera and wall surface (to avoid z-fighting).
const WALL_CLEARANCE: f32 = 0.3;

/// Minimum arm length regardless of wall distance.
const MIN_ARM: f32 = 0.3;

/// Speed at which the camera arm recovers toward full length after a wall clip (units/s).
const ARM_RECOVERY_SPEED: f32 = 4.0;

pub struct Camera {
    pub position: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub speed: f32,
    pub sensitivity: f32,
    pub fov: f32,
    pub mode: CameraMode,
    pub perspective: Perspective,
    /// Whether the player is holding free-look (C): camera pans without rotating the character.
    pub free_look: bool,
    /// True while the camera is interpolating back toward `character_yaw` after free-look release.
    pub free_look_return: bool,
    /// Seconds elapsed since the current free-look return started.
    pub free_look_return_elapsed: f32,
    /// The yaw the player body is facing — captured every frame when not in free-look.
    pub character_yaw: f32,
    /// User-controlled (zoom) arm length for third-person back. Clamped [ARM_MIN, ARM_MAX].
    pub arm_length_back: f32,
    /// User-controlled (zoom) arm length for third-person front. Clamped [ARM_MIN, ARM_MAX].
    pub arm_length_front: f32,
    /// Current effective back arm length, reduced by wall collision and smoothly recovered.
    effective_arm_back: f32,
    /// Current effective front arm length, reduced by wall collision and smoothly recovered.
    effective_arm_front: f32,
}

impl Camera {
    pub fn new() -> Self {
        Self {
            position: Vec3::new(0.0, 1.0, 3.0),
            yaw: -90.0_f32,
            pitch: 0.0,
            speed: 5.0,
            sensitivity: 0.1,
            fov: 45.0,
            mode: CameraMode::Player,
            perspective: Perspective::ThirdPersonBack,
            free_look: false,
            free_look_return: false,
            free_look_return_elapsed: 0.0,
            character_yaw: -90.0_f32,
            arm_length_back: DEFAULT_ARM_BACK,
            arm_length_front: DEFAULT_ARM_FRONT,
            effective_arm_back: DEFAULT_ARM_BACK,
            effective_arm_front: DEFAULT_ARM_FRONT,
        }
    }

    pub fn toggle_mode(&mut self) {
        self.mode = match self.mode {
            CameraMode::Player => CameraMode::Fly,
            CameraMode::Fly => CameraMode::Player,
        };
    }

    /// Cycle perspective and reset zoom state to defaults.
    pub fn toggle_perspective(&mut self) {
        self.perspective = match self.perspective {
            Perspective::ThirdPersonBack => Perspective::ThirdPersonFront,
            Perspective::ThirdPersonFront => Perspective::FirstPerson,
            Perspective::FirstPerson => Perspective::ThirdPersonBack,
        };
        // Reset zoom to default on mode switch (AC: "Zoom state resets to default").
        self.arm_length_back = DEFAULT_ARM_BACK;
        self.arm_length_front = DEFAULT_ARM_FRONT;
        self.effective_arm_back = DEFAULT_ARM_BACK;
        self.effective_arm_front = DEFAULT_ARM_FRONT;
        self.fov = 45.0;
    }

    pub fn is_third_person(&self) -> bool {
        matches!(self.perspective, Perspective::ThirdPersonBack | Perspective::ThirdPersonFront)
    }

    /// Adjust zoom based on scroll input. Third-person adjusts arm length; first-person adjusts FOV.
    pub fn apply_zoom(&mut self, scroll_dy: f32) {
        match self.perspective {
            Perspective::ThirdPersonBack => {
                self.arm_length_back = (self.arm_length_back - scroll_dy * 0.5).clamp(ARM_MIN, ARM_MAX);
            }
            Perspective::ThirdPersonFront => {
                self.arm_length_front = (self.arm_length_front - scroll_dy * 0.5).clamp(ARM_MIN, ARM_MAX);
            }
            Perspective::FirstPerson => {
                // Scroll up = zoom in = smaller FOV.
                self.fov = (self.fov - scroll_dy * 2.0).clamp(FOV_MIN, FOV_MAX);
            }
        }
    }

    /// Compute the world-space eye position (base of camera raycast).
    pub fn eye_pos(player_pos: Vec3, eye_height: f32) -> Vec3 {
        player_pos + Vec3::Y * eye_height
    }

    /// Compute the desired (unoccluded) camera position and the ray from eye to it.
    /// Returns `(eye, desired_pos)`.
    pub fn desired_follow_pos(&self, player_pos: Vec3, eye_height: f32, capsule_radius: f32) -> (Vec3, Vec3) {
        let eye = Self::eye_pos(player_pos, eye_height);
        let desired = match self.perspective {
            Perspective::ThirdPersonBack => {
                let back = -self.front();
                eye + back * self.arm_length_back + Vec3::Y * 0.5
            }
            Perspective::ThirdPersonFront => {
                let front = self.front();
                eye + front * self.arm_length_front + Vec3::Y * 0.25
            }
            Perspective::FirstPerson => {
                eye + self.front() * capsule_radius
            }
        };
        (eye, desired)
    }

    /// Update the camera position using wall-clip occlusion data.
    ///
    /// `eye`        — world-space eye position (origin of the camera ray)
    /// `desired`    — unclamped desired camera world position
    /// `hit_dist`   — ray distance to the nearest static geometry hit, if any
    /// `dt`         — frame delta time for smooth arm recovery
    pub fn apply_occlusion(&mut self, eye: Vec3, desired: Vec3, hit_dist: Option<f32>, dt: f32) {
        match self.perspective {
            Perspective::FirstPerson => {
                // First-person: no arm-length occlusion; physics prevents the player
                // from embedding in walls, so the camera follows without clamping.
                self.position = desired;
            }
            Perspective::ThirdPersonBack | Perspective::ThirdPersonFront => {
                let to_desired = desired - eye;
                let full_dist = to_desired.length();
                let ray_dir = if full_dist > 1e-6 { to_desired / full_dist } else { Vec3::NEG_Z };

                let eff = match self.perspective {
                    Perspective::ThirdPersonBack  => &mut self.effective_arm_back,
                    Perspective::ThirdPersonFront => &mut self.effective_arm_front,
                    _ => unreachable!(),
                };

                // Wall-clamped distance: clear sky = full_dist.
                let wall_dist = hit_dist
                    .map(|d| (d - WALL_CLEARANCE).max(MIN_ARM))
                    .unwrap_or(full_dist);

                if wall_dist < *eff {
                    // Wall is closer: snap camera in immediately to avoid clipping.
                    *eff = wall_dist;
                } else {
                    // Wall retreated or gone: lerp back toward the desired arm length.
                    *eff = (*eff + ARM_RECOVERY_SPEED * dt).min(wall_dist);
                }

                self.position = eye + ray_dir * *eff;
            }
        }
    }

    /// Advance the camera yaw back toward `character_yaw` with proportional ease-out.
    /// Speed scales with angular distance (fast when far, slows moderately when close).
    /// Returns `true` when the target is reached.
    pub fn tick_free_look_return(&mut self, dt: f32) -> bool {
        // Degrees/s per degree of remaining separation — gives the ease-out curve.
        const SPEED_FACTOR: f32 = 3.5;
        // Floor speed so the camera doesn't crawl at the end.
        const MIN_SPEED: f32 = 60.0;

        self.free_look_return_elapsed += dt;

        let diff = self.character_yaw - self.yaw;
        // Normalise to the shortest path in [-180, 180].
        let diff = diff - 360.0 * (diff / 360.0).round();

        let speed = (diff.abs() * SPEED_FACTOR).max(MIN_SPEED);
        let step = speed * dt;

        if diff.abs() <= step {
            self.yaw = self.character_yaw;
            true
        } else {
            self.yaw += diff.signum() * step;
            false
        }
    }

    pub fn look(&mut self, mouse_dx: f32, mouse_dy: f32) {
        self.yaw += mouse_dx * self.sensitivity;
        self.pitch -= mouse_dy * self.sensitivity;
        self.pitch = self.pitch.clamp(-89.0, 89.0);
    }

    pub fn move_wasd(&mut self, input: &InputState, dt: f32) {
        let front = self.front();
        let right = front.cross(Vec3::Y).normalize();
        let velocity = self.speed * dt;

        if input.is_key_held(Scancode::W) {
            self.position += front * velocity;
        }
        if input.is_key_held(Scancode::S) {
            self.position -= front * velocity;
        }
        if input.is_key_held(Scancode::A) {
            self.position -= right * velocity;
        }
        if input.is_key_held(Scancode::D) {
            self.position += right * velocity;
        }
    }

    pub fn front(&self) -> Vec3 {
        let yaw_rad = self.yaw.to_radians();
        let pitch_rad = self.pitch.to_radians();
        Vec3::new(
            yaw_rad.cos() * pitch_rad.cos(),
            pitch_rad.sin(),
            yaw_rad.sin() * pitch_rad.cos(),
        )
        .normalize()
    }

    pub fn view_matrix(&self) -> Mat4 {
        if self.perspective == Perspective::ThirdPersonFront {
            // Look back toward the player (opposite of front direction)
            let back = -self.front();
            Mat4::look_at_rh(self.position, self.position + back, Vec3::Y)
        } else {
            let front = self.front();
            Mat4::look_at_rh(self.position, self.position + front, Vec3::Y)
        }
    }

    pub fn projection_matrix(&self, aspect: f32) -> Mat4 {
        Mat4::perspective_rh_gl(self.fov.to_radians(), aspect, 0.1, 1000.0)
    }
}
