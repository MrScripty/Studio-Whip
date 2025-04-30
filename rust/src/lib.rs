use bevy_ecs::prelude::{Entity, Resource};
use std::sync::{Arc, Mutex};
use ash::vk;
use bevy_reflect::Reflect;
use std::collections::HashMap;
use yrs::TextRef;

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

#[derive(bevy_ecs::prelude::Resource)]
pub struct YrsDocResource {
    pub doc: Arc<yrs::Doc>,
    pub text_map: Arc<Mutex<HashMap<Entity, TextRef>>>,
}

/// Holds the prepared Vulkan handles and metadata needed for drawing a batch of text.
#[derive(Debug, Clone)]
pub struct PreparedTextDrawData {
    pub pipeline: vk::Pipeline,
    pub vertex_buffer: vk::Buffer,         // Handle to the *shared* text vertex buffer
    pub vertex_count: u32,             // Number of vertices for this batch
    pub projection_descriptor_set: vk::DescriptorSet, // Set 0: Global Projection UBO
    pub atlas_descriptor_set: vk::DescriptorSet,    // Set 1: Glyph Atlas Sampler
}

// --- Resources needed across framework/app ---

// Resource holding the global projection uniform buffer, its allocation, and descriptor set.
// Managed by BufferManager, used by shapes and text rendering prep.
#[derive(bevy_ecs::prelude::Resource)]
pub struct GlobalProjectionUboResource {
    pub buffer: vk::Buffer,
    pub allocation: vk_mem::Allocation,
    pub descriptor_set: vk::DescriptorSet,
}

// Resource holding Vulkan resources specifically for text rendering.
// Managed by a dedicated system in core plugin.
#[derive(bevy_ecs::prelude::Resource)]
pub struct TextRenderingResources {
    pub vertex_buffer: vk::Buffer,
    pub vertex_allocation: vk_mem::Allocation,
    pub vertex_buffer_capacity: u32, // Max number of TextVertex this buffer can hold
    pub pipeline: vk::Pipeline,
    pub atlas_descriptor_set: vk::DescriptorSet, // Single set pointing to the atlas texture/sampler
    // Add descriptor pool/layout if managed here? Or use global ones? Let's use global for now.
}

// Resource to hold the prepared text draw commands for the current frame
#[derive(bevy_ecs::prelude::Resource, Default, Debug)]
pub struct PreparedTextDrawsResource(pub Vec<PreparedTextDrawData>);


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

// Specific exports might be needed later, but often importing
// directly like `use rusty_whip::gui_framework::components::Visibility`
// in main.rs is clearer.