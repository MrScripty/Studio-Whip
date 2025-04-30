use bevy_ecs::prelude::Component;
use bevy_ecs::prelude::ReflectComponent;
use bevy_reflect::prelude::*;
use bevy_math::Vec2;
use bevy_color::Color; // Using bevy_color for simplicity

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
#[reflect(Component)] // Add this for reflection registration
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