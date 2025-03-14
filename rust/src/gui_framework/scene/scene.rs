use crate::Vertex;

#[derive(Debug)]
pub struct RenderObject {
    pub vertices: Vec<Vertex>,
    pub vertex_shader_filename: String,
    pub fragment_shader_filename: String,
    pub depth: f32,                    // For 2D layering
    pub on_window_resize_scale: bool,  // Scales to match window size
    pub on_window_resize_move: bool,   // Moves proportionally (GUI elements)
}

#[derive(Debug)]
pub struct Scene {
    pub render_objects: Vec<RenderObject>,
}

impl Scene {
    pub fn new() -> Self {
        Scene {
            render_objects: Vec::new(),
        }
    }

    pub fn add_object(&mut self, object: RenderObject) {
        self.render_objects.push(object);
    }
}