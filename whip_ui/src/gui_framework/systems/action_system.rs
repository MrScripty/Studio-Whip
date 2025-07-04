use bevy_ecs::prelude::*;
use bevy_log::{debug, info, warn};
use crate::gui_framework::events::{ActionEvent, BuiltinAction, ActionRegistry};
use crate::Visibility;
use bevy_hierarchy::Children;

/// System that processes action events and executes the corresponding actions
pub fn action_execution_system(
    mut action_events: EventReader<ActionEvent>,
    action_registry: Option<Res<ActionRegistry>>,
    mut visibility_query: Query<&mut Visibility>,
    children_query: Query<&Children>,
) {
    for action_event in action_events.read() {
        debug!("Processing action: {} from entity {:?}", action_event.action, action_event.source_entity);
        
        // First try to handle as builtin action
        if let Some(builtin_action) = BuiltinAction::from_action_event(action_event) {
            execute_builtin_action(
                builtin_action,
                action_event,
                &mut visibility_query,
                &children_query,
            );
            continue;
        }
        
        // Then try custom action handlers
        if let Some(registry) = &action_registry {
            if registry.handlers.contains_key(&action_event.action) {
                info!("🎯 ACTION CUSTOM: {} (Custom handlers not yet fully implemented)", action_event.action);
                continue;
            }
        }
        
        // If no handler found, log a warning
        warn!("No handler found for action: {}", action_event.action);
    }
}

/// Execute a builtin action
fn execute_builtin_action(
    action: BuiltinAction,
    _event: &ActionEvent,
    _visibility_query: &mut Query<&mut Visibility>,
    _children_query: &Query<&Children>,
) {
    match action {
        BuiltinAction::Debug { message } => {
            info!("🎯 ACTION DEBUG: {}", message);
        }
        
        BuiltinAction::Navigate { target } => {
            info!("🧭 ACTION NAVIGATE: Navigating to {}", target);
            // TODO: Implement navigation system integration
        }
        
        BuiltinAction::ToggleVisibility { target_id } => {
            info!("👁️ ACTION TOGGLE_VISIBILITY: Toggling visibility for {}", target_id);
            // TODO: Implement entity lookup by ID and toggle visibility
            // For now, just log the action
        }
        
        BuiltinAction::UpdateText { target_id, new_text } => {
            info!("📝 ACTION UPDATE_TEXT: Setting text for {} to '{}'", target_id, new_text);
            // TODO: Implement entity lookup by ID and update text
            // For now, just log the action
        }
        
        BuiltinAction::SetFocus { target_id } => {
            info!("🎯 ACTION SET_FOCUS: Setting focus to {}", target_id);
            // TODO: Implement focus management
            // For now, just log the action
        }
    }
}

/// System that generates action events from UI interaction events
pub fn interaction_to_action_system(
    mut action_events: EventWriter<ActionEvent>,
    mut click_events: EventReader<crate::gui_framework::events::EntityClicked>,
    action_bindings_query: Query<&crate::widgets::components::WidgetActionBindings>,
    // TODO: Add other interaction events (hover, focus, etc.)
) {
    for click_event in click_events.read() {
        debug!("Entity clicked: {:?}", click_event.entity);
        
        // Look up action bindings for this entity
        if let Ok(bindings) = action_bindings_query.get(click_event.entity) {
            // Check if there's a click action binding
            if let Some(binding) = bindings.bindings.get("click") {
                debug!("Found click binding: {:?}", binding);
                
                // Create action event from the binding
                let mut action = ActionEvent::new(
                    binding.action.clone(),
                    click_event.entity,
                    binding.event.clone(),
                );
                
                // Add parameters if they exist
                if let Some(ref params) = binding.params {
                    action = action.with_params(params.clone());
                }
                
                action_events.send(action);
            }
        }
        // Note: Entities without action bindings are intentionally non-interactive
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_ecs::world::World;
    use std::collections::HashMap;

    #[test]
    fn test_debug_action_execution() {
        let mut world = World::new();
        let entity = world.spawn_empty().id();
        
        let mut params = HashMap::new();
        params.insert("message".to_string(), toml::Value::String("Test debug message".to_string()));
        
        let action_event = ActionEvent::new(
            "debug".to_string(),
            entity,
            "click".to_string(),
        ).with_params(params);
        
        let builtin = BuiltinAction::from_action_event(&action_event).unwrap();
        
        // This should execute without panicking
        // In a real test, we'd capture the log output
        match builtin {
            BuiltinAction::Debug { message } => {
                assert_eq!(message, "Test debug message");
            }
            _ => panic!("Expected debug action"),
        }
    }

    #[test]
    fn test_action_event_generation_with_bindings() {
        let mut world = World::new();
        world.init_resource::<Events<ActionEvent>>();
        world.init_resource::<Events<crate::gui_framework::events::EntityClicked>>();
        
        // Create entity with action bindings
        let mut bindings = HashMap::new();
        bindings.insert("click".to_string(), crate::assets::definitions::ActionBinding {
            event: "click".to_string(),
            action: "debug".to_string(),
            params: Some({
                let mut params = HashMap::new();
                params.insert("message".to_string(), toml::Value::String("Test message".to_string()));
                params
            }),
        });
        
        let entity = world.spawn(crate::widgets::components::WidgetActionBindings { bindings }).id();
        
        let click_event = crate::gui_framework::events::EntityClicked { entity };
        world.resource_mut::<Events<crate::gui_framework::events::EntityClicked>>()
            .send(click_event);
        
        // Run the interaction to action system
        world.run_system_once(interaction_to_action_system);
        
        // Check that an action event was generated
        let action_events = world.resource::<Events<ActionEvent>>();
        let mut reader = action_events.get_reader();
        let events: Vec<_> = reader.read(action_events).collect();
        
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].action, "debug");
        assert_eq!(events[0].event_type, "click");
        assert_eq!(events[0].source_entity, entity);
    }

    #[test]
    fn test_no_action_without_bindings() {
        let mut world = World::new();
        world.init_resource::<Events<ActionEvent>>();
        world.init_resource::<Events<crate::gui_framework::events::EntityClicked>>();
        
        // Create entity WITHOUT action bindings
        let entity = world.spawn_empty().id();
        
        let click_event = crate::gui_framework::events::EntityClicked { entity };
        world.resource_mut::<Events<crate::gui_framework::events::EntityClicked>>()
            .send(click_event);
        
        // Run the interaction to action system
        world.run_system_once(interaction_to_action_system);
        
        // Check that NO action event was generated
        let action_events = world.resource::<Events<ActionEvent>>();
        let mut reader = action_events.get_reader();
        let events: Vec<_> = reader.read(action_events).collect();
        
        assert_eq!(events.len(), 0); // No actions should be generated
    }
}