use ash::vk;
use std::fs;
use std::marker::PhantomData;

pub fn load_shader(device: &ash::Device, filename: &str) -> vk::ShaderModule {
    let shader_path = format!("./shaders/{}", filename);
    let shader_code = fs::read(&shader_path).expect(&format!("Failed to read shader file: {}", shader_path));
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