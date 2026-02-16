#version 330 core

layout(location = 0) in vec2 a_pos;

uniform mat4 u_projection;

void main() {
    gl_Position = u_projection * vec4(a_pos, 0.0, 1.0);
}
