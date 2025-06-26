use bevy_asset::{AssetLoader, LoadContext};
use bevy_ecs::prelude::*;
use std::collections::{HashMap, HashSet};
use thiserror::Error;
use crate::widgets::blueprint::WidgetType;

use super::{definitions::{UiDefinition, UiDefinitionError, WidgetNode, StyleOverrides}, registry::{UiRegistry, UiRegistryError}};

/// Asset loader for hierarchical UI definitions
#[derive(Default)]
pub struct UiDefinitionLoader;

/// Errors that can occur during UI definition loading
#[derive(Error, Debug)]
pub enum UiDefinitionLoaderError {
    #[error("Failed to read UI definition file: {0}")]
    Io(#[from] std::io::Error),
    #[error("Failed to parse JSON: {0}")]
    JsonParse(#[from] serde_json::Error),
    #[error("Validation failed: {0}")]
    Validation(#[from] UiDefinitionError),
    #[error("Registry validation failed: {0}")]
    RegistryValidation(#[from] UiRegistryError),
    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),
    #[error("Widget type validation failed: {0}")]
    WidgetTypeValidation(String),
    #[error("Action validation failed: {0}")]
    ActionValidation(String),
    #[error("Style validation failed: {0}")]
    StyleValidation(String),
}


/// Implementation of the UiDefinitionLoader
impl AssetLoader for UiDefinitionLoader {
    type Asset = UiDefinition;
    type Settings = ();
    type Error = UiDefinitionLoaderError;

    async fn load(
        &self,
        reader: &mut dyn bevy_asset::io::Reader,
        _settings: &(),
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let content_str = std::str::from_utf8(&bytes)
            .map_err(|e| UiDefinitionLoaderError::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, e)))?;

        // Parse the JSON directly into UiDefinition using serde
        let ui_definition: UiDefinition = serde_json::from_str(content_str)
            .map_err(|e| {
                bevy_log::error!("Failed to parse UI definition JSON: {}", e);
                UiDefinitionLoaderError::JsonParse(e)
            })?;

        bevy_log::info!("Successfully loaded UI definition with {} global styles and {} global actions", 
                        ui_definition.styles.as_ref().map(|s| s.len()).unwrap_or(0),
                        ui_definition.actions.as_ref().map(|a| a.len()).unwrap_or(0));

        // Validate the loaded UI definition
        ui_definition.validate()
            .map_err(|e| {
                bevy_log::error!("UI definition validation failed: {}", e);
                UiDefinitionLoaderError::Validation(e)
            })?;

        bevy_log::info!("UI definition validation passed");

        Ok(ui_definition)
    }

    fn extensions(&self) -> &[&str] {
        &["json"]
    }
}

impl UiDefinitionLoader {
    /// Load and validate a UI definition with registry validation
    pub async fn load_with_registry(
        &self,
        reader: &mut dyn bevy_asset::io::Reader,
        registry: &UiRegistry,
        _settings: &(),
        _load_context: &mut LoadContext<'_>,
    ) -> Result<UiDefinition, UiDefinitionLoaderError> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let content_str = std::str::from_utf8(&bytes)
            .map_err(|e| UiDefinitionLoaderError::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, e)))?;

        // Parse the JSON directly
        let ui_definition: UiDefinition = serde_json::from_str(content_str)
            .map_err(|e| {
                bevy_log::error!("Failed to parse UI definition JSON: {}", e);
                UiDefinitionLoaderError::JsonParse(e)
            })?;

        bevy_log::info!("Successfully loaded UI definition with {} global styles and {} global actions", 
                        ui_definition.styles.as_ref().map(|s| s.len()).unwrap_or(0),
                        ui_definition.actions.as_ref().map(|a| a.len()).unwrap_or(0));

        // Validate with registry
        ui_definition.validate_with_registry(registry)
            .map_err(|e| {
                bevy_log::error!("UI definition registry validation failed: {}", e);
                UiDefinitionLoaderError::Validation(e)
            })?;

        bevy_log::info!("UI definition validation with registry passed");

        Ok(ui_definition)
    }







    /// Comprehensive validation of the loaded UI definition with registry
    pub fn validate_with_comprehensive_checks(&self, ui_definition: &UiDefinition, registry: &UiRegistry) -> Result<(), UiDefinitionLoaderError> {
        bevy_log::info!("Starting comprehensive validation with registry checks");

        // Step 1: Basic structure validation
        self.validate_basic_structure(ui_definition)?;

        // Step 2: Widget type validation with registry
        self.validate_widget_types_with_registry(&ui_definition.root, registry, 0)?;

        // Step 3: Style classes validation
        self.validate_style_classes(ui_definition)?;

        // Step 4: Action validation with registry
        self.validate_actions_with_registry(ui_definition, registry)?;

        // Step 5: Cross-references validation (style classes, action bindings)
        self.validate_cross_references(ui_definition)?;

        // Step 6: Semantic validation (layout consistency, hierarchy depth)
        self.validate_semantic_constraints(&ui_definition.root, registry, 0)?;

        bevy_log::info!("Comprehensive validation completed successfully");
        Ok(())
    }

    /// Validate basic UI definition structure
    fn validate_basic_structure(&self, ui_definition: &UiDefinition) -> Result<(), UiDefinitionLoaderError> {
        // Check window configuration validity
        if let Some(ref window) = ui_definition.window {
            if window.size[0] <= 0.0 || window.size[1] <= 0.0 {
                return Err(UiDefinitionLoaderError::InvalidConfiguration(
                    "Window size must be positive".to_string()
                ));
            }
            if window.size[0] > 10000.0 || window.size[1] > 10000.0 {
                return Err(UiDefinitionLoaderError::InvalidConfiguration(
                    "Window size is unreasonably large (max 10000x10000)".to_string()
                ));
            }
        }

        // Validate root widget exists and has required properties
        if ui_definition.root.id.is_none() {
            bevy_log::warn!("Root widget has no ID, this may cause issues with debugging");
        }

        Ok(())
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


// Additional validation methods for UiDefinitionLoader
impl UiDefinitionLoader {
    /// Validate widget types against registry
    fn validate_widget_types_with_registry(&self, node: &WidgetNode, registry: &UiRegistry, depth: usize) -> Result<(), UiDefinitionLoaderError> {
        // Check nesting depth
        if depth > registry.config.max_nesting_depth {
            return Err(UiDefinitionLoaderError::WidgetTypeValidation(
                format!("Nesting depth {} exceeds maximum of {}", depth, registry.config.max_nesting_depth)
            ));
        }

        // Validate widget type with registry
        let widget_type_name = match &node.widget_type {
            WidgetType::Container { .. } => "Container",
            WidgetType::Button { .. } => "Button",
            WidgetType::Text { .. } => "Text", 
            WidgetType::Shape { .. } => "Shape",
            WidgetType::Custom { component, .. } => component,
        };

        if let Err(e) = registry.validate_widget_type(&node.widget_type) {
            if registry.config.strict_validation && !registry.config.allow_custom_widgets {
                return Err(UiDefinitionLoaderError::RegistryValidation(e));
            } else {
                bevy_log::warn!("Widget type validation warning for '{}': {}", widget_type_name, e);
            }
        }

        // Validate widget-specific constraints
        match &node.widget_type {
            WidgetType::Button { text, .. } => {
                if let Some(text) = text {
                    if text.is_empty() {
                        return Err(UiDefinitionLoaderError::WidgetTypeValidation(
                            "Button text cannot be empty".to_string()
                        ));
                    }
                    if text.len() > 1000 {
                        return Err(UiDefinitionLoaderError::WidgetTypeValidation(
                            "Button text is too long (max 1000 characters)".to_string()
                        ));
                    }
                }
                // Note: Button actions are now handled via action bindings
            }
            WidgetType::Text { content, editable: _ } => {
                if content.len() > 50000 {
                    return Err(UiDefinitionLoaderError::WidgetTypeValidation(
                        "Text content is too long (max 50000 characters)".to_string()
                    ));
                }
            }
            WidgetType::Container { direction: _ } => {
                // Containers should have children or be marked as empty
                if node.children.is_empty() {
                    bevy_log::debug!("Container widget has no children: {:?}", node.id);
                }
            }
            WidgetType::Shape { shape_type } => {
                // Validate shape type specific constraints
                if let crate::widgets::blueprint::ShapeType::Custom { vertices } = shape_type {
                    if vertices.len() < 3 {
                        return Err(UiDefinitionLoaderError::WidgetTypeValidation(
                            "Custom shape must have at least 3 vertices".to_string()
                        ));
                    }
                    if vertices.len() > 1000 {
                        return Err(UiDefinitionLoaderError::WidgetTypeValidation(
                            "Custom shape has too many vertices (max 1000)".to_string()
                        ));
                    }
                }
            }
            WidgetType::Custom { component, properties } => {
                if component.is_empty() {
                    return Err(UiDefinitionLoaderError::WidgetTypeValidation(
                        "Custom widget component name cannot be empty".to_string()
                    ));
                }
                if properties.len() > 100 {
                    return Err(UiDefinitionLoaderError::WidgetTypeValidation(
                        "Custom widget has too many properties (max 100)".to_string()
                    ));
                }
            }
        }

        // Recursively validate children
        for child in &node.children {
            self.validate_widget_types_with_registry(child, registry, depth + 1)?;
        }

        Ok(())
    }

    /// Validate style classes reference existing global styles
    fn validate_style_classes(&self, ui_definition: &UiDefinition) -> Result<(), UiDefinitionLoaderError> {
        if let Some(ref global_styles) = ui_definition.styles {
            // Validate that all style class definitions are valid
            for (class_name, style_override) in global_styles {
                if class_name.is_empty() {
                    return Err(UiDefinitionLoaderError::StyleValidation(
                        "Style class name cannot be empty".to_string()
                    ));
                }
                
                // Validate style override contents
                self.validate_style_override(style_override, class_name)?;
            }
        }

        // Validate that all referenced style classes exist
        self.validate_style_class_references(&ui_definition.root, ui_definition.styles.as_ref())?;

        Ok(())
    }

    /// Validate style override configuration
    fn validate_style_override(&self, style_override: &StyleOverrides, class_name: &str) -> Result<(), UiDefinitionLoaderError> {
        // Validate color definitions if present
        if let Some(ref color_def) = style_override.background_color {
            self.validate_color_definition(color_def, &format!("Style class '{}' background_color", class_name))?;
        }
        if let Some(ref color_def) = style_override.text_color {
            self.validate_color_definition(color_def, &format!("Style class '{}' text_color", class_name))?;
        }
        if let Some(ref color_def) = style_override.border_color {
            self.validate_color_definition(color_def, &format!("Style class '{}' border_color", class_name))?;
        }

        // Validate numeric values
        if let Some(opacity) = style_override.opacity {
            if !(0.0..=1.0).contains(&opacity) {
                return Err(UiDefinitionLoaderError::StyleValidation(
                    format!("Style class '{}' opacity must be between 0.0 and 1.0", class_name)
                ));
            }
        }
        if let Some(border_width) = style_override.border_width {
            if border_width < 0.0 || border_width > 100.0 {
                return Err(UiDefinitionLoaderError::StyleValidation(
                    format!("Style class '{}' border_width must be between 0.0 and 100.0", class_name)
                ));
            }
        }
        if let Some(text_size) = style_override.text_size {
            if text_size <= 0.0 || text_size > 200.0 {
                return Err(UiDefinitionLoaderError::StyleValidation(
                    format!("Style class '{}' text_size must be between 0.0 and 200.0", class_name)
                ));
            }
        }

        Ok(())
    }

    /// Validate color definition format
    fn validate_color_definition(&self, color_def: &crate::widgets::blueprint::ColorDef, context: &str) -> Result<(), UiDefinitionLoaderError> {
        match color_def {
            crate::widgets::blueprint::ColorDef::Hex(hex) => {
                let hex = hex.trim_start_matches('#');
                if hex.len() != 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
                    return Err(UiDefinitionLoaderError::StyleValidation(
                        format!("{}: Invalid hex color format '{}' (expected #RRGGBB)", context, hex)
                    ));
                }
            }
            crate::widgets::blueprint::ColorDef::Rgb { r: _, g: _, b: _ } => {
                // RGB values are u8 (0-255) by type definition, no validation needed
            }
            crate::widgets::blueprint::ColorDef::Rgba { r: _, g: _, b: _, a } => {
                // RGB values are u8 (0-255) by type definition, only validate alpha
                if !(*a >= 0.0 && *a <= 1.0) {
                    return Err(UiDefinitionLoaderError::StyleValidation(
                        format!("{}: Alpha value must be between 0.0 and 1.0", context)
                    ));
                }
            }
            crate::widgets::blueprint::ColorDef::Named(name) => {
                let valid_names = ["red", "green", "blue", "black", "white", "gray", "grey", "yellow", "cyan", "magenta", "orange"];
                if !valid_names.contains(&name.to_lowercase().as_str()) {
                    bevy_log::warn!("{}: Unknown named color '{}', will use white as fallback", context, name);
                }
            }
        }
        Ok(())
    }

    /// Validate that referenced style classes exist in global styles
    fn validate_style_class_references(&self, node: &WidgetNode, global_styles: Option<&HashMap<String, StyleOverrides>>) -> Result<(), UiDefinitionLoaderError> {
        if let Some(ref classes) = node.classes {
            for class_name in classes {
                if let Some(global_styles) = global_styles {
                    if !global_styles.contains_key(class_name) {
                        return Err(UiDefinitionLoaderError::StyleValidation(
                            format!("Referenced style class '{}' does not exist in global styles", class_name)
                        ));
                    }
                } else {
                    return Err(UiDefinitionLoaderError::StyleValidation(
                        format!("Widget references style class '{}' but no global styles are defined", class_name)
                    ));
                }
            }
        }

        // Recursively validate children
        for child in &node.children {
            self.validate_style_class_references(child, global_styles)?;
        }

        Ok(())
    }

    /// Validate actions with registry
    fn validate_actions_with_registry(&self, ui_definition: &UiDefinition, registry: &UiRegistry) -> Result<(), UiDefinitionLoaderError> {
        // Validate global actions
        if let Some(ref global_actions) = ui_definition.actions {
            for (action_name, action_binding) in global_actions {
                if action_name.is_empty() {
                    return Err(UiDefinitionLoaderError::ActionValidation(
                        "Global action name cannot be empty".to_string()
                    ));
                }

                // Validate action binding with registry
                if let Err(e) = registry.validate_action_binding(action_binding) {
                    if registry.config.strict_validation && !registry.config.allow_custom_actions {
                        return Err(UiDefinitionLoaderError::RegistryValidation(e));
                    } else {
                        bevy_log::warn!("Action validation warning for '{}': {}", action_name, e);
                    }
                }
            }
        }

        // Validate widget-level action bindings
        self.validate_widget_action_bindings(&ui_definition.root, registry)?;

        Ok(())
    }

    /// Validate action bindings at widget level
    fn validate_widget_action_bindings(&self, node: &WidgetNode, registry: &UiRegistry) -> Result<(), UiDefinitionLoaderError> {
        if let Some(ref bindings) = node.bindings {
            for (event_name, action_binding) in bindings {
                if event_name.is_empty() {
                    return Err(UiDefinitionLoaderError::ActionValidation(
                        "Event name in widget binding cannot be empty".to_string()
                    ));
                }

                // Validate event type
                if let Err(e) = registry.validate_event_type(event_name) {
                    bevy_log::warn!("Event type validation warning: {}", e);
                }

                // Validate action binding
                if let Err(e) = registry.validate_action_binding(action_binding) {
                    if registry.config.strict_validation && !registry.config.allow_custom_actions {
                        return Err(UiDefinitionLoaderError::RegistryValidation(e));
                    } else {
                        bevy_log::warn!("Widget action binding validation warning: {}", e);
                    }
                }
            }
        }

        // Recursively validate children
        for child in &node.children {
            self.validate_widget_action_bindings(child, registry)?;
        }

        Ok(())
    }

    /// Validate cross-references between different parts of the definition
    fn validate_cross_references(&self, ui_definition: &UiDefinition) -> Result<(), UiDefinitionLoaderError> {
        // Collect all action names (from global actions and widget button actions)
        let mut available_actions = HashSet::new();
        
        if let Some(ref global_actions) = ui_definition.actions {
            for action_name in global_actions.keys() {
                available_actions.insert(action_name.clone());
            }
        }

        // Validate action references in widgets
        self.validate_action_references(&ui_definition.root, &available_actions)?;

        Ok(())
    }

    /// Validate that referenced actions exist
    fn validate_action_references(&self, node: &WidgetNode, available_actions: &HashSet<String>) -> Result<(), UiDefinitionLoaderError> {
        // Check button actions
        // Note: Button templates no longer have action fields - actions are handled via bindings

        // Check action bindings
        if let Some(ref bindings) = node.bindings {
            for (_, action_binding) in bindings {
                if !available_actions.contains(&action_binding.action) {
                    bevy_log::warn!("Widget binding references undefined action '{}' - this may be a built-in action", action_binding.action);
                }
            }
        }

        // Recursively validate children
        for child in &node.children {
            self.validate_action_references(child, available_actions)?;
        }

        Ok(())
    }

    /// Validate semantic constraints and layout consistency
    fn validate_semantic_constraints(&self, node: &WidgetNode, registry: &UiRegistry, depth: usize) -> Result<(), UiDefinitionLoaderError> {
        // Validate layout constraints
        if let Some(size) = node.layout.size {
            if size.x < 0.0 || size.y < 0.0 {
                return Err(UiDefinitionLoaderError::WidgetTypeValidation(
                    "Widget size cannot be negative".to_string()
                ));
            }
        }

        // Validate flex properties
        if let Some(flex_grow) = node.layout.flex_grow {
            if flex_grow < 0.0 {
                return Err(UiDefinitionLoaderError::WidgetTypeValidation(
                    "Flex grow cannot be negative".to_string()
                ));
            }
        }
        if let Some(flex_shrink) = node.layout.flex_shrink {
            if flex_shrink < 0.0 {
                return Err(UiDefinitionLoaderError::WidgetTypeValidation(
                    "Flex shrink cannot be negative".to_string()
                ));
            }
        }

        // Validate widget type specific constraints
        match &node.widget_type {
            WidgetType::Container { .. } => {
                // Containers should have reasonable child limits
                if node.children.len() > 1000 {
                    return Err(UiDefinitionLoaderError::WidgetTypeValidation(
                        "Container has too many children (max 1000)".to_string()
                    ));
                }
            }
            WidgetType::Text { editable: true, .. } => {
                // Editable text should have reasonable constraints
                if node.children.len() > 0 {
                    bevy_log::warn!("Editable text widget has children, which may cause interaction issues");
                }
            }
            _ => {}
        }

        // Check for potential ID conflicts
        if let Some(ref id) = node.id {
            self.validate_id_format(id)?;
        }

        // Recursively validate children
        for child in &node.children {
            self.validate_semantic_constraints(child, registry, depth + 1)?;
        }

        Ok(())
    }

    /// Validate ID format and conventions
    fn validate_id_format(&self, id: &str) -> Result<(), UiDefinitionLoaderError> {
        if id.is_empty() {
            return Err(UiDefinitionLoaderError::WidgetTypeValidation(
                "Widget ID cannot be empty".to_string()
            ));
        }
        if id.len() > 100 {
            return Err(UiDefinitionLoaderError::WidgetTypeValidation(
                "Widget ID is too long (max 100 characters)".to_string()
            ));
        }
        if !id.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
            return Err(UiDefinitionLoaderError::WidgetTypeValidation(
                "Widget ID can only contain alphanumeric characters, underscores, and hyphens".to_string()
            ));
        }
        if id.starts_with(char::is_numeric) {
            return Err(UiDefinitionLoaderError::WidgetTypeValidation(
                "Widget ID cannot start with a number".to_string()
            ));
        }
        Ok(())
    }

    /// Enhanced error logging for detailed debugging and diagnostics
    pub fn log_validation_summary(&self, ui_definition: &UiDefinition, registry: &UiRegistry) {
        bevy_log::info!("=== UI Definition Validation Summary ===");
        
        // Log basic structure info
        if let Some(ref window) = ui_definition.window {
            bevy_log::info!("Window: {}x{}", window.size[0], window.size[1]);
            if let Some(ref bg_color) = window.background_color {
                bevy_log::info!("Background: {:?}", bg_color);
            }
        } else {
            bevy_log::info!("Window: Using default configuration");
        }

        // Log global styles count
        let styles_count = ui_definition.styles.as_ref().map(|s| s.len()).unwrap_or(0);
        bevy_log::info!("Global styles: {} defined", styles_count);
        
        // Log global actions count
        let actions_count = ui_definition.actions.as_ref().map(|a| a.len()).unwrap_or(0);
        bevy_log::info!("Global actions: {} defined", actions_count);

        // Log widget hierarchy summary
        self.log_widget_hierarchy_summary(&ui_definition.root, 0);
        
        // Log registry configuration
        bevy_log::info!("Registry config: strict={}, custom_widgets={}, custom_actions={}, max_depth={}", 
                        registry.config.strict_validation,
                        registry.config.allow_custom_widgets,
                        registry.config.allow_custom_actions,
                        registry.config.max_nesting_depth);
        
        bevy_log::info!("=== End Validation Summary ===");
    }

    /// Log widget hierarchy structure for debugging
    fn log_widget_hierarchy_summary(&self, node: &WidgetNode, depth: usize) {
        let indent = "  ".repeat(depth);
        let widget_type_name = match &node.widget_type {
            WidgetType::Container { direction } => format!("Container({:?})", direction),
            WidgetType::Button { text, .. } => format!("Button(text={:?})", text),
            WidgetType::Text { content, editable } => format!("Text('{}', editable={})", 
                if content.len() > 20 { format!("{}...", &content[..20]) } else { content.clone() }, editable),
            WidgetType::Shape { shape_type } => format!("Shape({:?})", shape_type),
            WidgetType::Custom { component, properties } => format!("Custom('{}', {} props)", component, properties.len()),
        };
        
        let id_info = node.id.as_ref().map(|id| format!("#{}", id)).unwrap_or_else(|| "<no-id>".to_string());
        let classes_info = node.classes.as_ref()
            .map(|classes| format!(" classes=[{}]", classes.join(", ")))
            .unwrap_or_default();
        let bindings_count = node.bindings.as_ref().map(|b| b.len()).unwrap_or(0);
        let bindings_info = if bindings_count > 0 { format!(" bindings={}", bindings_count) } else { String::new() };

        bevy_log::info!("{}├─ {} {}{}{} ({} children)", 
                       indent, widget_type_name, id_info, classes_info, bindings_info, node.children.len());

        for child in &node.children {
            self.log_widget_hierarchy_summary(child, depth + 1);
        }
    }

    /// Log detailed validation errors with context
    pub fn log_validation_error_with_context(&self, error: &UiDefinitionLoaderError, ui_definition: &UiDefinition, context: &str) {
        bevy_log::error!("=== Validation Error Details ===");
        bevy_log::error!("Context: {}", context);
        bevy_log::error!("Error: {}", error);
        
        match error {
            UiDefinitionLoaderError::WidgetTypeValidation(msg) => {
                bevy_log::error!("Widget validation failed: {}", msg);
                self.log_widget_validation_context(ui_definition);
            }
            UiDefinitionLoaderError::StyleValidation(msg) => {
                bevy_log::error!("Style validation failed: {}", msg);
                self.log_style_validation_context(ui_definition);
            }
            UiDefinitionLoaderError::ActionValidation(msg) => {
                bevy_log::error!("Action validation failed: {}", msg);
                self.log_action_validation_context(ui_definition);
            }
            UiDefinitionLoaderError::RegistryValidation(err) => {
                bevy_log::error!("Registry validation failed: {}", err);
                // Registry errors already have good context
            }
            UiDefinitionLoaderError::InvalidConfiguration(msg) => {
                bevy_log::error!("Invalid configuration: {}", msg);
                if let Some(ref window) = ui_definition.window {
                    bevy_log::error!("Window config: size=[{}, {}]", window.size[0], window.size[1]);
                }
            }
            _ => {
                bevy_log::error!("General validation error occurred");
            }
        }
        
        bevy_log::error!("=== End Error Details ===");
    }

    /// Log widget validation context for debugging
    fn log_widget_validation_context(&self, ui_definition: &UiDefinition) {
        bevy_log::error!("Widget validation context:");
        let mut widget_counts = HashMap::new();
        self.count_widget_types(&ui_definition.root, &mut widget_counts);
        
        for (widget_type, count) in widget_counts {
            bevy_log::error!("  - {}: {} instances", widget_type, count);
        }
    }

    /// Count widget types for error reporting
    fn count_widget_types(&self, node: &WidgetNode, counts: &mut HashMap<String, usize>) {
        let widget_type_name = match &node.widget_type {
            WidgetType::Container { .. } => "Container".to_string(),
            WidgetType::Button { .. } => "Button".to_string(),
            WidgetType::Text { .. } => "Text".to_string(),
            WidgetType::Shape { .. } => "Shape".to_string(),
            WidgetType::Custom { component, .. } => format!("Custom({})", component),
        };
        
        *counts.entry(widget_type_name).or_insert(0) += 1;
        
        for child in &node.children {
            self.count_widget_types(child, counts);
        }
    }

    /// Log style validation context for debugging
    fn log_style_validation_context(&self, ui_definition: &UiDefinition) {
        bevy_log::error!("Style validation context:");
        
        if let Some(ref global_styles) = ui_definition.styles {
            bevy_log::error!("  Global styles defined: {}", global_styles.len());
            for (class_name, style_override) in global_styles {
                let properties = self.count_style_properties(style_override);
                bevy_log::error!("    - '{}': {} properties", class_name, properties);
            }
        } else {
            bevy_log::error!("  No global styles defined");
        }
        
        let mut style_references = Vec::new();
        self.collect_style_references(&ui_definition.root, &mut style_references);
        
        if !style_references.is_empty() {
            bevy_log::error!("  Style class references found:");
            for (widget_id, classes) in style_references {
                bevy_log::error!("    - Widget '{}': [{}]", widget_id, classes.join(", "));
            }
        }
    }

    /// Count style properties for error reporting
    fn count_style_properties(&self, style_override: &StyleOverrides) -> usize {
        let mut count = 0;
        if style_override.background_color.is_some() { count += 1; }
        if style_override.text_color.is_some() { count += 1; }
        if style_override.border_color.is_some() { count += 1; }
        if style_override.border_width.is_some() { count += 1; }
        if style_override.border_radius.is_some() { count += 1; }
        if style_override.text_size.is_some() { count += 1; }
        if style_override.opacity.is_some() { count += 1; }
        count
    }

    /// Collect style class references for error reporting
    fn collect_style_references(&self, node: &WidgetNode, references: &mut Vec<(String, Vec<String>)>) {
        if let Some(ref classes) = node.classes {
            if !classes.is_empty() {
                let widget_id = node.id.as_ref().cloned().unwrap_or_else(|| "<anonymous>".to_string());
                references.push((widget_id, classes.clone()));
            }
        }
        
        for child in &node.children {
            self.collect_style_references(child, references);
        }
    }

    /// Log action validation context for debugging
    fn log_action_validation_context(&self, ui_definition: &UiDefinition) {
        bevy_log::error!("Action validation context:");
        
        if let Some(ref global_actions) = ui_definition.actions {
            bevy_log::error!("  Global actions defined: {}", global_actions.len());
            for (action_name, action_binding) in global_actions {
                let param_count = action_binding.params.as_ref().map(|p| p.len()).unwrap_or(0);
                bevy_log::error!("    - '{}': event='{}', {} params", 
                               action_name, action_binding.event, param_count);
            }
        } else {
            bevy_log::error!("  No global actions defined");
        }
        
        let mut action_references = Vec::new();
        self.collect_action_references(&ui_definition.root, &mut action_references);
        
        if !action_references.is_empty() {
            bevy_log::error!("  Action references found:");
            for (widget_id, action_info) in action_references {
                bevy_log::error!("    - Widget '{}': {}", widget_id, action_info);
            }
        }
    }

    /// Collect action references for error reporting
    fn collect_action_references(&self, node: &WidgetNode, references: &mut Vec<(String, String)>) {
        let widget_id = node.id.as_ref().cloned().unwrap_or_else(|| "<anonymous>".to_string());
        
        // Note: Button actions are now handled via bindings, not direct action field
        
        // Check widget bindings
        if let Some(ref bindings) = node.bindings {
            for (event_name, action_binding) in bindings {
                references.push((widget_id.clone(), 
                    format!("binding '{}' -> '{}'", event_name, action_binding.action)));
            }
        }
        
        for child in &node.children {
            self.collect_action_references(child, references);
        }
    }

    /// Log validation warnings and suggestions
    pub fn log_validation_warnings(&self, ui_definition: &UiDefinition, _registry: &UiRegistry) {
        bevy_log::info!("=== Validation Warnings & Suggestions ===");
        
        // Check for potential issues
        self.check_widget_tree_issues(&ui_definition.root, 0);
        self.check_style_consistency_issues(ui_definition);
        self.check_action_consistency_issues(ui_definition);
        self.check_performance_suggestions(ui_definition);
        
        bevy_log::info!("=== End Warnings & Suggestions ===");
    }

    /// Check for widget tree issues
    fn check_widget_tree_issues(&self, node: &WidgetNode, depth: usize) {
        // Check for anonymous widgets
        if node.id.is_none() {
            bevy_log::warn!("Widget at depth {} has no ID, which may complicate debugging", depth);
        }
        
        // Check for empty containers
        if let WidgetType::Container { .. } = &node.widget_type {
            if node.children.is_empty() {
                bevy_log::warn!("Container widget '{}' has no children", 
                               node.id.as_ref().unwrap_or(&"<anonymous>".to_string()));
            }
        }
        
        // Check for deeply nested widgets
        if depth > 10 {
            bevy_log::warn!("Widget '{}' is deeply nested (depth {}), consider flattening hierarchy", 
                           node.id.as_ref().unwrap_or(&"<anonymous>".to_string()), depth);
        }
        
        // Check for widgets with many children
        if node.children.len() > 20 {
            bevy_log::warn!("Widget '{}' has {} children, consider grouping or pagination", 
                           node.id.as_ref().unwrap_or(&"<anonymous>".to_string()), node.children.len());
        }
        
        for child in &node.children {
            self.check_widget_tree_issues(child, depth + 1);
        }
    }

    /// Check for style consistency issues
    fn check_style_consistency_issues(&self, ui_definition: &UiDefinition) {
        if let Some(ref global_styles) = ui_definition.styles {
            // Check for unused global styles
            let mut used_classes = HashSet::new();
            self.collect_used_style_classes(&ui_definition.root, &mut used_classes);
            
            for class_name in global_styles.keys() {
                if !used_classes.contains(class_name) {
                    bevy_log::warn!("Global style class '{}' is defined but never used", class_name);
                }
            }
            
            // Check for potential naming conflicts
            for class_name in global_styles.keys() {
                if class_name.contains(" ") || class_name.contains("-") {
                    bevy_log::warn!("Style class '{}' contains spaces or hyphens, consider using underscores", class_name);
                }
            }
        }
    }

    /// Collect used style classes
    fn collect_used_style_classes(&self, node: &WidgetNode, used_classes: &mut HashSet<String>) {
        if let Some(ref classes) = node.classes {
            for class_name in classes {
                used_classes.insert(class_name.clone());
            }
        }
        
        for child in &node.children {
            self.collect_used_style_classes(child, used_classes);
        }
    }

    /// Check for action consistency issues
    fn check_action_consistency_issues(&self, ui_definition: &UiDefinition) {
        if let Some(ref global_actions) = ui_definition.actions {
            // Check for unused global actions
            let mut used_actions = HashSet::new();
            self.collect_used_actions(&ui_definition.root, &mut used_actions);
            
            for action_name in global_actions.keys() {
                if !used_actions.contains(action_name) {
                    bevy_log::warn!("Global action '{}' is defined but never used", action_name);
                }
            }
        }
    }

    /// Collect used actions
    fn collect_used_actions(&self, node: &WidgetNode, used_actions: &mut HashSet<String>) {
        // Note: Button actions are now handled via bindings, not direct action field
        
        // Check widget bindings
        if let Some(ref bindings) = node.bindings {
            for (_, action_binding) in bindings {
                used_actions.insert(action_binding.action.clone());
            }
        }
        
        for child in &node.children {
            self.collect_used_actions(child, used_actions);
        }
    }

    /// Check for performance suggestions
    fn check_performance_suggestions(&self, ui_definition: &UiDefinition) {
        let total_widgets = self.count_total_widgets(&ui_definition.root);
        
        if total_widgets > 100 {
            bevy_log::warn!("UI definition contains {} widgets, consider virtualization for large lists", total_widgets);
        }
        
        if let Some(ref global_styles) = ui_definition.styles {
            if global_styles.len() > 50 {
                bevy_log::warn!("Large number of global styles ({}), consider style consolidation", global_styles.len());
            }
        }
    }

    /// Count total widgets in hierarchy
    fn count_total_widgets(&self, node: &WidgetNode) -> usize {
        1 + node.children.iter().map(|child| self.count_total_widgets(child)).sum::<usize>()
    }
}