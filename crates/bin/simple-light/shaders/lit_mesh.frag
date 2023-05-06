#version 450

// TODO: Put this in a DescriptorSet later
float AMBIENT_LIGHT_STRENGTH = 0.2;

layout (location = 0) in vec3 i_color;
layout (location = 1) in vec3 i_normal;
layout (location = 2) in vec3 i_light_pos;
layout (location = 3) in vec3 i_light_dir;
// TODO: light color

layout (location = 0) out vec4 uFragColor;

void main() {
    float diff = max(dot(i_normal, i_light_dir), 0.0);
    vec3 diffuse = diff * vec3(1.0);

    vec3 ambient = vec3(AMBIENT_LIGHT_STRENGTH);
    vec3 result = (ambient + diffuse) * i_color;

    uFragColor = vec4(result, 1.0);
}