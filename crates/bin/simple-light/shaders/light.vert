
#version 460

const vec3 DEFAULT_CAMERA_DIR = vec3(1.0, 0.0, 0.0);

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

float angle_vectors(vec3 a, vec3 b) {
    vec3 a_n = normalize(a);
    vec3 b_n = normalize(b);

    return acos(dot(a_n, b_n));
}

mat4 rotation_matrix(vec3 axis, float angle)
{
    axis = normalize(axis);
    float s = sin(angle);
    float c = cos(angle);
    float oc = 1.0 - c;
    
    return mat4(oc * axis.x * axis.x + c,           oc * axis.x * axis.y - axis.z * s,  oc * axis.z * axis.x + axis.y * s,  0.0,
                oc * axis.x * axis.y + axis.z * s,  oc * axis.y * axis.y + c,           oc * axis.y * axis.z - axis.x * s,  0.0,
                oc * axis.z * axis.x - axis.y * s,  oc * axis.y * axis.z + axis.x * s,  oc * axis.z * axis.z + c,           0.0,
                0.0,                                0.0,                                0.0,                                1.0);
}

void main() {
    o_color = vec4(PushConstants.color, 1.0);

    // TODO: A bit more work on properly rotating the light
    vec3 camera_dir = normalize(l_ubo.position.xyz - l_ubo.look_at.xyz);
    vec3 camera_dir_no_y = normalize(vec3(camera_dir.x, 0.0, camera_dir.z));
    float angle = angle_vectors(DEFAULT_CAMERA_DIR, camera_dir_no_y);
    mat4 rotation_m = rotation_matrix(vec3(0.0, 1.0, 0.0), angle);

    mat4 translate_m = translate_matrix(l_ubo.position.xyz);

    mat4 model = translate_m * rotation_m;

    gl_Position = c_ubo.proj * c_ubo.view * model * vec4(i_pos, 1.0);
}