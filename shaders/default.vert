
#version 460

layout(set = 0, binding = 0) uniform UniformBufferObject {
    mat4 proj_view;
} ubo;

layout( push_constant ) uniform constants
{
	vec4 model;
} PushConstants;

layout(location = 0) in vec3 i_pos;
layout(location = 1) in vec2 i_uv;
layout(location = 2) in vec4 i_color;

layout (location = 0) out vec4 o_color;
layout (location = 1) out vec2 o_uv;

void main() {
    o_color = i_color;
    o_uv = i_uv;
    vec4 pos = ubo.proj_view * vec4(i_pos, 1.0);
    gl_Position = pos;
}