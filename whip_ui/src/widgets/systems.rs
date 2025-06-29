use bevy_ecs::prelude::*;
use bevy_transform::prelude::{Transform, GlobalTransform};
use bevy_math::Vec3;
use std::collections::HashMap;
use yrs::{Transact, Text as YrsTextTrait};
use crate::{
    widgets::{
        blueprint::{WidgetBlueprint, WidgetCollection, WidgetType},
        components::*,
        templates::{get_widget_templates, TemplateType},
    },
    gui_framework::components::{ShapeData, Visibility, Interaction, InteractionState, Text, TextAlignment, EditableText},
    layout::{PositionControl, UiNode, Styleable, coordinate_system::{BevyCoords, create_ui_transform, update_ui_transform}},
    Vertex, YrsDocResource,
};
use bevy_color::Color;
use bevy_math::Vec2;
use taffy;

/// Resource containing loaded widget collections
#[derive(Resource)]
pub struct WidgetRegistry {
    pub collections: HashMap<String, WidgetCollection>,
}

impl Default for WidgetRegistry {
    fn default() -> Self {
        Self {
            collections: HashMap::new(),
        }
    }
}

/// Event to trigger widget spawning from TOML
#[derive(Event)]
pub struct SpawnWidgetEvent {
    pub collection_id: String,
    pub widget_id: Option<String>, // None = spawn root widget
}

/// Event for widget actions (button clicks, etc.)
#[derive(Event)]
pub struct WidgetActionEvent {
    pub entity: Entity,
    pub action: String,
}

/// System to load widget collections from TOML
pub fn load_widget_collection_system(
    registry: ResMut<WidgetRegistry>,
    mut spawn_events: EventReader<SpawnWidgetEvent>,
) {
    for event in spawn_events.read() {
        // In a real implementation, this would load from files
        // For now, we'll just register the collection if not already loaded
        if !registry.collections.contains_key(&event.collection_id) {
            bevy_log::info!("Widget collection '{}' not found in registry", event.collection_id);
        }
    }
}

/// System to spawn widgets from blueprints
pub fn spawn_widget_system(
    mut commands: Commands,
    mut spawn_events: EventReader<SpawnWidgetEvent>,
    registry: Res<WidgetRegistry>,
    yrs_res: Res<YrsDocResource>,
) {
    for event in spawn_events.read() {
        if let Some(collection) = registry.collections.get(&event.collection_id) {
            let widget_id = event.widget_id.as_ref()
                .or(collection.root.as_ref())
                .cloned();
                
            if let Some(id) = widget_id {
                if let Some(blueprint) = collection.get_widget(&id) {
                    spawn_widget_recursive(&mut commands, blueprint, collection, &yrs_res, None);
                }
            }
        }
    }
}

/// Expand template widgets into primitive blueprints
pub fn expand_template_widget(blueprint: &WidgetBlueprint) -> Vec<WidgetBlueprint> {
    match &blueprint.widget_type {
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
            // Create template override from TOML values
            let template_override = TemplateType::Button {
                text: text.clone(),
                background_color: background_color.clone(),
                text_color: text_color.clone(),
                size: *size,
                text_size: *text_size,
                border_width: *border_width,
                border_color: border_color.clone(),
                border_radius: *border_radius,
            };

            // Get templates and create blueprints
            let templates = get_widget_templates();
            let (shape_blueprint, text_blueprint) = templates.button.create_blueprint(
                blueprint.id.clone(),
                Some(&template_override),
                Some(blueprint.layout.clone()),
                Some(blueprint.behavior.clone()),
            );

            vec![shape_blueprint, text_blueprint]
        }
        _ => {
            // Not a template widget, return as-is
            vec![blueprint.clone()]
        }
    }
}

