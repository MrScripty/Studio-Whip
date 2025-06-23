use super::super::*;
use crate::widgets::blueprint::{ColorDef, FlexDirection};
use std::collections::HashMap;

/// Test comprehensive validation with registry checks
#[test]
fn test_comprehensive_validation() {
    let loader = UiDefinitionLoader;
    let registry = UiRegistry::new();
    
    // Test valid UI definition
    let valid_ui_def = create_valid_ui_definition();
    let result = loader.validate_with_comprehensive_checks(&valid_ui_def, &registry);
    assert!(result.is_ok(), "Valid UI definition should pass comprehensive validation: {:?}", result.err());

    // Test invalid widget type validation
    let mut invalid_widget_ui_def = create_valid_ui_definition();
    invalid_widget_ui_def.root.widget_type = WidgetType::Button {
        text: "".to_string(), // Empty text should fail
        action: None,
    };
    let result = loader.validate_with_comprehensive_checks(&invalid_widget_ui_def, &registry);
    assert!(result.is_err(), "Empty button text should fail validation");
    
    // Test invalid style class validation
    let mut invalid_style_ui_def = create_valid_ui_definition();
    invalid_style_ui_def.root.classes = Some(vec!["nonexistent_class".to_string()]);
    let result = loader.validate_with_comprehensive_checks(&invalid_style_ui_def, &registry);
    assert!(result.is_err(), "Nonexistent style class should fail validation");

    // Test invalid color format
    let mut invalid_color_ui_def = create_valid_ui_definition();
    invalid_color_ui_def.styles = Some({
        let mut styles = HashMap::new();
        styles.insert("bad_color".to_string(), StyleOverrides {
            background_color: Some(ColorDef::Hex("invalid".to_string())),
            text_color: None,
            border_color: None,
            border_width: None,
            border_radius: None,
            text_size: None,
            opacity: None,
        });
        styles
    });
    invalid_color_ui_def.root.classes = Some(vec!["bad_color".to_string()]);
    let result = loader.validate_with_comprehensive_checks(&invalid_color_ui_def, &registry);
    assert!(result.is_err(), "Invalid hex color should fail validation");

    // Test negative layout values
    let mut invalid_layout_ui_def = create_valid_ui_definition();
    invalid_layout_ui_def.root.layout.size = Some([-100.0, 200.0].into());
    let result = loader.validate_with_comprehensive_checks(&invalid_layout_ui_def, &registry);
    assert!(result.is_err(), "Negative widget size should fail validation");

    // Test excessive nesting depth
    let mut deep_nesting_ui_def = create_valid_ui_definition();
    let mut current_node = &mut deep_nesting_ui_def.root;
    
    // Create nesting deeper than the maximum (50)
    for i in 0..60 {
        let child = WidgetNode {
            id: Some(format!("deep_child_{}", i)),
            widget_type: WidgetType::Container { direction: FlexDirection::Column },
            layout: LayoutConfig::default(),
            style: StyleConfig::default(),
            behavior: BehaviorConfig::default(),
            classes: None,
            style_overrides: None,
            bindings: None,
            children: vec![],
        };
        current_node.children.push(child);
        current_node = current_node.children.last_mut().unwrap();
    }
    
    let result = loader.validate_with_comprehensive_checks(&deep_nesting_ui_def, &registry);
    assert!(result.is_err(), "Excessive nesting depth should fail validation");

    bevy_log::info!("All comprehensive validation tests passed!");
}

