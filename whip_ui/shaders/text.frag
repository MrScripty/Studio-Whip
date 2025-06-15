#version 450

layout(location = 0) in vec2 fragUV;

// The glyph atlas texture sampler
layout(set = 1, binding = 0) uniform sampler2D texSampler;

layout(location = 0) out vec4 outColor;

void main() {
    // Sample the texture (R8_UNORM). The 'r' component contains the alpha.
    float alpha = texture(texSampler, fragUV).r;

    // Use the sampled alpha. Output white text for now.
    // Later, you might pass the text color via vertex attributes or another UBO.
    outColor = vec4(1.0, 1.0, 1.0, alpha);

    // Discard fragments that are fully transparent (optional optimization)
    // if (alpha < 0.01) {
    //     discard;
    // }
}