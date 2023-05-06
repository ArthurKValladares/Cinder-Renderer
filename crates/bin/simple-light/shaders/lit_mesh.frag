#version 450

// TODO: Put this in a DescriptorSet later
float AMBIENT_LIGHT_STRENGTH = 0.2;
float SPECULAR_LIGHT_STRENGTH = 0.5;

// TODO: light color
layout (location = 0) in vec3 i_pos;
layout (location = 1) in vec3 i_color;
layout (location = 2) in vec3 i_normal;
layout (location = 3) in vec3 i_light_pos;
layout (location = 4) in vec3 i_view_from;

layout (location = 0) out vec4 uFragColor;

void main() {
    vec3 light_color = vec3(1.0);

    // Ambient
    vec3 ambient = AMBIENT_LIGHT_STRENGTH * light_color;

    // Diffuse
    vec3 norm = normalize(i_normal);
    vec3 light_dir = normalize(i_light_pos - i_pos);
    float diff = max(dot(norm, light_dir), 0.0);
    vec3 diffuse = diff * light_color;

    // Specular
    vec3 view_dir = normalize(i_view_from - i_pos);
    vec3 reflect_dir = reflect(-light_dir, norm);
    float spec = pow(max(dot(view_dir, reflect_dir), 0.0), 32);
    vec3 specular = SPECULAR_LIGHT_STRENGTH * spec * light_color; 

    vec3 result = (ambient + diffuse + specular) * i_color;
    uFragColor = vec4(result, 1.0);
}