/// Recursively spawn a widget and its children
pub fn spawn_widget_recursive(
    commands: &mut Commands,
    blueprint: &WidgetBlueprint,
    collection: &WidgetCollection,
    yrs_res: &YrsDocResource,
    parent: Option<Entity>,
) -> Entity {
    // Check if this is a template widget that needs expansion
    let expanded_blueprints = expand_template_widget(blueprint);
    
    if expanded_blueprints.len() > 1 {
        // This was a template widget - spawn the hierarchy
        let shape_blueprint = &expanded_blueprints[0]; // Shape (parent)
        let text_blueprint = &expanded_blueprints[1];  // Text (child)
        
        // Spawn shape entity (parent)
        let shape_entity = spawn_widget_entity(commands, shape_blueprint, yrs_res);
        
        // Spawn text entity (child)
        let text_entity = spawn_widget_entity(commands, text_blueprint, yrs_res);
        
        // Set up parent-child relationship
        commands.entity(shape_entity).insert(WidgetHierarchy {
            parent,
            children: vec![text_entity],
        });
        
        commands.entity(text_entity).insert(WidgetHierarchy {
            parent: Some(shape_entity),
            children: vec![],
        });
        
        // Return the shape entity (parent) as the main entity
        shape_entity
    } else {
        // Regular primitive widget
        let entity = spawn_widget_entity(commands, blueprint, yrs_res);
        
        // Spawn children
        let mut child_entities = Vec::new();
        for child_id in &blueprint.children {
            if let Some(child_blueprint) = collection.get_widget(child_id) {
                let child_entity = spawn_widget_recursive(commands, child_blueprint, collection, yrs_res, Some(entity));
                child_entities.push(child_entity);
            }
        }
        
        // Update hierarchy component
        commands.entity(entity).insert(WidgetHierarchy {
            parent,
            children: child_entities,
        });
        
        entity
    }
}

/// Spawn a single widget entity directly from WidgetNode (unified architecture)
pub fn spawn_widget_entity_from_node(
    commands: &mut Commands,
    node: &crate::assets::definitions::WidgetNode,
    yrs_res: &YrsDocResource,
    window_height: f32,
    _parent_entity: Option<Entity>,
    _parent_position: Option<Vec3>,
) -> Entity {
    let widget_id = node.id.clone().unwrap_or_else(|| "unnamed".to_string());
    bevy_log::debug!("🏗️  Spawning entity for widget '{}'", widget_id);
    
    let layout = WidgetLayout::from(&node.layout);
    let style = WidgetStyle::from(&node.style);
    let behavior = WidgetBehavior::from(&node.behavior);
    
    // Get position control from behavior config, default to Layout
    let position_control = node.behavior.position_control.clone().unwrap_or(PositionControl::Layout);
    let uses_layout = position_control.uses_layout();
    
    
    // Convert TOML coordinates to Bevy coordinates for initial Transform
    // Use z_index from behavior for the z-coordinate
    let z_coord = behavior.z_index as f32;
    
    let initial_bevy_position = match layout.position {
        Some(toml_pos) => {
            match position_control {
                PositionControl::Manual => {
                    // For manual positioning, convert TOML coords to Bevy coords immediately
                    let mut pos = toml_pos.to_bevy(window_height);
                    // Override z with z_index from behavior
                    pos = BevyCoords::new(pos.raw().x, pos.raw().y, z_coord);
                    pos
                }
                PositionControl::Layout | PositionControl::LayoutThenManual => {
                    // For layout positioning, position is only used for initial placement
                    // Use 0,0 for x,y and z_index for z
                    BevyCoords::new(0.0, 0.0, z_coord)
                }
            }
        }
        None => BevyCoords::new(0.0, 0.0, z_coord),
    };
    
    // Create transform using typed coordinates
    let transform = create_ui_transform(initial_bevy_position);
    bevy_log::debug!("   🎯 Entity '{}' TOML position: {:?} -> Bevy position: {:?} -> Transform: {:?}", 
        widget_id, node.layout.position, initial_bevy_position.raw(), transform.translation);
    let visibility = Visibility(behavior.visible);
    let interaction = Interaction {
        clickable: behavior.clickable,
        draggable: behavior.draggable,
    };
    
    // Store values we'll need later before they're moved
    let computed_size = layout.computed_size;
    let background_color = style.background_color;
    let text_size = style.text_size.unwrap_or(16.0);
    let text_color = style.text_color.unwrap_or(Color::BLACK);
    let is_interactive = behavior.clickable || behavior.draggable;
    
    
    let mut entity_commands = commands.spawn((
        Widget {
            id: widget_id.clone(),
            blueprint: WidgetBlueprint {
                id: widget_id.clone(),
                widget_type: node.widget_type.clone(),
                layout: node.layout.clone(),
                style: node.style.clone(),
                behavior: node.behavior.clone(),
                children: vec![], // Not used in unified architecture
            },
        },
        layout,
        style,
        behavior,
        WidgetState::default(),
        transform,
        GlobalTransform::default(), // Bevy will compute the correct world transform
        visibility,
        interaction,
        position_control.clone(),
    ));
    
    // Only add UiNode and Styleable components for widgets that use layout positioning
    if uses_layout {
        entity_commands.insert((
            UiNode::default(),
            Styleable(convert_layout_config_to_taffy_style(&node.layout, &position_control)),
        ));
    }
    
    // Add InteractionState component for interactive widgets
    if is_interactive {
        entity_commands.insert(InteractionState::new());
    }
    
    // Add widget-type-specific components
    match &node.widget_type {
        WidgetType::Container { direction } => {
            entity_commands.insert(WidgetContainer {
                flex_direction: direction.clone(),
                computed_content_size: Vec2::ZERO,
            });
        }
        
        WidgetType::Button { .. } => {
            // Button templates should be expanded before reaching this point
            bevy_log::error!("Button template was not expanded before entity spawning for widget '{}'", widget_id);
        }
        
        WidgetType::Text { content, editable } => {
            entity_commands.insert((
                WidgetText {
                    content: content.clone(),
                    editable: *editable,
                    cursor_position: 0,
                    selection_start: None,
                    selection_end: None,
                },
                Text {
                    size: text_size,
                    color: text_color,
                    alignment: TextAlignment::Left,
                    bounds: None, // Let text system calculate dynamic bounds
                },
            ));
            
            // ALL text gets YRS mapping for network sync
            let text_handle = {
                let text_ref = yrs_res.doc.get_or_insert_text(widget_id.as_str());
                let mut txn = yrs_res.doc.transact_mut();
                text_ref.insert(&mut txn, 0, content);
                text_ref
            };
            
            // Get the entity ID and map it to the YRS text handle
            let entity_id = entity_commands.id();
            if let Ok(mut text_map) = yrs_res.text_map.lock() {
                text_map.insert(entity_id, text_handle);
                bevy_log::debug!("Mapped Entity {:?} to YrsText '{}' (editable: {})", entity_id, widget_id, editable);
            } else {
                bevy_log::error!("Failed to lock text_map mutex for entity {:?}", entity_id);
            }
            
            // Only editable text gets the EditableText component for user interaction
            if *editable {
                entity_commands.insert(EditableText);
            }
        }
        
        WidgetType::Shape { shape_type } => {
            let vertices = create_shape_vertices(shape_type, computed_size);
            entity_commands.insert(WidgetShape {
                shape_type: shape_type.clone(),
                vertices: vertices.clone(),
            });
            
            if let Some(bg_color) = background_color {
                entity_commands.insert(ShapeData::new(vertices, bg_color));
                bevy_log::debug!("✓ Created Shape entity '{}' with background color {:?} and size {:?}", 
                    widget_id, bg_color, computed_size);
            } else {
                bevy_log::error!("✗ Shape entity '{}' has no background color - will not be visible!", widget_id);
            }
        }
        
        WidgetType::Custom { component: _, properties } => {
            // Store custom properties in widget state
            let mut custom_data = HashMap::new();
            for (key, value) in properties {
                custom_data.insert(key.clone(), value.to_string());
            }
            entity_commands.insert(WidgetState {
                custom_data,
                ..Default::default()
            });
        }
    }
    
    entity_commands.id()
}

