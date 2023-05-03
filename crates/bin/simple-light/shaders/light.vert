
#version 460

layout(location = 0) in vec3 i_pos;

layout (location = 0) out vec4 o_color;

layout(set = 0, binding = 0) uniform CameraUniformBufferObject {
    mat4 view;
    mat4 proj;
} c_ubo;

layout(set = 0, binding = 1) uniform GlobalLightData {
    vec4 position;
    vec4 look_at;
} l_ubo;

layout( push_constant ) uniform constants
{
	vec3 color;
} PushConstants;

mat4 translate_matrix(vec3 pos) {
    return mat4(
        1.0, 0.0, 0.0, 0.0,
        0.0, 1.0, 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0,
        pos.x, pos.y, pos.z, 1.0
    );
}

void main() {
    o_color = vec4(PushConstants.color, 1.0);

    // TODO: figure out how to get light model matrix from LightData
    mat4 transform = translate_matrix(l_ubo.position.xyz);
    gl_Position = c_ubo.proj * c_ubo.view * transform * vec4(i_pos, 1.0);
}