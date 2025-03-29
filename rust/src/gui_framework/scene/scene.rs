use crate::Vertex;

#[derive(Debug, Clone)]
pub struct InstanceData {
    pub offset: [f32; 2],
}

#[derive(Debug)]
pub struct ElementPool {
    elements: Vec<RenderObject>,
    free_indices: Vec<usize>,
}

impl ElementPool {
    pub fn new(capacity: usize) -> Self {
        Self {
            elements: Vec::with_capacity(capacity),
            free_indices: Vec::new(),
        }
    }

    pub fn acquire(&mut self, template: RenderObject) -> usize {
        if let Some(index) = self.free_indices.pop() {
            self.elements[index] = template;
            index
        } else {
            let index = self.elements.len();
            self.elements.push(template);
            index
        }
    }

    pub fn release(&mut self, index: usize) {
        if index < self.elements.len() {
            self.free_indices.push(index);
        }
    }

    pub fn len(&self) -> usize {
        self.elements.len()
    }
    
    pub fn iter(&self) -> std::slice::Iter<RenderObject> {
        self.elements.iter()
    }
    
    pub fn iter_mut(&mut self) -> std::slice::IterMut<RenderObject> {
        self.elements.iter_mut()
    }

    pub fn get(&self, index: usize) -> Option<&RenderObject> {
        self.elements.get(index)
    }
}

#[derive(Debug)]
pub struct RenderObject {
    pub vertices: Vec<Vertex>,
    pub vertex_shader_filename: String,
    pub fragment_shader_filename: String,
    pub depth: f32,
    pub on_window_resize_scale: bool,
    pub on_window_resize_move: bool,
    pub offset: [f32; 2],
    pub is_draggable: bool,
    pub instances: Vec<InstanceData>,
}

pub trait HitTestable {
    fn contains(&self, x: f32, y: f32, window_height: f32, offset: [f32; 2]) -> bool;
}

impl HitTestable for RenderObject {
    fn contains(&self, x: f32, y: f32, window_height: f32, offset: [f32; 2]) -> bool {
        let adjusted_y = window_height - y;
        let (min_x, max_x, min_y, max_y) = self.vertices.iter().fold(
            (f32::INFINITY, f32::NEG_INFINITY, f32::INFINITY, f32::NEG_INFINITY),
            |acc, v| {
                let pos_x = v.position[0] + offset[0];
                let pos_y = v.position[1] + offset[1];
                (acc.0.min(pos_x), acc.1.max(pos_x), acc.2.min(pos_y), acc.3.max(pos_y))
            }
        );
        //println!("Checking object (depth {}): x=[{}, {}], y=[{}, {}], click=({}, {}), offset={:?}",
                 //self.depth, min_x, max_x, min_y, max_y, x, adjusted_y, offset);
        x >= min_x && x <= max_x && adjusted_y >= min_y && adjusted_y <= max_y
    }
}

#[derive(Debug)]
pub struct Scene {
    pub pool: ElementPool,
    pub width: f32,
    pub height: f32,
}

impl Scene {
    pub fn new() -> Self {
        Scene {
            pool: ElementPool::new(10000),
            width: 600.0,
            height: 300.0,
        }
    }

    pub fn add_object(&mut self, object: RenderObject) -> usize {
        self.pool.acquire(object)
    }

    pub fn add_objects(&mut self, templates: Vec<RenderObject>) -> Vec<usize> {
        templates.into_iter().map(|t| self.pool.acquire(t)).collect()
    }

    pub fn update_element(&mut self, element_id: usize, new_offset: [f32; 2]) {
        self.pool.elements[element_id].offset = new_offset;
    }

    pub fn add_instance(&mut self, object_id: usize, offset: [f32; 2]) -> usize {
        let instance_data = InstanceData { offset };
        self.pool.elements[object_id].instances.push(instance_data);
        self.pool.elements[object_id].instances.len() - 1
    }

    pub fn pick_object_at(&self, x: f32, y: f32) -> Option<(usize, Option<usize>)> {
        self.pool.iter().enumerate()
            .filter(|(_, obj)| obj.is_draggable)
            .flat_map(|(id, obj)| {
                let mut hits = Vec::new();
                if obj.contains(x, y, self.height, obj.offset) {
                    hits.push((id, None, obj.depth));
                }
                for (instance_id, instance) in obj.instances.iter().enumerate() {
                    let total_offset = [obj.offset[0] + instance.offset[0], obj.offset[1] + instance.offset[1]];
                    if obj.contains(x, y, self.height, total_offset) {
                        hits.push((id, Some(instance_id), obj.depth));
                    }
                }
                hits
            })
            .max_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(id, instance_id, _)| (id, instance_id))
    }

    pub fn translate_object(&mut self, index: usize, dx: f32, dy: f32, instance_id: Option<usize>) {
        if index < self.pool.len() {
            let obj = &mut self.pool.elements[index];
            match instance_id {
                Some(i) if i < obj.instances.len() => {
                    obj.instances[i].offset[0] += dx;
                    obj.instances[i].offset[1] += dy;
                }
                _ => {
                    obj.offset[0] += dx;
                    obj.offset[1] += dy;
                }
            }
        }
    }

    pub fn update_dimensions(&mut self, width: u32, height: u32) {
        self.width = width as f32;
        self.height = height as f32;
    }
}