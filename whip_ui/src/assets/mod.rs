pub mod systems;
pub mod plugin;

use bevy_asset::{Asset, AssetLoader, LoadContext};
use bevy_ecs::prelude::*;
use bevy_reflect::TypePath;
use std::collections::{HashMap, HashSet};
use thiserror::Error;
use serde::{Deserialize, de::Error as DeError};
use crate::widgets::blueprint::{WidgetBlueprint, WidgetCollection, WidgetType, LayoutConfig, StyleConfig, BehaviorConfig};

// Re-export modules
pub use systems::*;
pub use plugin::*;

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

// ==================== Phase 2: Hierarchical Data Structures ====================

/// New hierarchical UI definition that represents source data from TOML
#[derive(Asset, TypePath, Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct UiDefinition {
    /// Window configuration
    pub window: Option<WindowConfig>,
    /// Root widget node defining the UI hierarchy
    pub root: WidgetNode,
    /// Global styles that can be referenced by class name
    pub styles: Option<HashMap<String, StyleOverrides>>,
    /// Global actions that can be referenced by widgets
    pub actions: Option<HashMap<String, ActionBinding>>,
}

/// Recursive widget node structure representing the UI hierarchy
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct WidgetNode {
    /// Unique identifier for this widget
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Widget type and its configuration
    pub widget_type: WidgetType,
    /// Layout configuration
    #[serde(default)]
    pub layout: LayoutConfig,
    /// Style configuration
    #[serde(default)]
    pub style: StyleConfig,
    /// Behavior configuration
    #[serde(default)]
    pub behavior: BehaviorConfig,
    /// Style class names to apply
    #[serde(skip_serializing_if = "Option::is_none")]
    pub classes: Option<Vec<String>>,
    /// Style overrides that take precedence over classes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style_overrides: Option<StyleOverrides>,
    /// Interaction bindings for this widget
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bindings: Option<HashMap<String, ActionBinding>>,
    /// Child widget nodes
    #[serde(default)]
    pub children: Vec<WidgetNode>,
}

/// Style overrides that can be applied to widgets
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, Default)]
pub struct StyleOverrides {
    /// Background color override
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background_color: Option<crate::widgets::blueprint::ColorDef>,
    /// Border color override
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border_color: Option<crate::widgets::blueprint::ColorDef>,
    /// Border width override
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border_width: Option<f32>,
    /// Border radius override
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border_radius: Option<f32>,
    /// Text color override
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_color: Option<crate::widgets::blueprint::ColorDef>,
    /// Text size override
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_size: Option<f32>,
    /// Opacity override
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opacity: Option<f32>,
}

/// Action binding that connects UI events to actions
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ActionBinding {
    /// Action type (e.g., "click", "hover", "focus")
    pub event: String,
    /// Action to execute (e.g., "navigate_home", "toggle_settings")
    pub action: String,
    /// Optional parameters for the action
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<HashMap<String, toml::Value>>,
}

impl UiDefinition {
    /// Validate the UI definition structure
    pub fn validate(&self) -> Result<(), UiDefinitionError> {
        // Validate window config
        self.validate_window_config()?;
        
        // Validate global styles for internal consistency
        self.validate_global_styles()?;
        
        // Validate global actions
        self.validate_global_actions()?;
        
        // Validate widget hierarchy
        self.validate_widget_node(&self.root, &HashSet::new())?;
        
        Ok(())
    }

    /// Validate the UI definition structure with registry-based validation
    pub fn validate_with_registry(&self, registry: &UiRegistry) -> Result<(), UiDefinitionError> {
        // First run basic validation
        self.validate()?;
        
        // Then run registry-based validation
        self.validate_widget_node_with_registry(&self.root, &HashSet::new(), registry, 0)?;
        
        // Validate global actions against registry
        if let Some(ref actions) = self.actions {
            for (_, action_binding) in actions {
                if let Err(err) = registry.validate_action_binding(action_binding) {
                    return Err(UiDefinitionError::RegistryValidation(err.to_string()));
                }
            }
        }
        
        Ok(())
    }

    /// Validate window configuration
    fn validate_window_config(&self) -> Result<(), UiDefinitionError> {
        if let Some(ref window) = self.window {
            // Validate window size
            if window.size[0] <= 0.0 || window.size[1] <= 0.0 {
                return Err(UiDefinitionError::Validation(
                    "Window size must be positive".to_string()
                ));
            }
            
            // Validate background color if present
            if let Some(ref color) = window.background_color {
                self.validate_color_def(color)?;
            }
        }
        Ok(())
    }