/// Spawn a single widget entity with appropriate components (legacy - for backward compatibility)
pub fn spawn_widget_entity(
    commands: &mut Commands,
    blueprint: &WidgetBlueprint,
    yrs_res: &YrsDocResource,
) -> Entity {
    let layout = WidgetLayout::from(&blueprint.layout);
    let style = WidgetStyle::from(&blueprint.style);
    let behavior = WidgetBehavior::from(&blueprint.behavior);
    
    // Get position control from behavior config, default to Layout
    let position_control = blueprint.behavior.position_control.clone().unwrap_or(PositionControl::Layout);
    
    // Use the computed position from TOML as the initial Transform
    // Taffy will override this for Layout and LayoutThenManual entities
    let mut initial_position = layout.computed_position;
    // Override z with z_index from behavior
    initial_position = BevyCoords::new(
        initial_position.raw().x, 
        initial_position.raw().y, 
        behavior.z_index as f32
    );
    let transform = create_ui_transform(initial_position);
    let visibility = Visibility(behavior.visible);
    let interaction = Interaction {
        clickable: behavior.clickable,
        draggable: behavior.draggable,
    };
    
    // Store values we'll need later before they're moved
    let computed_size = layout.computed_size;
    let background_color = style.background_color;
    let text_size = style.text_size.unwrap_or(16.0);
    let text_color = style.text_color.unwrap_or(Color::BLACK);
    let is_interactive = behavior.clickable || behavior.draggable;
    
    let mut entity_commands = commands.spawn((
        Widget {
            id: blueprint.id.clone(),
            blueprint: blueprint.clone(),
        },
        layout,
        style,
        behavior,
        WidgetState::default(),
        transform,
        GlobalTransform::default(), // Bevy will compute the correct world transform
        visibility,
        interaction,
        position_control.clone(),
        UiNode::default(),
        Styleable(convert_layout_config_to_taffy_style(&blueprint.layout, &position_control)),
    ));
    
    // Add InteractionState component for interactive widgets
    if is_interactive {
        entity_commands.insert(InteractionState::new());
    }
    
    // Add widget-type-specific components
    match &blueprint.widget_type {
        WidgetType::Container { direction } => {
            entity_commands.insert(WidgetContainer {
                flex_direction: direction.clone(),
                computed_content_size: Vec2::ZERO,
            });
        }
        
        WidgetType::Button { .. } => {
            // Button templates are handled by template expansion system
            // This case should not be reached as templates expand before entity spawning
            bevy_log::error!("Button template was not expanded before entity spawning");
        }
        
        WidgetType::Text { content, editable } => {
            entity_commands.insert((
                WidgetText {
                    content: content.clone(),
                    editable: *editable,
                    cursor_position: 0,
                    selection_start: None,
                    selection_end: None,
                },
                Text {
                    size: text_size,
                    color: text_color,
                    alignment: TextAlignment::Left,
                    bounds: None, // Let text system calculate dynamic bounds
                },
            ));
            
            // ALL text gets YRS mapping for network sync
            // Create YRS text reference for network sync
            let text_handle = {
                let text_ref = yrs_res.doc.get_or_insert_text(blueprint.id.as_str());
                let mut txn = yrs_res.doc.transact_mut();
                text_ref.insert(&mut txn, 0, content);
                text_ref
            };
            
            // Get the entity ID and map it to the YRS text handle
            let entity_id = entity_commands.id();
            if let Ok(mut text_map) = yrs_res.text_map.lock() {
                text_map.insert(entity_id, text_handle);
                bevy_log::debug!("Mapped Entity {:?} to YrsText '{}' (editable: {})", entity_id, blueprint.id, editable);
            } else {
                bevy_log::error!("Failed to lock text_map mutex for entity {:?}", entity_id);
            }
            
            // Only editable text gets the EditableText component for user interaction
            if *editable {
                entity_commands.insert(EditableText);
            }
        }
        
        WidgetType::Shape { shape_type } => {
            entity_commands.insert(WidgetShape {
                shape_type: shape_type.clone(),
                vertices: create_shape_vertices(shape_type, computed_size),
            });
            
            if let Some(bg_color) = background_color {
                let vertices = create_shape_vertices(shape_type, computed_size);
                entity_commands.insert(ShapeData::new(vertices, bg_color));
            }
        }
        
        WidgetType::Custom { component: _, properties } => {
            // Store custom properties in widget state
            let mut custom_data = HashMap::new();
            for (key, value) in properties {
                custom_data.insert(key.clone(), value.to_string());
            }
            entity_commands.insert(WidgetState {
                custom_data,
                ..Default::default()
            });
        }
    }
    
    entity_commands.id()
}

