# compile_shaders.ps1
# Compiles all .vert and .frag files in the shaders folder to SPIR-V using glslc, preserving full names (e.g., background.vert.spv)

# Ensure the shaders directory exists
$shadersDir = ".\shaders"
if (-not (Test-Path $shadersDir)) {
    Write-Host "Error: 'shaders' directory not found in the project root."
    exit 1
}

# Check if glslc is available in PATH
try {
    $null = Get-Command glslc -ErrorAction Stop
} catch {
    Write-Host "Error: 'glslc' not found. Please install the Vulkan SDK and add glslc to your PATH."
    exit 1
}

# Get all .vert and .frag files in the shaders directory
$shaderFiles = Get-ChildItem -Path $shadersDir -Filter "*.vert", "*.frag" -Include "*.vert", "*.frag"

if ($shaderFiles.Count -eq 0) {
    Write-Host "No .vert or .frag files found in the shaders directory."
    exit 0
}

# Compile each shader file to .spv, preserving the full name
foreach ($file in $shaderFiles) {
    $inputFile = $file.FullName
    $outputFile = Join-Path $shadersDir ($file.Name + ".spv") # e.g., background.vert.spv
    Write-Host "Compiling $inputFile to $outputFile ..."
    & glslc $inputFile -o $outputFile
    if ($LASTEXITCODE -eq 0) {
        Write-Host "Successfully compiled $inputFile"
    } else {
        Write-Host "Failed to compile $inputFile"
        exit 1
    }
}

Write-Host "All shaders compiled successfully!"