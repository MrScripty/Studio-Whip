pub mod platform;
pub mod vulkan_core;
pub mod renderer;
pub mod window_management;
pub mod scene;

pub use platform::Platform;
pub use scene::{Scene, RenderObject};
pub use renderer::Renderer;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Vertex {
    pub position: [f32; 2],
}