    /// Validate global styles
    fn validate_global_styles(&self) -> Result<(), UiDefinitionError> {
        if let Some(ref styles) = self.styles {
            for (style_name, style_overrides) in styles {
                if style_name.is_empty() {
                    return Err(UiDefinitionError::Validation(
                        "Style class name cannot be empty".to_string()
                    ));
                }
                
                // Validate color definitions in style overrides
                if let Some(ref color) = style_overrides.background_color {
                    self.validate_color_def(color)?;
                }
                if let Some(ref color) = style_overrides.border_color {
                    self.validate_color_def(color)?;
                }
                if let Some(ref color) = style_overrides.text_color {
                    self.validate_color_def(color)?;
                }
                
                // Validate numeric values
                if let Some(width) = style_overrides.border_width {
                    if width < 0.0 {
                        return Err(UiDefinitionError::Validation(
                            format!("Border width must be non-negative in style '{}'", style_name)
                        ));
                    }
                }
                
                if let Some(radius) = style_overrides.border_radius {
                    if radius < 0.0 {
                        return Err(UiDefinitionError::Validation(
                            format!("Border radius must be non-negative in style '{}'", style_name)
                        ));
                    }
                }
                
                if let Some(size) = style_overrides.text_size {
                    if size <= 0.0 {
                        return Err(UiDefinitionError::Validation(
                            format!("Text size must be positive in style '{}'", style_name)
                        ));
                    }
                }
                
                if let Some(opacity) = style_overrides.opacity {
                    if !(0.0..=1.0).contains(&opacity) {
                        return Err(UiDefinitionError::Validation(
                            format!("Opacity must be between 0.0 and 1.0 in style '{}'", style_name)
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    /// Validate global actions
    fn validate_global_actions(&self) -> Result<(), UiDefinitionError> {
        if let Some(ref actions) = self.actions {
            for (action_name, action_binding) in actions {
                if action_name.is_empty() {
                    return Err(UiDefinitionError::Validation(
                        "Action name cannot be empty".to_string()
                    ));
                }
                
                if action_binding.event.is_empty() {
                    return Err(UiDefinitionError::Validation(
                        format!("Event type cannot be empty for action '{}'", action_name)
                    ));
                }
                
                if action_binding.action.is_empty() {
                    return Err(UiDefinitionError::Validation(
                        format!("Action string cannot be empty for action '{}'", action_name)
                    ));
                }
                
                // Validate known event types
                let valid_events = ["click", "hover", "focus", "blur", "change", "submit"];
                if !valid_events.contains(&action_binding.event.as_str()) {
                    return Err(UiDefinitionError::Validation(
                        format!("Unknown event type '{}' for action '{}'", action_binding.event, action_name)
                    ));
                }
            }
        }
        Ok(())
    }

    /// Validate a color definition
    fn validate_color_def(&self, color: &crate::widgets::blueprint::ColorDef) -> Result<(), UiDefinitionError> {
        match color {
            crate::widgets::blueprint::ColorDef::Hex(hex) => {
                let hex = hex.trim_start_matches('#');
                if hex.len() != 6 && hex.len() != 8 {
                    return Err(UiDefinitionError::Validation(
                        format!("Invalid hex color format: '{:?}'. Expected #RRGGBB or #RRGGBBAA", color)
                    ));
                }
                
                // Check if all characters are valid hex
                if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
                    return Err(UiDefinitionError::Validation(
                        format!("Invalid hex color format: '{:?}'. Contains non-hex characters", color)
                    ));
                }
            }
            crate::widgets::blueprint::ColorDef::Rgba { a, .. } => {
                if !(0.0..=1.0).contains(a) {
                    return Err(UiDefinitionError::Validation(
                        "Alpha channel must be between 0.0 and 1.0".to_string()
                    ));
                }
            }
            crate::widgets::blueprint::ColorDef::Named(name) => {
                let valid_names = ["red", "green", "blue", "black", "white", "gray", "grey", 
                                 "yellow", "cyan", "magenta", "orange", "transparent"];
                if !valid_names.contains(&name.to_lowercase().as_str()) {
                    return Err(UiDefinitionError::Validation(
                        format!("Unknown named color: '{}'", name)
                    ));
                }
            }
            _ => {} // RGB is always valid since u8 values are constrained
        }
        Ok(())
    }

    /// Recursively validate a widget node and its children
    fn validate_widget_node(&self, node: &WidgetNode, used_ids: &HashSet<String>) -> Result<(), UiDefinitionError> {
        // Check for duplicate IDs
        if let Some(ref id) = node.id {
            if used_ids.contains(id) {
                return Err(UiDefinitionError::DuplicateId(id.clone()));
            }
            
            // Validate ID format
            if id.is_empty() {
                return Err(UiDefinitionError::Validation("Widget ID cannot be empty".to_string()));
            }
            
            // ID should only contain alphanumeric characters, underscore, and hyphen
            if !id.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
                return Err(UiDefinitionError::Validation(
                    format!("Widget ID '{}' contains invalid characters. Only alphanumeric, underscore, and hyphen are allowed", id)
                ));
            }
        }

        // Validate widget type specific constraints
        self.validate_widget_type(&node.widget_type)?;

        // Validate layout configuration
        self.validate_layout_config(&node.layout)?;

        // Validate style configuration
        self.validate_style_config(&node.style)?;

        // Validate behavior configuration
        self.validate_behavior_config(&node.behavior)?;

        // Validate style classes exist
        if let Some(ref classes) = node.classes {
            if let Some(ref global_styles) = self.styles {
                for class_name in classes {
                    if !global_styles.contains_key(class_name) {
                        return Err(UiDefinitionError::UnknownStyleClass(class_name.clone()));
                    }
                }
            } else if !classes.is_empty() {
                return Err(UiDefinitionError::StyleClassesWithoutGlobalStyles);
            }
        }

        // Validate style overrides
        if let Some(ref overrides) = node.style_overrides {
            self.validate_style_overrides(overrides)?;
        }

        // Validate action bindings
        if let Some(ref bindings) = node.bindings {
            for (event_name, binding) in bindings {
                if event_name.is_empty() {
                    return Err(UiDefinitionError::Validation("Binding event name cannot be empty".to_string()));
                }
                
                if let Some(ref global_actions) = self.actions {
                    if !global_actions.contains_key(&binding.action) {
                        return Err(UiDefinitionError::UnknownAction(binding.action.clone()));
                    }
                } else {
                    return Err(UiDefinitionError::Validation(
                        "Action bindings specified but no global actions defined".to_string()
                    ));
                }
            }
        }

        // Recursively validate children
        let mut child_ids = used_ids.clone();
        if let Some(ref id) = node.id {
            child_ids.insert(id.clone());
        }

        // Check for duplicate IDs among siblings
        let mut sibling_ids = HashSet::new();
        for child in &node.children {
            if let Some(ref child_id) = child.id {
                if sibling_ids.contains(child_id) {
                    return Err(UiDefinitionError::DuplicateId(child_id.clone()));
                }
                sibling_ids.insert(child_id.clone());
            }
        }

        for child in &node.children {
            self.validate_widget_node(child, &child_ids)?;
        }

        Ok(())
    }

    /// Validate widget type specific constraints
    fn validate_widget_type(&self, widget_type: &WidgetType) -> Result<(), UiDefinitionError> {
        match widget_type {
            WidgetType::Button { text, .. } => {
                if text.is_empty() {
                    return Err(UiDefinitionError::Validation("Button text cannot be empty".to_string()));
                }
            }
            WidgetType::Text { content, .. } => {
                // Text content can be empty (for placeholder text)
                if content.len() > 10000 {
                    return Err(UiDefinitionError::Validation("Text content too long (max 10000 characters)".to_string()));
                }
            }
            _ => {} // Other widget types don't have specific validation yet
        }
        Ok(())
    }

    /// Validate layout configuration
    fn validate_layout_config(&self, layout: &LayoutConfig) -> Result<(), UiDefinitionError> {
        if let Some(size) = layout.size {
            if size.x < 0.0 || size.y < 0.0 {
                return Err(UiDefinitionError::Validation("Layout size must be non-negative".to_string()));
            }
            if size.x > 10000.0 || size.y > 10000.0 {
                return Err(UiDefinitionError::Validation("Layout size is unreasonably large".to_string()));
            }
        }

        if let Some(flex_grow) = layout.flex_grow {
            if flex_grow < 0.0 {
                return Err(UiDefinitionError::Validation("Flex grow must be non-negative".to_string()));
            }
        }

        if let Some(flex_shrink) = layout.flex_shrink {
            if flex_shrink < 0.0 {
                return Err(UiDefinitionError::Validation("Flex shrink must be non-negative".to_string()));
            }
        }

        Ok(())
    }

    /// Validate style configuration
    fn validate_style_config(&self, style: &StyleConfig) -> Result<(), UiDefinitionError> {
        if let Some(ref color) = style.background_color {
            self.validate_color_def(color)?;
        }
        if let Some(ref color) = style.border_color {
            self.validate_color_def(color)?;
        }
        if let Some(ref color) = style.text_color {
            self.validate_color_def(color)?;
        }

        if let Some(width) = style.border_width {
            if width < 0.0 {
                return Err(UiDefinitionError::Validation("Border width must be non-negative".to_string()));
            }
        }

        if let Some(radius) = style.border_radius {
            if radius < 0.0 {
                return Err(UiDefinitionError::Validation("Border radius must be non-negative".to_string()));
            }
        }

        if let Some(size) = style.text_size {
            if size <= 0.0 {
                return Err(UiDefinitionError::Validation("Text size must be positive".to_string()));
            }
        }

        if let Some(opacity) = style.opacity {
            if !(0.0..=1.0).contains(&opacity) {
                return Err(UiDefinitionError::Validation("Opacity must be between 0.0 and 1.0".to_string()));
            }
        }

        Ok(())
    }

    /// Validate behavior configuration
    fn validate_behavior_config(&self, _behavior: &BehaviorConfig) -> Result<(), UiDefinitionError> {
        // Most behavior settings are just booleans, which are always valid
        // Could add validation for z_index ranges if needed
        Ok(())
    }

    /// Validate style overrides
    fn validate_style_overrides(&self, overrides: &StyleOverrides) -> Result<(), UiDefinitionError> {
        if let Some(ref color) = overrides.background_color {
            self.validate_color_def(color)?;
        }
        if let Some(ref color) = overrides.border_color {
            self.validate_color_def(color)?;
        }
        if let Some(ref color) = overrides.text_color {
            self.validate_color_def(color)?;
        }

        if let Some(width) = overrides.border_width {
            if width < 0.0 {
                return Err(UiDefinitionError::Validation("Border width override must be non-negative".to_string()));
            }
        }

        if let Some(radius) = overrides.border_radius {
            if radius < 0.0 {
                return Err(UiDefinitionError::Validation("Border radius override must be non-negative".to_string()));
            }
        }

        if let Some(size) = overrides.text_size {
            if size <= 0.0 {
                return Err(UiDefinitionError::Validation("Text size override must be positive".to_string()));
            }
        }

        if let Some(opacity) = overrides.opacity {
            if !(0.0..=1.0).contains(&opacity) {
                return Err(UiDefinitionError::Validation("Opacity override must be between 0.0 and 1.0".to_string()));
            }
        }

        Ok(())
    }

    /// Recursively validate a widget node with registry-based validation
    fn validate_widget_node_with_registry(&self, node: &WidgetNode, used_ids: &HashSet<String>, registry: &UiRegistry, depth: usize) -> Result<(), UiDefinitionError> {
        // Validate nesting depth
        if let Err(err) = registry.validate_nesting_depth(depth) {
            return Err(UiDefinitionError::RegistryValidation(err.to_string()));
        }

        // Check for duplicate IDs (same as regular validation)
        if let Some(ref id) = node.id {
            if used_ids.contains(id) {
                return Err(UiDefinitionError::DuplicateId(id.clone()));
            }
            
            // Validate ID format
            if id.is_empty() {
                return Err(UiDefinitionError::Validation("Widget ID cannot be empty".to_string()));
            }
            
            // ID should only contain alphanumeric characters, underscore, and hyphen
            if !id.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
                return Err(UiDefinitionError::Validation(
                    format!("Widget ID '{}' contains invalid characters. Only alphanumeric, underscore, and hyphen are allowed", id)
                ));
            }
        }

        // Registry-based widget type validation
        if let Err(err) = registry.validate_widget_type(&node.widget_type) {
            return Err(UiDefinitionError::RegistryValidation(err.to_string()));
        }

        // Validate widget children capability
        let has_children = !node.children.is_empty();
        if let Err(err) = registry.validate_widget_children(&node.widget_type, has_children) {
            return Err(UiDefinitionError::RegistryValidation(err.to_string()));
        }

        // Validate layout configuration (use existing validation)
        self.validate_layout_config(&node.layout)?;

        // Validate style configuration (use existing validation)
        self.validate_style_config(&node.style)?;

        // Validate behavior configuration (use existing validation)
        self.validate_behavior_config(&node.behavior)?;

        // Validate style classes exist (use existing validation)
        if let Some(ref classes) = node.classes {
            if let Some(ref global_styles) = self.styles {
                for class_name in classes {
                    if !global_styles.contains_key(class_name) {
                        return Err(UiDefinitionError::UnknownStyleClass(class_name.clone()));
                    }
                }
            } else if !classes.is_empty() {
                return Err(UiDefinitionError::StyleClassesWithoutGlobalStyles);
            }
        }

        // Validate style overrides (use existing validation)
        if let Some(ref overrides) = node.style_overrides {
            self.validate_style_overrides(overrides)?;
        }

        // Registry-based action binding validation
        if let Some(ref bindings) = node.bindings {
            for (event_name, binding) in bindings {
                if event_name.is_empty() {
                    return Err(UiDefinitionError::Validation("Binding event name cannot be empty".to_string()));
                }
                
                // Use registry to validate action bindings
                if let Err(err) = registry.validate_action_binding(binding) {
                    return Err(UiDefinitionError::RegistryValidation(err.to_string()));
                }
            }
        }

        // Check for duplicate IDs among siblings (same as regular validation)
        let mut sibling_ids = HashSet::new();
        for child in &node.children {
            if let Some(ref child_id) = child.id {
                if sibling_ids.contains(child_id) {
                    return Err(UiDefinitionError::DuplicateId(child_id.clone()));
                }
                sibling_ids.insert(child_id.clone());
            }
        }

        // Recursively validate children with registry
        let mut child_ids = used_ids.clone();
        if let Some(ref id) = node.id {
            child_ids.insert(id.clone());
        }

        for child in &node.children {
            self.validate_widget_node_with_registry(child, &child_ids, registry, depth + 1)?;
        }

        Ok(())
    }

    /// Convert this hierarchical definition to a flat widget collection for backward compatibility
    pub fn to_widget_collection(&self) -> WidgetCollection {
        let mut widgets = HashMap::new();
        let mut counter = 0;

        // Generate a unique ID for the root if it doesn't have one
        let root_id = self.root.id.clone().unwrap_or_else(|| {
            counter += 1;
            format!("root_{}", counter)
        });

        self.collect_widgets_recursive(&self.root, &root_id, &mut widgets, &mut counter);

        WidgetCollection {
            widgets,
            root: Some(root_id),
        }
    }

    /// Recursively collect widgets into a flat structure
    fn collect_widgets_recursive(
        &self,
        node: &WidgetNode,
        node_id: &str,
        widgets: &mut HashMap<String, WidgetBlueprint>,
        counter: &mut usize,
    ) {
        // Collect child IDs
        let mut child_ids = Vec::new();
        for child in &node.children {
            let child_id = child.id.clone().unwrap_or_else(|| {
                *counter += 1;
                format!("widget_{}", counter)
            });
            child_ids.push(child_id.clone());
            
            // Recursively process child
            self.collect_widgets_recursive(child, &child_id, widgets, counter);
        }

        // Apply style overrides to create final style config
        let mut final_style = node.style.clone();
        
        // Apply global styles first
        if let Some(ref classes) = node.classes {
            if let Some(ref global_styles) = self.styles {
                for class_name in classes {
                    if let Some(class_style) = global_styles.get(class_name) {
                        Self::apply_style_overrides(&mut final_style, class_style);
                    }
                }
            }
        }

        // Apply local style overrides last (highest priority)
        if let Some(ref overrides) = node.style_overrides {
            Self::apply_style_overrides(&mut final_style, overrides);
        }

        // Create widget blueprint
        let blueprint = WidgetBlueprint {
            id: node_id.to_string(),
            widget_type: node.widget_type.clone(),
            layout: node.layout.clone(),
            style: final_style,
            behavior: node.behavior.clone(),
            children: child_ids,
        };

        widgets.insert(node_id.to_string(), blueprint);
    }

    /// Apply style overrides to a style config
    fn apply_style_overrides(style: &mut StyleConfig, overrides: &StyleOverrides) {
        if let Some(ref color) = overrides.background_color {
            style.background_color = Some(color.clone());
        }
        if let Some(ref color) = overrides.border_color {
            style.border_color = Some(color.clone());
        }
        if let Some(width) = overrides.border_width {
            style.border_width = Some(width);
        }
        if let Some(radius) = overrides.border_radius {
            style.border_radius = Some(radius);
        }
        if let Some(ref color) = overrides.text_color {
            style.text_color = Some(color.clone());
        }
        if let Some(size) = overrides.text_size {
            style.text_size = Some(size);
        }
        if let Some(opacity) = overrides.opacity {
            style.opacity = Some(opacity);
        }
    }
}

/// Errors that can occur during UI definition validation
#[derive(Error, Debug)]
pub enum UiDefinitionError {
    #[error("Duplicate widget ID: {0}")]
    DuplicateId(String),
    #[error("Unknown style class: {0}")]
    UnknownStyleClass(String),
    #[error("Style classes specified but no global styles defined")]
    StyleClassesWithoutGlobalStyles,
    #[error("Unknown action: {0}")]
    UnknownAction(String),
    #[error("Validation error: {0}")]
    Validation(String),
    #[error("Registry validation error: {0}")]
    RegistryValidation(String),
}

// ==================== End Phase 2 Structures ====================

// ==================== Phase 2: UiRegistry Resource ====================

/// Registry resource for widget type validation and state management
#[derive(Resource, Debug, Clone)]
pub struct UiRegistry {
    /// Map of widget type names to their asset paths for validation
    pub widget_type_mappings: HashMap<String, WidgetTypeInfo>,
    /// Map of registered state types for binding validation
    pub registered_state_types: HashMap<String, StateTypeInfo>,
    /// Map of valid action names to their descriptions
    pub valid_actions: HashMap<String, ActionInfo>,
    /// Registry configuration
    pub config: UiRegistryConfig,
}

/// Information about a registered widget type
#[derive(Debug, Clone)]
pub struct WidgetTypeInfo {
    /// Display name of the widget type
    pub display_name: String,
    /// Asset path where the widget definition can be found (optional)
    pub asset_path: Option<String>,
    /// Required properties for this widget type
    pub required_properties: Vec<String>,
    /// Optional properties for this widget type
    pub optional_properties: Vec<String>,
    /// Whether this widget type can have children
    pub can_have_children: bool,
}

/// Information about a registered state type for data binding
#[derive(Debug, Clone)]
pub struct StateTypeInfo {
    /// Display name of the state type
    pub display_name: String,
    /// The type identifier (e.g., "String", "i32", "CustomState")
    pub type_id: String,
    /// Valid operations for this state type
    pub valid_operations: Vec<String>,
    /// Default value serialized as TOML value
    pub default_value: Option<toml::Value>,
}

/// Information about a registered action
#[derive(Debug, Clone)]
pub struct ActionInfo {
    /// Display name of the action
    pub display_name: String,
    /// Description of what this action does
    pub description: String,
    /// Expected parameter types
    pub parameter_types: HashMap<String, String>,
    /// Whether this action requires specific conditions to execute
    pub requires_conditions: Vec<String>,
}

/// Configuration for the UI registry
#[derive(Debug, Clone)]
pub struct UiRegistryConfig {
    /// Whether to enable strict validation (fail on unknown widgets/actions)
    pub strict_validation: bool,
    /// Whether to allow custom widget types not in the registry
    pub allow_custom_widgets: bool,
    /// Whether to allow custom actions not in the registry
    pub allow_custom_actions: bool,
    /// Maximum nesting depth for widget hierarchies
    pub max_nesting_depth: usize,
}

impl Default for UiRegistry {
    fn default() -> Self {
        Self {
            widget_type_mappings: HashMap::new(),
            registered_state_types: HashMap::new(),
            valid_actions: HashMap::new(),
            config: UiRegistryConfig::default(),
        }
    }
}

impl Default for UiRegistryConfig {
    fn default() -> Self {
        Self {
            strict_validation: true,
            allow_custom_widgets: false,
            allow_custom_actions: false,
            max_nesting_depth: 50,
        }
    }
}

impl UiRegistry {
    /// Create a new UI registry with default built-in types
    pub fn new() -> Self {
        let mut registry = Self::default();
        registry.register_builtin_types();
        registry
    }

    /// Create a new UI registry with custom configuration
    pub fn with_config(config: UiRegistryConfig) -> Self {
        let mut registry = Self {
            config,
            ..Self::default()
        };
        registry.register_builtin_types();
        registry
    }

    /// Register built-in widget types, state types, and actions
    fn register_builtin_types(&mut self) {
        // Register built-in widget types
        self.register_widget_type("Container", WidgetTypeInfo {
            display_name: "Container".to_string(),
            asset_path: None,
            required_properties: vec!["direction".to_string()],
            optional_properties: vec![],
            can_have_children: true,
        });

        self.register_widget_type("Button", WidgetTypeInfo {
            display_name: "Button".to_string(),
            asset_path: None,
            required_properties: vec!["text".to_string()],
            optional_properties: vec!["action".to_string()],
            can_have_children: false,
        });

        self.register_widget_type("Text", WidgetTypeInfo {
            display_name: "Text".to_string(),
            asset_path: None,
            required_properties: vec!["content".to_string(), "editable".to_string()],
            optional_properties: vec![],
            can_have_children: false,
        });

        self.register_widget_type("Shape", WidgetTypeInfo {
            display_name: "Shape".to_string(),
            asset_path: None,
            required_properties: vec!["shape_type".to_string()],
            optional_properties: vec![],
            can_have_children: false,
        });

        // Register built-in state types
        self.register_state_type("String", StateTypeInfo {
            display_name: "String".to_string(),
            type_id: "String".to_string(),
            valid_operations: vec!["set".to_string(), "get".to_string(), "append".to_string()],
            default_value: Some(toml::Value::String("".to_string())),
        });

        self.register_state_type("Boolean", StateTypeInfo {
            display_name: "Boolean".to_string(),
            type_id: "bool".to_string(),
            valid_operations: vec!["set".to_string(), "get".to_string(), "toggle".to_string()],
            default_value: Some(toml::Value::Boolean(false)),
        });

        self.register_state_type("Integer", StateTypeInfo {
            display_name: "Integer".to_string(),
            type_id: "i32".to_string(),
            valid_operations: vec!["set".to_string(), "get".to_string(), "increment".to_string(), "decrement".to_string()],
            default_value: Some(toml::Value::Integer(0)),
        });

        // Register built-in actions
        self.register_action("navigate_home", ActionInfo {
            display_name: "Navigate Home".to_string(),
            description: "Navigate to the home screen".to_string(),
            parameter_types: HashMap::new(),
            requires_conditions: vec![],
        });

        self.register_action("open_settings", ActionInfo {
            display_name: "Open Settings".to_string(),
            description: "Open the settings panel".to_string(),
            parameter_types: HashMap::new(),
            requires_conditions: vec![],
        });

        self.register_action("toggle_visibility", ActionInfo {
            display_name: "Toggle Visibility".to_string(),
            description: "Toggle the visibility of a UI element".to_string(),
            parameter_types: {
                let mut params = HashMap::new();
                params.insert("target".to_string(), "String".to_string());
                params
            },
            requires_conditions: vec!["target_exists".to_string()],
        });
    }

    /// Register a new widget type
    pub fn register_widget_type(&mut self, name: &str, info: WidgetTypeInfo) {
        self.widget_type_mappings.insert(name.to_string(), info);
    }

    /// Register a new state type
    pub fn register_state_type(&mut self, name: &str, info: StateTypeInfo) {
        self.registered_state_types.insert(name.to_string(), info);
    }

    /// Register a new action
    pub fn register_action(&mut self, name: &str, info: ActionInfo) {
        self.valid_actions.insert(name.to_string(), info);
    }

    /// Check if a widget type is registered
    pub fn is_widget_type_registered(&self, widget_type: &str) -> bool {
        self.widget_type_mappings.contains_key(widget_type)
    }

    /// Check if a state type is registered
    pub fn is_state_type_registered(&self, state_type: &str) -> bool {
        self.registered_state_types.contains_key(state_type)
    }

    /// Check if an action is registered
    pub fn is_action_registered(&self, action: &str) -> bool {
        self.valid_actions.contains_key(action)
    }

    /// Get widget type information
    pub fn get_widget_type_info(&self, widget_type: &str) -> Option<&WidgetTypeInfo> {
        self.widget_type_mappings.get(widget_type)
    }

    /// Get state type information
    pub fn get_state_type_info(&self, state_type: &str) -> Option<&StateTypeInfo> {
        self.registered_state_types.get(state_type)
    }

    /// Get action information
    pub fn get_action_info(&self, action: &str) -> Option<&ActionInfo> {
        self.valid_actions.get(action)
    }

    /// Validate a widget type against the registry
    pub fn validate_widget_type(&self, widget_type: &WidgetType) -> Result<(), UiRegistryError> {
        let type_name = self.extract_widget_type_name(widget_type);
        
        // Check if widget type is registered
        if !self.is_widget_type_registered(&type_name) {
            if self.config.strict_validation && !self.config.allow_custom_widgets {
                return Err(UiRegistryError::UnknownWidgetType(type_name));
            }
        }

        // Get widget type info for validation
        if let Some(info) = self.get_widget_type_info(&type_name) {
            // Validate required and optional properties
            self.validate_widget_properties(widget_type, info)?;
        }

        Ok(())
    }

    /// Extract the widget type name from a WidgetType enum
    fn extract_widget_type_name(&self, widget_type: &WidgetType) -> String {
        match widget_type {
            WidgetType::Container { .. } => "Container".to_string(),
            WidgetType::Button { .. } => "Button".to_string(),
            WidgetType::Text { .. } => "Text".to_string(),
            WidgetType::Shape { .. } => "Shape".to_string(),
            WidgetType::Custom { component, .. } => component.clone(),
        }
    }

    /// Validate widget properties against registered requirements
    fn validate_widget_properties(&self, widget_type: &WidgetType, info: &WidgetTypeInfo) -> Result<(), UiRegistryError> {
        match widget_type {
            WidgetType::Container { direction: _ } => {
                // Container requires direction property
                if info.required_properties.contains(&"direction".to_string()) {
                    // Direction is present - validation passes
                } else if info.required_properties.iter().any(|prop| prop == "direction") {
                    return Err(UiRegistryError::MissingRequiredProperty {
                        widget_type: "Container".to_string(),
                        property: "direction".to_string(),
                    });
                }
            },
            WidgetType::Button { text, action } => {
                // Button requires text property
                if info.required_properties.contains(&"text".to_string()) && text.is_empty() {
                    return Err(UiRegistryError::InvalidPropertyValue {
                        widget_type: "Button".to_string(),
                        property: "text".to_string(),
                        reason: "Button text cannot be empty".to_string(),
                    });
                }
                
                // Check if action is provided when it's required
                if info.required_properties.contains(&"action".to_string()) && action.is_none() {
                    return Err(UiRegistryError::MissingRequiredProperty {
                        widget_type: "Button".to_string(),
                        property: "action".to_string(),
                    });
                }

                // Validate action if provided
                if let Some(action_name) = action {
                    if !self.is_action_registered(action_name) && self.config.strict_validation && !self.config.allow_custom_actions {
                        return Err(UiRegistryError::UnknownAction(action_name.clone()));
                    }
                }
            },
            WidgetType::Text { content, editable: _ } => {
                // Text validation - content length check
                if content.len() > 10000 {
                    return Err(UiRegistryError::InvalidPropertyValue {
                        widget_type: "Text".to_string(),
                        property: "content".to_string(),
                        reason: "Text content exceeds maximum length of 10000 characters".to_string(),
                    });
                }
                
                // editable is always present as it's a boolean, so no validation needed
            },
            WidgetType::Shape { shape_type } => {
                // Shape validation could include checking for valid shape types
                match shape_type {
                    crate::widgets::blueprint::ShapeType::Custom { vertices } => {
                        if vertices.len() < 3 {
                            return Err(UiRegistryError::InvalidPropertyValue {
                                widget_type: "Shape".to_string(),
                                property: "vertices".to_string(),
                                reason: "Custom shape must have at least 3 vertices".to_string(),
                            });
                        }
                    },
                    _ => {}, // Built-in shapes are always valid
                }
            },
            WidgetType::Custom { component, properties } => {
                // Custom widget validation
                if !self.config.allow_custom_widgets && self.config.strict_validation {
                    return Err(UiRegistryError::UnknownWidgetType(component.clone()));
                }
                
                // If the custom widget is registered, validate its properties
                if let Some(info) = self.get_widget_type_info(component) {
                    for required_prop in &info.required_properties {
                        if !properties.contains_key(required_prop) {
                            return Err(UiRegistryError::MissingRequiredProperty {
                                widget_type: component.clone(),
                                property: required_prop.clone(),
                            });
                        }
                    }
                }
            },
        }

        Ok(())
    }

    /// Validate if a widget type can have children
    pub fn validate_widget_children(&self, widget_type: &WidgetType, has_children: bool) -> Result<(), UiRegistryError> {
        let type_name = self.extract_widget_type_name(widget_type);
        
        if let Some(info) = self.get_widget_type_info(&type_name) {
            if has_children && !info.can_have_children {
                return Err(UiRegistryError::InvalidWidgetStructure {
                    widget_type: type_name,
                    reason: "This widget type cannot have children".to_string(),
                });
            }
        }

        Ok(())
    }

    /// Validate nesting depth
    pub fn validate_nesting_depth(&self, depth: usize) -> Result<(), UiRegistryError> {
        if depth > self.config.max_nesting_depth {
            return Err(UiRegistryError::ExcessiveNesting {
                current_depth: depth,
                max_depth: self.config.max_nesting_depth,
            });
        }
        Ok(())
    }

    /// Validate action bindings against registered actions and state types
    pub fn validate_action_binding(&self, binding: &ActionBinding) -> Result<(), UiRegistryError> {
        // Validate the action exists
        if !self.is_action_registered(&binding.action) {
            if self.config.strict_validation && !self.config.allow_custom_actions {
                return Err(UiRegistryError::UnknownAction(binding.action.clone()));
            }
        }

        // Validate the event type
        self.validate_event_type(&binding.event)?;

        // If the action is registered, validate its parameters
        if let Some(action_info) = self.get_action_info(&binding.action) {
            self.validate_action_parameters(binding, action_info)?;
        }

        Ok(())
    }

    /// Validate event type for action bindings
    fn validate_event_type(&self, event: &str) -> Result<(), UiRegistryError> {
        let valid_events = [
            "click", "hover", "focus", "blur", "change", "submit",
            "key_press", "key_release", "mouse_enter", "mouse_leave",
            "drag_start", "drag_end", "resize", "scroll"
        ];

        if !valid_events.contains(&event) {
            return Err(UiRegistryError::ValidationError(
                format!("Unknown event type: '{}'. Valid events are: {}", event, valid_events.join(", "))
            ));
        }

        Ok(())
    }

    /// Validate action parameters against expected types
    fn validate_action_parameters(&self, binding: &ActionBinding, action_info: &ActionInfo) -> Result<(), UiRegistryError> {
        if let Some(ref params) = binding.params {
            // Check that all required parameters are provided
            for (param_name, param_type) in &action_info.parameter_types {
                if !params.contains_key(param_name) {
                    return Err(UiRegistryError::MissingRequiredProperty {
                        widget_type: format!("Action '{}'", binding.action),
                        property: param_name.clone(),
                    });
                }

                // Validate parameter type
                let param_value = &params[param_name];
                self.validate_parameter_type(param_value, param_type, param_name, &binding.action)?;
            }

            // Check for unexpected parameters
            for param_name in params.keys() {
                if !action_info.parameter_types.contains_key(param_name) {
                    return Err(UiRegistryError::ValidationError(
                        format!("Unexpected parameter '{}' for action '{}'", param_name, binding.action)
                    ));
                }
            }
        } else if !action_info.parameter_types.is_empty() {
            // Action expects parameters but none were provided
            return Err(UiRegistryError::ValidationError(
                format!("Action '{}' requires parameters: {:?}", binding.action, action_info.parameter_types.keys().collect::<Vec<_>>())
            ));
        }

        Ok(())
    }

    /// Validate a parameter value against its expected type
    fn validate_parameter_type(&self, value: &toml::Value, expected_type: &str, param_name: &str, action_name: &str) -> Result<(), UiRegistryError> {
        let is_valid = match expected_type.to_lowercase().as_str() {
            "string" => matches!(value, toml::Value::String(_)),
            "integer" | "int" | "i32" | "i64" => matches!(value, toml::Value::Integer(_)),
            "float" | "f32" | "f64" => matches!(value, toml::Value::Float(_)),
            "boolean" | "bool" => matches!(value, toml::Value::Boolean(_)),
            "array" => matches!(value, toml::Value::Array(_)),
            "table" | "object" => matches!(value, toml::Value::Table(_)),
            _ => {
                // Check if it's a registered state type
                if self.is_state_type_registered(expected_type) {
                    // For custom state types, we need more sophisticated validation
                    // For now, accept any value and let the runtime handle conversion
                    true
                } else {
                    return Err(UiRegistryError::UnknownStateType(expected_type.to_string()));
                }
            }
        };

        if !is_valid {
            return Err(UiRegistryError::InvalidPropertyValue {
                widget_type: format!("Action '{}'", action_name),
                property: param_name.to_string(),
                reason: format!("Expected type '{}', got {:?}", expected_type, value),
            });
        }

        Ok(())
    }

    /// Validate state type operations
    pub fn validate_state_operation(&self, state_type: &str, operation: &str) -> Result<(), UiRegistryError> {
        if let Some(state_info) = self.get_state_type_info(state_type) {
            if !state_info.valid_operations.contains(&operation.to_string()) {
                return Err(UiRegistryError::ValidationError(
                    format!("Invalid operation '{}' for state type '{}'. Valid operations: {:?}", 
                            operation, state_type, state_info.valid_operations)
                ));
            }
        } else if self.config.strict_validation {
            return Err(UiRegistryError::UnknownStateType(state_type.to_string()));
        }

        Ok(())
    }

    /// Get default value for a state type
    pub fn get_default_value_for_state_type(&self, state_type: &str) -> Option<toml::Value> {
        self.get_state_type_info(state_type)
            .and_then(|info| info.default_value.clone())
    }

    /// Register a custom state type with validation
    pub fn register_custom_state_type(&mut self, name: &str, type_id: &str, operations: Vec<String>, default_value: Option<toml::Value>) -> Result<(), UiRegistryError> {
        // Validate state type name
        if name.is_empty() {
            return Err(UiRegistryError::ValidationError("State type name cannot be empty".to_string()));
        }

        // Validate type ID
        if type_id.is_empty() {
            return Err(UiRegistryError::ValidationError("State type ID cannot be empty".to_string()));
        }

        // Validate operations
        if operations.is_empty() {
            return Err(UiRegistryError::ValidationError("State type must have at least one valid operation".to_string()));
        }

        let valid_base_operations = ["get", "set", "toggle", "increment", "decrement", "append", "clear", "reset"];
        for operation in &operations {
            if operation.is_empty() {
                return Err(UiRegistryError::ValidationError("Operation name cannot be empty".to_string()));
            }
            
            // Allow custom operations, but warn about unknown base operations
            if !valid_base_operations.contains(&operation.as_str()) && operation.starts_with(|c: char| c.is_ascii_lowercase()) {
                // This is likely a typo in a base operation
                let suggestions: Vec<&str> = valid_base_operations.iter()
                    .filter(|&op| op.starts_with(&operation[..1]))
                    .copied()
                    .collect();
                
                if !suggestions.is_empty() {
                    return Err(UiRegistryError::ValidationError(
                        format!("Unknown operation '{}'. Did you mean one of: {:?}", operation, suggestions)
                    ));
                }
            }
        }

        // Register the state type
        let state_info = StateTypeInfo {
            display_name: name.to_string(),
            type_id: type_id.to_string(),
            valid_operations: operations,
            default_value,
        };

        self.register_state_type(name, state_info);
        Ok(())
    }
}

/// Errors that can occur during registry validation
#[derive(Error, Debug)]
pub enum UiRegistryError {
    #[error("Unknown widget type: {0}")]
    UnknownWidgetType(String),
    #[error("Unknown action: {0}")]
    UnknownAction(String),
    #[error("Unknown state type: {0}")]
    UnknownStateType(String),
    #[error("Missing required property '{property}' for widget type '{widget_type}'")]
    MissingRequiredProperty { widget_type: String, property: String },
    #[error("Invalid property value for '{property}' in widget type '{widget_type}': {reason}")]
    InvalidPropertyValue { widget_type: String, property: String, reason: String },
    #[error("Invalid widget structure for '{widget_type}': {reason}")]
    InvalidWidgetStructure { widget_type: String, reason: String },
    #[error("Excessive nesting depth: {current_depth} exceeds maximum of {max_depth}")]
    ExcessiveNesting { current_depth: usize, max_depth: usize },
    #[error("Registry validation error: {0}")]
    ValidationError(String),
}

// ==================== End UiRegistry ====================

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
                        window_config.background_color = Some(crate::widgets::blueprint::ColorDef::Rgba {
                            r: r as u8,
                            g: g as u8,
                            b: b as u8,
                            a: a as f32,
                        });
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

// ==================== Phase 2: Unit Tests ====================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::blueprint::{ColorDef, FlexDirection};

    /// Test basic UiDefinition deserialization
    #[test]
    fn test_basic_ui_definition_parsing() {
        let toml_str = r##"
[root]
widget_type = { type = "Container", direction = "Column" }

[root.layout]
size = [800.0, 600.0]

[root.style] 
background_color = "#2D3748"

[root.behavior]
visible = true
"##;

        let ui_def: Result<UiDefinition, toml::de::Error> = toml::from_str(toml_str);
        assert!(ui_def.is_ok(), "Should parse basic UI definition: {:?}", ui_def.err());
        
        let ui_def = ui_def.unwrap();
        assert!(matches!(ui_def.root.widget_type, WidgetType::Container { .. }));
        assert_eq!(ui_def.root.layout.size, Some([800.0, 600.0].into()));
        assert!(ui_def.root.style.background_color.is_some());
    }

    /// Test hierarchical children parsing
    #[test]
    fn test_hierarchical_children_parsing() {
        let toml_str = r##"
[root]
widget_type = { type = "Container", direction = "Column" }

[[root.children]]
widget_type = { type = "Text", content = "Hello", editable = false }
id = "text1"

[[root.children]]
widget_type = { type = "Button", text = "Click me", action = "test_action" }
id = "button1"

[[root.children.children]]
widget_type = { type = "Text", content = "Nested", editable = false }
id = "nested_text"
"##;

        let ui_def: Result<UiDefinition, toml::de::Error> = toml::from_str(toml_str);
        assert!(ui_def.is_ok(), "Should parse hierarchical structure");
        
        let ui_def = ui_def.unwrap();
        assert_eq!(ui_def.root.children.len(), 2);
        
        // Check first child
        let first_child = &ui_def.root.children[0];
        assert_eq!(first_child.id, Some("text1".to_string()));
        assert!(matches!(first_child.widget_type, WidgetType::Text { .. }));
        
        // Check second child has nested children
        let second_child = &ui_def.root.children[1];
        assert_eq!(second_child.id, Some("button1".to_string()));
        assert_eq!(second_child.children.len(), 1);
        
        let nested_child = &second_child.children[0];
        assert_eq!(nested_child.id, Some("nested_text".to_string()));
    }

    /// Test style overrides and classes
    #[test] 
    fn test_style_overrides_and_classes() {
        let toml_str = r##"
[styles.primary]
background_color = "#3182CE"
text_color = "white"

[styles.large]
text_size = 24.0

[root]
widget_type = { type = "Button", text = "Styled Button", action = "test" }
classes = ["primary", "large"]

[root.style_overrides]
border_radius = 8.0
opacity = 0.9
"##;

        let ui_def: Result<UiDefinition, toml::de::Error> = toml::from_str(toml_str);
        assert!(ui_def.is_ok(), "Should parse style overrides and classes");
        
        let ui_def = ui_def.unwrap();
        
        // Check global styles
        assert!(ui_def.styles.is_some());
        let styles = ui_def.styles.as_ref().unwrap();
        assert!(styles.contains_key("primary"));
        assert!(styles.contains_key("large"));
        
        // Check style classes on widget
        assert_eq!(ui_def.root.classes, Some(vec!["primary".to_string(), "large".to_string()]));
        
        // Check style overrides
        assert!(ui_def.root.style_overrides.is_some());
        let overrides = ui_def.root.style_overrides.as_ref().unwrap();
        assert_eq!(overrides.border_radius, Some(8.0));
        assert_eq!(overrides.opacity, Some(0.9));
    }

    /// Test action bindings
    #[test]
    fn test_action_bindings() {
        let toml_str = r##"
[actions.navigate_home]
event = "click"
action = "navigate_home"

[actions.toggle_settings]
event = "hover"
action = "toggle_settings"
params = { target = "main_panel" }

[root]
widget_type = { type = "Container", direction = "Row" }

[root.bindings.click]
event = "click"
action = "navigate_home"

[root.bindings.hover]
event = "hover"
action = "toggle_settings"
"##;

        let ui_def: Result<UiDefinition, toml::de::Error> = toml::from_str(toml_str);
        assert!(ui_def.is_ok(), "Should parse action bindings");
        
        let ui_def = ui_def.unwrap();
        
        // Check global actions
        assert!(ui_def.actions.is_some());
        let actions = ui_def.actions.as_ref().unwrap();
        assert!(actions.contains_key("navigate_home"));
        assert!(actions.contains_key("toggle_settings"));
        
        // Check widget bindings
        assert!(ui_def.root.bindings.is_some());
        let bindings = ui_def.root.bindings.as_ref().unwrap();
        assert!(bindings.contains_key("click"));
        assert!(bindings.contains_key("hover"));
        
        let click_binding = &bindings["click"];
        assert_eq!(click_binding.action, "navigate_home");
        assert_eq!(click_binding.event, "click");
    }

    /// Test validation success cases
    #[test]
    fn test_validation_success() {
        let ui_def = create_valid_ui_definition();
        assert!(ui_def.validate().is_ok(), "Valid UI definition should pass validation");
    }

    /// Test validation error cases
    #[test]
    fn test_validation_errors() {
        // Test duplicate IDs
        let mut ui_def = create_valid_ui_definition();
        ui_def.root.children.push(WidgetNode {
            id: Some("test_button".to_string()), // Duplicate ID
            widget_type: WidgetType::Text { content: "Duplicate".to_string(), editable: false },
            layout: LayoutConfig::default(),
            style: StyleConfig::default(),
            behavior: BehaviorConfig::default(),
            classes: None,
            style_overrides: None,
            bindings: None,
            children: vec![],
        });
        
        let result = ui_def.validate();
        assert!(result.is_err(), "Should fail validation due to duplicate ID: {:?}", result);
        match result.unwrap_err() {
            UiDefinitionError::DuplicateId(_) => {}, // Expected
            other => panic!("Expected DuplicateId error, got: {:?}", other),
        }
        
        // Test invalid color format
        let mut ui_def = create_valid_ui_definition();
        ui_def.root.style.background_color = Some(ColorDef::Hex("invalid".to_string()));
        
        let result = ui_def.validate();
        assert!(result.is_err());
        match result.unwrap_err() {
            UiDefinitionError::Validation(_) => {}, // Expected
            other => panic!("Expected Validation error, got: {:?}", other),
        }
        
        // Test style classes without global styles
        let mut ui_def = create_valid_ui_definition();
        ui_def.root.classes = Some(vec!["unknown_class".to_string()]);
        
        let result = ui_def.validate();
        assert!(result.is_err());
        match result.unwrap_err() {
            UiDefinitionError::StyleClassesWithoutGlobalStyles => {}, // Expected
            other => panic!("Expected StyleClassesWithoutGlobalStyles error, got: {:?}", other),
        }
        
        // Test unknown style class (with global styles defined)
        let mut ui_def = create_valid_ui_definition();
        ui_def.styles = Some({
            let mut styles = HashMap::new();
            styles.insert("valid_class".to_string(), StyleOverrides { 
                background_color: None, border_color: None, border_width: None, 
                border_radius: None, text_color: None, text_size: None, opacity: None 
            });
            styles
        });
        ui_def.root.classes = Some(vec!["unknown_class".to_string()]);
        
        let result = ui_def.validate();
        assert!(result.is_err());
        match result.unwrap_err() {
            UiDefinitionError::UnknownStyleClass(_) => {}, // Expected
            other => panic!("Expected UnknownStyleClass error, got: {:?}", other),
        }
    }

    /// Test conversion to widget collection
    #[test]
    fn test_to_widget_collection() {
        let ui_def = create_valid_ui_definition();
        let collection = ui_def.to_widget_collection();
        
        assert!(!collection.widgets.is_empty());
        assert!(collection.root.is_some());
        
        // Check that the root widget exists in the collection
        let root_id = collection.root.as_ref().unwrap();
        assert!(collection.widgets.contains_key(root_id));
        
        // Check that child widgets exist
        let root_widget = collection.widgets.get(root_id).unwrap();
        for child_id in &root_widget.children {
            assert!(collection.widgets.contains_key(child_id), "Child widget {} should exist", child_id);
        }
    }

    /// Test style override application
    #[test]
    fn test_style_override_application() {
        let mut ui_def = UiDefinition {
            window: None,
            root: WidgetNode {
                id: Some("root".to_string()),
                widget_type: WidgetType::Container { direction: FlexDirection::Column },
                layout: LayoutConfig::default(),
                style: StyleConfig::default(), // Start with no styles
                behavior: BehaviorConfig::default(),
                classes: None,
                style_overrides: None,
                bindings: None,
                children: vec![],
            },
            styles: None,
            actions: None,
        };
        
        // Add global styles
        let mut styles = HashMap::new();
        styles.insert("primary".to_string(), StyleOverrides {
            background_color: Some(ColorDef::Hex("#FF0000".to_string())),
            text_size: Some(16.0),
            border_color: None,
            border_width: None,
            border_radius: None,
            text_color: None,
            opacity: None,
        });
        ui_def.styles = Some(styles);
        
        // Apply style class and overrides to root
        ui_def.root.classes = Some(vec!["primary".to_string()]);
        ui_def.root.style_overrides = Some(StyleOverrides {
            text_size: Some(24.0), // Override the class style
            border_radius: Some(4.0),
            background_color: None,
            border_color: None,
            border_width: None,
            text_color: None,
            opacity: None,
        });
        
        let collection = ui_def.to_widget_collection();
        let root_widget = collection.widgets.values().next().unwrap();
        
        // Check that style overrides were applied correctly
        assert_eq!(root_widget.style.background_color, Some(ColorDef::Hex("#FF0000".to_string())));
        assert_eq!(root_widget.style.text_size, Some(24.0)); // Override should win
        assert_eq!(root_widget.style.border_radius, Some(4.0));
    }

    /// Test color validation
    #[test]
    fn test_color_validation() {
        let ui_def = UiDefinition {
            window: None,
            root: WidgetNode {
                id: Some("test".to_string()),
                widget_type: WidgetType::Container { direction: FlexDirection::Column },
                layout: LayoutConfig::default(),
                style: StyleConfig {
                    background_color: Some(ColorDef::Hex("invalid_color".to_string())),
                    ..StyleConfig::default()
                },
                behavior: BehaviorConfig::default(),
                classes: None,
                style_overrides: None,
                bindings: None,
                children: vec![],
            },
            styles: None,
            actions: None,
        };
        
        let result = ui_def.validate();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), UiDefinitionError::Validation(_)));
    }

