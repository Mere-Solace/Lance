use glam::Vec3;

/// Directional light component (sun-like). Casts shadows via cascaded shadow mapping.
pub struct DirectionalLight {
    pub direction: Vec3,
    pub color: Vec3,
    pub intensity: f32,
    /// Per-cascade shadow map resolution (width = height). Default 2048.
    pub shadow_resolution: u32,
}

/// Point light component with distance attenuation.
pub struct PointLight {
    pub color: Vec3,
    pub intensity: f32,
    pub radius: f32,
    pub constant: f32,
    pub linear: f32,
    pub quadratic: f32,
}

impl PointLight {
    pub fn new(color: Vec3, intensity: f32, radius: f32) -> Self {
        Self {
            color,
            intensity,
            radius,
            constant: 1.0,
            linear: 4.5 / radius,
            quadratic: 75.0 / (radius * radius),
        }
    }
}

/// Spot light component with cone angle and falloff.
pub struct SpotLight {
    pub direction: Vec3,
    pub color: Vec3,
    pub intensity: f32,
    pub inner_cone: f32,
    pub outer_cone: f32,
    pub radius: f32,
    pub constant: f32,
    pub linear: f32,
    pub quadratic: f32,
}

impl SpotLight {
    pub fn new(direction: Vec3, color: Vec3, intensity: f32, inner_deg: f32, outer_deg: f32, radius: f32) -> Self {
        Self {
            direction: direction.normalize(),
            color,
            intensity,
            inner_cone: inner_deg.to_radians().cos(),
            outer_cone: outer_deg.to_radians().cos(),
            radius,
            constant: 1.0,
            linear: 4.5 / radius,
            quadratic: 75.0 / (radius * radius),
        }
    }
}
