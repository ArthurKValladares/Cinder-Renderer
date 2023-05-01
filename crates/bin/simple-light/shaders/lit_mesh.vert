#version 460

layout(location = 0) in vec3 i_pos;
layout(location = 1) in vec3 i_normal;

layout (location = 0) out vec4 o_color;

layout(set = 0, binding = 0) uniform CameraUniformBufferObject {
    mat4 view;
    mat4 proj;
} c_ubo;

layout(set = 1, binding = 0 ) uniform ModelUniformBufferObject {
    mat4 model;
} m_ubo;

layout( push_constant ) uniform constants
{
    vec4 light_pos;
    vec3 color;
} PushConstants;

void main() {
    o_color = vec4(PushConstants.color, 1.0);

    gl_Position = c_ubo.proj * c_ubo.view * m_ubo.model * vec4(i_pos, 1.0);
}