    // ==================== Phase 2: UiRegistry Tests ====================

    /// Test UiRegistry basic functionality
    #[test]
    fn test_ui_registry_creation() {
        let registry = UiRegistry::new();
        
        // Check that built-in widget types are registered
        assert!(registry.is_widget_type_registered("Container"));
        assert!(registry.is_widget_type_registered("Button"));
        assert!(registry.is_widget_type_registered("Text"));
        assert!(registry.is_widget_type_registered("Shape"));
        
        // Check that built-in state types are registered
        assert!(registry.is_state_type_registered("String"));
        assert!(registry.is_state_type_registered("Boolean"));
        assert!(registry.is_state_type_registered("Integer"));
        
        // Check that built-in actions are registered
        assert!(registry.is_action_registered("navigate_home"));
        assert!(registry.is_action_registered("open_settings"));
        assert!(registry.is_action_registered("toggle_visibility"));
    }

    /// Test widget type validation
    #[test]
    fn test_widget_type_validation() {
        let registry = UiRegistry::new();
        
        // Test valid widget types
        let container = WidgetType::Container { direction: FlexDirection::Column };
        assert!(registry.validate_widget_type(&container).is_ok());
        
        let button = WidgetType::Button { text: "Click me".to_string(), action: Some("navigate_home".to_string()) };
        assert!(registry.validate_widget_type(&button).is_ok());
        
        let text = WidgetType::Text { content: "Hello".to_string(), editable: false };
        assert!(registry.validate_widget_type(&text).is_ok());
        
        // Test invalid widget types
        let empty_button = WidgetType::Button { text: "".to_string(), action: None };
        assert!(registry.validate_widget_type(&empty_button).is_err());
        
        let long_text = WidgetType::Text { content: "x".repeat(10001), editable: false };
        assert!(registry.validate_widget_type(&long_text).is_err());
    }

