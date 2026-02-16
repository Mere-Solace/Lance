#version 330 core

in vec2 v_texcoord;
out vec4 frag_color;

uniform sampler2D u_font_atlas;
uniform vec3 u_text_color;

void main() {
    float alpha = texture(u_font_atlas, v_texcoord).r;
    if (alpha < 0.5) discard;
    frag_color = vec4(u_text_color, alpha);
}
