#version 450

layout(location = 0) in vec2 inPosition;

// Set 0: Global and Per-Object Data
layout(set = 0, binding = 0) uniform GlobalUbo {
    mat4 projection;
} globalData;

layout(set = 0, binding = 1) uniform ObjectUbo {
    mat4 transform;
} objectData;

void main() {
    // Combine projection and object transform
    // Note: Background quad might ignore objectData.transform if its UBO isn't updated,
    // or we could add a flag/special handling if needed. For now, assume all shapes use it.
    gl_Position = globalData.projection * objectData.transform * vec4(inPosition, 0.0, 1.0);
}