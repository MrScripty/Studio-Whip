#version 450

// Input from vertex shader
layout(location = 0) in vec2 fragUV;

// Output color
layout(location = 0) out vec4 outColor;

// Texture Sampler for the glyph atlas
// Binding 0, Set 1 (Different set from UBOs)
layout(binding = 0, set = 1) uniform sampler2D glyphAtlasSampler;

void main() {
    // Sample the glyph atlas texture using the UV coordinates
    // The texture contains the alpha mask (grayscale)
    float alpha = texture(glyphAtlasSampler, fragUV).r; // Sample red channel (it's grayscale)

    // Output white color modulated by the sampled alpha
    // TODO: Add support for vertex color or uniform color later
    outColor = vec4(1.0, 1.0, 1.0, alpha);

    // Discard fully transparent pixels to potentially improve performance (optional)
    if (alpha < 0.01) {
        discard;
    }
}