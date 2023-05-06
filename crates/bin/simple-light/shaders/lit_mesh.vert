#version 460

layout(location = 0) in vec3 i_pos;
layout(location = 1) in vec3 i_normal;

layout (location = 0) out vec4 o_color;

// TODO: Put this in a DescriptorSet later
float AMBIENT_LIGHT_STRENGTH = 0.2;

layout(set = 0, binding = 0) uniform CameraUniformBufferObject {
    mat4 view;
    mat4 proj;
} c_ubo;

layout(set = 0, binding = 1) uniform GlobalLightData {
    vec3 position;
    vec3 look_at;
} l_ubo;

layout(set = 1, binding = 0 ) uniform ModelUniformBufferObject {
    mat4 model;
} m_ubo;

layout( push_constant ) uniform constants
{
    vec3 color;
} PushConstants;

void main() {
    vec3 ambient_color = AMBIENT_LIGHT_STRENGTH * PushConstants.color;

    o_color = vec4(ambient_color, 1.0);

    gl_Position = c_ubo.proj * c_ubo.view * m_ubo.model * vec4(i_pos, 1.0);
}