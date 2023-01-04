#version 460

#extension GL_EXT_nonuniform_qualifier : enable

layout (set = 0, binding = 2) uniform sampler2D textures[];

layout (location = 0) in vec4 i_color;
layout (location = 1) in vec4 i_normal;
layout (location = 2) in vec2 i_uv;

layout (location = 0) out vec4 uFragColor;

layout(push_constant) uniform constants
{
	layout (offset=16) uint texture_idx;
    uint pad;
};

void main() {
    uFragColor = texture(textures[texture_idx], i_uv) * i_color;
}