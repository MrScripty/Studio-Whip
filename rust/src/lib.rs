use bevy_ecs::prelude::Entity;
use std::sync::Arc;
use ash::vk;

pub mod gui_framework;
// Remove the broad re-export:
// pub use gui_framework::*;

// Keep Vertex definition accessible
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Vertex {
    pub position: [f32; 2],
}

/// Holds the prepared Vulkan handles needed for a single draw call.
#[derive(Debug, Clone)]
pub struct PreparedDrawData {
    pub pipeline: vk::Pipeline,
    pub vertex_buffer: vk::Buffer,
    pub vertex_count: u32,
    pub descriptor_set: vk::DescriptorSet, // Per-entity set (bindings 0=global proj, 1=entity offset)
    // Add instance buffer/count later if needed
}

// --- Moved Render Command Data Here ---
#[derive(Debug, Clone)]
pub struct RenderCommandData {
    pub entity_id: Entity,
    pub transform_matrix: bevy_math::Mat4, // Pre-calculated world matrix
    pub vertices: Arc<Vec<Vertex>>, // Use Arc for potential sharing
    pub vertex_shader_path: String,
    pub fragment_shader_path: String,
    pub depth: f32, // For sorting
    // Add instancing info later if needed
}


// Specific exports might be needed later, but often importing
// directly like `use rusty_whip::gui_framework::components::Visibility`
// in main.rs is clearer.