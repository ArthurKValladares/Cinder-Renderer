#version 460

layout(location = 0) in vec3 i_pos;
// TODO: Right now I can't just skip this field in the pipeline vertex attributes, but I will
// Add that functionality. Need to be able to "override" auto-gen stuff, but one specific pieces
// Will also allow be to get rid of hacky low precision stuff
layout(location = 1) in vec3 i_normal;

layout(set = 0, binding = 0) uniform CameraUniformBufferObject {
    mat4 view;
    mat4 proj;
} c_ubo;

layout(set = 1, binding = 0 ) uniform ModelUniformBufferObject {
    mat4 model;
} m_ubo;

void main() {
    gl_Position = c_ubo.proj * c_ubo.view * m_ubo.model * vec4(i_pos, 1.0);
}