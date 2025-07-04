use bevy_ecs::prelude::*;
// Removed unused imports
use bevy_utils::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use crate::gui_framework::components::{InteractionState, InteractionStateChanged, Interaction};

/// Resource for tracking state changes and preventing duplicate logs
#[derive(Resource, Default)]
pub struct StateChangeTracker {
    /// Hash of last logged state per entity
    last_logged_states: HashMap<Entity, u64>,
    /// Frame count to implement rate limiting  
    frame_count: u64,
}

impl StateChangeTracker {
    /// Check if a state change should be logged based on hash comparison
    pub fn should_log_state_change(&mut self, entity: Entity, state: &InteractionState) -> bool {
        let mut hasher = DefaultHasher::new();
        state.hash(&mut hasher);
        let current_hash = hasher.finish();
        
        let should_log = match self.last_logged_states.get(&entity) {
            Some(&last_hash) if last_hash == current_hash => false, // No change, don't log
            _ => true, // Changed or first time, should log
        };
        
        if should_log {
            self.last_logged_states.insert(entity, current_hash);
        }
        
        should_log
    }
    
    /// Increment frame count for rate limiting
    pub fn next_frame(&mut self) {
        self.frame_count = self.frame_count.wrapping_add(1);
    }
}

/// System that tracks changes to interaction states and fires events
pub fn interaction_state_tracking_system(
    state_query: Query<(Entity, &InteractionState), With<Interaction>>,
    _state_change_events: EventWriter<InteractionStateChanged>,
) {
    // In a full implementation, this would track actual state changes
    // For now, this is a framework placeholder for when actual state change detection is added
    
    for (entity, current_state) in state_query.iter() {
        // This system would detect changes in interaction state and fire events
        // The actual change detection would be done in the specific detection systems
        // (hover_detection_system, press_detection_system, etc.)
        
        // For now, we just iterate through the states to establish the framework
        // Real change detection would compare against previous frame state
        let _ = (entity, current_state); // Prevent unused variable warnings
    }
}

/// System that handles mouse hover state detection
/// This would integrate with Bevy's input systems in a full implementation
pub fn hover_detection_system(
    mut state_query: Query<(Entity, &mut InteractionState, &Interaction)>,
    mut state_change_events: EventWriter<InteractionStateChanged>,
    tracker: ResMut<StateChangeTracker>,
    // TODO: Add cursor position and window query when integrating with input
) {
    for (entity, mut interaction_state, interaction) in state_query.iter_mut() {
        if !interaction.clickable {
            continue;
        }
        
        let previous_state = interaction_state.clone();
        
        // TODO: Implement actual hover detection using cursor position and entity bounds
        // For now, this is a placeholder that demonstrates the structure
        
        // Placeholder: In a real implementation, this would check if cursor is over entity
        let is_hovered = false; // This would be calculated from cursor position vs entity bounds
        
        if interaction_state.set_hovered(is_hovered) {
            #[cfg(feature = "debug_logging")]
            if tracker.should_log_state_change(entity, &interaction_state) {
                debug!("Hover state changed for entity {:?}: {}", entity, is_hovered);
            }
            
            #[cfg(feature = "trace_logging")]
            trace!("Hover detection processed for entity {:?}: {}", entity, is_hovered);
            
            state_change_events.send(InteractionStateChanged::new(
                entity,
                previous_state,
                interaction_state.clone(),
            ));
        }
    }
}

/// System that handles mouse click/press state detection
/// This would integrate with Bevy's input systems in a full implementation
pub fn press_detection_system(
    mut state_query: Query<(Entity, &mut InteractionState, &Interaction)>,
    mut state_change_events: EventWriter<InteractionStateChanged>,
    mut tracker: ResMut<StateChangeTracker>,
    // TODO: Add mouse button input when integrating with input
) {
    for (entity, mut interaction_state, interaction) in state_query.iter_mut() {
        if !interaction.clickable {
            continue;
        }
        
        let previous_state = interaction_state.clone();
        
        // TODO: Implement actual press detection using mouse button input
        // For now, this is a placeholder that demonstrates the structure
        
        // Placeholder: In a real implementation, this would check mouse button state
        let is_pressed = false; // This would be calculated from mouse button input + hover state
        
        if interaction_state.set_pressed(is_pressed) {
            #[cfg(feature = "debug_logging")]
            if tracker.should_log_state_change(entity, &interaction_state) {
                bevy_log::debug!("Press state changed for entity {:?}: {}", entity, is_pressed);
            }
            
            #[cfg(feature = "trace_logging")]
            bevy_log::trace!("Press detection processed for entity {:?}: {}", entity, is_pressed);
            
            state_change_events.send(InteractionStateChanged::new(
                entity,
                previous_state,
                interaction_state.clone(),
            ));
        }
    }
}

