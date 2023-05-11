#version 450

// TODO: Put this in a DescriptorSet later
float AMBIENT_LIGHT_STRENGTH = 0.15;
float SPECULAR_LIGHT_STRENGTH = 0.5;

// TODO: light color
layout (location = 0) in vec3 i_pos;
layout (location = 1) in vec3 i_color;
layout (location = 2) in vec3 i_normal;
layout (location = 3) in vec3 i_light_pos;
layout (location = 4) in vec3 i_view_from;
layout (location = 5) in vec3 i_light_look_at;
layout (location = 6) in vec3 i_light_color;

layout (location = 0) out vec4 uFragColor;

void main() {
    vec3 light_dir = normalize(i_light_pos - i_light_look_at);
    vec3 ray_dir = normalize(i_light_pos - i_pos);

    // Ambient
    vec3 ambient = (AMBIENT_LIGHT_STRENGTH * vec3(1.0)) * i_color;
    
    // Spotlight
    float theta = acos(dot(ray_dir, light_dir));
    float cutoff = 0.35;
    if (theta < cutoff) {    
        // Diffuse
        vec3 diffuse = max(dot(normalize(i_normal), light_dir), 0.0) * i_light_color * i_color;

        uFragColor = vec4(diffuse + ambient, 1.0);
    } else {
        uFragColor = vec4(ambient, 1.0);
    }
}