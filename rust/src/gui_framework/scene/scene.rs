use crate::Vertex;

#[derive(Debug, Clone)] // New: Instance data structure
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
}

#[derive(Debug)]
pub struct Group {
    element_ids: Vec<usize>,
    is_draggable: bool,
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

#[derive(Debug)]
pub struct Scene {
    pub pool: ElementPool,
    pub groups: Vec<Group>,
    pub width: f32,  // Current window width
    pub height: f32, // Current window height
}

pub trait HitTestable {
    fn contains(&self, x: f32, y: f32, window_height: f32) -> bool;
}

impl HitTestable for RenderObject {
    fn contains(&self, x: f32, y: f32, window_height: f32) -> bool {
        let adjusted_y = window_height - y; // Dynamic window height
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
            pool: ElementPool::new(10000),
            groups: Vec::new(),
            width: 600.0,  // Initial window width
            height: 300.0, // Initial window height
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

    pub fn add_group(&mut self, elements: Vec<RenderObject>, is_draggable: bool) -> usize {
        let ids = self.add_objects(elements);
        let group_id = self.groups.len();
        self.groups.push(Group { element_ids: ids, is_draggable });
        group_id
    }

    pub fn add_to_group(&mut self, group_id: usize, elements: Vec<RenderObject>) {
        let ids = self.add_objects(elements);
        self.groups[group_id].element_ids.extend(ids);
    }

    pub fn add_instance(&mut self, object_id: usize, offset: [f32; 2]) -> usize {
        let instance_data = InstanceData { offset };
        self.pool.elements[object_id].instances.push(instance_data);
        self.pool.elements[object_id].instances.len() - 1 // Return instance index
    }

    pub fn pick_object_at(&self, x: f32, y: f32) -> Option<usize> {
        // Check groups first
        self.groups.iter().enumerate()
            .filter(|(_, group)| group.is_draggable)
            .find_map(|(group_id, group)| {
                group.element_ids.iter().any(|&id| {
                    self.pool.elements[id].contains(x, y, self.height)
                }).then_some(group_id)
            })
            .or_else(|| {
                // Fallback to individual draggable objects
                self.pool.iter().enumerate()
                    .filter(|(_, obj)| obj.is_draggable && obj.contains(x, y, self.height))
                    .max_by(|a, b| a.1.depth.partial_cmp(&b.1.depth).unwrap_or(std::cmp::Ordering::Equal))
                    .map(|(i, _)| i)
            })
    }

    pub fn translate_object(&mut self, index: usize, dx: f32, dy: f32) {
        if index < self.groups.len() {
            for &id in &self.groups[index].element_ids {
                self.pool.elements[id].offset[0] += dx;
                self.pool.elements[id].offset[1] += dy;
            }
        } else if index < self.pool.len() {
            let obj = &mut self.pool.elements[index];
            obj.offset[0] += dx;
            obj.offset[1] += dy;
        }
    }

    pub fn update_dimensions(&mut self, width: u32, height: u32) {
        self.width = width as f32;
        self.height = height as f32;
    }
}