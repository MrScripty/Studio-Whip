use bevy_ecs::prelude::Component;
use crate::Vertex;
use std::sync::Arc; // Add Arc

/// Component holding the visual representation data for an entity.
#[derive(Component, Debug, Clone)]
pub struct ShapeData {
    /// Vertices defining the shape's geometry. Use Arc for potential sharing.
    pub vertices: Arc<Vec<Vertex>>, // Changed to Arc<Vec<Vertex>>
    /// Path to the vertex shader file (relative to shader directory).
    pub vertex_shader_path: String,
    /// Path to the fragment shader file (relative to shader directory).
    pub fragment_shader_path: String,
    // Note: Depth is now handled by Transform.translation.z / GlobalTransform
}