#version 450

// TODO: Put this in a DescriptorSet later
float AMBIENT_LIGHT_STRENGTH = 0.15;

// TODO: light color
layout (location = 0) in vec3 i_pos;
layout (location = 1) in vec3 i_color;
layout (location = 2) in vec3 i_normal;
layout (location = 3) in vec3 i_light_pos;
layout (location = 4) in vec3 i_view_from;
layout (location = 5) in vec3 i_light_look_at;
layout (location = 6) in vec3 i_light_color;
layout (location = 7) in vec4 i_light_space_pos;

layout (location = 0) out vec4 uFragColor;

layout (set = 2, binding = 0) uniform sampler2D image;

float compute_shadow_factor(vec4 light_space_pos, sampler2D shadow_map)
{
   // Convert light space position to NDC
   vec3 light_space_ndc = light_space_pos.xyz /= light_space_pos.w;
 
   // If the fragment is outside the light's projection then it is outside
   // the light's influence, which means it is in the shadow (notice that
   // such sample would be outside the shadow map image)
   if (abs(light_space_ndc.x) > 1.0 ||
       abs(light_space_ndc.y) > 1.0 ||
       abs(light_space_ndc.z) > 1.0)
      return 0.0;
 
   // Translate from NDC to shadow map space (Vulkan's Z is already in [0..1])
   vec2 shadow_map_coord = light_space_ndc.xy * 0.5 + 0.5;
 
   // Check if the sample is in the light or in the shadow
   if (light_space_ndc.z > texture(shadow_map, shadow_map_coord.xy).x)
      return 0.0; // In the shadow
 
   // In the light
   return 1.0;
}  

void main() {
    vec3 light_dir = normalize(i_light_pos - i_light_look_at);
    vec3 ray_dir = normalize(i_light_pos - i_pos);

    float shadow_factor = compute_shadow_factor(i_light_space_pos, image);
    if (shadow_factor < 1.0) {
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
    } else {
        uFragColor = vec4(vec3(0.0), 1.0);
    }
}