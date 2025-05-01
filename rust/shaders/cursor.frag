#version 450

layout(location = 0) out vec4 outColor;

void main() {
    // Simple solid white cursor for now.
    // Could later use uniforms for color or blinking.
    outColor = vec4(1.0, 0.0, 0.0, 1.0);
}