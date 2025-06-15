use bevy_ecs::prelude::*;
use bevy_math::{Vec2, Vec3};
use std::collections::HashMap;
use crate::widgets::blueprint::{WidgetBlueprint, LayoutConfig, StyleConfig, BehaviorConfig};

/// Component that marks an entity as a widget with its blueprint
#[derive(Component, Debug, Clone)]
pub struct Widget {
    pub id: String,
    pub blueprint: WidgetBlueprint,
}

/// Component for widget hierarchy relationships
#[derive(Component, Debug, Default)]
pub struct WidgetHierarchy {
    pub parent: Option<Entity>,
    pub children: Vec<Entity>,
}

/// Component for widget layout properties (derived from blueprint)
#[derive(Component, Debug, Clone)]
pub struct WidgetLayout {
    pub position: Option<Vec3>,
    pub size: Option<Vec2>,
    pub margin: Option<(f32, f32, f32, f32)>, // top, right, bottom, left
    pub padding: Option<(f32, f32, f32, f32)>,
    pub flex_grow: Option<f32>,
    pub flex_shrink: Option<f32>,
    pub computed_position: Vec3, // Final computed position
    pub computed_size: Vec2,     // Final computed size
}

/// Component for widget styling (derived from blueprint)
#[derive(Component, Debug, Clone)]
pub struct WidgetStyle {
    pub background_color: Option<bevy_color::Color>,
    pub border_color: Option<bevy_color::Color>,
    pub border_width: Option<f32>,
    pub border_radius: Option<f32>,
    pub text_color: Option<bevy_color::Color>,
    pub text_size: Option<f32>,
    pub opacity: Option<f32>,
}

/// Component for widget behavior (derived from blueprint)
#[derive(Component, Debug, Clone)]
pub struct WidgetBehavior {
    pub visible: bool,
    pub interactive: bool,
    pub draggable: bool,
    pub clickable: bool,
    pub focusable: bool,
    pub z_index: i32,
    pub is_focused: bool,
    pub is_hovered: bool,
}

/// Component for widget state and events
#[derive(Component, Debug, Default)]
pub struct WidgetState {
    pub is_dirty: bool, // Needs re-layout/re-render
    pub custom_data: HashMap<String, String>, // For custom widget properties
}

/// Component for container widgets that manage child layout
#[derive(Component, Debug)]
pub struct WidgetContainer {
    pub flex_direction: FlexDirection,
    pub computed_content_size: Vec2,
}

/// Component for button widgets
#[derive(Component, Debug)]
pub struct WidgetButton {
    pub text: String,
    pub action: Option<String>,
    pub is_pressed: bool,
}

/// Component for text widgets
#[derive(Component, Debug)]
pub struct WidgetText {
    pub content: String,
    pub editable: bool,
    pub cursor_position: usize,
    pub selection_start: Option<usize>,
    pub selection_end: Option<usize>,
}

/// Component for shape widgets
#[derive(Component, Debug)]
pub struct WidgetShape {
    pub shape_type: ShapeType,
    pub vertices: Vec<crate::Vertex>, // Computed vertices as Vertex structs
}

// Re-export from blueprint to avoid duplication
pub use crate::widgets::blueprint::{FlexDirection, ShapeType};

impl From<&LayoutConfig> for WidgetLayout {
    fn from(config: &LayoutConfig) -> Self {
        let margin = config.margin.as_ref().map(|s| (s.top, s.right, s.bottom, s.left));
        let padding = config.padding.as_ref().map(|s| (s.top, s.right, s.bottom, s.left));
        
        Self {
            position: config.position,
            size: config.size,
            margin,
            padding,
            flex_grow: config.flex_grow,
            flex_shrink: config.flex_shrink,
            computed_position: config.position.unwrap_or(Vec3::ZERO),
            computed_size: config.size.unwrap_or(Vec2::new(100.0, 100.0)),
        }
    }
}

impl From<&StyleConfig> for WidgetStyle {
    fn from(config: &StyleConfig) -> Self {
        Self {
            background_color: config.background_color.as_ref().map(|c| c.to_color()),
            border_color: config.border_color.as_ref().map(|c| c.to_color()),
            border_width: config.border_width,
            border_radius: config.border_radius,
            text_color: config.text_color.as_ref().map(|c| c.to_color()),
            text_size: config.text_size,
            opacity: config.opacity,
        }
    }
}

impl From<&BehaviorConfig> for WidgetBehavior {
    fn from(config: &BehaviorConfig) -> Self {
        Self {
            visible: config.visible.unwrap_or(true),
            interactive: config.interactive.unwrap_or(false),
            draggable: config.draggable.unwrap_or(false),
            clickable: config.clickable.unwrap_or(false),
            focusable: config.focusable.unwrap_or(false),
            z_index: config.z_index.unwrap_or(0),
            is_focused: false,
            is_hovered: false,
        }
    }
}

// Remove duplicate From implementations since we're re-exporting the same types