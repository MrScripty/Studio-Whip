pub mod systems;
pub mod plugin;

use bevy_asset::{Asset, AssetLoader, LoadContext};
use bevy_ecs::prelude::*;
use bevy_reflect::TypePath;
use std::collections::HashMap;
use thiserror::Error;
use serde::{Deserialize, de::Error as DeError};
use crate::widgets::blueprint::{WidgetBlueprint, WidgetCollection, WidgetType, LayoutConfig, StyleConfig, BehaviorConfig};

// Re-export modules
pub use systems::*;
pub use plugin::*;

/// Window configuration loaded from TOML
#[derive(Debug, Clone, bevy_ecs::prelude::Resource)]
pub struct WindowConfig {
    /// Window size [width, height]
    pub size: [f32; 2],
    /// Background color for the window
    pub background_color: Option<bevy_color::Color>,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            size: [600.0, 300.0],
            background_color: Some(bevy_color::Color::srgba(0.129, 0.161, 0.165, 1.0)),
        }
    }
}

/// Custom asset that represents a UI tree loaded from TOML files
#[derive(Asset, TypePath, Debug, Clone)]
pub struct UiTree {
    /// Window configuration
    pub window: WindowConfig,
    /// The widget collection loaded from TOML
    pub collection: WidgetCollection,
    /// Resolved includes and processed blueprints
    pub resolved_widgets: HashMap<String, WidgetBlueprint>,
    /// Root widget ID to spawn
    pub root: Option<String>,
}

impl UiTree {
    /// Create a new UiTree from a widget collection and window config
    pub fn new(collection: WidgetCollection, window: WindowConfig) -> Self {
        let root = collection.root.clone();
        let resolved_widgets = collection.widgets.clone();
        
        Self {
            window,
            collection,
            resolved_widgets,
            root,
        }
    }

    /// Get a resolved widget blueprint by ID
    pub fn get_widget(&self, id: &str) -> Option<&WidgetBlueprint> {
        self.resolved_widgets.get(id)
    }

