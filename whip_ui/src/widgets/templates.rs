use crate::widgets::blueprint::{
    WidgetBlueprint, WidgetType, LayoutConfig, StyleConfig, BehaviorConfig, 
    ShapeType, ColorDef
};
use bevy_math::{Vec2, Vec3};
use serde::{Deserialize, Serialize};

/// Built-in widget templates with sensible defaults
#[derive(Debug, Clone)]
pub struct WidgetTemplates {
    /// Built-in button template
    pub button: ButtonTemplate,
}

impl Default for WidgetTemplates {
    fn default() -> Self {
        Self {
            button: ButtonTemplate::default(),
        }
    }
}

/// Template for button widgets composed of Shape + Text primitives
#[derive(Debug, Clone)]
pub struct ButtonTemplate {
    /// Default background color for the button shape
    pub background_color: ColorDef,
    /// Default text content
    pub text: String,
    /// Default button size [width, height]
    pub size: Vec2,
    /// Default text color
    pub text_color: ColorDef,
    /// Default text size
    pub text_size: f32,
    /// Default behavior settings
    pub clickable: bool,
    /// Default border settings
    pub border_width: Option<f32>,
    pub border_color: Option<ColorDef>,
    pub border_radius: Option<f32>,
}

impl Default for ButtonTemplate {
    fn default() -> Self {
        Self {
            background_color: ColorDef::Named("blue".to_string()),
            text: "Button".to_string(),
            size: Vec2::new(100.0, 40.0),
            text_color: ColorDef::Named("white".to_string()),
            text_size: 16.0,
            clickable: true,
            border_width: None,
            border_color: None,
            border_radius: Some(4.0),
        }
    }
}

/// Template definition that can be parsed from TOML
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TemplateType {
    Button {
        /// Override default text content
        #[serde(skip_serializing_if = "Option::is_none")]
        text: Option<String>,
        /// Override default background color
        #[serde(skip_serializing_if = "Option::is_none")]
        background_color: Option<ColorDef>,
        /// Override default text color
        #[serde(skip_serializing_if = "Option::is_none")]
        text_color: Option<ColorDef>,
        /// Override default size
        #[serde(skip_serializing_if = "Option::is_none")]
        size: Option<Vec2>,
        /// Override default text size
        #[serde(skip_serializing_if = "Option::is_none")]
        text_size: Option<f32>,
        /// Override border properties
        #[serde(skip_serializing_if = "Option::is_none")]
        border_width: Option<f32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        border_color: Option<ColorDef>,
        #[serde(skip_serializing_if = "Option::is_none")]
        border_radius: Option<f32>,
    },
}

