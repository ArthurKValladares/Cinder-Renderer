#version 460

#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable
 
layout(location = 0) in vec2 i_pos;
layout(location = 1) in vec2 i_uv;
 
layout(location = 0) out vec2 o_uv;
 
void main()
{
   gl_Position = vec4(i_pos.x, i_pos.y, 0.0, 1.0);
   o_uv = i_uv;
}