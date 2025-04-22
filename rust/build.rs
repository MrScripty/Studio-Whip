// In build.rs

use std::path::{Path, PathBuf};
use std::fs;
use std::process::{Command, Stdio}; // Added Stdio
use std::env;
use walkdir::WalkDir; // Added walkdir

fn main() {
    println!("cargo:warning=Running build script...");
    compile_and_copy_shaders();
    copy_user_files();
    println!("cargo:warning=Build script finished.");
}

fn copy_user_files() {
    println!("cargo:warning=Running copy_user_files function...");

    // --- Source Path ---
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let source_user_dir = manifest_dir.join("user");
    let source_hotkeys_file = source_user_dir.join("hotkeys.toml");

    let abs_source_path = match fs::canonicalize(&source_hotkeys_file) {
         Ok(p) => p.display().to_string(),
         Err(e) => format!("{} (canonicalize failed: {})", source_hotkeys_file.display(), e),
    };
    println!("cargo:warning=Source hotkeys path (calculated): {}", source_hotkeys_file.display());
    println!("cargo:warning=Source hotkeys path (absolute attempt): {}", abs_source_path);

    if !source_hotkeys_file.exists() {
        println!("cargo:warning=Source hotkeys file check FAILED at {}, skipping copy.", source_hotkeys_file.display());
        return;
    } else {
        println!("cargo:warning=Source hotkeys file check SUCCEEDED.");
    }

    // --- Determine Destination Directory ---
    let target_dir = match env::var("CARGO_TARGET_DIR") {
        Ok(target) => PathBuf::from(target),
        Err(_) => manifest_dir.join("target"), // rust/target
    };
    let profile = env::var("PROFILE").unwrap(); // "debug" or "release"
    let exe_dir = target_dir.join(&profile); // rust/target/debug or rust/target/release

    println!("cargo:warning=Env CARGO_MANIFEST_DIR: {}", manifest_dir.display());
    println!("cargo:warning=Env PROFILE: {}", profile);
    println!("cargo:warning=Env CARGO_TARGET_DIR: {:?}", env::var("CARGO_TARGET_DIR").ok());
    println!("cargo:warning=Base target directory determined as: {}", target_dir.display());
    println!("cargo:warning=Calculated executable directory: {}", exe_dir.display());

    if !exe_dir.exists() {
         println!("cargo:warning=Executable directory {} does NOT exist. Will attempt creation of subdirs.", exe_dir.display());
    } else {
         println!("cargo:warning=Executable directory {} exists.", exe_dir.display());
    }

    // --- Prepare Destination Paths ---
    let dest_user_dir = exe_dir.join("user");
    let dest_hotkeys_file = dest_user_dir.join("hotkeys.toml");

    println!("cargo:rerun-if-changed={}", source_hotkeys_file.display()); // Track source file
    println!("cargo:warning=Destination user directory path: {}", dest_user_dir.display());
    println!("cargo:warning=Destination hotkeys file path: {}", dest_hotkeys_file.display());

    // --- Create Destination Directory ---
    println!("cargo:warning=Attempting to create destination directory (if needed): {}", dest_user_dir.display());
    if let Err(e) = fs::create_dir_all(&dest_user_dir) {
        println!("cargo:warning=Error creating destination user directory {}: {}. Cannot copy file.", dest_user_dir.display(), e);
        return;
    } else {
        println!("cargo:warning=Ensured destination directory exists.");
    }

    // --- Copy the File ---
    println!("cargo:warning=Attempting to copy file...");
    match fs::copy(&source_hotkeys_file, &dest_hotkeys_file) {
        Ok(bytes) => println!("cargo:warning=SUCCESS: Copied {} bytes from {} to {}", bytes, source_hotkeys_file.display(), dest_hotkeys_file.display()),
        Err(e) => println!("cargo:warning=FAILURE: Error copying hotkeys file: {}", e),
    }

    println!("cargo:warning=Finished copy_user_files function.");
}


