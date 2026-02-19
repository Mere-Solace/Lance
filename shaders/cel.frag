#version 330 core

in vec3  v_world_pos;
in vec3  v_normal;
in float v_view_z;
in vec4  v_cascade_pos[3];

// Directional light (sun)
uniform vec3  u_dir_light_dir;
uniform vec3  u_dir_light_color;
uniform float u_dir_light_intensity;

// Cascaded shadow maps (3 cascades, separate samplers for GL 3.3 compatibility)
uniform sampler2D u_shadow_map_0;
uniform sampler2D u_shadow_map_1;
uniform sampler2D u_shadow_map_2;
uniform int       u_shadows_enabled;
// Camera-depth thresholds (positive, metres): [C0→C1 boundary, C1→C2 boundary]
uniform float     u_cascade_splits[2];

// Point lights (max 8)
#define MAX_POINT_LIGHTS 8
uniform int   u_num_point_lights;
uniform vec3  u_point_light_pos[MAX_POINT_LIGHTS];
uniform vec3  u_point_light_color[MAX_POINT_LIGHTS];
uniform float u_point_light_intensity[MAX_POINT_LIGHTS];
uniform float u_point_light_constant[MAX_POINT_LIGHTS];
uniform float u_point_light_linear[MAX_POINT_LIGHTS];
uniform float u_point_light_quadratic[MAX_POINT_LIGHTS];

// Spot lights (max 4)
#define MAX_SPOT_LIGHTS 4
uniform int   u_num_spot_lights;
uniform vec3  u_spot_light_pos[MAX_SPOT_LIGHTS];
uniform vec3  u_spot_light_dir[MAX_SPOT_LIGHTS];
uniform vec3  u_spot_light_color[MAX_SPOT_LIGHTS];
uniform float u_spot_light_intensity[MAX_SPOT_LIGHTS];
uniform float u_spot_light_inner_cone[MAX_SPOT_LIGHTS];
uniform float u_spot_light_outer_cone[MAX_SPOT_LIGHTS];
uniform float u_spot_light_constant[MAX_SPOT_LIGHTS];
uniform float u_spot_light_linear[MAX_SPOT_LIGHTS];
uniform float u_spot_light_quadratic[MAX_SPOT_LIGHTS];

uniform vec3  u_object_color;
uniform vec3  u_object_color_2;
uniform vec3  u_ambient_color;
uniform vec3  u_camera_pos;
uniform vec3  u_fog_color;
uniform float u_fog_start;
uniform float u_fog_end;
uniform int   u_checkerboard;

out vec4 frag_color;

// Cel-shade an NdotL value into 3-band discrete intensity
float cel_band(float ndotl) {
    if (ndotl > 0.6)  return 1.0;
    if (ndotl > 0.2)  return 0.6;
    if (ndotl > -0.1) return 0.35;
    return 0.2;
}

// PCF 3x3 shadow test for one cascade
float pcf_shadow(sampler2D shadow_map, vec4 ls_pos, float bias) {
    vec3 proj = ls_pos.xyz / ls_pos.w;
    proj = proj * 0.5 + 0.5;
    if (proj.z > 1.0) return 0.0;

    float shadow = 0.0;
    vec2 texel_size = 1.0 / textureSize(shadow_map, 0);
    for (int x = -1; x <= 1; ++x) {
        for (int y = -1; y <= 1; ++y) {
            float d = texture(shadow_map, proj.xy + vec2(x, y) * texel_size).r;
            shadow += (proj.z - bias > d) ? 1.0 : 0.0;
        }
    }
    return shadow / 9.0;
}

// Select cascade by camera depth and sample the appropriate shadow map
float calc_shadow(vec3 N) {
    if (u_shadows_enabled == 0) return 0.0;

    float bias = max(0.005 * (1.0 - dot(N, normalize(-u_dir_light_dir))), 0.001);
    float depth = -v_view_z; // positive camera distance

    if (depth < u_cascade_splits[0])
        return pcf_shadow(u_shadow_map_0, v_cascade_pos[0], bias);
    else if (depth < u_cascade_splits[1])
        return pcf_shadow(u_shadow_map_1, v_cascade_pos[1], bias);
    else
        return pcf_shadow(u_shadow_map_2, v_cascade_pos[2], bias);
}

void main() {
    vec3 N = normalize(v_normal);

    // Base color (optional checkerboard)
    vec3 base_color = u_object_color;
    if (u_checkerboard != 0) {
        float checker = mod(floor(v_world_pos.x) + floor(v_world_pos.z), 2.0);
        base_color = mix(u_object_color, u_object_color_2, checker);
    }

    // Directional light (sun) with cascaded shadows
    vec3  L_dir      = normalize(-u_dir_light_dir);
    float ndotl_dir  = dot(N, L_dir);
    float shadow     = calc_shadow(N);
    vec3  dir_contribution = u_dir_light_color * u_dir_light_intensity
                           * cel_band(ndotl_dir) * (1.0 - shadow);

    // Point lights
    vec3 point_contribution = vec3(0.0);
    for (int i = 0; i < u_num_point_lights; i++) {
        vec3  to_light  = u_point_light_pos[i] - v_world_pos;
        float dist      = length(to_light);
        vec3  L         = to_light / dist;
        float intensity = cel_band(dot(N, L));
        float atten     = 1.0 / (u_point_light_constant[i]
                               + u_point_light_linear[i]    * dist
                               + u_point_light_quadratic[i] * dist * dist);
        point_contribution += u_point_light_color[i] * u_point_light_intensity[i] * intensity * atten;
    }

    // Spot lights
    vec3 spot_contribution = vec3(0.0);
    for (int i = 0; i < u_num_spot_lights; i++) {
        vec3  to_light  = u_spot_light_pos[i] - v_world_pos;
        float dist      = length(to_light);
        vec3  L         = to_light / dist;
        float intensity = cel_band(dot(N, L));
        float theta     = dot(L, normalize(-u_spot_light_dir[i]));
        float epsilon   = u_spot_light_inner_cone[i] - u_spot_light_outer_cone[i];
        float spot_fac  = clamp((theta - u_spot_light_outer_cone[i]) / epsilon, 0.0, 1.0);
        float atten     = 1.0 / (u_spot_light_constant[i]
                               + u_spot_light_linear[i]    * dist
                               + u_spot_light_quadratic[i] * dist * dist);
        spot_contribution += u_spot_light_color[i] * u_spot_light_intensity[i] * intensity * atten * spot_fac;
    }

    // Combine lighting
    vec3 total_light = u_ambient_color + dir_contribution + point_contribution + spot_contribution;
    vec3 lit_color   = base_color * total_light;

    // Linear depth fog
    float fog_dist   = length(v_world_pos - u_camera_pos);
    float fog_factor = clamp((u_fog_end - fog_dist) / (u_fog_end - u_fog_start), 0.0, 1.0);
    frag_color = vec4(mix(u_fog_color, lit_color, fog_factor), 1.0);
}
