pub mod vulkan_context;
pub mod vulkan_setup;
pub mod renderer;
pub mod window_handler;
pub mod scene;

pub use vulkan_context::VulkanContext;
pub use scene::{Scene, RenderObject};
pub use renderer::Renderer;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Vertex {
    pub position: [f32; 2],
}