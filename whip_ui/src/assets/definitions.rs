use bevy_asset::Asset;
use bevy_reflect::TypePath;
use std::collections::{HashMap, HashSet};
use thiserror::Error;
use crate::widgets::blueprint::{WidgetBlueprint, WidgetCollection, WidgetType, LayoutConfig, StyleConfig, BehaviorConfig};

use super::{WindowConfig, UiRegistry};

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