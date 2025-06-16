use bevy_app::{App, Plugin, Update, PostUpdate};
use bevy_ecs::prelude::*;
use crate::layout::{
    TaffyResource, 
    build_taffy_tree_system,
    compute_and_apply_layout_system,
    update_shape_vertices_system,
};

/// Plugin that provides Taffy layout integration for UI elements
pub struct TaffyLayoutPlugin;

impl Plugin for TaffyLayoutPlugin {
    fn build(&self, app: &mut App) {
        // Initialize the Taffy resource
        app.init_resource::<TaffyResource>();
        
        // Add layout systems in the correct order
        app.add_systems(
            Update,
            (
                // First: Build the Taffy tree from ECS hierarchy
                build_taffy_tree_system,
                // Second: Compute layout and apply to transforms
                compute_and_apply_layout_system,
            ).chain()
        );
        
        // Add shape vertex update system in PostUpdate to ensure it runs after layout
        app.add_systems(
            PostUpdate,
            update_shape_vertices_system
        );
        
        bevy_log::info!("TaffyLayoutPlugin initialized");
    }
}