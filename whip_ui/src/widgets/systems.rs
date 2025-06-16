use bevy_ecs::prelude::*;
use bevy_transform::prelude::Transform;
use std::collections::HashMap;
use yrs::{Transact, Text as YrsTextTrait};
use crate::{
    widgets::{
        blueprint::{WidgetBlueprint, WidgetCollection, WidgetType},
        components::*,
    },
    gui_framework::components::{ShapeData, Visibility, Interaction, Text, TextAlignment, EditableText},
    layout::{PositionControl, UiNode, Styleable},
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

/// Recursively spawn a widget and its children
pub fn spawn_widget_recursive(
    commands: &mut Commands,
    blueprint: &WidgetBlueprint,
    collection: &WidgetCollection,
    yrs_res: &YrsDocResource,
    parent: Option<Entity>,
) -> Entity {
    let entity = spawn_widget_entity(commands, blueprint, yrs_res);
    
    // Note: Hierarchy setup will be handled in a separate system pass
    
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

/// Spawn a single widget entity with appropriate components
fn spawn_widget_entity(
    commands: &mut Commands,
    blueprint: &WidgetBlueprint,
    yrs_res: &YrsDocResource,
) -> Entity {
    let layout = WidgetLayout::from(&blueprint.layout);
    let style = WidgetStyle::from(&blueprint.style);
    let behavior = WidgetBehavior::from(&blueprint.behavior);
    
    let transform = Transform::from_translation(layout.computed_position);
    let visibility = Visibility(behavior.visible);
    let interaction = Interaction {
        clickable: behavior.clickable,
        draggable: behavior.draggable,
    };
    
    // Get position control from behavior config, default to Layout
    let position_control = blueprint.behavior.position_control.clone().unwrap_or(PositionControl::Layout);
    
    // Store values we'll need later before they're moved
    let computed_size = layout.computed_size;
    let background_color = style.background_color;
    let text_size = style.text_size.unwrap_or(16.0);
    let text_color = style.text_color.unwrap_or(Color::BLACK);
    
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
        visibility,
        interaction,
        position_control,
        UiNode::default(),
        Styleable(taffy::Style::default()), // TODO: Convert from LayoutConfig to taffy::Style
    ));
    
    // Add widget-type-specific components
    match &blueprint.widget_type {
        WidgetType::Container { direction } => {
            entity_commands.insert(WidgetContainer {
                flex_direction: direction.clone(),
                computed_content_size: Vec2::ZERO,
            });
        }
        
        WidgetType::Button { text, action } => {
            entity_commands.insert((
                WidgetButton {
                    text: text.clone(),
                    action: action.clone(),
                    is_pressed: false,
                },
            ));
            
            // Add visual representation - for now, use a simple rectangle
            if let Some(bg_color) = background_color {
                entity_commands.insert(ShapeData::rectangle(
                    computed_size.x, 
                    computed_size.y, 
                    bg_color
                ));
            }
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
                    bounds: Some(computed_size),
                },
            ));
            
            if *editable {
                entity_commands.insert(EditableText);
                
                // Create YRS text reference for collaborative editing
                let _text_handle = {
                    let text_ref = yrs_res.doc.get_or_insert_text(blueprint.id.as_str());
                    let mut txn = yrs_res.doc.transact_mut();
                    text_ref.insert(&mut txn, 0, content);
                    text_ref
                };
                
                // Note: YRS mapping will be handled after entity is spawned
                // yrs_res.text_map.lock().unwrap().insert(entity_id, text_handle);
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
        // Update transform based on computed layout
        transform.translation = layout.computed_position;
        
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

/// System to handle widget actions
pub fn widget_action_system(
    mut action_events: EventReader<WidgetActionEvent>,
) {
    for event in action_events.read() {
        bevy_log::info!("Widget action: {} from entity {:?}", event.action, event.entity);
        
        // Handle different action types
        match event.action.as_str() {
            "navigate_home" => {
                bevy_log::info!("Navigating to home");
            }
            "open_settings" => {
                bevy_log::info!("Opening settings");
            }
            "show_widgets" => {
                bevy_log::info!("Showing widgets panel");
            }
            "show_examples" => {
                bevy_log::info!("Showing examples panel");
            }
            "show_docs" => {
                bevy_log::info!("Showing documentation");
            }
            _ => {
                bevy_log::info!("Unknown action: {}", event.action);
            }
        }
    }
}