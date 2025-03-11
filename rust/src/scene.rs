use crate::Vertex;

#[derive(Debug)]
pub struct RenderObject {
    pub vertices: Vec<Vertex>,
    pub vertex_shader_filename: String,
    pub fragment_shader_filename: String,
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