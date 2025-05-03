#version 450

// Define the push constant block
layout(push_constant) uniform PushConsts {
    vec4 color; // Expecting RGBA color
} pc;

layout(location = 0) out vec4 outColor;

void main() {
    // Output the color passed via push constants
    outColor = pc.color;
}