#version 460
layout(binding = 0) uniform UBO {
    mat4 projection;
} ubo;
layout(binding = 1) uniform Offset {
    vec2 offset;
};

layout(location = 0) in vec2 inPosition;
layout(location = 1) in vec2 instanceOffset;

void main() {
    gl_Position = ubo.projection * vec4(inPosition + offset + instanceOffset, 0.0, 1.0);
}