#version 460

#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable
 
// TODO: If this is not zero, pipeline creation fails
layout (binding = 0) uniform sampler2D image;
 
layout(location = 0) in vec2 i_uv;
 
layout(location = 0) out vec4 o_color;
 
void main()
{
   float depth = texture(image, i_uv).r;
   o_color = vec4(vec3(sqrt(depth)), 1.0);
}