#version 450

layout(binding = 0) uniform sampler2D texSampler;

layout (location = 0) in vec2 i_uv;

layout (location = 0) out vec4 uFragColor;

void main() {
    vec3 color = texture(texSampler, i_uv).rgb;
    uFragColor = vec4(vec3(sqrt(color.r)), 1.0);
}