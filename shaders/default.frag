#version 460

layout (binding = 1) uniform sampler2D albedo_texture;

layout (location = 0) in vec3 i_color;
layout (location = 1) in vec2 i_uv;

layout (location = 0) out vec4 uFragColor;


void main() {
    uFragColor = texture(albedo_texture, i_uv) * vec4(i_color, 1.0);
}