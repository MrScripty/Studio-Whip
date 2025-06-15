// Build script for whip_ui_example
// Copies shaders and user files needed by the example

use std::path::PathBuf;
use std::fs;
use std::process::{Command, Stdio};
use std::env;
use walkdir::WalkDir;

fn main() {
    println!("cargo:warning=Running whip_ui_example build script...");
    compile_and_copy_shaders();
    copy_user_files();
    println!("cargo:warning=whip_ui_example build script finished.");
}

fn copy_user_files() {
    println!("cargo:warning=Running copy_user_files function...");

    // Source is in the whip_ui library directory
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let whip_ui_dir = manifest_dir.parent().unwrap().join("whip_ui");
    let source_user_dir = whip_ui_dir.join("user");
    let source_hotkeys_file = source_user_dir.join("hotkeys.toml");

    println!("cargo:warning=Source hotkeys path: {}", source_hotkeys_file.display());

    if !source_hotkeys_file.exists() {
        println!("cargo:warning=Source hotkeys file not found at {}, skipping copy.", source_hotkeys_file.display());
        return;
    }

    // Destination is in the example's target directory
    let target_dir = match env::var("CARGO_TARGET_DIR") {
        Ok(target) => PathBuf::from(target),
        Err(_) => manifest_dir.parent().unwrap().join("target"),
    };
    let profile = env::var("PROFILE").unwrap();
    let exe_dir = target_dir.join(&profile);
    let dest_user_dir = exe_dir.join("user");
    let dest_hotkeys_file = dest_user_dir.join("hotkeys.toml");

    println!("cargo:warning=Destination hotkeys path: {}", dest_hotkeys_file.display());

    println!("cargo:rerun-if-changed={}", source_hotkeys_file.display());

    if let Err(e) = fs::create_dir_all(&dest_user_dir) {
        println!("cargo:warning=Error creating destination user directory {}: {}", dest_user_dir.display(), e);
        return;
    }

    match fs::copy(&source_hotkeys_file, &dest_hotkeys_file) {
        Ok(bytes) => println!("cargo:warning=SUCCESS: Copied {} bytes from {} to {}", bytes, source_hotkeys_file.display(), dest_hotkeys_file.display()),
        Err(e) => println!("cargo:warning=FAILURE: Error copying hotkeys file: {}", e),
    }
}

fn compile_and_copy_shaders() {
    println!("cargo:warning=Running compile_and_copy_shaders function...");

    // Source shaders are in the whip_ui library directory
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let whip_ui_dir = manifest_dir.parent().unwrap().join("whip_ui");
    let source_shaders_dir = whip_ui_dir.join("shaders");

    // Destination is in the example's target directory  
    let target_dir = match env::var("CARGO_TARGET_DIR") {
        Ok(target) => PathBuf::from(target),
        Err(_) => manifest_dir.parent().unwrap().join("target"),
    };
    let profile = env::var("PROFILE").unwrap();
    let exe_dir = target_dir.join(&profile);
    let dest_shaders_dir = exe_dir.join("shaders");

    println!("cargo:warning=Source shaders directory: {}", source_shaders_dir.display());
    println!("cargo:warning=Destination shaders directory: {}", dest_shaders_dir.display());

    if let Err(e) = fs::create_dir_all(&dest_shaders_dir) {
        println!("cargo:warning=Error creating destination shaders directory {}: {}", dest_shaders_dir.display(), e);
        return;
    }

    // Find glslc
    let vulkan_sdk_path = env::var("VULKAN_SDK").ok();
    let glslc_path = vulkan_sdk_path
        .map(|sdk| PathBuf::from(&sdk).join("Bin").join("glslc.exe"))
        .filter(|p| p.exists())
        .unwrap_or_else(|| PathBuf::from("glslc"));

    println!("cargo:warning=Using glslc path: {}", glslc_path.display());

    println!("cargo:rerun-if-changed={}", source_shaders_dir.display());

    let mut shaders_compiled = 0;
    let mut shaders_failed = 0;

    for entry in WalkDir::new(&source_shaders_dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                if ext == "vert" || ext == "frag" {
                    println!("cargo:rerun-if-changed={}", path.display());

                    let source_path_str = path.to_str().unwrap();
                    let output_filename = format!("{}.spv", path.file_name().unwrap().to_str().unwrap());
                    let dest_path = dest_shaders_dir.join(&output_filename);

                    println!("cargo:warning=Compiling: {} -> {}", path.display(), dest_path.display());

                    let output = Command::new(&glslc_path)
                        .arg(source_path_str)
                        .arg("-o")
                        .arg(&dest_path)
                        .stdout(Stdio::piped())
                        .stderr(Stdio::piped())
                        .output();

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
                            }
                        }
                        Err(e) => {
                            shaders_failed += 1;
                            println!("cargo:warning=FAILURE: Failed to execute glslc for {}: {}", path.display(), e);
                        }
                    }
                }
            }
        }
    }

    println!("cargo:warning=Finished compile_and_copy_shaders. Compiled: {}, Failed: {}", shaders_compiled, shaders_failed);
    if shaders_failed > 0 {
        println!("cargo:warning=WARNING: {} shader(s) failed to compile!", shaders_failed);
    }
}