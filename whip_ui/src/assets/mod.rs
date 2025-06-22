pub mod systems;
pub mod plugin;
pub mod definitions;
pub mod registry;
pub mod loaders;

#[cfg(test)]
mod tests;

use bevy_asset::Asset;
use bevy_reflect::TypePath;
use std::collections::HashMap;
use crate::widgets::blueprint::{WidgetBlueprint, WidgetCollection};

// Re-export modules
pub use systems::*;
pub use plugin::*;
pub use definitions::*;
pub use registry::*;
pub use loaders::*;

/// Window configuration loaded from TOML
#[derive(Debug, Clone, bevy_ecs::prelude::Resource, serde::Deserialize, serde::Serialize)]
pub struct WindowConfig {
    /// Window size [width, height]
    pub size: [f32; 2],
    /// Background color for the window
    pub background_color: Option<crate::widgets::blueprint::ColorDef>,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            size: [600.0, 300.0],
            background_color: Some(crate::widgets::blueprint::ColorDef::Rgba { r: 33, g: 41, b: 42, a: 1.0 }),
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