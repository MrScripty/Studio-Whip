use bevy_ecs::prelude::Entity;
use std::sync::Arc;
use ash::vk;
use bevy_reflect::Reflect;

pub mod gui_framework;

// Keep Vertex definition accessible
#[repr(C)]
#[derive(Debug, Clone, Copy, Reflect)]
pub struct Vertex {
    pub position: [f32; 2],
}

// Vertex format specifically for text rendering
#[repr(C)]
#[derive(Debug, Clone, Copy)] // No need for Reflect for now
pub struct TextVertex {
    pub position: [f32; 2],
    pub uv: [f32; 2],
}

// --- Resources needed across framework/app ---
// Resource holding the Arc<Mutex<VulkanContext>>
#[derive(bevy_ecs::prelude::Resource, Clone)]
pub struct VulkanContextResource(pub std::sync::Arc<std::sync::Mutex<gui_framework::VulkanContext>>);

#[derive(bevy_ecs::prelude::Resource, Clone)]
pub struct RendererResource(pub std::sync::Arc<std::sync::Mutex<gui_framework::rendering::render_engine::Renderer>>);

#[derive(bevy_ecs::prelude::Resource, Debug, Clone, Default, bevy_reflect::Reflect)]
pub struct HotkeyResource(pub gui_framework::interaction::hotkeys::HotkeyConfig);

#[derive(bevy_ecs::prelude::Resource, Clone)]
pub struct GlyphAtlasResource(pub std::sync::Arc<std::sync::Mutex<gui_framework::rendering::glyph_atlas::GlyphAtlas>>);

#[derive(bevy_ecs::prelude::Resource, Clone)]
  pub struct FontServerResource(pub std::sync::Arc<std::sync::Mutex<gui_framework::rendering::font_server::FontServer>>);

#[derive(bevy_ecs::prelude::Resource)]
pub struct SwashCacheResource(pub std::sync::Mutex<cosmic_text::SwashCache>);

/// Holds the prepared Vulkan handles needed for a single draw call.
#[derive(Debug, Clone)]
pub struct PreparedDrawData {
    pub pipeline: vk::Pipeline,
    pub vertex_buffer: vk::Buffer,
    pub vertex_count: u32,
    pub descriptor_set: vk::DescriptorSet, // Per-entity set (bindings 0=global proj, 1=entity offset)
    // Add instance buffer/count later if needed
}

#[derive(Debug, Clone)]
pub struct RenderCommandData {
    pub entity_id: Entity,
    pub transform_matrix: bevy_math::Mat4, // Pre-calculated world matrix
    pub vertices: Arc<Vec<Vertex>>,
    pub vertex_shader_path: String,
    pub fragment_shader_path: String,
    pub depth: f32, // For sorting
    pub vertices_changed: bool, // For background quad resizing
    // Add instancing info later if needed
}

/// Holds the prepared Vulkan handles and data needed for drawing text.
/// This might represent a batch of text from one or more entities.
#[derive(Debug, Clone)]
pub struct TextRenderCommandData {
    pub vertex_buffer_offset: u32, // Starting index in the Renderer's dynamic text vertex buffer
    pub vertex_count: u32,         // Number of vertices for this batch
    // Pipeline and descriptor set are likely bound once per text rendering phase
}


// Specific exports might be needed later, but often importing
// directly like `use rusty_whip::gui_framework::components::Visibility`
// in main.rs is clearer.