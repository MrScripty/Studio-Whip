use bevy_app::{App, Plugin, Update};
use bevy_asset::AssetApp;
use bevy_ecs::prelude::*;
use crate::assets::{
    UiTree,
    UiDefinition,
    UiAssetLoader,
    UiDefinitionLoader,
    LoadUiRequest,
    LoadingUiAssets,
    UiRegistry,
    ui_asset_request_system,
    ui_asset_loaded_system,
    ui_asset_error_system,
};

/// Plugin that adds UI asset loading capabilities to the app
pub struct UiAssetPlugin;

impl Plugin for UiAssetPlugin {
    fn build(&self, app: &mut App) {
        // Register the UiTree asset type and its legacy loader
        app.init_asset::<UiTree>()
           .register_asset_loader(UiAssetLoader);
        
        // Register the new UiDefinition asset type and its loader
        app.init_asset::<UiDefinition>()
           .register_asset_loader(UiDefinitionLoader);
        
        // Add the LoadUiRequest event
        app.add_event::<LoadUiRequest>();
        
        // Add resources
        app.init_resource::<LoadingUiAssets>();
        
        // Initialize the UI registry with built-in types
        app.insert_resource(UiRegistry::new());
        
        // Add systems for asset loading and processing
        app.add_systems(
            Update,
            (
                ui_asset_request_system,
                ui_asset_loaded_system,
                ui_asset_error_system,
            ).chain(), // Run in order: request -> loaded -> error handling
        );
        
        bevy_log::info!("UiAssetPlugin initialized");
    }
}