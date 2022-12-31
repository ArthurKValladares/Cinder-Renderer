#version 450

#extension GL_EXT_nonuniform_qualifier : enable

layout (set = 0, binding = 0) uniform sampler2D textures[];

layout (location = 0) in vec2 i_uv;
layout (location = 1) in vec4 i_color;

layout (location = 0) out vec4 uFragColor;

layout(push_constant) uniform constants
{
	layout (offset=8) uint texture_idx;
    uint pad;
};

void main() {
    uFragColor = i_color * texture(textures[texture_idx], i_uv);
}