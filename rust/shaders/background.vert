#version 460

layout(binding = 0) uniform UBO {
    mat4 projection;
} ubo;

// Binding 1 still exists but is unused by this shader
layout(binding = 1) uniform ObjectTransform {
    mat4 transform;
} object;

layout(location = 0) in vec2 inPosition;

void main() {
    // ONLY apply projection to the input position
    gl_Position = ubo.projection * vec4(inPosition, 0.0, 1.0);
}