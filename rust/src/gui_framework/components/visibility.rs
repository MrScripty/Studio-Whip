use bevy_ecs::prelude::Component;
use bevy_reflect::Reflect;

/// Custom visibility component to avoid Bevy's rendering stack.
/// Controls whether the entity is processed by the custom Vulkan renderer.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Reflect)]
pub struct Visibility(pub bool);

impl Default for Visibility {
    /// Entities are visible by default.
    fn default() -> Self {
        Self(true)
    }
}

impl Visibility {
    pub fn is_visible(&self) -> bool {
        self.0
    }
}