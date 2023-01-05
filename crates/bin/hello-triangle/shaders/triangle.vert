
#version 460

layout(location = 0) in vec2 i_pos;
layout(location = 1) in vec4 i_color;

layout (location = 0) out vec4 o_color;

layout( push_constant ) uniform constants
{
	mat4 transform;
} PushConstants;

void main() {
    o_color = i_color;

    gl_Position = PushConstants.transform * vec4(
        i_pos.x,
        i_pos.y,
        0.0,
        1.0
    );
}