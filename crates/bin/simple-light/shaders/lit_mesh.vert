#version 460

layout(location = 0) in vec3 i_pos;
layout(location = 1) in vec3 i_normal;

layout (location = 0) out vec3 o_pos;
layout (location = 1) out vec3 o_color;
layout (location = 2) out vec3 o_normal;
layout (location = 3) out vec3 o_light_pos;
layout (location = 4) out vec3 o_view_from;

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
    // TODO: this should be in a buffer
    vec3 view_from;
} PushConstants;

void main() {
    o_pos = vec3(m_ubo.model * vec4(i_pos, 1.0));
    o_color = PushConstants.color;
    o_normal = mat3(transpose(inverse(m_ubo.model))) * i_normal;
    o_light_pos = l_ubo.position;
    o_view_from = PushConstants.view_from;

    gl_Position = c_ubo.proj * c_ubo.view * m_ubo.model * vec4(i_pos, 1.0);
}