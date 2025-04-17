use bevy_ecs::prelude::Component;
use crate::Vertex; // Assuming Vertex is defined in src/lib.rs or crate root

/// Component holding the visual representation data for an entity.
#[derive(Component, Debug, Clone)]
pub struct ShapeData {
    /// Vertices defining the shape's geometry.
    pub vertices: Vec<Vertex>,
    /// Path to the vertex shader file (relative to shader directory).
    pub vertex_shader_path: String,
    /// Path to the fragment shader file (relative to shader directory).
    pub fragment_shader_path: String,
    // Note: Depth is now handled by Transform.translation.z
}