    /// Test widget children validation
    #[test]
    fn test_widget_children_validation() {
        let registry = UiRegistry::new();
        
        // Container can have children
        let container = WidgetType::Container { direction: FlexDirection::Column };
        assert!(registry.validate_widget_children(&container, true).is_ok());
        assert!(registry.validate_widget_children(&container, false).is_ok());
        
        // Button cannot have children
        let button = WidgetType::Button { text: "Click".to_string(), action: None };
        assert!(registry.validate_widget_children(&button, false).is_ok());
        assert!(registry.validate_widget_children(&button, true).is_err());
    }

    /// Test action binding validation
    #[test]
    fn test_action_binding_validation() {
        let registry = UiRegistry::new();
        
        // Valid action binding
        let valid_binding = ActionBinding {
            event: "click".to_string(),
            action: "navigate_home".to_string(),
            params: None,
        };
        assert!(registry.validate_action_binding(&valid_binding).is_ok());
        
        // Invalid event type
        let invalid_event = ActionBinding {
            event: "invalid_event".to_string(),
            action: "navigate_home".to_string(),
            params: None,
        };
        assert!(registry.validate_action_binding(&invalid_event).is_err());
        
        // Unknown action (with strict validation)
        let unknown_action = ActionBinding {
            event: "click".to_string(),
            action: "unknown_action".to_string(),
            params: None,
        };
        assert!(registry.validate_action_binding(&unknown_action).is_err());
    }

