
#version 460

layout(location = 0) in vec3 i_pos;
layout(location = 1) in vec2 i_uv;

layout (location = 0) out vec2 o_uv;

layout(binding = 0) uniform UniformBufferObject {
    mat4 model;
    mat4 view;
    mat4 proj;
} ubo;

void main() {
    o_uv = i_uv;

    gl_Position = ubo.proj * ubo.view * ubo.model * vec4(i_pos, 1.0);
}