/// Create rectangle vertices for the given size
fn create_rectangle_vertices(size: Vec2) -> Vec<Vertex> {
    let half_width = size.x / 2.0;
    let half_height = size.y / 2.0;
    
    vec![
        // Triangle 1
        Vertex { position: [-half_width, -half_height] },
        Vertex { position: [-half_width, half_height] },
        Vertex { position: [half_width, -half_height] },
        // Triangle 2
        Vertex { position: [half_width, -half_height] },
        Vertex { position: [-half_width, half_height] },
        Vertex { position: [half_width, half_height] },
    ]
}

/// Create vertices for different shape types
fn create_shape_vertices(shape_type: &crate::widgets::blueprint::ShapeType, size: Vec2) -> Vec<Vertex> {
    match shape_type {
        crate::widgets::blueprint::ShapeType::Rectangle => create_rectangle_vertices(size),
        crate::widgets::blueprint::ShapeType::Circle => create_circle_vertices(size, 32),
        crate::widgets::blueprint::ShapeType::Triangle => create_triangle_vertices(size),
        crate::widgets::blueprint::ShapeType::Custom { vertices } => {
            vertices.iter().map(|v| Vertex { position: [v.x, v.y] }).collect()
        }
    }
}

/// Create circle vertices (approximated with triangles)
fn create_circle_vertices(size: Vec2, segments: usize) -> Vec<Vertex> {
    let mut vertices = Vec::new();
    let radius = size.x.min(size.y) / 2.0;
    let center = Vertex { position: [0.0, 0.0] };
    
    for i in 0..segments {
        let angle1 = (i as f32 / segments as f32) * 2.0 * std::f32::consts::PI;
        let angle2 = ((i + 1) as f32 / segments as f32) * 2.0 * std::f32::consts::PI;
        
        vertices.push(center);
        vertices.push(Vertex {
            position: [radius * angle1.cos(), radius * angle1.sin()],
        });
        vertices.push(Vertex {
            position: [radius * angle2.cos(), radius * angle2.sin()],
        });
    }
    
    vertices
}

