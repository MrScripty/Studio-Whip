#version 460

// Binding 0: Global projection matrix
layout(binding = 0) uniform UBO {
    mat4 projection;
} ubo;

// Binding 1: Per-object transformation matrix
// (Even though the background might have an identity matrix,
// the binding point expects a mat4 based on the Rust code)
layout(binding = 1) uniform ObjectTransform {
    mat4 transform; // Changed from vec2 offset to mat4 transform
} object;

// Location 0: Input vertex position (local space)
layout(location = 0) in vec2 inPosition;

void main() {
    // Apply object transform first, then projection
    gl_Position = ubo.projection * object.transform * vec4(inPosition, 0.0, 1.0);
}