use bevy_ecs::prelude::*;
use bevy_hierarchy::{Children, Parent};
use bevy_transform::prelude::Transform;
use bevy_window;
use std::sync::Mutex;
use taffy::{TaffyTree, Style, NodeId};

pub mod plugin;
pub mod position_control;

pub use plugin::TaffyLayoutPlugin;
pub use position_control::{PositionControl, LayoutPositioned};

/// Core UI node component that marks an entity as part of the layout system
#[derive(Component, Debug)]
pub struct UiNode {
    /// Optional Taffy node ID for layout calculations
    pub taffy_node: Option<NodeId>,
    /// Whether this node needs layout recalculation
    pub needs_layout: bool,
}

impl Default for UiNode {
    fn default() -> Self {
        Self {
            taffy_node: None,
            needs_layout: true,
        }
    }
}

/// Component that wraps Taffy's Style for layout properties
#[derive(Component, Debug, Clone)]
pub struct Styleable(pub Style);

impl Default for Styleable {
    fn default() -> Self {
        Self(Style::default())
    }
}

/// Component that stores the Taffy node reference for an entity
#[derive(Component, Debug)]
pub struct TaffyNode(pub NodeId);

/// Resource that manages the Taffy layout tree
#[derive(Resource)]
pub struct TaffyResource(pub Mutex<TaffyTree<Entity>>);

impl Default for TaffyResource {
    fn default() -> Self {
        Self(Mutex::new(TaffyTree::new()))
    }
}

impl TaffyResource {
    /// Create a new Taffy resource
    pub fn new() -> Self {
        Self::default()
    }

    /// Get a reference to the Taffy tree (locks the mutex)
    pub fn with_tree<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut TaffyTree<Entity>) -> R,
    {
        let mut tree = self.0.lock().unwrap();
        f(&mut tree)
    }
}

/// Bundle for creating layout-enabled entities
#[derive(Bundle, Default)]
pub struct LayoutBundle {
    pub ui_node: UiNode,
    pub style: Styleable,
}

/// Bundle for entities that already have a Taffy node
#[derive(Bundle)]
pub struct TaffyBundle {
    pub ui_node: UiNode,
    pub style: Styleable,
    pub taffy_node: TaffyNode,
}

/// System set for layout-related systems
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum LayoutSet {
    /// Systems that create or modify Taffy nodes
    CreateNodes,
    /// Systems that calculate layout
    ComputeLayout,
    /// Systems that apply layout results to transforms
    ApplyLayout,
}

/// System that builds the Taffy layout tree from the Bevy ECS hierarchy
pub fn build_taffy_tree_system(
    taffy_resource: ResMut<TaffyResource>,
    mut ui_node_query: Query<(Entity, &mut UiNode, &Styleable), Added<UiNode>>,
    _children_query: Query<&Children>,
    _parent_query: Query<&Parent>,
) {
    // Process newly added UI nodes
    for (entity, mut ui_node, styleable) in ui_node_query.iter_mut() {
        taffy_resource.with_tree(|tree| {
            // Create a new Taffy node for this entity
            let taffy_node = tree.new_leaf(styleable.0.clone()).unwrap();
            ui_node.taffy_node = Some(taffy_node);
            ui_node.needs_layout = true;
            
            bevy_log::debug!("Created Taffy node for entity {:?}", entity);
        });
    }
    
    // TODO: Handle hierarchy changes and parent-child relationships
    // This is a simplified version - full implementation would handle:
    // - Adding children to parent nodes
    // - Removing nodes when entities are despawned
    // - Updating the tree when hierarchy changes
}