/// Create triangle vertices
fn create_triangle_vertices(size: Vec2) -> Vec<Vertex> {
    let half_width = size.x / 2.0;
    let half_height = size.y / 2.0;
    
    vec![
        Vertex { position: [0.0, half_height] },      // Top
        Vertex { position: [-half_width, -half_height] }, // Bottom left
        Vertex { position: [half_width, -half_height] },  // Bottom right
    ]
}

/// System to handle widget layout updates
pub fn widget_layout_system(
    mut widget_query: Query<(&WidgetLayout, &mut Transform), With<Widget>>,
    _state_query: Query<&WidgetState, With<Widget>>,
) {
    for (layout, mut transform) in widget_query.iter_mut() {
        // Update transform based on computed layout using typed coordinates
        update_ui_transform(&mut transform, layout.computed_position);
        
        // In a full implementation, this would handle:
        // - Flex layout calculations
        // - Constraint solving
        // - Parent-child size dependencies
        // - Text wrapping and sizing
    }
}

/// System to handle widget interaction updates
pub fn widget_interaction_system(
    mut widget_query: Query<(&WidgetBehavior, &Interaction), With<Widget>>,
    _action_events: EventWriter<WidgetActionEvent>,
    button_query: Query<(Entity, &WidgetButton), With<Widget>>,
) {
    // Handle button clicks
    for (entity, button) in button_query.iter() {
        if let Ok((behavior, interaction)) = widget_query.get_mut(entity) {
            // This would be connected to actual input handling
            // For now, it's just a placeholder structure
            if behavior.clickable && interaction.clickable {
                if let Some(_action) = &button.action {
                    // Would trigger on actual click events
                    // action_events.send(WidgetActionEvent {
                    //     entity,
                    //     action: action.clone(),
                    // });
                }
            }
        }
    }
}

/// Convert LayoutConfig to Taffy Style for layout calculations
fn convert_layout_config_to_taffy_style(
    layout_config: &crate::widgets::blueprint::LayoutConfig,
    position_control: &PositionControl,
) -> taffy::Style {
    let mut style = taffy::Style::default();
    
    // Set positioning based on position control
    match position_control {
        PositionControl::Manual | PositionControl::LayoutThenManual => {
            // Manual items use absolute positioning
            style.position = taffy::Position::Absolute;
            // For manual positioning, we'll need to handle coordinates differently
            if let Some(position) = layout_config.position {
                style.inset = taffy::Rect {
                    left: taffy::LengthPercentageAuto::Length(position.x),
                    top: taffy::LengthPercentageAuto::Length(position.y),
                    right: taffy::LengthPercentageAuto::Auto,
                    bottom: taffy::LengthPercentageAuto::Auto,
                };
            }
        }
        PositionControl::Layout => {
            // Layout items use grid positioning (relative within grid cell)
            style.position = taffy::Position::Relative;
            // Grid alignment is handled by the parent container
        }
    }
    
    // Convert size if specified
    if let Some(size) = layout_config.size {
        style.size = taffy::Size {
            width: taffy::Dimension::Length(size.x),
            height: taffy::Dimension::Length(size.y),
        };
    }
    
    // Convert margin if specified
    if let Some(margin) = &layout_config.margin {
        style.margin = taffy::Rect {
            left: taffy::LengthPercentageAuto::Length(margin.left),
            right: taffy::LengthPercentageAuto::Length(margin.right),
            top: taffy::LengthPercentageAuto::Length(margin.top),
            bottom: taffy::LengthPercentageAuto::Length(margin.bottom),
        };
    }
    
    // Convert padding if specified
    if let Some(padding) = &layout_config.padding {
        style.padding = taffy::Rect {
            left: taffy::LengthPercentage::Length(padding.left),
            right: taffy::LengthPercentage::Length(padding.right),
            top: taffy::LengthPercentage::Length(padding.top),
            bottom: taffy::LengthPercentage::Length(padding.bottom),
        };
    }
    
    // Convert flex properties if specified
    if let Some(flex_grow) = layout_config.flex_grow {
        style.flex_grow = flex_grow;
    }
    
    if let Some(flex_shrink) = layout_config.flex_shrink {
        style.flex_shrink = flex_shrink;
    }
    
    // Convert grid positioning if specified
    if let Some(grid_row) = layout_config.grid_row {
        // Convert 1-based TOML values for Taffy (Taffy uses 1-based grid lines like CSS)
        style.grid_row = taffy::Line { 
            start: taffy::GridPlacement::Line((grid_row as i16).into()), 
            end: taffy::GridPlacement::Auto 
        };
    }
    
    if let Some(grid_column) = layout_config.grid_column {
        // Convert 1-based TOML values for Taffy (Taffy uses 1-based grid lines like CSS)
        style.grid_column = taffy::Line { 
            start: taffy::GridPlacement::Line((grid_column as i16).into()), 
            end: taffy::GridPlacement::Auto 
        };
    }
    
    style
}