/// Test detailed error logging functionality
#[test]
fn test_detailed_error_logging() {
    let loader = UiDefinitionLoader;
    let registry = UiRegistry::new();
    
    // Create a valid UI definition for summary logging
    let valid_ui_def = create_valid_ui_definition();
    
    // Test validation summary logging (should not panic)
    loader.log_validation_summary(&valid_ui_def, &registry);
    
    // Test validation warnings (should not panic)
    loader.log_validation_warnings(&valid_ui_def, &registry);
    
    // Create an invalid UI definition for error logging
    let mut invalid_ui_def = create_valid_ui_definition();
    invalid_ui_def.root.widget_type = WidgetType::Button {
        text: "".to_string(), // This will trigger validation error
        action: None,
    };
    
    // Test error validation and logging
    let validation_result = loader.validate_with_comprehensive_checks(&invalid_ui_def, &registry);
    assert!(validation_result.is_err(), "Invalid UI definition should fail validation");
    
    if let Err(error) = validation_result {
        // Test detailed error logging (should not panic)
        loader.log_validation_error_with_context(&error, &invalid_ui_def, "Test error context");
    }
    
    // Test with style validation error
    let mut invalid_style_ui_def = create_valid_ui_definition();
    invalid_style_ui_def.styles = Some({
        let mut styles = HashMap::new();
        styles.insert("test_style".to_string(), StyleOverrides {
            background_color: Some(ColorDef::Hex("invalid_hex".to_string())),
            text_color: None,
            border_color: None,
            border_width: None,
            border_radius: None,
            text_size: None,
            opacity: None,
        });
        styles
    });
    invalid_style_ui_def.root.classes = Some(vec!["test_style".to_string()]);
    
    let style_validation_result = loader.validate_with_comprehensive_checks(&invalid_style_ui_def, &registry);
    assert!(style_validation_result.is_err(), "Invalid style should fail validation");
    
    if let Err(error) = style_validation_result {
        loader.log_validation_error_with_context(&error, &invalid_style_ui_def, "Style validation test");
    }
    
    // Test with unused styles and actions for warnings
    let mut warning_ui_def = create_valid_ui_definition();
    warning_ui_def.styles = Some({
        let mut styles = HashMap::new();
        styles.insert("unused_style".to_string(), StyleOverrides {
            background_color: Some(ColorDef::Hex("#FF0000".to_string())),
            text_color: None,
            border_color: None,
            border_width: None,
            border_radius: None,
            text_size: None,
            opacity: None,
        });
        styles
    });
    warning_ui_def.actions = Some({
        let mut actions = HashMap::new();
        actions.insert("unused_action".to_string(), ActionBinding {
            event: "click".to_string(),
            action: "unused_action".to_string(),
            params: None,
        });
        actions
    });
    
    // This should generate warnings about unused styles and actions
    loader.log_validation_warnings(&warning_ui_def, &registry);
    
    bevy_log::info!("All detailed error logging tests passed!");
}

