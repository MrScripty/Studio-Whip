use crate::gui_framework::rendering::glyph_atlas::GlyphInfo;
use bevy_ecs::prelude::Component;
use bevy_math::Vec2;
use bevy_reflect::{Reflect, FromReflect};
use cosmic_text::LayoutGlyph;
use ash::vk;

// Represents a single glyph quad ready for rendering
#[derive(Debug, Clone)]
pub struct PositionedGlyph {
    // Info from the atlas (UVs, etc.) - might duplicate some info from LayoutGlyph
    pub glyph_info: GlyphInfo,
    // Position/size info from cosmic-text layout
    pub layout_glyph: LayoutGlyph,
    // Calculated vertex positions in world space (or relative to entity?)
    // For now, let's store relative positions and combine with Transform in rendering system
    pub vertices: [Vec2; 4], // Quad vertices (e.g., top-left, top-right, bottom-right, bottom-left)
}

// Component added to entities with Text, holding the results of layout
#[derive(Component, Debug, Clone, Default)]
pub struct TextLayoutOutput {
    pub glyphs: Vec<PositionedGlyph>,
    // Store overall bounds if needed
    // pub bounds: Rect,
}

/// Component storing per-entity Vulkan resource handles and metadata for text rendering.
/// Managed by `prepare_text_rendering_system` and used by `rendering_system`.
#[derive(Component)] // Add Clone to allow cloning in prepare_text_rendering_system
pub struct TextRenderData {
    // pub vertex_offset: u32, // REMOVED
    /// Number of vertices for this entity.
    pub vertex_count: u32,
    /// Handle to the per-entity vertex buffer.
    pub vertex_buffer: vk::Buffer, // ADDED
    /// Allocation info for the vertex buffer.
    pub vertex_alloc: vk_mem::Allocation, // ADDED
    /// Handle to the per-entity uniform buffer holding the transform matrix.
    pub transform_ubo: vk::Buffer,
    /// Allocation info for the transform UBO.
    pub transform_alloc: vk_mem::Allocation,
    /// Handle to the per-entity descriptor set (Set 0: Global Projection, Entity Transform).
    pub descriptor_set_0: vk::DescriptorSet,
}