/// System to handle widget actions
pub fn widget_action_system(
    mut action_events: EventReader<WidgetActionEvent>,
) {
    for event in action_events.read() {
        bevy_log::debug!("Widget action: {} from entity {:?}", event.action, event.entity);
        
        // Handle different action types
        match event.action.as_str() {
            "navigate_home" => {
                bevy_log::debug!("Navigating to home");
            }
            "open_settings" => {
                bevy_log::debug!("Opening settings");
            }
            "show_widgets" => {
                bevy_log::debug!("Showing widgets panel");
            }
            "show_examples" => {
                bevy_log::debug!("Showing examples panel");
            }
            "show_docs" => {
                bevy_log::debug!("Showing documentation");
            }
            _ => {
                bevy_log::debug!("Unknown action: {}", event.action);
            }
        }
    }
}

/// Debug system to log all shape entities and their visibility (reduced frequency)
pub fn debug_shape_visibility_system(
    shape_query: Query<(Entity, &crate::gui_framework::components::ShapeData, &Transform, &crate::gui_framework::components::Visibility)>,
    widget_query: Query<(Entity, &Widget)>,
    mut frame_count: Local<u32>,
) {
    *frame_count += 1;
    
    // Log every 120 frames (2 seconds at 60fps) to reduce spam
    if *frame_count % 120 == 1 {
        bevy_log::debug!("=== SHAPE VISIBILITY DEBUG (Frame {}) ===", *frame_count);
        
        let widget_count = widget_query.iter().count();
        let shape_count = shape_query.iter().count();
        
        bevy_log::debug!("All widgets found: {}", widget_count);
        for (entity, widget) in widget_query.iter() {
            bevy_log::debug!("  Widget '{}' (Entity {:?})", widget.id, entity);
        }
        
        bevy_log::debug!("All shapes found: {}", shape_count);
        for (entity, shape, transform, visibility) in shape_query.iter() {
            bevy_log::debug!(
                "  Shape Entity {:?}: visible={}, pos={:?}, color={:?}, vertices={}",
                entity,
                visibility.0,
                transform.translation,
                shape.color,
                shape.vertices.len()
            );
        }
        
        if widget_count == 0 && *frame_count > 60 {
            bevy_log::error!("❌ NO WIDGETS FOUND after {} frames - widgets may not be spawning with Widget component", *frame_count);
        }
        
        if shape_count <= 1 && *frame_count > 60 {
            bevy_log::error!("❌ ONLY BACKGROUND SHAPE FOUND after {} frames - widget shapes missing ShapeData component", *frame_count);
        }
        
        bevy_log::debug!("=== END SHAPE DEBUG ===");
    }
}

/// Debug system to track red rectangle position changes
pub fn debug_red_rectangle_position_system(
    widget_query: Query<(Entity, &Widget, &Transform, &PositionControl), With<Widget>>,
    mut frame_count: Local<u32>,
    mut last_position: Local<Option<Vec3>>,
) {
    *frame_count += 1;
    
    // Look for the red rectangle
    for (entity, widget, transform, position_control) in widget_query.iter() {
        if widget.id == "test_red_rect" {
            let current_position = transform.translation;
            
            // Check if position changed
            if let Some(last_pos) = *last_position {
                if (current_position - last_pos).length() > 0.01 {
                    bevy_log::debug!("🔴 RED RECTANGLE POSITION CHANGED (Frame {}):", *frame_count);
                    bevy_log::debug!("   Entity: {:?}", entity);
                    bevy_log::debug!("   Previous position: {:?}", last_pos);
                    bevy_log::debug!("   Current position: {:?}", current_position);
                    bevy_log::debug!("   Delta: {:?}", current_position - last_pos);
                    bevy_log::debug!("   Position control: {:?}", position_control);
                }
            } else {
                bevy_log::debug!("🔴 RED RECTANGLE INITIAL POSITION (Frame {}):", *frame_count);
                bevy_log::debug!("   Entity: {:?}", entity);
                bevy_log::debug!("   Position: {:?}", current_position);
                bevy_log::debug!("   Position control: {:?}", position_control);
            }
            
            *last_position = Some(current_position);
            break;
        }
    }
}

