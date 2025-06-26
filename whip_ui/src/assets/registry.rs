use bevy_ecs::prelude::Resource;
use std::collections::HashMap;
use thiserror::Error;
use crate::widgets::blueprint::WidgetType;

use super::definitions::ActionBinding;

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
                params.insert("target_id".to_string(), "String".to_string());
                params
            },
            requires_conditions: vec!["target_exists".to_string()],
        });

        self.register_action("debug", ActionInfo {
            display_name: "Debug".to_string(),
            description: "Log a debug message to the console".to_string(),
            parameter_types: {
                let mut params = HashMap::new();
                params.insert("message".to_string(), "String".to_string());
                params
            },
            requires_conditions: vec![],
        });

        self.register_action("navigate", ActionInfo {
            display_name: "Navigate".to_string(),
            description: "Navigate to a specified target".to_string(),
            parameter_types: {
                let mut params = HashMap::new();
                params.insert("target".to_string(), "String".to_string());
                params
            },
            requires_conditions: vec![],
        });

        self.register_action("update_text", ActionInfo {
            display_name: "Update Text".to_string(),
            description: "Update the text content of a UI element".to_string(),
            parameter_types: {
                let mut params = HashMap::new();
                params.insert("target_id".to_string(), "String".to_string());
                params.insert("text".to_string(), "String".to_string());
                params
            },
            requires_conditions: vec!["target_exists".to_string()],
        });

        self.register_action("set_focus", ActionInfo {
            display_name: "Set Focus".to_string(),
            description: "Set focus to a specific UI element".to_string(),
            parameter_types: {
                let mut params = HashMap::new();
                params.insert("target_id".to_string(), "String".to_string());
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
            WidgetType::Button { text, .. } => {
                // Button text validation (optional but if provided, cannot be empty)
                if let Some(text) = text {
                    if info.required_properties.contains(&"text".to_string()) && text.is_empty() {
                        return Err(UiRegistryError::InvalidPropertyValue {
                            widget_type: "Button".to_string(),
                            property: "text".to_string(),
                            reason: "Button text cannot be empty".to_string(),
                        });
                    }
                }
                
                // Note: Button templates handle actions via bindings, not direct action field
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
    pub fn validate_event_type(&self, event: &str) -> Result<(), UiRegistryError> {
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
    fn validate_parameter_type(&self, value: &serde_json::Value, expected_type: &str, param_name: &str, action_name: &str) -> Result<(), UiRegistryError> {
        let is_valid = match expected_type.to_lowercase().as_str() {
            "string" => matches!(value, serde_json::Value::String(_)),
            "integer" | "int" | "i32" | "i64" => matches!(value, serde_json::Value::Number(n) if n.is_i64()),
            "float" | "f32" | "f64" => matches!(value, serde_json::Value::Number(n) if n.is_f64()),
            "boolean" | "bool" => matches!(value, serde_json::Value::Bool(_)),
            "array" => matches!(value, serde_json::Value::Array(_)),
            "table" | "object" => matches!(value, serde_json::Value::Object(_)),
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