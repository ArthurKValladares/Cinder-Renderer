
#version 460

#extension GL_GOOGLE_include_directive: require
#extension GL_EXT_scalar_block_layout: enable

#include "shared.glsl"

layout(set = 0, binding = 0) uniform UniformBufferObject {
    mat4 proj;
    mat4 view;
} ubo;

layout(set = 0, binding = 1, scalar) readonly buffer Vertices
{
	Vertex vertices[];
};

layout(push_constant) uniform constants
{
	vec4 pc_color;
};

layout (location = 0) out vec4 o_color;
layout (location = 1) out vec4 o_normal;
layout (location = 2) out vec2 o_uv;

void main() {
    Vertex v = vertices[gl_VertexIndex];

    o_color = mix(vec4(v.color, 1.0), pc_color, 0.5);
    o_normal = vec4(v.normal, 1.0);
    o_uv = v.uv;

    gl_Position = ubo.proj * ubo.view * v.pos;
}