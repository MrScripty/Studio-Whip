use crate::Vertex;

#[derive(Debug)]
pub struct RenderObject {
    pub vertices: Vec<Vertex>,
    pub vertex_shader_filename: String,
    pub fragment_shader_filename: String,
    pub depth: f32,
    pub on_window_resize_scale: bool,
    pub on_window_resize_move: bool,
    pub offset: [f32; 2],
    pub is_draggable: bool, // Added to control drag behavior
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
        let window_height = 300.0; // Hardcoded for now; ideally passed from context
        let adjusted_y = window_height - y; // Convert top-left Y to bottom-left Y
        let (min_x, max_x, min_y, max_y) = self.vertices.iter().fold(
            (f32::INFINITY, f32::NEG_INFINITY, f32::INFINITY, f32::NEG_INFINITY),
            |acc, v| {
                let pos_x = v.position[0] + self.offset[0];
                let pos_y = v.position[1] + self.offset[1];
                (acc.0.min(pos_x), acc.1.max(pos_x), acc.2.min(pos_y), acc.3.max(pos_y))
            }
        );
        println!("Checking object (depth {}): x=[{}, {}], y=[{}, {}], click=({}, {})",
                 self.depth, min_x, max_x, min_y, max_y, x, adjusted_y);
        x >= min_x && x <= max_x && adjusted_y >= min_y && adjusted_y <= max_y
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
        self.render_objects.iter().enumerate()
            .filter(|(_, obj)| obj.is_draggable && obj.contains(x, y)) // Only draggable objects
            .max_by(|a, b| a.1.depth.partial_cmp(&b.1.depth).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| i)
    }

    pub fn translate_object(&mut self, index: usize, dx: f32, dy: f32) {
        let obj = &mut self.render_objects[index];
        obj.offset[0] += dx;
        obj.offset[1] += dy;
    }
}