use crate::gui_framework::rendering::glyph_atlas::GlyphInfo;
use bevy_ecs::prelude::Component;
use bevy_math::Vec2;
use bevy_reflect::{Reflect, FromReflect};
use cosmic_text::LayoutGlyph;

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