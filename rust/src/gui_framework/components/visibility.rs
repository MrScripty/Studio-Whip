use bevy_ecs::prelude::Component;
// If reflection is needed later (Task 6.4), add:
// use bevy_reflect::Reflect;

/// Custom visibility component to avoid Bevy's rendering stack.
/// Controls whether the entity is processed by the custom Vulkan renderer.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
// #[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Reflect)] // With reflection
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