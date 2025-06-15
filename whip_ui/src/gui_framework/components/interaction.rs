use bevy_ecs::prelude::Component;
use bevy_reflect::Reflect;

/// Component defining how an entity can be interacted with via mouse/input.
#[derive(Component, Debug, Clone, Copy, Reflect)]
pub struct Interaction {
    pub clickable: bool,
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