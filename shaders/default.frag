#version 460


layout (location = 0) in vec4 i_color;

layout (location = 0) out vec4 uFragColor;


void main() {
    uFragColor = i_color;
}