/// Debug system to verify widget spawning and coordinate conversion
#[cfg(debug_assertions)]
pub fn debug_widget_spawning_system(
    widget_query: Query<(Entity, &Widget, &Transform, &WidgetLayout), Added<Widget>>,
) {
    for (entity, widget, transform, layout) in widget_query.iter() {
        bevy_log::debug!(
            "🔍 Widget '{}' spawned: Entity={:?}, TOML pos={:?}, Bevy pos={:?}, computed pos={:?}",
            widget.id,
            entity,
            layout.position,
            transform.translation,
            layout.computed_position.raw()
        );
        
        // Verify coordinate conversion matches expectation
        if let Some(toml_pos) = layout.position {
            use crate::layout::coordinate_system::{TomlCoords, BevyCoords};
            let expected_bevy = TomlCoords::from(toml_pos).to_bevy(300.0); // Window height from main.toml
            let actual_bevy = BevyCoords::from(transform.translation);
            
            if (expected_bevy.raw() - actual_bevy.raw()).length() > 0.01 {
                bevy_log::warn!(
                    "⚠️  Coordinate mismatch for widget '{}': expected={:?}, actual={:?}",
                    widget.id,
                    expected_bevy.raw(),
                    actual_bevy.raw()
                );
            } else {
                bevy_log::debug!("✅ Coordinate conversion correct for widget '{}'", widget.id);
            }
        }
    }
}

/// Debug system to track component assignment and entity lifecycle issues
#[cfg(debug_assertions)]
pub fn debug_entity_components_system(
    // Track entities with different component combinations
    widget_only: Query<Entity, (With<Widget>, Without<crate::gui_framework::components::ShapeData>)>,
    shape_only: Query<Entity, (With<crate::gui_framework::components::ShapeData>, Without<Widget>)>,
    widget_and_shape: Query<Entity, (With<Widget>, With<crate::gui_framework::components::ShapeData>)>,
    no_global_transform: Query<Entity, (With<Transform>, Without<bevy_transform::prelude::GlobalTransform>)>,
    mut frame_count: Local<u32>,
) {
    *frame_count += 1;
    
    // Only log on specific frames to avoid spam
    if *frame_count == 120 || *frame_count == 300 { // 2 seconds and 5 seconds
        bevy_log::debug!("🔧 ENTITY COMPONENT DEBUG (Frame {}):", *frame_count);
        
        let widget_only_count = widget_only.iter().count();
        let shape_only_count = shape_only.iter().count();  
        let widget_and_shape_count = widget_and_shape.iter().count();
        let no_global_transform_count = no_global_transform.iter().count();
        
        bevy_log::debug!("   Widget-only entities: {}", widget_only_count);
        bevy_log::debug!("   Shape-only entities: {}", shape_only_count);
        bevy_log::debug!("   Widget+Shape entities: {}", widget_and_shape_count);
        bevy_log::debug!("   Missing GlobalTransform: {}", no_global_transform_count);
        
        if widget_only_count > 0 {
            bevy_log::warn!("⚠️  Found {} widgets without ShapeData - these won't render as shapes", widget_only_count);
            for (i, entity) in widget_only.iter().enumerate() {
                if i < 3 {
                    bevy_log::debug!("     Widget-only entity: {:?}", entity);
                }
            }
        }
        
        if shape_only_count > 1 { // >1 because background is shape-only
            bevy_log::debug!("   Found {} shape-only entities (including background)", shape_only_count);
        }
        
        if no_global_transform_count > 0 {
            bevy_log::error!("❌ Found {} entities missing GlobalTransform - Bevy transform propagation issue", no_global_transform_count);
        }
        
        if widget_and_shape_count > 0 {
            bevy_log::debug!("✅ Found {} entities with both Widget and ShapeData", widget_and_shape_count);
        }
    }
}

