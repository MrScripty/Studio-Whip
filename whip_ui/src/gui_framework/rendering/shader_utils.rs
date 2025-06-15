use ash::vk;
use std::fs;
use std::path::PathBuf;
use std::marker::PhantomData;

pub fn load_shader(device: &ash::Device, filename: &str) -> vk::ShaderModule {
    // Look for shaders in the target directory where they're compiled by build.rs
    let mut shader_path = std::env::current_exe()
        .expect("Failed to get current executable path")
        .parent()
        .expect("Failed to get executable directory")
        .to_path_buf();
    shader_path.push("shaders");
    shader_path.push(filename);
    let shader_code = fs::read(&shader_path).expect(&format!("Failed to read shader file: {:?}", shader_path));
    let shader_module_info = vk::ShaderModuleCreateInfo {
        s_type: vk::StructureType::SHADER_MODULE_CREATE_INFO,
        p_next: std::ptr::null(),
        flags: vk::ShaderModuleCreateFlags::empty(),
        code_size: shader_code.len(),
        p_code: shader_code.as_ptr() as *const u32,
        _marker: PhantomData,
    };
    unsafe { device.create_shader_module(&shader_module_info, None) }
        .expect(&format!("Failed to create shader module from: {}", filename))
}