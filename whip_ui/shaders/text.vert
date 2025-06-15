#version 450

// Input vertex attributes
layout(location = 0) in vec2 inPosition; // Relative position
layout(location = 1) in vec2 inUV;

// Input uniform buffers
layout(set = 0, binding = 0) uniform GlobalUbo {
    mat4 projection;
} global_ubo;

// Per-entity transform
layout(set = 0, binding = 1) uniform ObjectUbo {
    mat4 transform; // This holds the entity's world matrix
} object_ubo;

// Output to fragment shader
layout(location = 0) out vec2 fragUV;

void main() {
    // Apply object transform FIRST, then projection
    gl_Position = global_ubo.projection * object_ubo.transform * vec4(inPosition, 0.0, 1.0);
    fragUV = inUV;
}