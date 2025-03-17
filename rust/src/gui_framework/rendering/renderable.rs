use ash::vk;

pub struct Renderable {
    pub vertex_buffer: vk::Buffer,
    pub vertex_allocation: vk_mem::Allocation,
    pub vertex_shader: vk::ShaderModule,
    pub fragment_shader: vk::ShaderModule,
    pub pipeline: vk::Pipeline,
    pub vertex_count: u32,
    pub depth: f32,
    pub on_window_resize_scale: bool,
    pub on_window_resize_move: bool,
    pub original_positions: Vec<[f32; 2]>,
    pub fixed_size: [f32; 2],
    pub center_ratio: [f32; 2],
    pub offset_uniform: vk::Buffer,      // Added for shader-based offset
    pub offset_allocation: vk_mem::Allocation, // Added for shader-based offset
}