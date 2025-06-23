use bevy_ecs::prelude::*;

/// Component that tracks the current interaction state of a widget
#[derive(Component, Debug, Clone, Default)]
pub struct InteractionState {
    /// Whether the widget is currently being hovered
    pub hovered: bool,
    /// Whether the widget is currently being pressed/clicked
    pub pressed: bool,
    /// Whether the widget currently has focus
    pub focused: bool,
    /// Whether the widget is currently being dragged
    pub dragged: bool,
}

impl InteractionState {
    /// Create a new interaction state with all states set to false
    pub fn new() -> Self {
        Self {
            hovered: false,
            pressed: false,
            focused: false,
            dragged: false,
        }
    }

    /// Check if any interaction state is active
    pub fn has_any_interaction(&self) -> bool {
        self.hovered || self.pressed || self.focused || self.dragged
    }

    /// Check if the widget is in an active state (pressed or focused)
    pub fn is_active(&self) -> bool {
        self.pressed || self.focused
    }

    /// Reset all interaction states to false
    pub fn reset(&mut self) {
        self.hovered = false;
        self.pressed = false;
        self.focused = false;
        self.dragged = false;
    }

    /// Set hover state and return true if state changed
    pub fn set_hovered(&mut self, hovered: bool) -> bool {
        let changed = self.hovered != hovered;
        self.hovered = hovered;
        changed
    }

    /// Set pressed state and return true if state changed
    pub fn set_pressed(&mut self, pressed: bool) -> bool {
        let changed = self.pressed != pressed;
        self.pressed = pressed;
        changed
    }

    /// Set focused state and return true if state changed
    pub fn set_focused(&mut self, focused: bool) -> bool {
        let changed = self.focused != focused;
        self.focused = focused;
        changed
    }

    /// Set dragged state and return true if state changed
    pub fn set_dragged(&mut self, dragged: bool) -> bool {
        let changed = self.dragged != dragged;
        self.dragged = dragged;
        changed
    }
}

/// Event fired when an interaction state changes
#[derive(Event, Debug, Clone)]
pub struct InteractionStateChanged {
    /// The entity whose state changed
    pub entity: Entity,
    /// The previous state
    pub previous_state: InteractionState,
    /// The new state
    pub new_state: InteractionState,
}

impl InteractionStateChanged {
    /// Create a new state change event
    pub fn new(entity: Entity, previous_state: InteractionState, new_state: InteractionState) -> Self {
        Self {
            entity,
            previous_state,
            new_state,
        }
    }

    /// Check if hover state changed
    pub fn hover_changed(&self) -> bool {
        self.previous_state.hovered != self.new_state.hovered
    }

    /// Check if pressed state changed
    pub fn pressed_changed(&self) -> bool {
        self.previous_state.pressed != self.new_state.pressed
    }

    /// Check if focused state changed
    pub fn focused_changed(&self) -> bool {
        self.previous_state.focused != self.new_state.focused
    }

    /// Check if dragged state changed
    pub fn dragged_changed(&self) -> bool {
        self.previous_state.dragged != self.new_state.dragged
    }

    /// Get a description of what changed
    pub fn changed_states(&self) -> Vec<String> {
        let mut changes = Vec::new();
        
        if self.hover_changed() {
            changes.push(format!("hovered: {} -> {}", self.previous_state.hovered, self.new_state.hovered));
        }
        if self.pressed_changed() {
            changes.push(format!("pressed: {} -> {}", self.previous_state.pressed, self.new_state.pressed));
        }
        if self.focused_changed() {
            changes.push(format!("focused: {} -> {}", self.previous_state.focused, self.new_state.focused));
        }
        if self.dragged_changed() {
            changes.push(format!("dragged: {} -> {}", self.previous_state.dragged, self.new_state.dragged));
        }
        
        changes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interaction_state_creation() {
        let state = InteractionState::new();
        assert!(!state.hovered);
        assert!(!state.pressed);
        assert!(!state.focused);
        assert!(!state.dragged);
        assert!(!state.has_any_interaction());
        assert!(!state.is_active());
    }

    #[test]
    fn test_interaction_state_setters() {
        let mut state = InteractionState::new();

        // Test hover
        assert!(state.set_hovered(true));
        assert!(state.hovered);
        assert!(state.has_any_interaction());
        assert!(!state.is_active());
        assert!(!state.set_hovered(true)); // No change

        // Test pressed
        assert!(state.set_pressed(true));
        assert!(state.pressed);
        assert!(state.is_active());
        assert!(!state.set_pressed(true)); // No change

        // Test focused
        assert!(state.set_focused(true));
        assert!(state.focused);
        assert!(state.is_active());

        // Test dragged
        assert!(state.set_dragged(true));
        assert!(state.dragged);

        // Test reset
        state.reset();
        assert!(!state.has_any_interaction());
        assert!(!state.is_active());
    }

    #[test]
    fn test_state_change_event() {
        let entity = Entity::from_raw(42);
        let previous = InteractionState::new();
        let mut new_state = InteractionState::new();
        new_state.set_hovered(true);
        new_state.set_pressed(true);

        let event = InteractionStateChanged::new(entity, previous, new_state);

        assert_eq!(event.entity, entity);
        assert!(event.hover_changed());
        assert!(event.pressed_changed());
        assert!(!event.focused_changed());
        assert!(!event.dragged_changed());

        let changes = event.changed_states();
        assert_eq!(changes.len(), 2);
        assert!(changes.contains(&"hovered: false -> true".to_string()));
        assert!(changes.contains(&"pressed: false -> true".to_string()));
    }
}