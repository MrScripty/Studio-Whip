use super::super::*;
use crate::widgets::blueprint::{ColorDef, FlexDirection, WidgetType, LayoutConfig, StyleConfig, BehaviorConfig};
use std::collections::HashMap;

/// Test basic UiDefinition deserialization
#[test]
fn test_basic_ui_definition_parsing() {
    let json_str = r##"{
  "root": {
    "widget_type": {
      "type": "Container",
      "direction": "Column"
    },
    "layout": {
      "size": [800.0, 600.0]
    },
    "style": {
      "background_color": "#2D3748"
    },
    "behavior": {
      "visible": true
    }
  }
}"##;

    let ui_def: Result<UiDefinition, serde_json::Error> = serde_json::from_str(json_str);
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