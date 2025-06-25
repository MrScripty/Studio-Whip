use bevy_ecs::prelude::*;
use bevy_hierarchy::{Children, Parent};
use bevy_transform::prelude::Transform;
use bevy_math::Vec3;
use bevy_window;
use std::sync::Mutex;
use taffy::{TaffyTree, Style, NodeId};

pub mod plugin;
pub mod position_control;
pub mod coordinate_system;

pub use plugin::TaffyLayoutPlugin;
pub use position_control::{PositionControl, LayoutPositioned};
pub use coordinate_system::{TomlCoords, BevyCoords, TaffyCoords, VulkanCoords, create_ui_transform, update_ui_transform};

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

/// Resource to store the window root container node and track window dimensions
#[derive(Resource)]
pub struct WindowRootNode {
    pub node_id: Option<NodeId>,
    pub current_width: f32,
    pub current_height: f32,
}

impl Default for WindowRootNode {
    fn default() -> Self {
        Self {
            node_id: None,
            current_width: 0.0,
            current_height: 0.0,
        }
    }
}

impl WindowRootNode {
    pub fn needs_resize(&self, new_width: f32, new_height: f32) -> bool {
        (self.current_width - new_width).abs() > f32::EPSILON ||
        (self.current_height - new_height).abs() > f32::EPSILON
    }

    pub fn update_size(&mut self, width: f32, height: f32) {
        self.current_width = width;
        self.current_height = height;
    }
}

/// System that builds the Taffy layout tree from the Bevy ECS hierarchy
pub fn build_taffy_tree_system(
    taffy_resource: ResMut<TaffyResource>,
    mut window_root: ResMut<WindowRootNode>,
    mut ui_node_query: Query<(Entity, &mut UiNode, &Styleable), Added<UiNode>>,
    window_query: Query<&bevy_window::Window, bevy_ecs::query::With<bevy_window::PrimaryWindow>>,
    _children_query: Query<&Children>,
    _parent_query: Query<&Parent>,
) {
    taffy_resource.with_tree(|tree| {
        // Create window root container if it doesn't exist
        if window_root.node_id.is_none() {
            if let Ok(window) = window_query.get_single() {
                let window_style = taffy::Style {
                    position: taffy::Position::Relative,
                    display: taffy::Display::Grid,            // Use Grid container for proper alignment
                    size: taffy::Size {
                        width: taffy::Dimension::Length(window.width()),
                        height: taffy::Dimension::Length(window.height()),
                    },
                    // Create a 10x10 grid to allow flexible positioning
                    grid_template_columns: vec![
                        taffy::TrackSizingFunction::Single(taffy::MinMax {
                            min: taffy::MinTrackSizingFunction::Auto,
                            max: taffy::MaxTrackSizingFunction::Fraction(1.0),
                        }); 10
                    ],
                    grid_template_rows: vec![
                        taffy::TrackSizingFunction::Single(taffy::MinMax {
                            min: taffy::MinTrackSizingFunction::Auto,
                            max: taffy::MaxTrackSizingFunction::Fraction(1.0),
                        }); 10
                    ],
                    // Align items to start (top-left) by default
                    align_items: Some(taffy::AlignItems::Start),
                    justify_items: Some(taffy::JustifyItems::Start),
                    // No margins or padding on the container itself
                    margin: taffy::Rect::zero(),
                    padding: taffy::Rect::zero(),
                    ..Default::default()
                };
                
                // Use new_with_children instead of new_leaf to create a proper container
                let root_node = tree.new_with_children(window_style, &[]).unwrap();
                window_root.node_id = Some(root_node);
                window_root.update_size(window.width(), window.height());
                bevy_log::info!("Created window root grid container: {}x{} (Display::Grid, align/justify: End)", 
                    window.width(), window.height());
            }
        }
        
        // Process newly added UI nodes - REVERT TO ORIGINAL: Add all to window root for now
        for (entity, mut ui_node, styleable) in ui_node_query.iter_mut() {
            if let Some(root_node) = window_root.node_id {
                let is_red_rect = entity.index() == 8; // Based on logs showing 8v1#4294967304
                
                if is_red_rect {
                    bevy_log::info!("🔴 RED RECTANGLE TAFFY TREE SETUP:");
                    bevy_log::info!("   Entity: {:?}", entity);
                    bevy_log::info!("   Taffy style being applied: {:?}", styleable.0);
                }
                
                // Create a new Taffy node for this entity
                let taffy_node = tree.new_leaf(styleable.0.clone()).unwrap();
                
                // Add this node as a child of the window root (original behavior)
                tree.add_child(root_node, taffy_node).unwrap();
                
                ui_node.taffy_node = Some(taffy_node);
                ui_node.needs_layout = true;
                
                if is_red_rect {
                    bevy_log::info!("   Created Taffy node: {:?}", taffy_node);
                    bevy_log::info!("   Added as child of window root: {:?}", root_node);
                } else {
                    bevy_log::debug!("Created Taffy node for entity {:?} as child of window root", entity);
                }
            }
        }
    });
}

