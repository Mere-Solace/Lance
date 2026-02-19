#version 330 core

layout(location = 0) in vec3 a_position;
layout(location = 1) in vec3 a_normal;

uniform mat4 u_model;
uniform mat4 u_view;
uniform mat4 u_projection;
uniform mat4 u_cascade_light_space[3];

out vec3  v_world_pos;
out vec3  v_normal;
out float v_view_z;
out vec4  v_cascade_pos[3];

void main() {
    vec4 world    = u_model * vec4(a_position, 1.0);
    vec4 view_pos = u_view * world;
    v_world_pos   = world.xyz;
    v_normal      = mat3(transpose(inverse(u_model))) * a_normal;
    v_view_z      = view_pos.z; // negative in right-handed (fragment uses -v_view_z for depth)
    for (int i = 0; i < 3; ++i) {
        v_cascade_pos[i] = u_cascade_light_space[i] * world;
    }
    gl_Position = u_projection * view_pos;
}
