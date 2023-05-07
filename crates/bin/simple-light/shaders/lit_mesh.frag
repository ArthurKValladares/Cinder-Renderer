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
layout (location = 5) in vec4 i_light_data;
layout (location = 6) in vec3 i_light_look_at;

layout (location = 0) out vec4 uFragColor;

void main() {
    vec3 light_color = vec3(1.0);
    vec3 light_dir = normalize(i_light_pos - i_light_look_at);

    float cutoff = i_light_data.x;
    float constant = i_light_data.y;
    float linear = i_light_data.z;
    float quadratic = i_light_data.w;

    // Ambient
    vec3 ambient = (AMBIENT_LIGHT_STRENGTH * light_color) * i_color;
    
    // Diffuse
    vec3 norm = normalize(i_normal);
    vec3 ray_dir = normalize(i_light_pos - i_pos);
    float diff = max(dot(norm, ray_dir), 0.0);
    vec3 diffuse = (diff * light_color) * i_color;

    // spotlight
    float theta = dot(ray_dir, -light_dir);
    float outer_cutoff = cutoff * 1.2;
    float epsilon = (cutoff - outer_cutoff);
    float intensity = clamp((theta - outer_cutoff) / epsilon, 0.0, 1.0);
    diffuse  *= intensity;

    // attenuation
    float distance = length(i_light_pos - i_pos);
    float attenuation = 1.0 / (constant + linear * distance + quadratic * (distance * distance));   
    diffuse *= attenuation;

    vec3 result = ambient + diffuse;
    uFragColor = vec4(result, 1.0);
}