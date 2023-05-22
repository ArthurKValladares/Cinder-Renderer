
#version 460

const vec3 DEFAULT_CAMERA_DIR = vec3(0.0, 1.0, 0.0);

layout(location = 0) in vec3 i_pos;

layout (location = 0) out vec4 o_color;

layout(set = 0, binding = 0) uniform CameraUniformBufferObject {
    mat4 view;
    mat4 proj;
} c_ubo;

layout(set = 0, binding = 1) uniform GlobalLightData {
    mat4 view;
    mat4 proj;
    vec4 position;
    vec4 look_at;
} l_ubo;

layout( push_constant ) uniform constants
{
	vec3 color;
} PushConstants;

float angle_vectors(vec3 a, vec3 b) {
    return acos(dot(a, b));
}

mat4 translate_matrix(vec3 pos) {
    return mat4(
        1.0, 0.0, 0.0, 0.0,
        0.0, 1.0, 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0,
        pos.x, pos.y, pos.z, 1.0
    );
}

mat4 rotation_matrix(vec3 axis, float angle)
{
    axis = normalize(axis);
    float s = sin(angle);
    float c = cos(angle);
    float d = 1.0 - c;
    
    float x = axis.x * d;
    float y = axis.y * d;
    float z = axis.z * d;

    float axay = x * axis.y;
    float axaz = x * axis.z;
    float ayaz = y * axis.z;

    return mat4(c + x * axis.x,    axay + s * axis.z, axaz - s * axis.y, 0.0,
                axay - s * axis.z, c + y * axis.y,    ayaz + s * axis.x, 0.0,
                axaz + s * axis.y, ayaz - s * axis.x, c + z * axis.z,    0.0,
                0.0,               0.0,               0.0,               1.0);
}

void main() {
    o_color = vec4(PushConstants.color, 1.0);

    vec3 camera_dir = normalize(l_ubo.position.xyz - l_ubo.look_at.xyz);
    float rotation_angle = angle_vectors(DEFAULT_CAMERA_DIR, camera_dir);
    vec3 rotation_axis = normalize(cross(DEFAULT_CAMERA_DIR, camera_dir));
    mat4 rotation_m = rotation_matrix(rotation_axis, rotation_angle);

    mat4 translate_m = translate_matrix(l_ubo.position.xyz);

    mat4 model = translate_m * rotation_m;

    gl_Position = c_ubo.proj * c_ubo.view * model * vec4(i_pos, 1.0);
}