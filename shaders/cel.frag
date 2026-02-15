#version 330 core

in vec3 v_world_pos;
in vec3 v_normal;

uniform vec3 u_light_dir;
uniform vec3 u_object_color;
uniform vec3 u_ambient_color;
uniform vec3 u_camera_pos;
uniform vec3 u_fog_color;
uniform float u_fog_start;
uniform float u_fog_end;

out vec4 frag_color;

void main() {
    vec3 N = normalize(v_normal);
    vec3 L = normalize(-u_light_dir);
    float NdotL = dot(N, L);

    // 3-band cel shading
    float intensity;
    if (NdotL > 0.6) {
        intensity = 1.0;
    } else if (NdotL > 0.2) {
        intensity = 0.6;
    } else if (NdotL > -0.1) {
        intensity = 0.35;
    } else {
        intensity = 0.2;
    }

    vec3 lit_color = u_object_color * (u_ambient_color + vec3(intensity));

    // Linear depth fog
    float dist = length(v_world_pos - u_camera_pos);
    float fog_factor = clamp((u_fog_end - dist) / (u_fog_end - u_fog_start), 0.0, 1.0);
    vec3 final_color = mix(u_fog_color, lit_color, fog_factor);

    frag_color = vec4(final_color, 1.0);
}
