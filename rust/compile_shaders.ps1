# compile_shaders.ps1 (debug v2)
$shadersDir = ".\shaders"
if (-not (Test-Path $shadersDir)) {
    Write-Host "Error: 'shaders' directory not found in the project root."
    exit 1
}
try {
    $null = Get-Command glslc -ErrorAction Stop
} catch {
    Write-Host "Error: 'glslc' not found. Please install the Vulkan SDK and add glslc to your PATH."
    exit 1
}
$fullShadersDir = (Get-Item $shadersDir).FullName
Write-Host "Searching in: $fullShadersDir"
# Match .vert and .frag files
$shaderFiles = Get-ChildItem -Path $shadersDir -File | Where-Object { $_.Extension -eq ".vert" -or $_.Extension -eq ".frag" }
Write-Host "Found $($shaderFiles.Count) shader files"
if ($shaderFiles.Count -eq 0) {
    Write-Host "No .vert or .frag files found in the shaders directory."
    exit 0
}

foreach ($file in $shaderFiles) {
    $inputFile = $file.FullName
    $outputFile = Join-Path $shadersDir ($file.Name + ".spv")
    & glslc $inputFile -o $outputFile
    if ($LASTEXITCODE -eq 0) {
        Write-Host "Successfully compiled $inputFile"
    } else {
        Write-Host "Failed to compile $inputFile"
        exit 1
    }
}
Write-Host "All shaders compiled successfully!"