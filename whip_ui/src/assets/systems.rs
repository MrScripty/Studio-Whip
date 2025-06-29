use bevy_ecs::prelude::*;
use bevy_asset::{Assets, AssetServer, Handle};
use bevy_log::{info, error};
use bevy_hierarchy::BuildChildren;
use bevy_math::Vec3;
use std::collections::HashMap;
use crate::assets::{UiDefinition, LoadUiRequest};
use crate::widgets::systems;
use crate::YrsDocResource;

/// Resource to track loading UI assets
#[derive(Resource, Default)]
pub struct LoadingUiAssets {
    /// Map of asset handles to pending load requests
    pub pending_loads: HashMap<Handle<UiDefinition>, LoadUiRequest>,
}

/// System to listen for LoadUiRequest events and initiate asset loading
pub fn ui_asset_request_system(
    mut load_requests: EventReader<LoadUiRequest>,
    mut loading_assets: ResMut<LoadingUiAssets>,
    asset_server: Res<AssetServer>,
) {
    for request in load_requests.read() {
        info!("Loading UI asset: {}", request.asset_path);
        
        // Load the UI asset through Bevy's asset server
        let handle: Handle<UiDefinition> = asset_server.load(&request.asset_path);
        
        // Track this loading request
        loading_assets.pending_loads.insert(handle, request.clone());
    }
}

/// System to process loaded UI assets and spawn widgets
pub fn ui_asset_loaded_system(
    mut commands: Commands,
    mut loading_assets: ResMut<LoadingUiAssets>,
    ui_assets: Res<Assets<UiDefinition>>,
    yrs_res: Res<YrsDocResource>,
    window_query: Query<&bevy_window::Window, With<bevy_window::PrimaryWindow>>,
) {
    let mut completed_loads = Vec::new();
    
    // Check all pending loads for completion
    for (handle, request) in &loading_assets.pending_loads {
        if let Some(ui_definition) = ui_assets.get(handle) {
            info!("UI asset loaded successfully: {}", request.asset_path);
            
            // Get window height for coordinate conversion
            let window_height = window_query.get_single()
                .map(|window| window.height())
                .unwrap_or(300.0); // Default fallback
            
            // Spawn the UI definition
            spawn_ui_definition(&mut commands, ui_definition, request, &yrs_res, window_height);
            
            // Mark this load as completed
            completed_loads.push(handle.clone());
        }
    }
    
    // Remove completed loads from pending
    for handle in completed_loads {
        loading_assets.pending_loads.remove(&handle);
    }
}

/// Spawn a UI definition from a loaded asset
fn spawn_ui_definition(
    commands: &mut Commands,
    ui_definition: &UiDefinition,
    request: &LoadUiRequest,
    yrs_res: &YrsDocResource,
    window_height: f32,
) {
    bevy_log::debug!("🎯 Spawning UI definition from: {}", request.asset_path);
    bevy_log::debug!("   Window height for coordinate conversion: {}", window_height);
    bevy_log::debug!("   Root widget type: {:?}", ui_definition.root.widget_type);
    bevy_log::debug!("   Root widget has {} children", ui_definition.root.children.len());
    
    // Spawn the root widget and its hierarchy
    let widget_entity = spawn_widget_from_node(
        commands,
        &ui_definition.root,
        yrs_res,
        request.parent,
        window_height,
        None,  // Root widget has no parent position
    );
    
    // Apply position override if specified
    if let Some(position) = request.position_override {
        if let Some(mut entity_commands) = commands.get_entity(widget_entity) {
            entity_commands.insert(bevy_transform::prelude::Transform::from_translation(position));
        }
    }
    
    bevy_log::debug!("✅ Successfully spawned UI definition root entity: {:?}", widget_entity);
}

