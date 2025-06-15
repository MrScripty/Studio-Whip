pub mod systems;
pub mod plugin;

use bevy_asset::{Asset, AssetLoader, AsyncReadExt, LoadContext};
use bevy_ecs::prelude::*;
use bevy_reflect::TypePath;
use std::collections::HashMap;
use thiserror::Error;
use crate::widgets::blueprint::{WidgetBlueprint, WidgetCollection};

// Re-export modules
pub use systems::*;
pub use plugin::*;

/// Custom asset that represents a UI tree loaded from TOML files
#[derive(Asset, TypePath, Debug, Clone)]
pub struct UiTree {
    /// The widget collection loaded from TOML
    pub collection: WidgetCollection,
    /// Resolved includes and processed blueprints
    pub resolved_widgets: HashMap<String, WidgetBlueprint>,
    /// Root widget ID to spawn
    pub root: Option<String>,
}

impl UiTree {
    /// Create a new UiTree from a widget collection
    pub fn new(collection: WidgetCollection) -> Self {
        let root = collection.root.clone();
        let resolved_widgets = collection.widgets.clone();
        
        Self {
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

        // Parse the TOML into a widget collection
        let mut collection: WidgetCollection = toml::from_str(toml_str)?;

        // Process includes and resolve widget blueprints
        let resolved_widgets = self.resolve_includes(&mut collection, load_context).await?;

        // Create the UiTree asset
        let mut ui_tree = UiTree::new(collection);
        ui_tree.resolved_widgets = resolved_widgets;

        Ok(ui_tree)
    }

    fn extensions(&self) -> &[&str] {
        &["ui.toml", "layout.toml"]
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