/// System that computes layout using Taffy and applies results to Transform components
pub fn compute_and_apply_layout_system(
    taffy_resource: Res<TaffyResource>,
    window_root: Res<WindowRootNode>,
    mut ui_node_query: Query<(Entity, &mut UiNode, &mut Transform, Option<&PositionControl>, Option<&mut LayoutPositioned>), With<Styleable>>,
    mut commands: Commands,
    _children_query: Query<&Children>,
    window_query: Query<&bevy_window::Window, bevy_ecs::query::With<bevy_window::PrimaryWindow>>,
) {
    taffy_resource.with_tree(|tree| {
        // Get window dimensions for coordinate conversion
        let window_height = if let Ok(window) = window_query.get_single() {
            window.height()
        } else {
            300.0 // fallback
        };

        // Compute layout for the window root container
        if let Some(root_node) = window_root.node_id {
            let available_space = taffy::Size {
                width: taffy::AvailableSpace::MaxContent,
                height: taffy::AvailableSpace::MaxContent,
            };
            
            if let Ok(_) = tree.compute_layout(root_node, available_space) {
                bevy_log::debug!("Computed layout for window root container");
                
                // Phase 1: Collect entities that need layout updates (immutable borrow)
                let mut entities_to_update = Vec::new();
                for (entity, ui_node, _, position_control, _) in ui_node_query.iter() {
                    if let Some(taffy_node) = ui_node.taffy_node {
                        let control = position_control.unwrap_or(&PositionControl::Layout);
                        if control.uses_layout() {
                            entities_to_update.push((entity, taffy_node));
                        }
                    }
                }
                
                // Phase 2: Apply layout updates (mutable borrow)
                for (entity, taffy_node) in entities_to_update {
                    apply_layout_to_entity(tree, taffy_node, entity, &mut ui_node_query, &mut commands, window_height);
                }
            }
        }
    });
}

