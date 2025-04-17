use bevy_ecs::prelude::{Entity, Event};
use bevy_math::Vec2;

/// Event sent when a clickable entity is clicked.
#[derive(Event, Debug, Clone, Copy)]
pub struct EntityClicked {
    pub entity: Entity,
    // Add button, click count, etc. if needed later
}

/// Event sent when a draggable entity is being dragged.
#[derive(Event, Debug, Clone, Copy)]
pub struct EntityDragged {
    pub entity: Entity,
    /// The change in position since the last drag event for this entity.
    pub delta: Vec2,
}

/// Event sent when a configured hotkey combination is pressed.
#[derive(Event, Debug, Clone)]
pub struct HotkeyActionTriggered {
    /// The action string associated with the hotkey in the config file.
    pub action: String,
}