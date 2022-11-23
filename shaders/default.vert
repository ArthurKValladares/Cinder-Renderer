
#version 460

#extension GL_GOOGLE_include_directive: require

#include "shared.glsl"

layout(set = 0, binding = 0) uniform UniformBufferObject {
    mat4 proj;
    mat4 view;
} ubo;

layout(set = 0, binding = 1) readonly buffer Vertices
{
	Vertex vertices[];
};

layout(push_constant) uniform constants
{
	vec4 pc_color;
};

layout (location = 0) out vec4 o_color;
layout (location = 1) out vec2 o_uv;

void main() {
    Vertex v = vertices[gl_VertexIndex];

    o_color = v.color + (pc_color * 0.2);
    o_uv = v.uv;
    gl_Position = ubo.proj * ubo.view * v.pos;
}