    /// Test action parameters validation
    #[test]
    fn test_action_parameters_validation() {
        let registry = UiRegistry::new();
        
        // Action with required parameters
        let mut params = HashMap::new();
        params.insert("target".to_string(), toml::Value::String("main_panel".to_string()));
        
        let binding_with_params = ActionBinding {
            event: "click".to_string(),
            action: "toggle_visibility".to_string(),
            params: Some(params),
        };
        assert!(registry.validate_action_binding(&binding_with_params).is_ok());
        
        // Missing required parameter
        let binding_missing_param = ActionBinding {
            event: "click".to_string(),
            action: "toggle_visibility".to_string(),
            params: None,
        };
        assert!(registry.validate_action_binding(&binding_missing_param).is_err());
    }

    /// Test custom state type registration
    #[test]
    fn test_custom_state_type_registration() {
        let mut registry = UiRegistry::new();
        
        // Register a valid custom state type
        let result = registry.register_custom_state_type(
            "CustomCounter",
            "u32",
            vec!["get".to_string(), "set".to_string(), "increment".to_string()],
            Some(toml::Value::Integer(0))
        );
        assert!(result.is_ok());
        assert!(registry.is_state_type_registered("CustomCounter"));
        
        // Try to register invalid state type
        let invalid_result = registry.register_custom_state_type(
            "",
            "invalid",
            vec![],
            None
        );
        assert!(invalid_result.is_err());
    }

