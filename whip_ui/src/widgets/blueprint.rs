// Remove unused ECS import since this is just data structures
use bevy_math::{Vec2, Vec3};
use bevy_color::Color;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a widget definition loaded from TOML
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetBlueprint {
    pub id: String,
    pub widget_type: WidgetType,
    pub layout: LayoutConfig,
    pub style: StyleConfig,
    pub behavior: BehaviorConfig,
    pub children: Vec<String>, // IDs of child widgets
}

/// Types of widgets supported by the framework
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WidgetType {
    Container {
        direction: FlexDirection,
    },
    Button {
        text: String,
        action: Option<String>,
    },
    Text {
        content: String,
        editable: bool,
    },
    Shape {
        shape_type: ShapeType,
    },
    Custom {
        component: String,
        properties: HashMap<String, toml::Value>,
    },
}

/// Layout configuration for widgets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutConfig {
    pub position: Option<Vec3>, // Absolute position (x, y, z)
    pub size: Option<Vec2>,     // Width, height
    pub margin: Option<Spacing>,
    pub padding: Option<Spacing>,
    pub flex_grow: Option<f32>,
    pub flex_shrink: Option<f32>,
    pub align_self: Option<AlignSelf>,
}

/// Style configuration for widgets  
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleConfig {
    pub background_color: Option<ColorDef>,
    pub border_color: Option<ColorDef>,
    pub border_width: Option<f32>,
    pub border_radius: Option<f32>,
    pub text_color: Option<ColorDef>,
    pub text_size: Option<f32>,
    pub opacity: Option<f32>,
}

/// Behavior configuration for widgets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorConfig {
    pub visible: Option<bool>,
    pub interactive: Option<bool>,
    pub draggable: Option<bool>,
    pub clickable: Option<bool>,
    pub focusable: Option<bool>,
    pub z_index: Option<i32>,
}

/// Flexible layout direction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FlexDirection {
    Row,
    Column,
    RowReverse,
    ColumnReverse,
}

/// Shape types for shape widgets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ShapeType {
    Rectangle,
    Circle,
    Triangle,
    Custom { vertices: Vec<Vec2> },
}

/// Spacing configuration (margin/padding)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Spacing {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

/// Alignment options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlignSelf {
    Auto,
    FlexStart,
    FlexEnd,
    Center,
    Baseline,
    Stretch,
}

/// Color definition that supports multiple formats
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ColorDef {
    Hex(String),        // "#FF0000"
    Rgb { r: u8, g: u8, b: u8 },
    Rgba { r: u8, g: u8, b: u8, a: f32 },
    Named(String),      // "red", "blue", etc.
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            position: None,
            size: None,
            margin: None,
            padding: None,
            flex_grow: None,
            flex_shrink: None,
            align_self: None,
        }
    }
}

impl Default for StyleConfig {
    fn default() -> Self {
        Self {
            background_color: None,
            border_color: None,
            border_width: None,
            border_radius: None,
            text_color: None,
            text_size: None,
            opacity: None,
        }
    }
}

impl Default for BehaviorConfig {
    fn default() -> Self {
        Self {
            visible: Some(true),
            interactive: Some(false),
            draggable: Some(false),
            clickable: Some(false),
            focusable: Some(false),
            z_index: Some(0),
        }
    }
}

impl ColorDef {
    /// Convert to Bevy Color
    pub fn to_color(&self) -> Color {
        match self {
            ColorDef::Hex(hex) => {
                // Parse hex string like "#FF0000" or "FF0000"
                let hex = hex.trim_start_matches('#');
                if hex.len() == 6 {
                    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0) as f32 / 255.0;
                    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0) as f32 / 255.0;
                    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0) as f32 / 255.0;
                    Color::srgb(r, g, b)
                } else {
                    Color::WHITE // fallback
                }
            }
            ColorDef::Rgb { r, g, b } => {
                Color::srgb(*r as f32 / 255.0, *g as f32 / 255.0, *b as f32 / 255.0)
            }
            ColorDef::Rgba { r, g, b, a } => {
                Color::srgba(*r as f32 / 255.0, *g as f32 / 255.0, *b as f32 / 255.0, *a)
            }
            ColorDef::Named(name) => {
                // Basic named color support
                match name.to_lowercase().as_str() {
                    "red" => Color::srgb(1.0, 0.0, 0.0),
                    "green" => Color::srgb(0.0, 1.0, 0.0),
                    "blue" => Color::srgb(0.0, 0.0, 1.0),
                    "black" => Color::BLACK,
                    "white" => Color::WHITE,
                    "gray" | "grey" => Color::srgb(0.5, 0.5, 0.5),
                    "yellow" => Color::srgb(1.0, 1.0, 0.0),
                    "cyan" => Color::srgb(0.0, 1.0, 1.0),
                    "magenta" => Color::srgb(1.0, 0.0, 1.0),
                    "orange" => Color::srgb(1.0, 0.5, 0.0),
                    _ => Color::WHITE, // fallback
                }
            }
        }
    }
}

/// A collection of widget blueprints loaded from TOML
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetCollection {
    pub widgets: HashMap<String, WidgetBlueprint>,
    pub root: Option<String>, // ID of the root widget
}

impl WidgetCollection {
    /// Load widget collection from TOML string
    pub fn from_toml(toml_str: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(toml_str)
    }

    /// Get a widget blueprint by ID
    pub fn get_widget(&self, id: &str) -> Option<&WidgetBlueprint> {
        self.widgets.get(id)
    }

    /// Get all child widgets recursively
    pub fn get_children_recursive(&self, widget_id: &str) -> Vec<&WidgetBlueprint> {
        let mut children = Vec::new();
        if let Some(widget) = self.get_widget(widget_id) {
            for child_id in &widget.children {
                if let Some(child) = self.get_widget(child_id) {
                    children.push(child);
                    children.extend(self.get_children_recursive(child_id));
                }
            }
        }
        children
    }
}