impl ButtonTemplate {
    /// Create a button widget blueprint with user overrides applied
    pub fn create_blueprint(
        &self, 
        id: String, 
        overrides: Option<&TemplateType>,
        user_layout: Option<LayoutConfig>,
        user_behavior: Option<BehaviorConfig>,
    ) -> (WidgetBlueprint, WidgetBlueprint) {
        // Apply user overrides to template defaults
        let (text, background_color, text_color, size, text_size, border_width, border_color, border_radius) = 
            if let Some(TemplateType::Button { 
                text: override_text, 
                background_color: override_bg, 
                text_color: override_text_color,
                size: override_size,
                text_size: override_text_size,
                border_width: override_border_width,
                border_color: override_border_color,
                border_radius: override_border_radius,
            }) = overrides {
                (
                    override_text.clone().unwrap_or_else(|| self.text.clone()),
                    override_bg.clone().unwrap_or_else(|| self.background_color.clone()),
                    override_text_color.clone().unwrap_or_else(|| self.text_color.clone()),
                    override_size.unwrap_or(self.size),
                    override_text_size.unwrap_or(self.text_size),
                    override_border_width.or(self.border_width),
                    override_border_color.clone().or_else(|| self.border_color.clone()),
                    override_border_radius.or(self.border_radius),
                )
            } else {
                (
                    self.text.clone(),
                    self.background_color.clone(),
                    self.text_color.clone(),
                    self.size,
                    self.text_size,
                    self.border_width,
                    self.border_color.clone(),
                    self.border_radius,
                )
            };

        // Create shape blueprint (interactive container)
        let shape_id = format!("{}_shape", id);
        let shape_blueprint = WidgetBlueprint {
            id: shape_id.clone(),
            widget_type: WidgetType::Shape {
                shape_type: ShapeType::Rectangle,
            },
            layout: user_layout.unwrap_or_else(|| LayoutConfig {
                size: Some(size),
                position: None,
                margin: None,
                padding: None,
                flex_grow: None,
                flex_shrink: None,
                align_self: None,
                grid_row: None,
                grid_column: None,
            }),
            style: StyleConfig {
                background_color: Some(background_color),
                border_color: border_color,
                border_width,
                border_radius,
                text_color: None, // Shape doesn't need text color
                text_size: None,  // Shape doesn't need text size
                opacity: None,
                states: None,
            },
            behavior: user_behavior.unwrap_or_else(|| BehaviorConfig {
                visible: Some(true),
                interactive: Some(true),
                draggable: Some(false),
                clickable: Some(self.clickable),
                focusable: Some(true),
                z_index: Some(0),
                position_control: Some(crate::layout::PositionControl::Layout),
            }),
            children: vec![format!("{}_text", id)], // Text is child of shape
        };

        // Create text blueprint (display label)
        let text_id = format!("{}_text", id);
        let text_blueprint = WidgetBlueprint {
            id: text_id,
            widget_type: WidgetType::Text {
                content: text,
                editable: false,
            },
            layout: LayoutConfig {
                size: None, // Text size determined by content
                position: Some(Vec3::new(0.0, 0.0, 0.1)), // Center of parent shape (relative positioning)
                margin: None,
                padding: None,
                flex_grow: None,
                flex_shrink: None,
                align_self: None,
                grid_row: None,
                grid_column: None,
            },
            style: StyleConfig {
                background_color: None, // Transparent background
                border_color: None,
                border_width: None,
                border_radius: None,
                text_color: Some(text_color),
                text_size: Some(text_size),
                opacity: None,
                states: None,
            },
            behavior: BehaviorConfig {
                visible: Some(true),
                interactive: Some(false), // Text is not interactive
                draggable: Some(false),
                clickable: Some(false),
                focusable: Some(false),
                z_index: Some(1), // Above the shape
                position_control: Some(crate::layout::PositionControl::Manual),
            },
            children: vec![], // Text has no children
        };

        (shape_blueprint, text_blueprint)
    }
}

/// Global widget templates registry
static WIDGET_TEMPLATES: std::sync::LazyLock<WidgetTemplates> = 
    std::sync::LazyLock::new(|| WidgetTemplates::default());

/// Get the global widget templates
pub fn get_widget_templates() -> &'static WidgetTemplates {
    &WIDGET_TEMPLATES
}