fn compile_and_copy_shaders() {
    println!("cargo:warning=Running compile_and_copy_shaders function...");

    // --- Source & Destination Paths ---
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let source_shaders_dir = manifest_dir.join("shaders"); // rust/shaders/

    let target_dir = match env::var("CARGO_TARGET_DIR") {
        Ok(target) => PathBuf::from(target),
        Err(_) => manifest_dir.join("target"), // rust/target
    };
    let profile = env::var("PROFILE").unwrap(); // "debug" or "release"
    let exe_dir = target_dir.join(&profile); // rust/target/debug or rust/target/release
    let dest_shaders_dir = exe_dir.join("shaders"); // e.g., rust/target/debug/shaders/

    println!("cargo:warning=Source shaders directory: {}", source_shaders_dir.display());
    println!("cargo:warning=Destination shaders directory: {}", dest_shaders_dir.display());

    // --- Create Destination Directory ---
    if let Err(e) = fs::create_dir_all(&dest_shaders_dir) {
        println!("cargo:warning=Error creating destination shaders directory {}: {}. Cannot compile/copy shaders.", dest_shaders_dir.display(), e);
        return;
    } else {
        println!("cargo:warning=Ensured destination shaders directory exists.");
    }

    // --- Find glslc ---
    // Check common locations or rely on PATH. Add Vulkan SDK path if needed.
    // Example: Check VULKAN_SDK environment variable
    let vulkan_sdk_path = env::var("VULKAN_SDK").ok();
    let glslc_path = vulkan_sdk_path
        .map(|sdk| PathBuf::from(&sdk).join("Bin").join("glslc.exe")) // Adjust for Linux/macOS if needed
        .filter(|p| p.exists())
        .unwrap_or_else(|| PathBuf::from("glslc")); // Fallback to assuming it's in PATH

    println!("cargo:warning=Using glslc path: {}", glslc_path.display());
    if !glslc_path.exists() && glslc_path == PathBuf::from("glslc") {
         println!("cargo:warning=glslc not found in PATH or VULKAN_SDK/Bin. Shader compilation might fail.");
         // Consider panicking here if compilation is mandatory:
         // panic!("glslc compiler not found. Please ensure it's in your PATH or set the VULKAN_SDK environment variable.");
    }


    // --- Iterate and Compile ---
    println!("cargo:warning=Scanning for shaders in {}...", source_shaders_dir.display());
    let mut shaders_compiled = 0;
    let mut shaders_failed = 0;

    // Ensure Cargo reruns this script if the source shaders directory or its contents change
    println!("cargo:rerun-if-changed={}", source_shaders_dir.display());

    for entry in WalkDir::new(&source_shaders_dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                // Compile .vert and .frag files
                if ext == "vert" || ext == "frag" {
                    // Tell Cargo to rerun if this specific source file changes
                    println!("cargo:rerun-if-changed={}", path.display());

                    let source_path_str = path.to_str().unwrap();
                    let output_filename = format!("{}.spv", path.file_name().unwrap().to_str().unwrap());
                    let dest_path = dest_shaders_dir.join(&output_filename);

                    println!("cargo:warning=Compiling: {} -> {}", path.display(), dest_path.display());

                    // Execute glslc
                    let output = Command::new(&glslc_path)
                        .arg(source_path_str)
                        .arg("-o")
                        .arg(&dest_path)
                        .stdout(Stdio::piped()) // Capture stdout
                        .stderr(Stdio::piped()) // Capture stderr
                        .output(); // Execute and wait

                    match output {
                        Ok(out) => {
                            if out.status.success() {
                                println!("cargo:warning=SUCCESS: Compiled {}", output_filename);
                                shaders_compiled += 1;
                            } else {
                                shaders_failed += 1;
                                let stdout = String::from_utf8_lossy(&out.stdout);
                                let stderr = String::from_utf8_lossy(&out.stderr);
                                println!("cargo:warning=FAILURE: Compiling {} failed. Status: {}", output_filename, out.status);
                                if !stdout.is_empty() { println!("cargo:warning=glslc stdout:\n{}", stdout); }
                                if !stderr.is_empty() { println!("cargo:warning=glslc stderr:\n{}", stderr); }
                                // Optional: Panic on failure
                                // panic!("Shader compilation failed for {}", path.display());
                            }
                        }
                        Err(e) => {
                            shaders_failed += 1;
                            println!("cargo:warning=FAILURE: Failed to execute glslc for {}: {}", path.display(), e);
                            // Optional: Panic on failure
                            // panic!("Failed to execute glslc: {}", e);
                        }
                    }
                }
            }
        }
    }

    println!("cargo:warning=Finished compile_and_copy_shaders function. Compiled: {}, Failed: {}", shaders_compiled, shaders_failed);
    if shaders_failed > 0 {
         // Consider panicking if any shader fails to compile
         // panic!("{} shader(s) failed to compile. See warnings above.", shaders_failed);
         println!("cargo:warning=WARNING: {} shader(s) failed to compile!", shaders_failed);
    }
}