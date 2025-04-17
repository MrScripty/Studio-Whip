use bevy_ecs::prelude::Component;

/// Component defining how an entity can be interacted with via mouse/input.
#[derive(Component, Debug, Clone, Copy)]
pub struct Interaction {
    /// Can this entity be clicked?
    pub clickable: bool,
    /// Can this entity be dragged?
    pub draggable: bool,
    // Add other interaction flags as needed (e.g., hoverable)
}

impl Default for Interaction {
    /// Entities are not interactive by default.
    fn default() -> Self {
        Self {
            clickable: false,
            draggable: false,
        }
    }
}