/// Runtime test system to verify UI elements are properly spawned after asset loading
#[cfg(debug_assertions)]
pub fn runtime_ui_verification_system(
    widget_query: Query<Entity, With<Widget>>,
    shape_query: Query<(Entity, &crate::gui_framework::components::ShapeData, &crate::gui_framework::components::Visibility)>,
    text_query: Query<Entity, With<crate::gui_framework::components::Text>>,
    // Query to check GlobalTransform propagation
    global_transform_query: Query<Entity, With<bevy_transform::prelude::GlobalTransform>>,
    // Query to check rendering-ready entities
    render_ready_query: Query<Entity, (With<crate::gui_framework::components::ShapeData>, With<bevy_transform::prelude::GlobalTransform>, With<crate::gui_framework::components::Visibility>)>,
    mut frame_count: Local<u32>,
) {
    *frame_count += 1;
    
    // Run verification at multiple intervals
    let should_log = match *frame_count {
        60 => true,  // 1 second
        180 => true, // 3 seconds  
        300 => true, // 5 seconds
        _ => *frame_count % 300 == 0, // Every 5 seconds after that
    };
    
    if should_log {
        let widget_count = widget_query.iter().count();
        let shape_count = shape_query.iter().count();
        let text_count = text_query.iter().count();
        let global_transform_count = global_transform_query.iter().count();
        let render_ready_count = render_ready_query.iter().count();
        
        bevy_log::debug!("🧪 RUNTIME UI VERIFICATION (Frame {}):", *frame_count);
        bevy_log::debug!("   Widget entities: {}", widget_count);
        bevy_log::debug!("   Shape entities: {}", shape_count);
        bevy_log::debug!("   Text entities: {}", text_count);
        bevy_log::debug!("   GlobalTransform entities: {}", global_transform_count);
        bevy_log::debug!("   Render-ready entities: {}", render_ready_count);
        
        // Expected counts (without button): main_container + triangle + square + sample_text + test_red_rect
        let expected_widgets = 5;
        let expected_shapes = 3; // triangle, square, red rectangle (+ background = 4 total)
        
        // Detailed analysis
        if widget_count == 0 {
            bevy_log::error!("❌ CRITICAL: No Widget components found - widget spawning failed completely");
        } else if widget_count < expected_widgets {
            bevy_log::warn!("⚠️  Widget count lower than expected: {} < {}", widget_count, expected_widgets);
        } else {
            bevy_log::debug!("✅ Widget count looks good: {}", widget_count);
        }
        
        if shape_count <= 1 { // Only background
            bevy_log::error!("❌ CRITICAL: Only background shape found - widget shapes missing ShapeData");
        } else if shape_count < expected_shapes + 1 { // +1 for background
            bevy_log::warn!("⚠️  Shape count lower than expected: {} < {}", shape_count, expected_shapes + 1);
        } else {
            bevy_log::debug!("✅ Shape count looks good: {}", shape_count);
        }
        
        if render_ready_count <= 1 { // Only background
            bevy_log::error!("❌ CRITICAL: Only background entity render-ready - widgets missing required components");
            bevy_log::debug!("   Required for rendering: ShapeData + GlobalTransform + Visibility");
        } else {
            bevy_log::debug!("✅ Found {} render-ready entities", render_ready_count);
        }
        
        // Log first few entities for debugging
        bevy_log::debug!("   Widget entities:");
        for (i, widget_entity) in widget_query.iter().enumerate() {
            if i < 3 { // Only first 3 to avoid spam
                bevy_log::debug!("     {:?}", widget_entity);
            }
        }
        
        bevy_log::debug!("   Shape entities:");
        for (i, (shape_entity, _, visibility)) in shape_query.iter().enumerate() {
            if i < 5 { // Only first 5 to avoid spam
                bevy_log::debug!("     {:?} (visible: {})", shape_entity, visibility.is_visible());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::blueprint::{WidgetType, ColorDef, LayoutConfig, StyleConfig, BehaviorConfig};
    use bevy_math::Vec2;
    
    #[test]
    fn test_button_template_expansion() {
        // Create a button blueprint
        let button_blueprint = WidgetBlueprint {
            id: "test_button".to_string(),
            widget_type: WidgetType::Button {
                text: Some("Test Button".to_string()),
                background_color: Some(ColorDef::Named("green".to_string())),
                text_color: Some(ColorDef::Named("white".to_string())),
                size: Some(Vec2::new(120.0, 40.0)),
                text_size: Some(16.0),
                border_width: None,
                border_color: None,
                border_radius: None,
            },
            layout: LayoutConfig::default(),
            style: StyleConfig::default(),
            behavior: BehaviorConfig::default(),
            children: vec![],
        };
        
        // Expand the template
        let expanded = expand_template_widget(&button_blueprint);
        
        // Should expand into 2 blueprints: shape + text
        assert_eq!(expanded.len(), 2, "Button should expand into 2 components");
        
        // Check shape blueprint
        let shape_blueprint = &expanded[0];
        assert_eq!(shape_blueprint.id, "test_button_shape");
        assert!(matches!(shape_blueprint.widget_type, WidgetType::Shape { .. }));
        
        // Check text blueprint  
        let text_blueprint = &expanded[1];
        assert_eq!(text_blueprint.id, "test_button_text");
        assert!(matches!(text_blueprint.widget_type, WidgetType::Text { .. }));
        
        if let WidgetType::Text { content, editable } = &text_blueprint.widget_type {
            assert_eq!(content, "Test Button");
            assert_eq!(*editable, false);
        }
        
        println!("Button template expansion test passed!");
    }
}