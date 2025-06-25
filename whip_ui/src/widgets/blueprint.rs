// Remove unused ECS import since this is just data structures
use bevy_math::{Vec2, Vec3};
use bevy_color::Color;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::layout::PositionControl;

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
/// Includes both primitives and templates
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WidgetType {
    // Primitive widgets
    Container {
        direction: FlexDirection,
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
    // Template widgets
    Button {
        /// Override default text content
        #[serde(skip_serializing_if = "Option::is_none")]
        text: Option<String>,
        /// Override default background color
        #[serde(skip_serializing_if = "Option::is_none")]
        background_color: Option<ColorDef>,
        /// Override default text color
        #[serde(skip_serializing_if = "Option::is_none")]
        text_color: Option<ColorDef>,
        /// Override default size
        #[serde(skip_serializing_if = "Option::is_none")]
        size: Option<Vec2>,
        /// Override default text size
        #[serde(skip_serializing_if = "Option::is_none")]
        text_size: Option<f32>,
        /// Override border properties
        #[serde(skip_serializing_if = "Option::is_none")]
        border_width: Option<f32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        border_color: Option<ColorDef>,
        #[serde(skip_serializing_if = "Option::is_none")]
        border_radius: Option<f32>,
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
    /// Grid row placement (1-based, like CSS Grid)
    pub grid_row: Option<u16>,
    /// Grid column placement (1-based, like CSS Grid)
    pub grid_column: Option<u16>,
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
    /// State-specific style overrides
    #[serde(skip_serializing_if = "Option::is_none")]
    pub states: Option<StateStyles>,
}

/// State-based style variants
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateStyles {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hover: Option<StyleOverrides>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pressed: Option<StyleOverrides>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub focused: Option<StyleOverrides>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled: Option<StyleOverrides>,
}

/// Style overrides for specific states
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleOverrides {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background_color: Option<ColorDef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border_color: Option<ColorDef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border_width: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border_radius: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_color: Option<ColorDef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_size: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
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
    pub position_control: Option<PositionControl>,
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
#[derive(Debug, Clone, PartialEq)]
pub enum ColorDef {
    Hex(String),        // "#FF0000"
    Rgb { r: u8, g: u8, b: u8 },
    Rgba { r: u8, g: u8, b: u8, a: f32 },
    Named(String),      // "red", "blue", etc.
}

impl<'de> serde::Deserialize<'de> for ColorDef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, Visitor};
        
        struct ColorDefVisitor;
        
        impl<'de> Visitor<'de> for ColorDefVisitor {
            type Value = ColorDef;
            
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a color definition (hex string, named color, or RGB/RGBA object)")
            }
            
            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                // Check if it's a hex color (starts with #)
                if value.starts_with('#') {
                    Ok(ColorDef::Hex(value.to_string()))
                } else {
                    // Otherwise treat as named color
                    Ok(ColorDef::Named(value.to_string()))
                }
            }
            
            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                let mut r = None;
                let mut g = None;
                let mut b = None;
                let mut a = None;
                
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "r" => r = Some(map.next_value()?),
                        "g" => g = Some(map.next_value()?),
                        "b" => b = Some(map.next_value()?),
                        "a" => a = Some(map.next_value()?),
                        _ => {
                            map.next_value::<serde::de::IgnoredAny>()?;
                        }
                    }
                }
                
                let r = r.ok_or_else(|| de::Error::missing_field("r"))?;
                let g = g.ok_or_else(|| de::Error::missing_field("g"))?;
                let b = b.ok_or_else(|| de::Error::missing_field("b"))?;
                
                if let Some(a) = a {
                    Ok(ColorDef::Rgba { r, g, b, a })
                } else {
                    Ok(ColorDef::Rgb { r, g, b })
                }
            }
        }
        
        deserializer.deserialize_any(ColorDefVisitor)
    }
}

impl serde::Serialize for ColorDef {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            ColorDef::Hex(s) => serializer.serialize_str(s),
            ColorDef::Named(s) => serializer.serialize_str(s),
            ColorDef::Rgb { r, g, b } => {
                use serde::ser::SerializeStruct;
                let mut state = serializer.serialize_struct("Rgb", 3)?;
                state.serialize_field("r", r)?;
                state.serialize_field("g", g)?;
                state.serialize_field("b", b)?;
                state.end()
            }
            ColorDef::Rgba { r, g, b, a } => {
                use serde::ser::SerializeStruct;
                let mut state = serializer.serialize_struct("Rgba", 4)?;
                state.serialize_field("r", r)?;
                state.serialize_field("g", g)?;
                state.serialize_field("b", b)?;
                state.serialize_field("a", a)?;
                state.end()
            }
        }
    }
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
            grid_row: None,
            grid_column: None,
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
            states: None,
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
            position_control: Some(PositionControl::Layout), // Default to layout control
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

impl StyleOverrides {
    /// Apply these overrides to a base StyleConfig, returning a new style
    pub fn apply_to(&self, base: &StyleConfig) -> StyleConfig {
        StyleConfig {
            background_color: self.background_color.clone().or_else(|| base.background_color.clone()),
            border_color: self.border_color.clone().or_else(|| base.border_color.clone()),
            border_width: self.border_width.or(base.border_width),
            border_radius: self.border_radius.or(base.border_radius),
            text_color: self.text_color.clone().or_else(|| base.text_color.clone()),
            text_size: self.text_size.or(base.text_size),
            opacity: self.opacity.or(base.opacity),
            states: base.states.clone(), // Keep original state definitions
        }
    }
}

impl StateStyles {
    /// Get the appropriate style override for the given interaction state
    pub fn get_for_state(&self, hovered: bool, pressed: bool, focused: bool, disabled: bool) -> Option<&StyleOverrides> {
        // Priority order: disabled > pressed > focused > hover
        if disabled {
            self.disabled.as_ref()
        } else if pressed {
            self.pressed.as_ref()
        } else if focused {
            self.focused.as_ref()
        } else if hovered {
            self.hover.as_ref()
        } else {
            None
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