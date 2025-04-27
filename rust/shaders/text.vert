#version 450

// Input vertex attributes
layout(location = 0) in vec2 inPosition;
layout(location = 1) in vec2 inUV;

// Output to fragment shader
layout(location = 0) out vec2 fragUV;

// Uniform Buffers
// Binding 0: Global projection matrix (same as shapes)
layout(binding = 0, set = 0) uniform GlobalUniformBufferObject {
    mat4 proj;
} globalUbo;

// Binding 1: Per-object transform matrix (same as shapes)
// We might not need this if transforms are applied CPU-side when generating vertices,
// but let's include it for potential future use (e.g., instancing).
// If CPU-side transform is used, this UBO might be unused for text.
layout(binding = 1, set = 0) uniform ObjectUniformBufferObject {
    mat4 model;
} objectUbo;


void main() {
    // Apply projection and model view matrices
    // For now, assume model transform is applied CPU-side, so just use projection.
    // If using objectUbo.model, multiply: globalUbo.proj * objectUbo.model * vec4(inPosition, 0.0, 1.0);
    gl_Position = globalUbo.proj * vec4(inPosition, 0.0, 1.0);

    // Pass UV coordinates to the fragment shader
    fragUV = inUV;
}