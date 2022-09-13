
#version 460

layout(location = 0) in vec3 i_pos;
layout(location = 1) in vec4 i_color;

layout (location = 0) out vec4 o_color;

void main() {
    o_color = i_color;
    gl_Position = vec4(i_pos, 1.0);
}