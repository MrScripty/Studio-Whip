use bevy_ecs::prelude::*;
use bevy_asset::{Assets, AssetServer, Handle};
use bevy_log::{info, error};
use std::collections::HashMap;
use crate::assets::{UiDefinition, LoadUiRequest};
use crate::widgets::{
    systems::spawn_widget_recursive,
};
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
) {
    let mut completed_loads = Vec::new();
    
    // Check all pending loads for completion
    for (handle, request) in &loading_assets.pending_loads {
        if let Some(ui_definition) = ui_assets.get(handle) {
            info!("UI asset loaded successfully: {}", request.asset_path);
            
            // Spawn the UI definition
            spawn_ui_definition(&mut commands, ui_definition, request, &yrs_res);
            
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
) {
    info!("Spawning UI definition from: {}", request.asset_path);
    
    // Spawn the root widget and its hierarchy
    let widget_entity = spawn_widget_from_node(
        commands,
        &ui_definition.root,
        yrs_res,
        request.parent,
    );
    
    // Apply position override if specified
    if let Some(position) = request.position_override {
        if let Some(mut entity_commands) = commands.get_entity(widget_entity) {
            entity_commands.insert(bevy_transform::prelude::Transform::from_translation(position));
        }
    }
    
    info!("Successfully spawned UI definition from: {}", request.asset_path);
}

/// Spawn a widget from a WidgetNode (hierarchical format)
fn spawn_widget_from_node(
    commands: &mut Commands,
    node: &crate::assets::definitions::WidgetNode,
    yrs_res: &YrsDocResource,
    parent: Option<Entity>,
) -> Entity {
    // Convert WidgetNode to WidgetBlueprint for the existing spawn system
    let widget_blueprint = crate::widgets::blueprint::WidgetBlueprint {
        id: node.id.clone().unwrap_or_else(|| "unnamed".to_string()),
        widget_type: node.widget_type.clone(),
        layout: node.layout.clone(),
        style: node.style.clone(),
        behavior: node.behavior.clone(),
        children: node.children.iter().enumerate().map(|(i, _)| format!("child_{}", i)).collect(),
    };
    
    // Create a temporary collection for compatibility
    let temp_collection = crate::widgets::blueprint::WidgetCollection {
        widgets: std::collections::HashMap::new(),
        root: Some(widget_blueprint.id.clone()),
    };
    
    // Spawn the widget
    let entity = spawn_widget_recursive(
        commands,
        &widget_blueprint,
        &temp_collection,
        yrs_res,
        parent,
    );
    
    // Add action bindings if they exist
    if let Some(ref bindings) = node.bindings {
        commands.entity(entity).insert(crate::widgets::components::WidgetActionBindings {
            bindings: bindings.clone(),
        });
    }
    
    // Recursively spawn children
    for child_node in &node.children {
        spawn_widget_from_node(commands, child_node, yrs_res, Some(entity));
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