/// System that handles window resize events and updates the Taffy window root container
pub fn window_root_resize_system(
    mut resize_reader: EventReader<bevy_window::WindowResized>,
    taffy_resource: ResMut<TaffyResource>,
    mut window_root: ResMut<WindowRootNode>,
) {
    for event in resize_reader.read() {
        if event.width > 0.0 && event.height > 0.0 {
            // Check if window size actually changed
            if window_root.needs_resize(event.width, event.height) {
                bevy_log::debug!("Window resized: {}x{} -> {}x{}", 
                    window_root.current_width, window_root.current_height,
                    event.width, event.height);
                
                // Update the Taffy root node with new window dimensions
                if let Some(root_node) = window_root.node_id {
                    taffy_resource.with_tree(|tree| {
                        let new_style = taffy::Style {
                            position: taffy::Position::Relative,
                            display: taffy::Display::Grid,             // Maintain grid layout
                            size: taffy::Size {
                                width: taffy::Dimension::Length(event.width),
                                height: taffy::Dimension::Length(event.height),
                            },
                            // Maintain 10x10 grid configuration  
                            grid_template_columns: vec![
                                taffy::TrackSizingFunction::Single(taffy::MinMax {
                                    min: taffy::MinTrackSizingFunction::Auto,
                                    max: taffy::MaxTrackSizingFunction::Fraction(1.0),
                                }); 10
                            ],
                            grid_template_rows: vec![
                                taffy::TrackSizingFunction::Single(taffy::MinMax {
                                    min: taffy::MinTrackSizingFunction::Auto,
                                    max: taffy::MaxTrackSizingFunction::Fraction(1.0),
                                }); 10
                            ],
                            align_items: Some(taffy::AlignItems::Start),
                            justify_items: Some(taffy::JustifyItems::Start),
                            // No margins or padding
                            margin: taffy::Rect::zero(),
                            padding: taffy::Rect::zero(),
                            ..Default::default()
                        };
                        
                        if let Err(e) = tree.set_style(root_node, new_style) {
                            bevy_log::error!("Failed to update window root style: {:?}", e);
                        } else {
                            bevy_log::debug!("Updated window root container to: {}x{}", event.width, event.height);
                        }
                    });
                }
                
                // Update tracked dimensions
                window_root.update_size(event.width, event.height);
            }
        }
    }
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
            // Check if this is the red rectangle for detailed logging
            let is_red_rect = entity.index() == 8; // Based on logs showing 8v1#4294967304
            
            if is_red_rect {
                bevy_log::info!("🔴 RED RECTANGLE LAYOUT DEBUG:");
                bevy_log::info!("   Entity: {:?}", entity);
                bevy_log::info!("   Taffy Node: {:?}", taffy_node);
                bevy_log::info!("   Taffy layout.location: x={}, y={}", layout.location.x, layout.location.y);
                bevy_log::info!("   Taffy layout.size: width={}, height={}", layout.size.width, layout.size.height);
                bevy_log::info!("   Window height: {}", window_height);
                bevy_log::info!("   Current transform.translation: {:?}", transform.translation);
                
                // Log the Taffy style for this node
                if let Ok(style) = tree.style(taffy_node) {
                    bevy_log::info!("   Taffy style.position: {:?}", style.position);
                    bevy_log::info!("   Taffy style.inset: {:?}", style.inset);
                    bevy_log::info!("   Taffy style.size: {:?}", style.size);
                }
            } else {
                bevy_log::debug!("Raw Taffy layout for entity {:?}: location=({}, {}), size=({}, {})", 
                    entity, layout.location.x, layout.location.y, layout.size.width, layout.size.height);
            }
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
                // Check positioning type to determine how to handle coordinates
                let positioning_type = if let Ok(style) = tree.style(taffy_node) {
                    style.position
                } else {
                    taffy::Position::Relative // Default fallback
                };
                
                match positioning_type {
                    taffy::Position::Absolute => {
                        // Absolute positioned elements: convert Taffy coords to Bevy coords
                        // Taffy's layout.location is in Taffy coordinate space (top-left origin, Y down)
                        
                        if is_red_rect {
                            bevy_log::warn!("🔴 ABSOLUTE POSITIONING: Taffy layout.location = ({}, {})", 
                                layout.location.x, layout.location.y);
                        }
                        
                        let taffy_coords = coordinate_system::TaffyCoords::new(layout.location.x, layout.location.y, transform.translation.z);
                        let bevy_coords = taffy_coords.to_bevy(window_height);
                        
                        // Update the transform with the computed position
                        coordinate_system::update_ui_transform(&mut transform, bevy_coords);
                        
                        if is_red_rect {
                            bevy_log::info!("   🔄 ABSOLUTE COORDINATE CONVERSION:");
                            bevy_log::info!("      Taffy layout.location: ({}, {})", layout.location.x, layout.location.y);
                            bevy_log::info!("      Window height: {}", window_height);
                            bevy_log::info!("      Converted to Bevy coords: {:?}", bevy_coords.raw());
                            bevy_log::info!("      Final transform.translation: {:?}", transform.translation);
                        } else {
                            bevy_log::debug!("Applied absolute layout to entity {:?}: Taffy pos=({}, {}) -> Bevy pos=({}, {}), size=({}, {})", 
                                entity, layout.location.x, layout.location.y, bevy_coords.raw().x, bevy_coords.raw().y, layout.size.width, layout.size.height);
                        }
                    }
                    taffy::Position::Relative => {
                        // Grid/flex items: Use Taffy's layout position directly (no coordinate conversion)
                        // Taffy already positions these relative to their grid cell or flex container
                        let final_position = Vec3::new(layout.location.x, layout.location.y, transform.translation.z);
                        transform.translation = final_position;
                        
                        if is_red_rect {
                            bevy_log::info!("   📐 GRID POSITIONING (no conversion):");
                            bevy_log::info!("      Taffy layout.location: ({}, {})", layout.location.x, layout.location.y);
                            bevy_log::info!("      Direct position: {:?}", final_position);
                        } else {
                            bevy_log::debug!("Applied grid layout to entity {:?}: pos=({}, {}), size=({}, {})", 
                                entity, layout.location.x, layout.location.y, layout.size.width, layout.size.height);
                        }
                    }
                }
                
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

