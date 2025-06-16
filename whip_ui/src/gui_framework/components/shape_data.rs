use bevy_ecs::prelude::Component;
use bevy_reflect::Reflect;
use crate::Vertex;
use std::sync::Arc;
use bevy_color::Color;
use serde::{Deserialize, Serialize};

/// Enum defining shape scaling behavior for Taffy layout integration
#[derive(Debug, Clone, PartialEq, Reflect, Serialize, Deserialize)]
pub enum ShapeScaling {
    /// No scaling - use vertices as-is (for hardcoded shapes)
    Fixed,
    /// Scale vertices uniformly based on Taffy computed size
    Uniform,
    /// Scale vertices to fit Taffy computed width/height independently 
    Stretch,
}

impl Default for ShapeScaling {
    fn default() -> Self {
        ShapeScaling::Fixed
    }
}

/// Component holding the visual representation data for an entity.
#[derive(Component, Debug, Clone, Reflect)]
pub struct ShapeData {
    /// Vertices defining the shape's geometry
    pub vertices: Arc<Vec<Vertex>>,
    /// Color of the shape (supports hex colors)
    pub color: Color,
    /// How this shape should be scaled by Taffy layout
    pub scaling: ShapeScaling,
    /// Original vertices before scaling (for recalculation)
    pub original_vertices: Option<Arc<Vec<Vertex>>>,
}

impl Default for ShapeData {
    fn default() -> Self {
        Self {
            vertices: Arc::new(Vec::new()),
            color: Color::srgb(0.5, 0.5, 0.5),
            scaling: ShapeScaling::Fixed,
            original_vertices: None,
        }
    }
}

impl ShapeData {
    /// Create a new shape with vertices and color (backwards compatible)
    pub fn new(vertices: Vec<Vertex>, color: Color) -> Self {
        Self {
            vertices: Arc::new(vertices),
            color,
            scaling: ShapeScaling::Fixed,
            original_vertices: None,
        }
    }
    
    /// Create a shape that can be scaled by Taffy layout
    pub fn scalable(vertices: Vec<Vertex>, color: Color, scaling: ShapeScaling) -> Self {
        Self {
            vertices: Arc::new(vertices.clone()),
            color,
            scaling,
            original_vertices: Some(Arc::new(vertices)), // Store originals for scaling
        }
    }
    
    /// Create a triangle shape (helper for common shapes)
    pub fn triangle(width: f32, height: f32, color: Color) -> Self {
        let half_w = width / 2.0;
        let half_h = height / 2.0;
        let vertices = vec![
            Vertex { position: [-half_w, -half_h] },
            Vertex { position: [0.0, half_h] },
            Vertex { position: [half_w, -half_h] },
        ];
        Self::new(vertices, color)
    }
    
    /// Create a rectangle shape (helper for common shapes)
    pub fn rectangle(width: f32, height: f32, color: Color) -> Self {
        let half_w = width / 2.0;
        let half_h = height / 2.0;
        let vertices = vec![
            // Triangle 1
            Vertex { position: [-half_w, -half_h] },
            Vertex { position: [-half_w, half_h] },
            Vertex { position: [half_w, -half_h] },
            // Triangle 2
            Vertex { position: [half_w, -half_h] },
            Vertex { position: [-half_w, half_h] },
            Vertex { position: [half_w, half_h] },
        ];
        Self::new(vertices, color)
    }
    
    /// Custom shape with explicit vertices (backwards compatibility)
    pub fn custom(vertices: Vec<Vertex>, color: Color) -> Self {
        Self::new(vertices, color)
    }
    
    /// Scale vertices based on Taffy computed size
    pub fn scale_vertices(&mut self, target_width: f32, target_height: f32) {
        if matches!(self.scaling, ShapeScaling::Fixed) {
            return; // Don't scale fixed shapes
        }
        
        let original_vertices = match &self.original_vertices {
            Some(orig) => orig.clone(),
            None => {
                // If no originals stored, use current vertices as baseline
                self.original_vertices = Some(self.vertices.clone());
                self.vertices.clone()
            }
        };
        
        // Calculate bounding box of original vertices
        let mut min_x = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        
        for vertex in original_vertices.iter() {
            min_x = min_x.min(vertex.position[0]);
            max_x = max_x.max(vertex.position[0]);
            min_y = min_y.min(vertex.position[1]);
            max_y = max_y.max(vertex.position[1]);
        }
        
        let original_width = max_x - min_x;
        let original_height = max_y - min_y;
        
        if original_width == 0.0 || original_height == 0.0 {
            return; // Can't scale zero-sized shapes
        }
        
        // Calculate scale factors
        let (scale_x, scale_y) = match self.scaling {
            ShapeScaling::Fixed => (1.0, 1.0),
            ShapeScaling::Uniform => {
                let scale = (target_width / original_width).min(target_height / original_height);
                (scale, scale)
            },
            ShapeScaling::Stretch => (target_width / original_width, target_height / original_height),
        };
        
        // Apply scaling
        let scaled_vertices: Vec<Vertex> = original_vertices.iter().map(|vertex| {
            Vertex {
                position: [
                    vertex.position[0] * scale_x,
                    vertex.position[1] * scale_y,
                ]
            }
        }).collect();
        
        self.vertices = Arc::new(scaled_vertices);
    }
    
    /// Parse hex color string to Color (e.g., "#FF0000" -> red)
    pub fn from_hex_color(hex: &str) -> Result<Color, &'static str> {
        if !hex.starts_with('#') || hex.len() != 7 {
            return Err("Invalid hex color format. Expected #RRGGBB");
        }
        
        let r = u8::from_str_radix(&hex[1..3], 16).map_err(|_| "Invalid red component")?;
        let g = u8::from_str_radix(&hex[3..5], 16).map_err(|_| "Invalid green component")?;
        let b = u8::from_str_radix(&hex[5..7], 16).map_err(|_| "Invalid blue component")?;
        
        Ok(Color::srgb(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0))
    }
}