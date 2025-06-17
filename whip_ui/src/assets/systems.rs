use bevy_ecs::prelude::*;
use bevy_asset::{Assets, AssetServer, Handle};
use bevy_log::{info, error};
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
    info!("Spawning UI tree from: {}", request.asset_path);
    
    // Create a temporary widget collection for the spawning system
    let temp_collection = crate::widgets::blueprint::WidgetCollection {
        widgets: ui_tree.resolved_widgets.clone(),
        root: ui_tree.root.clone(),
    };
    
    // Spawn all widgets as children of the window (no explicit root)
    // All widgets in the TOML are implicitly children of the window
    for (widget_id, widget_blueprint) in &ui_tree.resolved_widgets {
        info!("Spawning widget: {}", widget_id);
        
        let widget_entity = spawn_widget_recursive(
            commands,
            widget_blueprint,
            &temp_collection,
            yrs_res,
            request.parent, // This will be None for top-level widgets (children of window)
        );
        
        // Apply position override if specified and this is the first widget
        if let Some(position) = request.position_override {
            if let Some(mut entity_commands) = commands.get_entity(widget_entity) {
                entity_commands.insert(bevy_transform::prelude::Transform::from_translation(position));
            }
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