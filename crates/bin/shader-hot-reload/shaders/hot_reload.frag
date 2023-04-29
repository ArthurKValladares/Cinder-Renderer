#version 450

layout(binding = 0) uniform sampler2D texSampler;

layout (location = 0) in vec2 i_uv;

layout (location = 0) out vec4 uFragColor;

void main() {
    vec4 color = vec4(1.0, 0.5, 1.0, 1.0);
    uFragColor = texture(texSampler, i_uv) * color;
}