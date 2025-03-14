pub mod gui_framework;
pub use gui_framework::*; // Re-exports all public items from gui_framework (Renderer, VulkanContext, etc.)

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Vertex {
    pub position: [f32; 2],
}