    /// Test nesting depth validation
    #[test]
    fn test_nesting_depth_validation() {
        let registry = UiRegistry::new();
        
        // Valid nesting depth
        assert!(registry.validate_nesting_depth(5).is_ok());
        assert!(registry.validate_nesting_depth(50).is_ok());
        
        // Excessive nesting depth
        assert!(registry.validate_nesting_depth(51).is_err());
        assert!(registry.validate_nesting_depth(100).is_err());
    }

    /// Test UI definition validation with registry
    #[test]
    fn test_ui_definition_registry_validation() {
        let registry = UiRegistry::new();
        let ui_def = create_valid_ui_definition();
        
        // Valid UI definition should pass registry validation
        let result = ui_def.validate_with_registry(&registry);
        assert!(result.is_ok(), "Registry validation failed: {:?}", result.err());
        
        // UI definition with invalid widget type should fail
        let mut invalid_ui_def = create_valid_ui_definition();
        invalid_ui_def.root.widget_type = WidgetType::Button { 
            text: "".to_string(), // Empty text should fail validation
            action: None 
        };
        assert!(invalid_ui_def.validate_with_registry(&registry).is_err());
    }

    /// Test registry configuration
    #[test]
    fn test_registry_configuration() {
        let config = crate::assets::UiRegistryConfig {
            strict_validation: false,
            allow_custom_widgets: true,
            allow_custom_actions: true,
            max_nesting_depth: 100,
        };
        
        let registry = UiRegistry::with_config(config);
        
        // With relaxed validation, unknown actions should be allowed
        let unknown_action = ActionBinding {
            event: "click".to_string(),
            action: "custom_action".to_string(),
            params: None,
        };
        assert!(registry.validate_action_binding(&unknown_action).is_ok());
        
        // Custom widget should be allowed
        let custom_widget = WidgetType::Custom {
            component: "CustomWidget".to_string(),
            properties: HashMap::new(),
        };
        assert!(registry.validate_widget_type(&custom_widget).is_ok());
    }

