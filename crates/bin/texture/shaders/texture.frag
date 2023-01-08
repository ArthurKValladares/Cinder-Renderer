#version 450

layout(binding = 1) uniform sampler2D texSampler;

layout (location = 0) in vec2 i_uv;

layout (location = 0) out vec4 uFragColor;

void main() {
    uFragColor = texture(texSampler, i_uv);
}