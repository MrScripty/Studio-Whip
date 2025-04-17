pub mod gui_framework;
// Remove the broad re-export:
// pub use gui_framework::*;

// Keep Vertex definition accessible
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Vertex {
    pub position: [f32; 2],
}

// Specific exports might be needed later, but often importing
// directly like `use rusty_whip::gui_framework::components::Visibility`
// in main.rs is clearer.