    /// Helper function to create a valid UI definition for testing
    fn create_valid_ui_definition() -> UiDefinition {
        UiDefinition {
            window: Some(WindowConfig {
                size: [800.0, 600.0],
                background_color: Some(ColorDef::Hex("#2D3748".to_string())),
            }),
            root: WidgetNode {
                id: Some("root_container".to_string()),
                widget_type: WidgetType::Container { direction: FlexDirection::Column },
                layout: LayoutConfig {
                    size: Some([800.0, 600.0].into()),
                    ..LayoutConfig::default()
                },
                style: StyleConfig {
                    background_color: Some(ColorDef::Hex("#FFFFFF".to_string())),
                    ..StyleConfig::default()
                },
                behavior: BehaviorConfig {
                    visible: Some(true),
                    ..BehaviorConfig::default()
                },
                classes: None,
                style_overrides: None,
                bindings: None,
                children: vec![
                    WidgetNode {
                        id: Some("test_button".to_string()),
                        widget_type: WidgetType::Button {
                            text: "Test Button".to_string(),
                            action: Some("navigate_home".to_string()), // Use registered action
                        },
                        layout: LayoutConfig::default(),
                        style: StyleConfig::default(),
                        behavior: BehaviorConfig::default(),
                        classes: None,
                        style_overrides: None,
                        bindings: None,
                        children: vec![],
                    },
                ],
            },
            styles: None,
            actions: None,
        }
    }
}