#!/bin/bash
# compile_shaders.sh
# Compiles all .vert and .frag files in the shaders folder to SPIR-V using glslc, preserving full names (e.g., background.vert.spv)

# Ensure the shaders directory exists
SHADERS_DIR="./shaders"
if [ ! -d "$SHADERS_DIR" ]; then
    echo "Error: 'shaders' directory not found in the project root."
    exit 1
fi

# Check if glslc is available
if ! command -v glslc >/dev/null 2>&1; then
    echo "Error: 'glslc' not found. Please install the Vulkan SDK and ensure glslc is in your PATH."
    exit 1
fi

# Find all .vert and .frag files in the shaders directory
SHADER_FILES=$(find "$SHADERS_DIR" -maxdepth 1 -type f \( -name "*.vert" -o -name "*.frag" \))

if [ -z "$SHADER_FILES" ]; then
    echo "No .vert or .frag files found in the shaders directory."
    exit 0
fi

# Compile each shader file to .spv, preserving the full name
for file in $SHADER_FILES; do
    input_file="$file"
    output_file="${SHADERS_DIR}/$(basename "$file").spv" # e.g., background.vert.spv
    echo "Compiling $input_file to $output_file ..."
    glslc "$input_file" -o "$output_file"
    if [ $? -eq 0 ]; then
        echo "Successfully compiled $input_file"
    else
        echo "Failed to compile $input_file"
        exit 1
    fi
done

echo "All shaders compiled successfully!"