/// System that computes layout using Taffy and applies results to Transform components
pub fn compute_and_apply_layout_system(
    taffy_resource: Res<TaffyResource>,
    mut ui_node_query: Query<(Entity, &mut UiNode, &mut Transform, Option<&PositionControl>, Option<&mut LayoutPositioned>), With<Styleable>>,
    mut commands: Commands,
    _children_query: Query<&Children>,
    window_query: Query<&bevy_window::Window, bevy_ecs::query::With<bevy_window::PrimaryWindow>>,
) {
    taffy_resource.with_tree(|tree| {
        // Find root nodes (nodes without parents in the layout tree)
        let mut root_nodes = Vec::new();
        
        for (entity, ui_node, _, position_control, _) in ui_node_query.iter() {
            if let Some(taffy_node) = ui_node.taffy_node {
                let control = position_control.unwrap_or(&PositionControl::Layout);
                // Only include entities that should be positioned by layout
                if control.uses_layout() {
                    root_nodes.push((entity, taffy_node));
                }
            }
        }
        
        // Get window dimensions for coordinate conversion
        let window_height = if let Ok(window) = window_query.get_single() {
            window.height()
        } else {
            300.0 // fallback
        };

        // Compute layout for each root node
        for (root_entity, root_node) in root_nodes {
            // Use a reasonable container size for layout computation
            let available_space = taffy::Size {
                width: taffy::AvailableSpace::Definite(600.0), // Window width
                height: taffy::AvailableSpace::Definite(window_height), // Window height
            };
            
            if let Ok(_) = tree.compute_layout(root_node, available_space) {
                // Apply computed layout to transforms
                apply_layout_to_entity(tree, root_node, root_entity, &mut ui_node_query, &mut commands, window_height);
            }
        }
    });
}

/// Helper function to recursively apply layout to an entity and its children
fn apply_layout_to_entity(
    tree: &TaffyTree<Entity>,
    taffy_node: NodeId,
    entity: Entity,
    ui_node_query: &mut Query<(Entity, &mut UiNode, &mut Transform, Option<&PositionControl>, Option<&mut LayoutPositioned>), With<Styleable>>,
    commands: &mut Commands,
    window_height: f32,
) {
    if let Ok((_, mut ui_node, mut transform, position_control, layout_positioned)) = ui_node_query.get_mut(entity) {
        if let Ok(layout) = tree.layout(taffy_node) {
            let control = position_control.unwrap_or(&PositionControl::Layout);
            
            // Check if we should apply layout positioning
            let should_position = match control {
                PositionControl::Layout => true,
                PositionControl::Manual => false,
                PositionControl::LayoutThenManual => {
                    // Only position if not already positioned
                    layout_positioned.is_none()
                }
            };
            
            if should_position {
                // Convert Taffy's top-left coordinate system to Bevy's bottom-left
                let x = layout.location.x;
                let y = window_height - layout.location.y; // Convert from top-left to bottom-left
                
                // Update the transform with the computed position
                transform.translation.x = x;
                transform.translation.y = y;
                
                bevy_log::debug!("Applied layout to entity {:?}: pos=({}, {}), size=({}, {})", 
                    entity, x, y, layout.size.width, layout.size.height);
                
                // Mark LayoutThenManual entities as positioned
                if matches!(control, PositionControl::LayoutThenManual) && layout_positioned.is_none() {
                    commands.entity(entity).insert(LayoutPositioned);
                }
            }
            
            ui_node.needs_layout = false;
        }
    }
    
    // TODO: Recursively apply to children
}

/// System that updates shape vertices based on computed layout
pub fn update_shape_vertices_system(
    taffy_resource: Res<TaffyResource>,
    mut shape_query: Query<(Entity, &UiNode, &mut crate::gui_framework::components::ShapeData)>,
) {
    for (entity, ui_node, mut shape_data) in shape_query.iter_mut() {
        if let Some(taffy_node) = ui_node.taffy_node {
            taffy_resource.with_tree(|tree| {
                if let Ok(layout) = tree.layout(taffy_node) {
                    // Only scale if the shape allows scaling and has a valid size
                    if !matches!(shape_data.scaling, crate::gui_framework::components::ShapeScaling::Fixed) 
                        && layout.size.width > 0.0 && layout.size.height > 0.0 {
                        
                        shape_data.scale_vertices(layout.size.width, layout.size.height);
                        
                        bevy_log::debug!("Scaled vertices for entity {:?} to size: ({}, {})", 
                            entity, layout.size.width, layout.size.height);
                    }
                }
            });
        }
    }
}