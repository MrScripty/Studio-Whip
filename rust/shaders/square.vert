#version 460
layout(location = 0) in vec2 inPosition;
layout(binding = 0) uniform UniformBufferObject {
    mat4 projection;
} ubo;

void main() {
    gl_Position = ubo.projection * vec4(inPosition, 0.0, 1.0);
}