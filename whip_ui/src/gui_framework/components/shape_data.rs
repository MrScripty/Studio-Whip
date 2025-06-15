use bevy_ecs::prelude::Component;
use bevy_reflect::Reflect;
use crate::Vertex;
use std::sync::Arc;
use bevy_color::Color; // Import Bevy Color

/// Component holding the visual representation data for an entity.
#[derive(Component, Debug, Clone, Reflect)]
pub struct ShapeData {
    /// Vertices defining the shape's geometry. Use Arc for potential sharing.
    pub vertices: Arc<Vec<Vertex>>,
    // pub vertex_shader_path: String, // REMOVED
    // pub fragment_shader_path: String, // REMOVED
    /// Color of the shape.
    pub color: Color, // ADDED
}

// Optional: Add a default implementation if useful
impl Default for ShapeData {
    fn default() -> Self {
        Self {
            vertices: Arc::new(Vec::new()), // Default to empty vertices
            color: Color::srgb(100.0, 0.0, 58.0),
        }
    }
}