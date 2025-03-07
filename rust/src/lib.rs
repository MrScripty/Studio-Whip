pub mod application;
pub mod vulkan_core;
pub mod renderer;
pub mod window_management;

#[repr(C)]
pub struct Vertex {
    pub position: [f32; 2],
}