use bevy_app::{App, Plugin, Update};
use bevy_ecs::prelude::*;
use bevy_log::info;
use bevy_math::Vec2; // Needed for delta calculation interpretation
use bevy_transform::prelude::Transform;
// Import events from the gui_framework
use crate::gui_framework::events::EntityDragged;
// --- Default Movement Plugin Definition ---
/// A plugin providing a default system to move entities based on EntityDragged events.
pub struct GuiFrameworkDefaultMovementPlugin;
impl Plugin for GuiFrameworkDefaultMovementPlugin {
fn build(&self, app: &mut App) {
info!("Building GuiFrameworkDefaultMovementPlugin...");
app.add_systems(Update, movement_system);
info!("GuiFrameworkDefaultMovementPlugin built.");
}
}
// --- System Moved from main.rs ---
/// Update system: Applies movement deltas from EntityDragged events to Transform components.
/// This provides a default way to handle dragging; applications can disable this plugin
/// or add their own systems to handle EntityDragged differently.
fn movement_system(
mut drag_evr: EventReader<EntityDragged>, // Reads events from InteractionPlugin
mut query: Query<&mut Transform>,
) {
for ev in drag_evr.read() {
if let Ok(mut transform) = query.get_mut(ev.entity) {
// Apply delta directly based on Y-up world coordinates
transform.translation.x += ev.delta.x;
transform.translation.y -= ev.delta.y; // Screen Y is inverted relative to world Y
}
}
}

