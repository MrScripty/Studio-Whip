#version 450

layout(location = 0) in vec2 inPosition;

// Set 0: Bindings for projection and transformation
layout(set = 0, binding = 0) uniform GlobalProjection {
    mat4 proj;
} global_ubo;

layout(set = 0, binding = 1) uniform ObjectTransform {
    mat4 model;
} object_ubo;

void main() {
    // Standard transformation: position -> model -> projection
    // Note: Bevy's GlobalTransform already includes parent transforms,
    // so object_ubo.model is the final world matrix for the cursor entity.
    gl_Position = global_ubo.proj * object_ubo.model * vec4(inPosition, 0.0, 1.0);
}