/// Expand template widgets directly from WidgetNode (unified architecture)
pub fn expand_template_node(node: &crate::assets::definitions::WidgetNode) -> Vec<crate::assets::definitions::WidgetNode> {
    use crate::widgets::blueprint::{WidgetType, ShapeType};
    use crate::assets::definitions::WidgetNode;
    use bevy_math::Vec3;
    
    match &node.widget_type {
        WidgetType::Button { 
            text, 
            background_color, 
            text_color,
            size,
            text_size,
            border_width,
            border_color,
            border_radius,
        } => {
            let widget_id = node.id.clone().unwrap_or_else(|| "unnamed".to_string());
            
            // Get template defaults
            let templates = get_widget_templates();
            let button_template = &templates.button;
            
            // Apply template values with TOML overrides
            let final_text = text.clone().unwrap_or_else(|| button_template.text.clone());
            let final_bg_color = background_color.clone().unwrap_or_else(|| button_template.background_color.clone());
            let final_text_color = text_color.clone().unwrap_or_else(|| button_template.text_color.clone());
            // Prioritize layout size over widget_type size over template default
            let final_size = node.layout.size.or(*size).unwrap_or(button_template.size);
            let final_text_size = text_size.unwrap_or(button_template.text_size);
            
            // Create shape node (button background) - directly from WidgetNode data
            let shape_node = WidgetNode {
                id: Some(format!("{}_shape", widget_id)),
                widget_type: WidgetType::Shape { 
                    shape_type: ShapeType::Rectangle 
                },
                layout: crate::widgets::blueprint::LayoutConfig {
                    size: Some(final_size),
                    position: node.layout.position, // Inherit position from button
                    margin: node.layout.margin.clone(),
                    padding: node.layout.padding.clone(),
                    flex_grow: node.layout.flex_grow,
                    flex_shrink: node.layout.flex_shrink,
                    align_self: node.layout.align_self.clone(),
                    grid_row: node.layout.grid_row,
                    grid_column: node.layout.grid_column,
                },
                style: crate::widgets::blueprint::StyleConfig {
                    background_color: Some(final_bg_color),
                    border_color: border_color.clone().or(button_template.border_color.clone()),
                    border_width: border_width.or(button_template.border_width),
                    border_radius: border_radius.or(button_template.border_radius),
                    text_color: None, // Shape doesn't need text color
                    text_size: None,  // Shape doesn't need text size
                    opacity: node.style.opacity,
                    states: node.style.states.clone(),
                },
                behavior: crate::widgets::blueprint::BehaviorConfig {
                    visible: Some(true),
                    interactive: Some(true),
                    draggable: Some(false),
                    clickable: Some(button_template.clickable),
                    focusable: Some(true),
                    z_index: node.behavior.z_index,
                    position_control: node.behavior.position_control.clone(),
                },
                classes: node.classes.clone(),
                style_overrides: node.style_overrides.clone(),
                bindings: node.bindings.clone(), // Button actions go to shape
                children: vec![],
            };

            // Create text node (button label) - child of shape with relative positioning
            let text_node = WidgetNode {
                id: Some(format!("{}_text", widget_id)),
                widget_type: WidgetType::Text { 
                    content: final_text,
                    editable: false 
                },
                layout: crate::widgets::blueprint::LayoutConfig {
                    size: None, // Text size determined by content
                    position: Some(Vec3::new(0.0, 0.0, 0.1)), // Centered relative to parent shape
                    margin: None,
                    padding: None,
                    flex_grow: None,
                    flex_shrink: None,
                    align_self: None,
                    grid_row: None,
                    grid_column: None,
                },
                style: crate::widgets::blueprint::StyleConfig {
                    background_color: None, // Transparent background
                    border_color: None,
                    border_width: None,
                    border_radius: None,
                    text_color: Some(final_text_color),
                    text_size: Some(final_text_size),
                    opacity: None,
                    states: None,
                },
                behavior: crate::widgets::blueprint::BehaviorConfig {
                    visible: Some(true),
                    interactive: Some(false), // Text is not interactive
                    draggable: Some(false),
                    clickable: Some(false),
                    focusable: Some(false),
                    z_index: Some(1), // Above the shape
                    position_control: Some(crate::layout::PositionControl::Manual),
                },
                classes: None,
                style_overrides: None,
                bindings: None, // No direct bindings - parent shape handles interaction
                children: vec![],
            };

            bevy_log::info!("Expanded Button '{}' into Shape + Text components", widget_id);
            vec![shape_node, text_node]
        }
        _ => {
            // Not a template widget, return as-is
            vec![node.clone()]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::blueprint::{WidgetType, ColorDef, LayoutConfig, StyleConfig, BehaviorConfig};
    use crate::assets::definitions::WidgetNode;
    use bevy_math::Vec2;
    
    #[test]
    fn test_button_template_expansion_unified() {
        // Create a button WidgetNode (as would come from TOML) - matches main.toml exactly
        let button_node = WidgetNode {
            id: Some("save_button".to_string()),
            widget_type: WidgetType::Button {
                text: Some("Save File".to_string()),
                background_color: Some(ColorDef::Named("green".to_string())),
                text_color: Some(ColorDef::Named("white".to_string())),
                size: None, // Size comes from layout config
                text_size: None,
                border_width: None,
                border_color: None,
                border_radius: None,
            },
            layout: LayoutConfig {
                size: Some(Vec2::new(120.0, 40.0)),
                position: Some(bevy_math::Vec3::new(465.0, 245.0, 1.0)), // Bottom-right position
                margin: None,
                padding: None,
                flex_grow: None,
                flex_shrink: None,
                align_self: None,
                grid_row: None,
                grid_column: None,
            },
            style: StyleConfig::default(),
            behavior: BehaviorConfig {
                visible: Some(true),
                interactive: Some(true),
                draggable: Some(false),
                clickable: Some(true),
                focusable: Some(true),
                z_index: Some(1),
                position_control: None,
            },
            classes: None,
            style_overrides: None,
            bindings: None,
            children: vec![],
        };
        
        // Expand the template using unified architecture
        let expanded = expand_template_node(&button_node);
        
        // Should expand into 2 nodes: shape + text
        assert_eq!(expanded.len(), 2, "Button should expand into 2 components");
        
        // Check shape node
        let shape_node = &expanded[0];
        assert_eq!(shape_node.id.as_ref().unwrap(), "save_button_shape");
        assert!(matches!(shape_node.widget_type, WidgetType::Shape { .. }));
        
        // Verify shape inherits position and size from button
        assert_eq!(shape_node.layout.position, Some(bevy_math::Vec3::new(465.0, 245.0, 1.0)));
        assert_eq!(shape_node.layout.size, Some(Vec2::new(120.0, 40.0)));
        
        // Verify shape has green background
        assert_eq!(shape_node.style.background_color, Some(ColorDef::Named("green".to_string())));
        
        // Verify shape is clickable
        assert_eq!(shape_node.behavior.clickable, Some(true));
        
        // Check text node
        let text_node = &expanded[1];
        assert_eq!(text_node.id.as_ref().unwrap(), "save_button_text");
        assert!(matches!(text_node.widget_type, WidgetType::Text { .. }));
        
        if let WidgetType::Text { content, editable } = &text_node.widget_type {
            assert_eq!(content, "Save File");
            assert_eq!(*editable, false);
        }
        
        // Verify text has white color and proper positioning
        assert_eq!(text_node.style.text_color, Some(ColorDef::Named("white".to_string())));
        assert_eq!(text_node.layout.position, Some(bevy_math::Vec3::new(0.0, 0.0, 0.1))); // Relative to parent
        assert_eq!(text_node.behavior.interactive, Some(false)); // Text is not interactive
        assert_eq!(text_node.behavior.z_index, Some(1)); // Above shape
        
        println!("✓ Button template expansion test passed!");
        println!("  - Shape: id={:?}, size={:?}, bg_color={:?}", 
            shape_node.id, shape_node.layout.size, shape_node.style.background_color);
        println!("  - Text: id={:?}, content={:?}, color={:?}", 
            text_node.id, 
            if let WidgetType::Text { content, .. } = &text_node.widget_type { Some(content) } else { None },
            text_node.style.text_color);
    }

    #[test]
    fn test_button_template_defaults() {
        let template = ButtonTemplate::default();
        
        assert_eq!(template.text, "Button");
        assert_eq!(template.background_color, ColorDef::Named("blue".to_string()));
        assert_eq!(template.text_color, ColorDef::Named("white".to_string()));
        assert_eq!(template.size, Vec2::new(100.0, 40.0));
        assert_eq!(template.text_size, 16.0);
        assert!(template.clickable);
    }

    #[test]
    fn test_button_blueprint_creation() {
        let template = ButtonTemplate::default();
        let (shape_blueprint, text_blueprint) = template.create_blueprint(
            "test_button".to_string(),
            None,
            None,
            None,
        );

        // Test shape blueprint
        assert_eq!(shape_blueprint.id, "test_button_shape");
        assert!(matches!(shape_blueprint.widget_type, WidgetType::Shape { .. }));
        assert_eq!(shape_blueprint.children, vec!["test_button_text"]);
        assert_eq!(shape_blueprint.behavior.clickable, Some(true));

        // Test text blueprint
        assert_eq!(text_blueprint.id, "test_button_text");
        assert!(matches!(text_blueprint.widget_type, WidgetType::Text { content, .. } if content == "Button"));
        assert_eq!(text_blueprint.behavior.clickable, Some(false));
        assert!(text_blueprint.children.is_empty());
    }

    #[test]
    fn test_button_template_overrides() {
        let template = ButtonTemplate::default();
        let overrides = TemplateType::Button {
            text: Some("Save File".to_string()),
            background_color: Some(ColorDef::Named("green".to_string())),
            text_color: None,
            size: None,
            text_size: None,
            border_width: None,
            border_color: None,
            border_radius: None,
        };

        let (shape_blueprint, text_blueprint) = template.create_blueprint(
            "custom_button".to_string(),
            Some(&overrides),
            None,
            None,
        );

        // Test overrides applied
        assert_eq!(shape_blueprint.style.background_color, Some(ColorDef::Named("green".to_string())));
        assert!(matches!(text_blueprint.widget_type, WidgetType::Text { content, .. } if content == "Save File"));
        
        // Test defaults preserved
        assert_eq!(text_blueprint.style.text_color, Some(ColorDef::Named("white".to_string())));
    }
}