use crate::Vertex;

#[derive(Debug)]
pub struct RenderObject {
    pub vertices: Vec<Vertex>,
    pub vertex_shader_filename: String,
    pub fragment_shader_filename: String,
    pub depth: f32,                    // For 2D layering
    pub on_window_resize_scale: bool,  // Scales to match window size
    pub on_window_resize_move: bool,   // Moves proportionally (GUI elements)
    pub offset: [f32; 2],              // For dragging
}

#[derive(Debug)]
pub struct Scene {
    pub render_objects: Vec<RenderObject>,
}

pub trait HitTestable {
    fn contains(&self, x: f32, y: f32) -> bool;
}

impl HitTestable for RenderObject {
    fn contains(&self, x: f32, y: f32) -> bool {
        let (min_x, max_x, min_y, max_y) = self.vertices.iter().fold(
            (f32::MAX, f32::MIN, f32::MAX, f32::MIN),
            |acc, v| {
                let pos = v.position;
                (acc.0.min(pos[0]), acc.1.max(pos[0]), acc.2.min(pos[1]), acc.3.max(pos[1]))
            },
        );
        x >= min_x && x <= max_x && y >= min_y && y <= max_y
    }
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

    pub fn pick_object_at(&self, x: f32, y: f32) -> Option<usize> {
        self.render_objects
            .iter()
            .enumerate()
            .filter(|(_, obj)| obj.contains(x, y))
            .max_by(|a, b| a.1.depth.partial_cmp(&b.1.depth).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| i)
    }

    pub fn translate_object(&mut self, index: usize, dx: f32, dy: f32) {
        let obj = &mut self.render_objects[index];
        obj.offset[0] += dx;
        obj.offset[1] += dy;
    }
}