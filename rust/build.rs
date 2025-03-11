use std::path::Path;
use std::fs;
use std::process::Command;

fn main() {
    compile_shaders();
    //create_shaders_symlink();
}

fn compile_shaders() {
    let shaders_dir = Path::new("shaders");
    if !shaders_dir.exists() {
        println!("cargo:warning=Shaders directory not found, skipping compilation.");
        return;
    }

    // Check if glslc is available
    if Command::new("glslc").output().is_err() {
        panic!("Error: 'glslc' not found. Please install the Vulkan SDK and ensure glslc is in your PATH.");
    }

    // Find .vert and .frag files
    let shader_files: Vec<_> = fs::read_dir(shaders_dir)
        .unwrap()
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            let ext = path.extension()?.to_str()?;
            if ext == "vert" || ext == "frag" {
                Some(path)
            } else {
                None
            }
        })
        .collect();

    if shader_files.is_empty() {
        println!("cargo:warning=No .vert or .frag files found in shaders directory.");
        return;
    }

    // Compile each shader
    for input_path in shader_files {
        let input_str = input_path.to_str().unwrap();
        let output_path = input_path.with_extension(format!(
            "{}.spv",
            input_path.extension().unwrap().to_str().unwrap()
        ));
        let output_str = output_path.to_str().unwrap();

        println!("cargo:rerun-if-changed={}", input_str); // Rebuild if shader changes
        println!("Compiling {} to {}...", input_str, output_str);

        let status = Command::new("glslc")
            .arg(input_str)
            .arg("-o")
            .arg(output_str)
            .status()
            .expect("Failed to execute glslc");

        if status.success() {
            println!("Successfully compiled {}", input_str);
        } else {
            panic!("Failed to compile {}", input_str);
        }
    }
}

/* 
fn create_shaders_symlink() {
    let shaders_src = Path::new("../shaders"); // Relative to target/debug/, points to rust/shaders/
    let shaders_dest = Path::new("target/debug/shaders");

    // Remove existing symlink or directory if it exists
    if shaders_dest.exists() {
        fs::remove_dir_all(shaders_dest).unwrap_or_else(|e| println!("Failed to remove old shaders link: {}", e));
    }

    // Create symlink based on platform
    #[cfg(target_os = "linux")]
    {
        std::os::unix::fs::symlink(shaders_src, shaders_dest)
            .unwrap_or_else(|e| panic!("Failed to create symlink to shaders on Linux: {}", e));
    }

    #[cfg(target_os = "windows")]
    {
        std::os::windows::fs::symlink_dir(shaders_src, shaders_dest)
            .unwrap_or_else(|e| panic!("Failed to create symlink to shaders on Windows: {}", e));
    }

    println!("cargo:rerun-if-changed=shaders"); // Rebuild if shaders/ changes
}
*/