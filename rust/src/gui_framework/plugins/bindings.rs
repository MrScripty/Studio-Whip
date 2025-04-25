use bevy_app::{App, AppExit, Plugin, Update};
use bevy_ecs::{prelude::*, schedule::SystemSet};
use bevy_log::info;

// Import events from the gui_framework
use crate::gui_framework::events::HotkeyActionTriggered;

 // Import sets from other plugins for ordering
 use super::interaction::InteractionSet; // Use super:: to access sibling module

// --- System Sets ---
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum BindingsSet { HandleActions }

// --- Default Bindings Plugin Definition ---
/// A plugin providing a default system to handle basic hotkey actions like closing the app.
pub struct GuiFrameworkDefaultBindingsPlugin;

impl Plugin for GuiFrameworkDefaultBindingsPlugin {
    fn build(&self, app: &mut App) {
        info!("Building GuiFrameworkDefaultBindingsPlugin...");
        app.configure_sets(Update,
            BindingsSet::HandleActions.after(InteractionSet::InputHandling)
        );
        app.add_systems(Update, app_control_system.in_set(BindingsSet::HandleActions));
        info!("GuiFrameworkDefaultBindingsPlugin built.");
    }
}

/// Update system: Handles default application control actions based on `HotkeyActionTriggered` events.
/// Currently, only handles the "CloseRequested" action. Applications can disable this plugin
/// or add their own systems to handle actions differently.
fn app_control_system(
    mut hotkey_evr: EventReader<HotkeyActionTriggered>, // Reads events from InteractionPlugin
    mut app_exit_evw: EventWriter<AppExit>,
) {
    for ev in hotkey_evr.read() {
        // This system decides what specific actions mean by default
        if ev.action == "CloseRequested" {
            info!("'CloseRequested' hotkey action received, sending AppExit (Default Bindings).");
            app_exit_evw.send(AppExit::Success);
        }
        // Add other default hotkey action handling here if needed in the future
    }
}