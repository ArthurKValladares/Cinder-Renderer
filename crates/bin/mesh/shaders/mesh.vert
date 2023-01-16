
#version 460

layout(location = 0) in vec3 i_pos;
layout(location = 1) in vec3 i_normal;
layout(location = 2) in vec2 i_uv;

layout (location = 0) out vec4 o_color;

layout(binding = 0) uniform UniformBufferObject {
    mat4 model;
    mat4 view;
    mat4 proj;
} ubo;

void main() {
    o_color = vec4(i_normal, 1.0);

    gl_Position = ubo.proj * ubo.view * ubo.model * vec4(i_pos, 1.0);
}