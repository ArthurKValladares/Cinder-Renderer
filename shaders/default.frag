#version 460

layout (set = 0, binding = 2) uniform sampler2D albedo_texture;

layout (location = 0) in vec4 i_color;
layout (location = 1) in vec2 i_uv;

layout (location = 0) out vec4 uFragColor;


void main() {
    uFragColor = texture(albedo_texture, i_uv) * i_color;
}