/// Spawn a widget from a WidgetNode using unified architecture
fn spawn_widget_from_node(
    commands: &mut Commands,
    node: &crate::assets::definitions::WidgetNode,
    yrs_res: &YrsDocResource,
    parent: Option<Entity>,
    window_height: f32,
    parent_position: Option<Vec3>,
) -> Entity {
    use crate::widgets::templates::expand_template_node;
    
    let widget_id = node.id.clone().unwrap_or_else(|| "unnamed".to_string());
    bevy_log::debug!("🔍 Processing widget node '{}' with type: {:?}", widget_id, node.widget_type);
    bevy_log::debug!("   Layout: position={:?}, size={:?}", node.layout.position, node.layout.size);
    bevy_log::debug!("   Style: bg_color={:?}", node.style.background_color);
    bevy_log::debug!("   Behavior: visible={:?}, clickable={:?}, z_index={:?}", 
        node.behavior.visible, node.behavior.clickable, node.behavior.z_index);
    
    // Check if this is a template widget and expand it directly
    let expanded_nodes = expand_template_node(node);
    
    let entity = if expanded_nodes.len() > 1 {
        // Template widget - spawn hierarchy from expansion
        bevy_log::debug!("🎯 Widget '{}' is a template widget, expanding into {} components", widget_id, expanded_nodes.len());
        let shape_node = &expanded_nodes[0];
        let text_node = &expanded_nodes[1];
        
        bevy_log::debug!("   📦 Shape node: id={:?}, size={:?}, bg_color={:?}", 
            shape_node.id, shape_node.layout.size, shape_node.style.background_color);
        bevy_log::debug!("   📝 Text node: id={:?}, content={:?}, color={:?}", 
            text_node.id, 
            if let crate::widgets::blueprint::WidgetType::Text { content, .. } = &text_node.widget_type { 
                Some(content) 
            } else { 
                None 
            },
            text_node.style.text_color);
        
        // Debug position calculation
        bevy_log::debug!("   🎯 Original button position: {:?}", node.layout.position);
        bevy_log::debug!("   🎯 Shape node position after expansion: {:?}", shape_node.layout.position);
        
        // Spawn shape entity (background)
        let shape_entity = systems::spawn_widget_entity_from_node(commands, shape_node, yrs_res, window_height, None, None);
        bevy_log::debug!("   ✓ Shape entity created: {:?}", shape_entity);
        
        // Spawn text entity (label)
        let text_entity = systems::spawn_widget_entity_from_node(commands, text_node, yrs_res, window_height, Some(shape_entity), Some(shape_node.layout.position.unwrap_or(Vec3::ZERO)));
        bevy_log::debug!("   ✓ Text entity created: {:?}", text_entity);
        
        // Set up parent-child relationship for widget hierarchy
        commands.entity(shape_entity).insert(crate::widgets::components::WidgetHierarchy {
            parent,
            children: vec![text_entity],
        });
        
        commands.entity(text_entity).insert(crate::widgets::components::WidgetHierarchy {
            parent: Some(shape_entity),
            children: vec![],
        });
        
        // Set up Bevy's built-in parent-child relationship for Transform inheritance
        commands.entity(text_entity).set_parent(shape_entity);
        
        bevy_log::debug!("✓ Created template widget hierarchy: shape={:?}, text={:?}", shape_entity, text_entity);
        shape_entity
    } else {
        // Regular widget - spawn directly
        let entity = systems::spawn_widget_entity_from_node(commands, node, yrs_res, window_height, parent, parent_position);
        
        // Set up hierarchy for children
        let mut child_entities = Vec::new();
        for child_node in &node.children {
            let child_entity = spawn_widget_from_node(commands, child_node, yrs_res, Some(entity), window_height, node.layout.position);
            child_entities.push(child_entity);
            
            // Only set up Bevy's parent-child relationship if the child doesn't use Manual positioning
            // Manual positioned widgets should not inherit transforms from layout-positioned parents
            let child_position_control = child_node.behavior.position_control.clone().unwrap_or(crate::layout::PositionControl::Layout);
            if !child_position_control.is_manual() {
                commands.entity(entity).add_child(child_entity);
            } else {
                bevy_log::debug!("🔓 Skipping parent-child relationship for Manual positioned widget '{:?}' to prevent transform inheritance", child_node.id);
            }
        }
        
        commands.entity(entity).insert(crate::widgets::components::WidgetHierarchy {
            parent,
            children: child_entities,
        });
        
        entity
    };
    
    // Add action bindings if they exist
    if let Some(ref bindings) = node.bindings {
        commands.entity(entity).insert(crate::widgets::components::WidgetActionBindings {
            bindings: bindings.clone(),
        });
    }
    
    entity
}

/// System to handle asset loading errors
pub fn ui_asset_error_system(
    mut loading_assets: ResMut<LoadingUiAssets>,
    asset_server: Res<AssetServer>,
) {
    let mut failed_loads = Vec::new();
    
    for (handle, request) in &loading_assets.pending_loads {
        match asset_server.load_state(handle) {
            bevy_asset::LoadState::Failed(_) => {
                error!("Failed to load UI asset: {}", request.asset_path);
                failed_loads.push(handle.clone());
            }
            _ => {} // Still loading or loaded successfully
        }
    }
    
    // Remove failed loads from pending
    for handle in failed_loads {
        loading_assets.pending_loads.remove(&handle);
    }
}

/// Event writer helper for easier UI loading
#[derive(Resource)]
pub struct UiLoader<'w> {
    load_request_writer: EventWriter<'w, LoadUiRequest>,
}

impl<'w> UiLoader<'w> {
    /// Load a UI from an asset path
    pub fn load_ui(&mut self, asset_path: impl Into<String>) {
        self.load_request_writer.send(LoadUiRequest::new(asset_path));
    }
    
    /// Load a UI with a parent entity
    pub fn load_ui_with_parent(&mut self, asset_path: impl Into<String>, parent: Entity) {
        self.load_request_writer.send(LoadUiRequest::new(asset_path).with_parent(parent));
    }
    
    /// Load a UI with a position override
    pub fn load_ui_at_position(&mut self, asset_path: impl Into<String>, position: bevy_math::Vec3) {
        self.load_request_writer.send(LoadUiRequest::new(asset_path).with_position(position));
    }
}