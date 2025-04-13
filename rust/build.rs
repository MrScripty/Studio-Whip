// In build.rs

use std::path::{Path, PathBuf};
use std::fs;
use std::process::Command;
use std::env;

fn main() {
    println!("cargo:warning=Running build script...");
    compile_shaders();
    copy_user_files();
    println!("cargo:warning=Build script finished.");
}

fn copy_user_files() {
    println!("cargo:warning=Running copy_user_files function...");

    // --- Source Path ---
    // Source is relative to CARGO_MANIFEST_DIR (which is rust/)
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    // Path is simply rust/user/hotkeys.toml
    let source_user_dir = manifest_dir.join("user"); // <-- Corrected: user is inside rust/
    let source_hotkeys_file = source_user_dir.join("hotkeys.toml");

    // Use canonicalize for absolute path logging
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

    // --- Determine Destination Directory (Logic remains the same) ---
    let target_dir = match env::var("CARGO_TARGET_DIR") {
        Ok(target) => PathBuf::from(target),
        Err(_) => PathBuf::from(&manifest_dir).join("target"), // rust/target
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
    // --- End Determine Destination Directory ---

    // --- Prepare Destination Paths ---
    // We want user/hotkeys.toml inside the exe_dir (e.g., target/debug/user/hotkeys.toml)
    let dest_user_dir = exe_dir.join("user");
    let dest_hotkeys_file = dest_user_dir.join("hotkeys.toml");

    println!("cargo:rerun-if-changed={}", source_hotkeys_file.display());
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


fn compile_shaders() {
    println!("cargo:warning=Running compile_shaders function...");
    let shaders_dir = Path::new("shaders"); // Relative to rust/
    // ... rest of shader code ...
    println!("cargo:warning=Finished compile_shaders function.");
}