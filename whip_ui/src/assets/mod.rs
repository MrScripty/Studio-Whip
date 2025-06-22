pub mod systems;
pub mod plugin;
pub mod definitions;
pub mod registry;
pub mod loaders;

#[cfg(test)]
mod tests;

// Re-export modules
pub use systems::*;
pub use plugin::*;
pub use definitions::*;
pub use registry::*;
pub use loaders::*;

/// Window configuration loaded from TOML
#[derive(Debug, Clone, bevy_ecs::prelude::Resource, serde::Deserialize, serde::Serialize)]
pub struct WindowConfig {
    /// Window size [width, height]
    pub size: [f32; 2],
    /// Background color for the window
    pub background_color: Option<crate::widgets::blueprint::ColorDef>,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            size: [600.0, 300.0],
            background_color: Some(crate::widgets::blueprint::ColorDef::Rgba { r: 33, g: 41, b: 42, a: 1.0 }),
        }
    }
}