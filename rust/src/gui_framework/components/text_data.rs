use bevy_ecs::prelude::Component;
use bevy_ecs::prelude::ReflectComponent;
use bevy_reflect::prelude::*;
use bevy_math::Vec2;
use bevy_color::Color;

// Placeholder for FontId - will likely be replaced by something from cosmic-text/fontdb later
#[derive(Debug, Clone, PartialEq, Eq, Hash, Reflect)]
pub struct FontId(pub usize); // Simple usize for now

// Placeholder for text alignment - can be expanded later
#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect, Default)]
pub enum TextAlignment {
    #[default]
    Left,
    Center,
    Right,
}

#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct Text {
    // pub font_id: FontId, // Using default font for now
    pub size: f32,
    pub color: Color,
    pub alignment: TextAlignment,
    /// Optional bounds for text wrapping (width, height). None means no wrapping.
    pub bounds: Option<Vec2>,
    // Add line spacing, etc. later if needed
}

impl Default for Text {
    fn default() -> Self {
        Self {
            // font_id: FontId(0), // Default font ID
            size: 16.0, // Default font size
            color: Color::WHITE,
            alignment: TextAlignment::Left,
            bounds: None,
        }
    }
}

/// Marker component indicating that a Text entity can be edited.
#[derive(Component, Debug, Clone, Copy, Default, Reflect)]
#[reflect(Component)]
pub struct EditableText;

/// Marker component indicating that a Text entity currently has input focus.
#[derive(Component, Debug, Clone, Copy, Default, Reflect)]
#[reflect(Component)]
pub struct Focus;

/// Component storing the logical cursor position (byte offset) within the text.
/// Added to the entity that has `Focus`.
#[derive(Component, Debug, Clone, Copy, Default, Reflect)]
#[reflect(Component)]
pub struct CursorState {
    pub position: usize,
    pub line: usize,
    /// The desired horizontal pixel position. Used to maintain the column when moving up/down.
    /// `None` means it needs to be calculated on the next horizontal move.
    pub x_goal: Option<i32>,
    // Add selection range later if needed
}

/// Marker component for the entity that visually represents the cursor.
/// This entity is typically a child of the focused text entity.
#[derive(Component, Debug, Clone, Copy, Default, Reflect)]
#[reflect(Component)]
pub struct CursorVisual;

/// Component storing the text selection range (byte offsets).
/// Added to the entity that has `Focus`. `start == end` means no range is selected.
#[derive(Component, Debug, Clone, Copy, Default, Reflect)]
#[reflect(Component)]
pub struct TextSelection {
    pub start: usize,
    pub end: usize,
}