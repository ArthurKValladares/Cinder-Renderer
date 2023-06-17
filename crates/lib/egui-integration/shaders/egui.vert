#version 450

layout(location = 0) in vec2 i_pos;
layout(location = 1) in vec2 i_uv;
layout(location = 2) in vec4 i_color;
layout(binding = 1, location = 3) in vec2 test;

layout (location = 0) out vec2 o_uv;
layout (location = 1) out vec4 o_color;

layout( push_constant ) uniform constants
{
	vec2 screen_size;
} PushConstants;

vec3 srgb_to_linear(vec3 srgb) {
    bvec3 cutoff = lessThan(srgb, vec3(0.04045));
    vec3 lower = srgb / vec3(12.92);
    vec3 higher = pow((srgb + vec3(0.055)) / vec3(1.055), vec3(2.4));
    return mix(higher, lower, cutoff);
}

void main() {
    o_uv = i_uv;
    o_color = vec4(srgb_to_linear(i_color.rgb), i_color.a);
    
    gl_Position = vec4(
        2.0 * i_pos.x / PushConstants.screen_size.x - 1.0,
        2.0 * i_pos.y / PushConstants.screen_size.y - 1.0,
        0.0,
        1.0
    );
}