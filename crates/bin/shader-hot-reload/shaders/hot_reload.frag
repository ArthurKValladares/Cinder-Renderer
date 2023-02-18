#version 450

layout(binding = 0) uniform sampler2D texSampler;

layout (location = 0) in vec2 i_uv;

layout (location = 0) out vec4 uFragColor;

void main() {
    uFragColor = texture(texSampler, i_uv) * vec4(0.2, 0.7, 0.5, 1.0);
}