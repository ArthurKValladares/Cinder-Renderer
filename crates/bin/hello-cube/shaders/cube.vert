
#version 460

layout(location = 0) in vec3 i_pos;
layout(location = 1) in vec3 i_normal;

layout (location = 0) out vec4 o_color;

layout( push_constant ) uniform constants
{
	mat4 transform;
} PushConstants;

void main() {
    o_color = vec4(i_normal, 1.0);

    gl_Position = PushConstants.transform * vec4(i_pos, 1.0);
}