/// Test TOML loading with valid and invalid files
#[test]
fn test_valid_and_invalid_toml_files() {
    let loader = UiDefinitionLoader;
    let registry = UiRegistry::new();

    // Test 1: Valid hierarchical TOML
    let valid_toml = r##"
[window]
size = [1024.0, 768.0]
background_color = "#2D3748"

[styles.primary_button]
background_color = "#3182CE"
text_color = "#FFFFFF"
border_radius = 8.0
text_size = 16.0

[styles.container_style]
background_color = { r = 240, g = 240, b = 240, a = 0.9 }
border_width = 2.0
border_color = "gray"

[actions.show_settings]
event = "click"
action = "open_settings"

[actions.navigate_back]
event = "click"
action = "navigate_back"
params = { target = "home" }

[root]
id = "main_container"
widget_type = { type = "Container", direction = "Column" }
classes = ["container_style"]

    [[root.children]]
    id = "header_text"
    widget_type = { type = "Text", content = "Welcome to WhipUI", editable = false }
    classes = ["primary_button"]
    
        [root.children.style_overrides]
        text_size = 24.0
        text_color = "#1A202C"

    [[root.children]]
    id = "settings_button"
    widget_type = { type = "Button", text = "Settings", action = "show_settings" }
    classes = ["primary_button"]
    
        [root.children.bindings.hover]
        event = "hover"
        action = "show_settings"

    [[root.children]]
    id = "nested_container"
    widget_type = { type = "Container", direction = "Row" }
    
        [[root.children.children]]
        id = "back_button"
        widget_type = { type = "Button", text = "Back", action = "navigate_back" }

        [[root.children.children]]
        id = "info_shape"
        widget_type = { type = "Shape", shape_type = "Circle" }
        
            [root.children.children.style_overrides]
            background_color = { r = 100, g = 200, b = 150 }
"##;

    // Parse valid TOML
    let valid_result = toml::from_str::<UiDefinition>(valid_toml);
    assert!(valid_result.is_ok(), "Valid TOML should parse successfully: {:?}", valid_result.err());
    
    let valid_ui_def = valid_result.unwrap();
    
    // Validate with comprehensive checks
    let validation_result = loader.validate_with_comprehensive_checks(&valid_ui_def, &registry);
    assert!(validation_result.is_ok(), "Valid UI definition should pass validation: {:?}", validation_result.err());
    
    // Log validation summary for the valid definition
    loader.log_validation_summary(&valid_ui_def, &registry);
    loader.log_validation_warnings(&valid_ui_def, &registry);

    // Test 2: Invalid TOML - malformed structure
    let invalid_toml_1 = r##"
[window]
size = [1024.0, 768.0]
background_color = { InvalidColorType = "#FF0000" }  # Invalid color type

[root]
id = "test_container"
widget_type = { UnknownWidget = {} }  # Invalid widget type
"##;

    let invalid_result_1 = toml::from_str::<UiDefinition>(invalid_toml_1);
    assert!(invalid_result_1.is_err(), "Invalid TOML should fail to parse");

    // Test 3: Valid TOML structure but invalid content
    let invalid_toml_2 = r##"
[window]
size = [-100.0, 768.0]  # Negative width should fail validation

[styles.bad_style]
background_color = "not_a_hex_color"  # Invalid hex format
opacity = 2.0  # Invalid opacity range

[root]
id = "123_invalid_id"  # ID starting with number
widget_type = { type = "Button", text = "", action = "nonexistent_action" }  # Empty text, nonexistent action
classes = ["nonexistent_style"]  # Reference to undefined style
"##;

    let invalid_result_2 = toml::from_str::<UiDefinition>(invalid_toml_2);
    if let Ok(invalid_ui_def_2) = invalid_result_2 {
        // TOML parsing succeeded, but validation should fail
        let validation_result_2 = loader.validate_with_comprehensive_checks(&invalid_ui_def_2, &registry);
        assert!(validation_result_2.is_err(), "Invalid UI definition should fail validation");
        
        if let Err(error) = validation_result_2 {
            loader.log_validation_error_with_context(&error, &invalid_ui_def_2, "Invalid TOML content test");
        }
    }

    // Test 4: TOML with error recovery scenarios
    let recovery_toml = r##"
[window]
size = [800.0, 600.0]

[styles.good_style]
background_color = "#FF0000"

[styles.bad_style]
invalid_property = "this will be ignored"
background_color = "invalid_hex"  # This will cause issues

[actions.good_action]
event = "click"
action = "test_action"

[root]
id = "recovery_test"
widget_type = { type = "Container", direction = "Column" }
classes = ["good_style"]  # Valid reference

    [[root.children]]
    id = "good_child"
    widget_type = { type = "Text", content = "Valid text", editable = false }
    
    [[root.children]]
    id = "problematic_child"
    widget_type = { type = "Button", text = "", action = "nonexistent" }  # Empty text, bad action
    classes = ["bad_style"]  # Will reference style with invalid hex
"##;

    // Test error recovery parsing
    let recovery_result = loader.parse_with_error_recovery(recovery_toml);
    match recovery_result {
        Ok(recovered_ui_def) => {
            bevy_log::info!("Error recovery parsing succeeded");
            loader.log_validation_summary(&recovered_ui_def, &registry);
            
            // Even with recovery, some validation errors may remain
            let validation_result = loader.validate_with_comprehensive_checks(&recovered_ui_def, &registry);
            if let Err(error) = validation_result {
                loader.log_validation_error_with_context(&error, &recovered_ui_def, "Error recovery test");
            }
        }
        Err(error) => {
            bevy_log::warn!("Error recovery parsing failed: {}", error);
        }
    }

    // Test 5: Deeply nested structure
    let deep_nested_toml = r##"
[window]
size = [600.0, 400.0]

[root]
id = "level_0"
widget_type = { type = "Container", direction = "Column" }

    [[root.children]]
    id = "level_1"
    widget_type = { type = "Container", direction = "Row" }
    
        [[root.children.children]]
        id = "level_2"
        widget_type = { type = "Container", direction = "Column" }
        
            [[root.children.children.children]]
            id = "level_3"
            widget_type = { type = "Container", direction = "Row" }
            
                [[root.children.children.children.children]]
                id = "level_4"
                widget_type = { type = "Text", content = "Deep nested text", editable = false }
"##;

    let deep_result = toml::from_str::<UiDefinition>(deep_nested_toml);
    assert!(deep_result.is_ok(), "Deep nested TOML should parse successfully");
    
    let deep_ui_def = deep_result.unwrap();
    let deep_validation = loader.validate_with_comprehensive_checks(&deep_ui_def, &registry);
    assert!(deep_validation.is_ok(), "Deep nested structure should pass validation");

    // Test 6: Large widget collection
    let large_collection_toml = r##"
[window]
size = [1200.0, 800.0]

[root]
id = "large_container"
widget_type = { type = "Container", direction = "Column" }

    [[root.children]]
    id = "item_1"
    widget_type = { type = "Button", text = "Button 1" }
    
    [[root.children]]
    id = "item_2"
    widget_type = { type = "Button", text = "Button 2" }
    
    [[root.children]]
    id = "item_3"
    widget_type = { type = "Text", content = "Text 3", editable = true }
    
    [[root.children]]
    id = "item_4"
    widget_type = { type = "Shape", shape_type = "Rectangle" }
    
    [[root.children]]
    id = "item_5"
    widget_type = { type = "Container", direction = "Row" }
        
        [[root.children.children]]
        id = "sub_item_1"
        widget_type = { type = "Button", text = "Sub Button 1" }
        
        [[root.children.children]]
        id = "sub_item_2"
        widget_type = { type = "Button", text = "Sub Button 2" }
"##;

    let large_result = toml::from_str::<UiDefinition>(large_collection_toml);
    assert!(large_result.is_ok(), "Large collection TOML should parse successfully");
    
    let large_ui_def = large_result.unwrap();
    let large_validation = loader.validate_with_comprehensive_checks(&large_ui_def, &registry);
    assert!(large_validation.is_ok(), "Large collection should pass validation");
    
    loader.log_validation_warnings(&large_ui_def, &registry);

    bevy_log::info!("All TOML file loading tests passed!");
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