    /// Get the root widget blueprint
    pub fn get_root_widget(&self) -> Option<&WidgetBlueprint> {
        self.root.as_ref().and_then(|id| self.get_widget(id))
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

/// Asset loader for UI TOML files
#[derive(Default)]
pub struct UiAssetLoader;

/// Errors that can occur during UI asset loading
#[derive(Error, Debug)]
pub enum UiAssetLoaderError {
    #[error("Failed to read UI file: {0}")]
    Io(#[from] std::io::Error),
    #[error("Failed to parse TOML: {0}")]
    TomlParse(#[from] toml::de::Error),
    #[error("Failed to resolve include: {0}")]
    IncludeResolution(String),
}

impl AssetLoader for UiAssetLoader {
    type Asset = UiTree;
    type Settings = ();
    type Error = UiAssetLoaderError;

    async fn load(
        &self,
        reader: &mut dyn bevy_asset::io::Reader,
        _settings: &(),
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let toml_str = std::str::from_utf8(&bytes)
            .map_err(|e| UiAssetLoaderError::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, e)))?;

        // Parse the TOML using custom structure
        let parsed_toml: toml::Value = toml::from_str(toml_str)?;
        let mut collection = parse_custom_toml_structure(parsed_toml.clone())?;

        // Process includes and resolve widget blueprints
        let resolved_widgets = self.resolve_includes(&mut collection, load_context).await?;

        // Parse window configuration from the TOML
        let window_config = parse_window_config(&parsed_toml)?;
        
        // Create the UiTree asset
        let mut ui_tree = UiTree::new(collection, window_config);
        ui_tree.resolved_widgets = resolved_widgets;

        Ok(ui_tree)
    }

    fn extensions(&self) -> &[&str] {
        &["toml"]
    }
}

impl UiAssetLoader {
    /// Resolve include directives in widget definitions
    async fn resolve_includes(
        &self,
        collection: &mut WidgetCollection,
        _load_context: &mut LoadContext<'_>,
    ) -> Result<HashMap<String, WidgetBlueprint>, UiAssetLoaderError> {
        let mut resolved = HashMap::new();

        // For now, we'll skip include resolution and just use the widgets as-is
        // In a full implementation, this would:
        // 1. Find widgets with include directives
        // 2. Load the referenced TOML files
        // 3. Merge the included blueprint with overrides
        // 4. Replace the widget definition with the resolved version

        for (id, widget) in &collection.widgets {
            resolved.insert(id.clone(), widget.clone());
        }

        Ok(resolved)
    }
}

/// Event to request loading and spawning a UI from a TOML asset
#[derive(Event, Debug, Clone)]
pub struct LoadUiRequest {
    /// Path to the UI asset file
    pub asset_path: String,
    /// Optional entity to spawn the UI as a child of
    pub parent: Option<Entity>,
    /// Optional position override
    pub position_override: Option<bevy_math::Vec3>,
}

impl LoadUiRequest {
    /// Create a new request to load a UI asset
    pub fn new(asset_path: impl Into<String>) -> Self {
        Self {
            asset_path: asset_path.into(),
            parent: None,
            position_override: None,
        }
    }

    /// Set the parent entity for the loaded UI
    pub fn with_parent(mut self, parent: Entity) -> Self {
        self.parent = Some(parent);
        self
    }

    /// Set a position override for the loaded UI
    pub fn with_position(mut self, position: bevy_math::Vec3) -> Self {
        self.position_override = Some(position);
        self
    }
}

/// Parse window configuration from TOML
fn parse_window_config(toml_value: &toml::Value) -> Result<WindowConfig, UiAssetLoaderError> {
    let table = toml_value.as_table()
        .ok_or_else(|| UiAssetLoaderError::TomlParse(DeError::custom("Root must be a table")))?;
    
    let mut window_config = WindowConfig::default();
    
    if let Some(window_section) = table.get("window") {
        if let Some(window_table) = window_section.as_table() {
            // Parse window size
            if let Some(size_array) = window_table.get("size") {
                if let Some(size_arr) = size_array.as_array() {
                    if size_arr.len() == 2 {
                        if let (Some(width), Some(height)) = (size_arr[0].as_float(), size_arr[1].as_float()) {
                            window_config.size = [width as f32, height as f32];
                        }
                    }
                }
            }
            
            // Parse background color
            if let Some(bg_color) = window_table.get("background_color") {
                if let Some(color_table) = bg_color.as_table() {
                    if let (Some(r), Some(g), Some(b), Some(a)) = (
                        color_table.get("r").and_then(|v| v.as_integer()),
                        color_table.get("g").and_then(|v| v.as_integer()),
                        color_table.get("b").and_then(|v| v.as_integer()),
                        color_table.get("a").and_then(|v| v.as_float()),
                    ) {
                        window_config.background_color = Some(bevy_color::Color::srgba(
                            r as f32 / 255.0,
                            g as f32 / 255.0,
                            b as f32 / 255.0,
                            a as f32,
                        ));
                    }
                }
            }
        }
    }
    
    Ok(window_config)
}

/// Parse the custom TOML structure used by our layout files
fn parse_custom_toml_structure(toml_value: toml::Value) -> Result<WidgetCollection, UiAssetLoaderError> {
    let table = toml_value.as_table()
        .ok_or_else(|| UiAssetLoaderError::TomlParse(DeError::custom("Root must be a table")))?;
    
    let mut widgets = HashMap::new();
    let root = None; // No explicit root - all widgets are children of window
    
    // Parse widgets section
    if let Some(widgets_section) = table.get("widgets") {
        if let Some(widgets_table) = widgets_section.as_table() {
            for (widget_id, widget_data) in widgets_table {
                let widget = parse_widget_from_toml(widget_id, widget_data)?;
                widgets.insert(widget_id.clone(), widget);
            }
        }
    }
    
    Ok(WidgetCollection { widgets, root })
}

/// Parse a single widget from TOML data
fn parse_widget_from_toml(id: &str, toml_data: &toml::Value) -> Result<WidgetBlueprint, UiAssetLoaderError> {
    let table = toml_data.as_table()
        .ok_or_else(|| UiAssetLoaderError::TomlParse(DeError::custom("Widget must be a table")))?;
    
    // Parse widget_type using serde deserializer
    let widget_type = if let Some(wt) = table.get("widget_type") {
        WidgetType::deserialize(wt.clone())
            .map_err(|e| UiAssetLoaderError::TomlParse(DeError::custom(format!("Invalid widget_type: {}", e))))?
    } else {
        return Err(UiAssetLoaderError::TomlParse(DeError::custom("Missing widget_type")));
    };
    
    // Parse layout section
    let layout = if let Some(layout_section) = table.get("layout") {
        LayoutConfig::deserialize(layout_section.clone())
            .map_err(|e| UiAssetLoaderError::TomlParse(DeError::custom(format!("Invalid layout: {}", e))))?
    } else {
        LayoutConfig::default()
    };
    
    // Parse style section
    let style = if let Some(style_section) = table.get("style") {
        StyleConfig::deserialize(style_section.clone())
            .map_err(|e| UiAssetLoaderError::TomlParse(DeError::custom(format!("Invalid style: {}", e))))?
    } else {
        StyleConfig::default()
    };
    
    // Parse behavior section
    let behavior = if let Some(behavior_section) = table.get("behavior") {
        BehaviorConfig::deserialize(behavior_section.clone())
            .map_err(|e| UiAssetLoaderError::TomlParse(DeError::custom(format!("Invalid behavior: {}", e))))?
    } else {
        BehaviorConfig::default()
    };
    
    // Parse children
    let children = if let Some(children_section) = table.get("children") {
        if let Some(children_array) = children_section.as_array() {
            children_array.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };
    
    Ok(WidgetBlueprint {
        id: id.to_string(),
        widget_type,
        layout,
        style,
        behavior,
        children,
    })
}