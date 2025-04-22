use bevy_ecs::prelude::Component;
use bevy_reflect::Reflect;
use crate::Vertex;
use std::sync::Arc;

/// Component holding the visual representation data for an entity.
// Note: For Arc<Vec<Vertex>> reflection to work, Vertex in lib.rs
// must derive Reflect and TypePath.
#[derive(Component, Debug, Clone, Reflect)]
pub struct ShapeData {
    /// Vertices defining the shape's geometry. Use Arc for potential sharing.
    // #[reflect(skip_serializing)] // Optional: Add if serialization of vertices is not needed or causes issues
    pub vertices: Arc<Vec<Vertex>>,
    pub vertex_shader_path: String,
    pub fragment_shader_path: String,
    // Note: Depth is now handled by Transform.translation.z / GlobalTransform
}