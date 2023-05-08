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

layout (location = 0) out vec4 uFragColor;

void main() {
    vec3 light_color = vec3(1.0);
    vec3 light_dir = normalize(i_light_pos - i_light_look_at);
    vec3 ray_dir = normalize(i_light_pos - i_pos);

    // Pretty hacky lighting calculations, just to show shadow maps working
    float cutoff = 0.35;
    float constant = 1.0;
    float linear = 0.07;
    float quadratic = 0.017;

    // Ambient
    vec3 ambient = (AMBIENT_LIGHT_STRENGTH * light_color) * i_color;
    
    // spotlight
    float theta = acos(dot(ray_dir, light_dir));
    if (theta < cutoff) {
        vec3 norm = normalize(i_normal);
        vec3 light_dir = normalize(i_light_pos - i_pos);

        float diff = max(dot(norm, light_dir), 0.0);
        float diff_cutoff = 0.05;
        if (diff > diff_cutoff) {
            // Diffuse
            vec3 diffuse = diff * light_color * i_color;

            // Intensity
            float inner_cutoff = cutoff * 0.75;
            float epsilon = cutoff - inner_cutoff;
            float intensity = clamp((cutoff - theta) / epsilon, 0.0, 1.0);
            diffuse *= intensity;

            // Attenuation
            float distance = length(i_light_pos - i_pos);
            float attenuation = 1.0 / (constant + linear * distance + quadratic * (distance * distance));   
            diffuse *= attenuation;

            uFragColor = vec4(diffuse + ambient, 1.0);
        } else {
            uFragColor = vec4(ambient, 1.0);    
        }
    } else {
        uFragColor = vec4(ambient, 1.0);
    }
}