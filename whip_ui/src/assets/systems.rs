use bevy_ecs::prelude::*;
use bevy_asset::{Assets, AssetServer, Handle};
use bevy_log::{info, warn, error};
use std::collections::HashMap;
use crate::assets::{UiTree, LoadUiRequest};
use crate::widgets::{
    systems::spawn_widget_recursive,
};
use crate::YrsDocResource;

/// Resource to track loading UI assets
#[derive(Resource, Default)]
pub struct LoadingUiAssets {
    /// Map of asset handles to pending load requests
    pub pending_loads: HashMap<Handle<UiTree>, LoadUiRequest>,
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
        let handle: Handle<UiTree> = asset_server.load(&request.asset_path);
        
        // Track this loading request
        loading_assets.pending_loads.insert(handle, request.clone());
    }
}

/// System to process loaded UI assets and spawn widgets
pub fn ui_asset_loaded_system(
    mut commands: Commands,
    mut loading_assets: ResMut<LoadingUiAssets>,
    ui_assets: Res<Assets<UiTree>>,
    yrs_res: Res<YrsDocResource>,
) {
    let mut completed_loads = Vec::new();
    
    // Check all pending loads for completion
    for (handle, request) in &loading_assets.pending_loads {
        if let Some(ui_tree) = ui_assets.get(handle) {
            info!("UI asset loaded successfully: {}", request.asset_path);
            
            // Spawn the UI tree
            spawn_ui_tree(&mut commands, ui_tree, request, &yrs_res);
            
            // Mark this load as completed
            completed_loads.push(handle.clone());
        }
    }
    
    // Remove completed loads from pending
    for handle in completed_loads {
        loading_assets.pending_loads.remove(&handle);
    }
}

/// Spawn a UI tree from a loaded asset
fn spawn_ui_tree(
    commands: &mut Commands,
    ui_tree: &UiTree,
    request: &LoadUiRequest,
    yrs_res: &YrsDocResource,
) {
    // Find the root widget to spawn
    let root_widget_id = ui_tree.root.as_ref().or_else(|| {
        // If no root specified, try to find a reasonable default
        ui_tree.resolved_widgets.keys().next()
    });
    
    let Some(root_id) = root_widget_id else {
        warn!("No root widget found in UI tree: {}", request.asset_path);
        return;
    };
    
    let Some(root_widget) = ui_tree.get_widget(root_id) else {
        error!("Root widget '{}' not found in UI tree: {}", root_id, request.asset_path);
        return;
    };
    
    info!("Spawning UI tree with root widget: {}", root_id);
    
    // Create a temporary widget collection for the spawning system
    let temp_collection = crate::widgets::blueprint::WidgetCollection {
        widgets: ui_tree.resolved_widgets.clone(),
        root: Some(root_id.clone()),
    };
    
    // Spawn the root widget and its hierarchy
    let root_entity = spawn_widget_recursive(
        commands,
        root_widget,
        &temp_collection,
        yrs_res,
        request.parent,
    );
    
    // Apply position override if specified
    if let Some(position) = request.position_override {
        if let Some(mut entity_commands) = commands.get_entity(root_entity) {
            entity_commands.insert(bevy_transform::prelude::Transform::from_translation(position));
        }
    }
    
    info!("Successfully spawned UI tree from: {}", request.asset_path);
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