use bevy_app::{App, Plugin, Update};
use bevy_ecs::{prelude::*, schedule::SystemSet}; 
use bevy_log::{info, warn};
use bevy_transform::prelude::Transform;
// Import events from the gui_framework
use crate::gui_framework::events::EntityDragged;
// Import sets from other plugins for ordering
use super::interaction::InteractionSet; // Use super:: to access sibling module

// --- System Sets ---
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum MovementSet { ApplyMovement }

// --- Default Movement Plugin Definition ---
/// A plugin providing a default system to move entities based on EntityDragged events.
pub struct GuiFrameworkDefaultMovementPlugin;
impl Plugin for GuiFrameworkDefaultMovementPlugin {
fn build(&self, app: &mut App) {
info!("Building GuiFrameworkDefaultMovementPlugin...");
// Ensure movement happens after input is processed
app.configure_sets(Update,
    MovementSet::ApplyMovement.after(InteractionSet::InputHandling)
);
app.add_systems(Update, movement_system.in_set(MovementSet::ApplyMovement));

info!("GuiFrameworkDefaultMovementPlugin built.");
}
}

/// This provides a default way to handle dragging; applications can disable this plugin
/// or add their own systems to handle EntityDragged differently.
/// Update system: Applies movement deltas from EntityDragged events to Transform components.
fn movement_system(
    mut drag_evr: EventReader<EntityDragged>,
    mut query: Query<&mut Transform>,
) {
    // Check if the system is running at all
    let mut ran = false;
    for ev in drag_evr.read() {
        ran = true; // Mark that we entered the loop
        info!("[MovementSystem] Received EntityDragged: Entity={:?}, Delta={:?}", ev.entity, ev.delta);

        info!("[MovementSystem] Attempting to get Transform for {:?}", ev.entity);
        if let Ok(mut transform) = query.get_mut(ev.entity) {
            let old_pos = transform.translation; // Store old position for logging
            info!("[MovementSystem] Got Transform for {:?}. Before: {:?}", ev.entity, old_pos);

            // Apply delta directly based on Y-up world coordinates
            transform.translation.x += ev.delta.x;
            transform.translation.y -= ev.delta.y; // Ensure Y-inversion is active

            info!("[MovementSystem] Transform modified for {:?}. After: {:?}", ev.entity, transform.translation);
        } else {
            warn!("[MovementSystem] Could not get Transform for entity {:?}", ev.entity);
        }
    }
    // Log if the system ran but processed no events (useful for debugging event flow)
    // if !ran && !drag_evr.is_empty() { // This check might be tricky with event consumption
    //     info!("[MovementSystem] Ran, but no events read this frame (might have been consumed).");
    // } else if !ran {
    //      info!("[MovementSystem] Ran, no events found.");
    // }
}