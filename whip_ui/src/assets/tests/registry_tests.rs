use super::super::*;
use crate::widgets::blueprint::{ColorDef, FlexDirection};
use std::collections::HashMap;

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
    let config = UiRegistryConfig {
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