
#version 460

#extension GL_GOOGLE_include_directive: require
#extension GL_EXT_scalar_block_layout: enable

struct Vertex
{
	vec4 pos;
    vec3 color;
    vec3 normal;
    vec2 uv;
};

layout(binding = 0) uniform UniformBufferObject {
    mat4 model;
    mat4 view;
    mat4 proj;
} ubo;

layout(set = 0, binding = 1, scalar) readonly buffer Vertices
{
	Vertex vertices[];
};

layout (location = 0) out vec4 o_color;
layout (location = 1) out vec4 o_normal;
layout (location = 2) out vec2 o_uv;


void main() {
    Vertex v = vertices[gl_VertexIndex];

    o_color = vec4(v.color, 1.0);
    o_normal = vec4(v.normal, 1.0);
    o_uv = v.uv;

    gl_Position = ubo.proj * ubo.view * ubo.model * v.pos;
}