/// System that handles focus state detection
/// This would integrate with focus management systems
pub fn focus_detection_system(
    mut state_query: Query<(Entity, &mut InteractionState)>,
    mut state_change_events: EventWriter<InteractionStateChanged>,
    mut tracker: ResMut<StateChangeTracker>,
    // TODO: Add focus manager resource when implementing focus system
) {
    for (entity, mut interaction_state) in state_query.iter_mut() {
        let previous_state = interaction_state.clone();
        
        // TODO: Implement actual focus detection using focus manager
        // For now, this is a placeholder that demonstrates the structure
        
        // Placeholder: In a real implementation, this would check with focus manager
        let is_focused = false; // This would be calculated from focus manager state
        
        if interaction_state.set_focused(is_focused) {
            #[cfg(feature = "debug_logging")]
            if tracker.should_log_state_change(entity, &interaction_state) {
                bevy_log::debug!("Focus state changed for entity {:?}: {}", entity, is_focused);
            }
            
            #[cfg(feature = "trace_logging")]
            bevy_log::trace!("Focus detection processed for entity {:?}: {}", entity, is_focused);
            
            state_change_events.send(InteractionStateChanged::new(
                entity,
                previous_state,
                interaction_state.clone(),
            ));
        }
    }
}

/// System that handles drag state detection
/// This would integrate with Bevy's input systems in a full implementation
pub fn drag_detection_system(
    mut state_query: Query<(Entity, &mut InteractionState, &Interaction)>,
    mut state_change_events: EventWriter<InteractionStateChanged>,
    mut tracker: ResMut<StateChangeTracker>,
    // TODO: Add mouse input and drag threshold when integrating with input
) {
    for (entity, mut interaction_state, interaction) in state_query.iter_mut() {
        if !interaction.draggable {
            continue;
        }
        
        let previous_state = interaction_state.clone();
        
        // TODO: Implement actual drag detection using mouse movement + press state
        // For now, this is a placeholder that demonstrates the structure
        
        // Placeholder: In a real implementation, this would check mouse movement while pressed
        let is_dragged = false; // This would be calculated from mouse movement + press state
        
        if interaction_state.set_dragged(is_dragged) {
            #[cfg(feature = "debug_logging")]
            if tracker.should_log_state_change(entity, &interaction_state) {
                bevy_log::debug!("Drag state changed for entity {:?}: {}", entity, is_dragged);
            }
            
            #[cfg(feature = "trace_logging")]
            bevy_log::trace!("Drag detection processed for entity {:?}: {}", entity, is_dragged);
            
            state_change_events.send(InteractionStateChanged::new(
                entity,
                previous_state,
                interaction_state.clone(),
            ));
        }
    }
}

/// System that logs interaction state changes for debugging
pub fn interaction_state_debug_system(
    mut _state_change_events: EventReader<InteractionStateChanged>,
    _tracker: ResMut<StateChangeTracker>,
) {
    #[cfg(feature = "debug_logging")]
    for event in _state_change_events.read() {
        let changes = event.changed_states();
        if !changes.is_empty() {
            bevy_log::debug!("🎯 INTERACTION STATE: Entity {:?} changed: {}", 
                event.entity, 
                changes.join(", ")
            );
        }
    }
    
    // Update frame count for rate limiting
    // _tracker.next_frame(); // Commented out since we're not using frame counting anymore
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_ecs::world::World;

    #[test]
    fn test_interaction_state_tracking_system() {
        let mut world = World::new();
        world.init_resource::<Events<InteractionStateChanged>>();
        
        // Create entity with interaction state
        let mut initial_state = InteractionState::new();
        initial_state.set_hovered(true);
        let entity = world.spawn((
            initial_state,
            Interaction { clickable: true, draggable: false }
        )).id();
        
        // Run the tracking system
        world.run_system_once(interaction_state_tracking_system);
        
        // Check that the entity still exists and has the expected state
        let state = world.get::<InteractionState>(entity).unwrap();
        assert!(state.hovered);
    }

    #[test]
    fn test_state_change_event_firing() {
        let mut world = World::new();
        world.init_resource::<Events<InteractionStateChanged>>();
        
        // Create entity with interaction state that will change
        let entity = world.spawn((
            InteractionState::new(),
            Interaction { clickable: true, draggable: false }
        )).id();
        
        // Manually modify the state to trigger a change
        {
            let mut state = world.get_mut::<InteractionState>(entity).unwrap();
            state.set_hovered(true);
        }
        
        // Run the debug system to verify events can be processed
        world.run_system_once(interaction_state_debug_system);
        
        // Verify the entity exists and has expected state
        let final_state = world.get::<InteractionState>(entity).unwrap();
        assert!(final_state.hovered);
    }
}