#version 460

#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable
 
layout (set = 1, binding = 0) uniform sampler2D image;
 
layout(location = 0) in vec2 i_uv;
 
layout(location = 0) out vec4 o_color;
 
void main()
{
   float depth = texture(image, i_uv).r;
   o_color = vec4(1.0 - (1.0 - depth) * 100.0);
}