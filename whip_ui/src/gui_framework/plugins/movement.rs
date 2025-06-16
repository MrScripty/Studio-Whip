use bevy_app::{App, Plugin, Update};
use bevy_ecs::{prelude::*, schedule::SystemSet}; 
use bevy_log::{info, warn};
use bevy_transform::prelude::Transform;
// Import events from the gui_framework
use crate::gui_framework::events::EntityDragged;
// Import sets from other plugins for ordering
use super::interaction::InteractionSet; // Use super:: to access sibling module
// Import layout position control
use crate::layout::{PositionControl, LayoutPositioned};

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
    mut query: Query<(&mut Transform, Option<&mut PositionControl>)>,
    mut commands: Commands,
) {
    // Check if the system is running at all
    for ev in drag_evr.read() {
        info!("[MovementSystem] Received EntityDragged: Entity={:?}, Delta={:?}", ev.entity, ev.delta);

        info!("[MovementSystem] Attempting to get Transform and PositionControl for {:?}", ev.entity);
        if let Ok((mut transform, position_control)) = query.get_mut(ev.entity) {
            let old_pos = transform.translation; // Store old position for logging
            info!("[MovementSystem] Got Transform for {:?}. Before: {:?}", ev.entity, old_pos);

            // Check if this entity allows manual positioning
            let allows_manual = match position_control.as_ref() {
                Some(control) => control.allows_manual(),
                None => true, // Default to allowing manual control if no PositionControl component
            };

            if allows_manual {
                // Handle LayoutThenManual transition to Manual
                if let Some(mut control) = position_control {
                    control.take_manual_control();
                    if matches!(*control, PositionControl::Manual) {
                        // Remove LayoutPositioned marker if present
                        commands.entity(ev.entity).remove::<LayoutPositioned>();
                    }
                }

                // Apply delta directly based on Y-up world coordinates
                transform.translation.x += ev.delta.x;
                transform.translation.y += ev.delta.y; // Ensure Y-inversion is active

                info!("[MovementSystem] Transform modified for {:?}. After: {:?}", ev.entity, transform.translation);
            } else {
                info!("[MovementSystem] Entity {:?} has PositionControl::Layout, ignoring drag", ev.entity);
            }
        } else {
            warn!("[MovementSystem] Could not get Transform for entity {:?}", ev.entity);
        }
    }
}