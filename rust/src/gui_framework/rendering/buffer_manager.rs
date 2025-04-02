use ash::vk;
use vk_mem::{Alloc, Allocator}; // Added Allocator import
use std::marker::PhantomData;
use crate::Vertex;
use crate::gui_framework::context::vulkan_context::VulkanContext;
use crate::gui_framework::scene::scene::{Scene, RenderObject}; // Added RenderObject import
use crate::gui_framework::rendering::renderable::Renderable;
use crate::gui_framework::rendering::shader_utils::load_shader;
use glam::Mat4;

pub struct BufferManager {
    pub uniform_buffer: vk::Buffer,
    pub uniform_allocation: vk_mem::Allocation,
    pub renderables: Vec<Renderable>,
    pub descriptor_set_layout: vk::DescriptorSetLayout,
    pub descriptor_pool: vk::DescriptorPool,
    // Removed descriptor_set, it's managed by PipelineManager now
}

impl BufferManager {
    pub fn new(platform: &mut VulkanContext, scene: &Scene, pipeline_layout: vk::PipelineLayout) -> Self {
        let device = platform.device.as_ref().unwrap(); // Get device ref
        let allocator = platform.allocator.as_ref().unwrap(); // Get allocator ref

        // --- Uniform Buffer Setup ---
        let ortho = Mat4::orthographic_rh(0.0, 600.0, 300.0, 0.0, -1.0, 1.0).to_cols_array();
        let (uniform_buffer, uniform_allocation) = {
            let buffer_info = vk::BufferCreateInfo {
                s_type: vk::StructureType::BUFFER_CREATE_INFO,
                size: std::mem::size_of_val(&ortho) as u64,
                usage: vk::BufferUsageFlags::UNIFORM_BUFFER,
                sharing_mode: vk::SharingMode::EXCLUSIVE,
                ..Default::default()
            };
            let allocation_info = vk_mem::AllocationCreateInfo {
                usage: vk_mem::MemoryUsage::AutoPreferDevice,
                flags: vk_mem::AllocationCreateFlags::MAPPED | vk_mem::AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE,
                ..Default::default()
            };
            unsafe {
                match allocator.create_buffer(&buffer_info, &allocation_info) {
                    Ok((buffer, mut allocation)) => {
                        let data_ptr = allocator.map_memory(&mut allocation).unwrap().cast::<f32>();
                        data_ptr.copy_from_nonoverlapping(ortho.as_ptr(), ortho.len());
                        allocator.unmap_memory(&mut allocation);
                        (buffer, allocation)
                    }
                    Err(e) => panic!("Uniform buffer creation failed: {:?}", e),
                }
            }
        };

        // --- Descriptor Set Layout (Common for all renderables) ---
        let descriptor_set_layout = unsafe {
            let bindings = [
                // Binding 0: Projection Matrix (UBO)
                vk::DescriptorSetLayoutBinding {
                    binding: 0,
                    descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                    descriptor_count: 1,
                    stage_flags: vk::ShaderStageFlags::VERTEX,
                    ..Default::default()
                },
                // Binding 1: Object Offset (UBO)
                vk::DescriptorSetLayoutBinding {
                    binding: 1,
                    descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                    descriptor_count: 1,
                    stage_flags: vk::ShaderStageFlags::VERTEX,
                    ..Default::default()
                },
            ];
            match device.create_descriptor_set_layout(&vk::DescriptorSetLayoutCreateInfo {
                s_type: vk::StructureType::DESCRIPTOR_SET_LAYOUT_CREATE_INFO,
                binding_count: bindings.len() as u32,
                p_bindings: bindings.as_ptr(),
                ..Default::default()
            }, None) {
                Ok(layout) => layout,
                Err(e) => panic!("Descriptor set layout creation failed: {:?}", e),
            }
        };

        // --- Descriptor Pool (Sized for projection + all renderables) ---
        let descriptor_pool = unsafe {
            let pool_sizes = [
                vk::DescriptorPoolSize {
                    ty: vk::DescriptorType::UNIFORM_BUFFER,
                    // Need 2 UBOs (proj, offset) per renderable + 1 for global projection (though not used directly here)
                    descriptor_count: 2 * (scene.pool.len() as u32),
                },
            ];
            match device.create_descriptor_pool(&vk::DescriptorPoolCreateInfo {
                s_type: vk::StructureType::DESCRIPTOR_POOL_CREATE_INFO,
                // Allow individual sets to be freed if needed later
                flags: vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET,
                max_sets: scene.pool.len() as u32, // One set per renderable
                pool_size_count: pool_sizes.len() as u32,
                p_pool_sizes: pool_sizes.as_ptr(),
                ..Default::default()
            }, None) {
                Ok(pool) => pool,
                Err(e) => panic!("Descriptor pool creation failed: {:?}", e),
            }
        };

        // --- Process Each Object in Scene ---
        let mut renderables = Vec::new();
        const DEFAULT_INSTANCE_CAPACITY: u32 = 64; // Default capacity if creating instance buffer

        for obj in scene.pool.iter() { // Iterate directly over objects in pool
            // --- Vertex Buffer ---
            let vertices = &obj.vertices;
            let (vertex_buffer, vertex_allocation) = {
                let buffer_info = vk::BufferCreateInfo {
                    s_type: vk::StructureType::BUFFER_CREATE_INFO,
                    size: (vertices.len() * std::mem::size_of::<Vertex>()) as u64,
                    usage: vk::BufferUsageFlags::VERTEX_BUFFER,
                    sharing_mode: vk::SharingMode::EXCLUSIVE,
                    ..Default::default()
                };
                let allocation_info = vk_mem::AllocationCreateInfo {
                    usage: vk_mem::MemoryUsage::AutoPreferDevice,
                    flags: vk_mem::AllocationCreateFlags::MAPPED | vk_mem::AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE,
                    ..Default::default()
                };
                unsafe {
                    match allocator.create_buffer(&buffer_info, &allocation_info) {
                        Ok((buffer, mut allocation)) => {
                            let data_ptr = allocator.map_memory(&mut allocation).unwrap().cast::<Vertex>();
                            data_ptr.copy_from_nonoverlapping(vertices.as_ptr(), vertices.len());
                            allocator.unmap_memory(&mut allocation);
                            (buffer, allocation)
                        }
                        Err(e) => panic!("Vertex buffer creation failed: {:?}", e),
                    }
                }
            };

            // --- Offset Uniform Buffer ---
            let (offset_uniform, offset_allocation) = {
                let buffer_info = vk::BufferCreateInfo {
                    s_type: vk::StructureType::BUFFER_CREATE_INFO,
                    size: std::mem::size_of::<[f32; 2]>() as u64,
                    usage: vk::BufferUsageFlags::UNIFORM_BUFFER,
                    sharing_mode: vk::SharingMode::EXCLUSIVE,
                    ..Default::default()
                };
                let allocation_info = vk_mem::AllocationCreateInfo {
                    usage: vk_mem::MemoryUsage::AutoPreferDevice,
                    flags: vk_mem::AllocationCreateFlags::MAPPED | vk_mem::AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE,
                    ..Default::default()
                };
                unsafe {
                    match allocator.create_buffer(&buffer_info, &allocation_info) {
                        Ok((buffer, mut allocation)) => {
                            let data_ptr = allocator.map_memory(&mut allocation).unwrap().cast::<f32>();
                            data_ptr.copy_from_nonoverlapping(obj.offset.as_ptr(), 2);
                            allocator.unmap_memory(&mut allocation);
                            (buffer, allocation)
                        }
                        Err(e) => panic!("Offset uniform buffer creation failed: {:?}", e),
                    }
                }
            };

            // --- Instance Buffer (Conditional) ---
            let enable_instancing = obj.is_draggable || !obj.instances.is_empty();
            let mut instance_buffer_capacity = 0;

            let (instance_buffer, instance_allocation, initial_instance_count) = if enable_instancing {
                let instance_data: Vec<[f32; 2]> = obj.instances.iter().map(|i| i.offset).collect();
                let initial_count = instance_data.len() as u32;
                instance_buffer_capacity = std::cmp::max(initial_count, DEFAULT_INSTANCE_CAPACITY);

                let buffer_info = vk::BufferCreateInfo {
                    s_type: vk::StructureType::BUFFER_CREATE_INFO,
                    size: (instance_buffer_capacity as usize * std::mem::size_of::<[f32; 2]>()) as u64,
                    usage: vk::BufferUsageFlags::VERTEX_BUFFER,
                    sharing_mode: vk::SharingMode::EXCLUSIVE,
                    ..Default::default()
                };
                let allocation_info = vk_mem::AllocationCreateInfo {
                    usage: vk_mem::MemoryUsage::AutoPreferDevice,
                    flags: vk_mem::AllocationCreateFlags::MAPPED | vk_mem::AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE,
                    ..Default::default()
                };
                unsafe {
                    match allocator.create_buffer(&buffer_info, &allocation_info) {
                        Ok((buffer, mut allocation)) => {
                            if !instance_data.is_empty() {
                                let data_ptr = allocator.map_memory(&mut allocation).unwrap().cast::<f32>();
                                data_ptr.copy_from_nonoverlapping(instance_data.as_ptr() as *const f32, instance_data.len() * 2);
                                allocator.unmap_memory(&mut allocation);
                            }
                            (Some(buffer), Some(allocation), initial_count)
                        }
                        Err(e) => panic!("Instance buffer creation failed: {:?}", e),
                    }
                }
            } else {
                (None, None, 0)
            };

            // --- Descriptor Set (Per Object) ---
            let descriptor_set = unsafe {
                match device.allocate_descriptor_sets(&vk::DescriptorSetAllocateInfo {
                    s_type: vk::StructureType::DESCRIPTOR_SET_ALLOCATE_INFO,
                    descriptor_pool,
                    descriptor_set_count: 1,
                    p_set_layouts: &descriptor_set_layout,
                    ..Default::default()
                }) {
                    Ok(sets) => sets[0],
                    Err(e) => panic!("Failed to allocate descriptor set: {:?}", e),
                }
            };

            // Update the descriptor set
            unsafe {
                let proj_buffer_info = vk::DescriptorBufferInfo {
                    buffer: uniform_buffer, // Use the shared projection buffer
                    offset: 0,
                    range: std::mem::size_of_val(&ortho) as u64,
                };
                let offset_buffer_info = vk::DescriptorBufferInfo {
                    buffer: offset_uniform, // Use this object's offset buffer
                    offset: 0,
                    range: std::mem::size_of::<[f32; 2]>() as u64,
                };
                let write_sets = [
                    // Write for binding 0 (Projection)
                    vk::WriteDescriptorSet {
                        s_type: vk::StructureType::WRITE_DESCRIPTOR_SET,
                        dst_set: descriptor_set,
                        dst_binding: 0,
                        descriptor_count: 1,
                        descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                        p_buffer_info: &proj_buffer_info,
                        ..Default::default()
                    },
                    // Write for binding 1 (Offset)
                    vk::WriteDescriptorSet {
                        s_type: vk::StructureType::WRITE_DESCRIPTOR_SET,
                        dst_set: descriptor_set,
                        dst_binding: 1,
                        descriptor_count: 1,
                        descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                        p_buffer_info: &offset_buffer_info,
                        ..Default::default()
                    },
                ];
                device.update_descriptor_sets(&write_sets, &[]);
            }

            // --- Shaders ---
            let vertex_shader = load_shader(device, &obj.vertex_shader_filename);
            let fragment_shader = load_shader(device, &obj.fragment_shader_filename);

            // --- Pipeline (Conditional Instancing) ---
            let pipeline = {
                let vertex_stage = vk::PipelineShaderStageCreateInfo {
                    s_type: vk::StructureType::PIPELINE_SHADER_STAGE_CREATE_INFO,
                    stage: vk::ShaderStageFlags::VERTEX,
                    module: vertex_shader,
                    p_name: c"main".as_ptr(),
                    ..Default::default()
                };
                let fragment_stage = vk::PipelineShaderStageCreateInfo {
                    s_type: vk::StructureType::PIPELINE_SHADER_STAGE_CREATE_INFO,
                    stage: vk::ShaderStageFlags::FRAGMENT,
                    module: fragment_shader,
                    p_name: c"main".as_ptr(),
                    ..Default::default()
                };
                let stages = [vertex_stage, fragment_stage];

                let (vertex_attributes, vertex_bindings) = if enable_instancing {
                    (
                        vec![
                            vk::VertexInputAttributeDescription { location: 0, binding: 0, format: vk::Format::R32G32_SFLOAT, offset: 0 },
                            vk::VertexInputAttributeDescription { location: 1, binding: 1, format: vk::Format::R32G32_SFLOAT, offset: 0 },
                        ],
                        vec![
                            vk::VertexInputBindingDescription { binding: 0, stride: std::mem::size_of::<Vertex>() as u32, input_rate: vk::VertexInputRate::VERTEX },
                            vk::VertexInputBindingDescription { binding: 1, stride: std::mem::size_of::<[f32; 2]>() as u32, input_rate: vk::VertexInputRate::INSTANCE },
                        ],
                    )
                } else {
                    (
                         vec![vk::VertexInputAttributeDescription { location: 0, binding: 0, format: vk::Format::R32G32_SFLOAT, offset: 0 }],
                         vec![vk::VertexInputBindingDescription { binding: 0, stride: std::mem::size_of::<Vertex>() as u32, input_rate: vk::VertexInputRate::VERTEX }],
                    )
                };

                let vertex_input = vk::PipelineVertexInputStateCreateInfo {
                    s_type: vk::StructureType::PIPELINE_VERTEX_INPUT_STATE_CREATE_INFO,
                    vertex_binding_description_count: vertex_bindings.len() as u32,
                    p_vertex_binding_descriptions: vertex_bindings.as_ptr(),
                    vertex_attribute_description_count: vertex_attributes.len() as u32,
                    p_vertex_attribute_descriptions: vertex_attributes.as_ptr(),
                    ..Default::default()
                };

                let input_assembly = vk::PipelineInputAssemblyStateCreateInfo {
                    s_type: vk::StructureType::PIPELINE_INPUT_ASSEMBLY_STATE_CREATE_INFO,
                    topology: if vertices.len() == 3 { vk::PrimitiveTopology::TRIANGLE_LIST } else { vk::PrimitiveTopology::TRIANGLE_FAN },
                    primitive_restart_enable: vk::FALSE,
                    ..Default::default()
                };

                let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
                let dynamic_state = vk::PipelineDynamicStateCreateInfo {
                    s_type: vk::StructureType::PIPELINE_DYNAMIC_STATE_CREATE_INFO,
                    dynamic_state_count: dynamic_states.len() as u32,
                    p_dynamic_states: dynamic_states.as_ptr(),
                    ..Default::default()
                };
                let viewport_state = vk::PipelineViewportStateCreateInfo {
                    s_type: vk::StructureType::PIPELINE_VIEWPORT_STATE_CREATE_INFO,
                    viewport_count: 1,
                    scissor_count: 1,
                    ..Default::default()
                };
                let rasterization_state = vk::PipelineRasterizationStateCreateInfo {
                    s_type: vk::StructureType::PIPELINE_RASTERIZATION_STATE_CREATE_INFO,
                    polygon_mode: vk::PolygonMode::FILL,
                    cull_mode: vk::CullModeFlags::NONE,
                    front_face: vk::FrontFace::COUNTER_CLOCKWISE, // Adjust if needed
                    line_width: 1.0,
                    ..Default::default()
                };
                let multisample_state = vk::PipelineMultisampleStateCreateInfo {
                    s_type: vk::StructureType::PIPELINE_MULTISAMPLE_STATE_CREATE_INFO,
                    rasterization_samples: vk::SampleCountFlags::TYPE_1,
                    ..Default::default()
                };
                let color_blend_attachment = vk::PipelineColorBlendAttachmentState {
                    blend_enable: vk::FALSE, // Basic: no blending
                    color_write_mask: vk::ColorComponentFlags::RGBA,
                    ..Default::default()
                };
                let color_blend_state = vk::PipelineColorBlendStateCreateInfo {
                    s_type: vk::StructureType::PIPELINE_COLOR_BLEND_STATE_CREATE_INFO,
                    attachment_count: 1,
                    p_attachments: &color_blend_attachment,
                    ..Default::default()
                };

                let pipeline_info = vk::GraphicsPipelineCreateInfo {
                    s_type: vk::StructureType::GRAPHICS_PIPELINE_CREATE_INFO,
                    stage_count: stages.len() as u32,
                    p_stages: stages.as_ptr(),
                    p_vertex_input_state: &vertex_input,
                    p_input_assembly_state: &input_assembly,
                    p_viewport_state: &viewport_state,
                    p_rasterization_state: &rasterization_state,
                    p_multisample_state: &multisample_state,
                    p_color_blend_state: &color_blend_state,
                    p_dynamic_state: &dynamic_state,
                    layout: pipeline_layout, // Use the layout passed in
                    render_pass: platform.render_pass.unwrap(), // Assumes render pass exists
                    subpass: 0,
                    ..Default::default()
                };

                unsafe {
                    match device.create_graphics_pipelines(vk::PipelineCache::null(), &[pipeline_info], None) {
                        Ok(pipelines) => pipelines[0],
                        Err((_, e)) => panic!("Graphics pipeline creation failed: {:?}", e), // Use Err((_, e)) for tuple result
                    }
                }
            };

            // --- Calculate Bounding Box Info ---
            let (min_x, max_x, min_y, max_y) = vertices.iter().fold(
                (f32::INFINITY, f32::NEG_INFINITY, f32::INFINITY, f32::NEG_INFINITY),
                |acc, v| (acc.0.min(v.position[0]), acc.1.max(v.position[0]), acc.2.min(v.position[1]), acc.3.max(v.position[1]))
            );
            let center_x = (min_x + max_x) / 2.0;
            let center_y = (min_y + max_y) / 2.0;

            // --- Create Renderable ---
            renderables.push(Renderable {
                vertex_buffer,
                vertex_allocation,
                vertex_shader,
                fragment_shader,
                pipeline,
                offset_uniform,
                offset_allocation,
                descriptor_set,
                vertex_count: vertices.len() as u32,
                depth: obj.depth,
                on_window_resize_scale: obj.on_window_resize_scale,
                on_window_resize_move: obj.on_window_resize_move,
                original_positions: vertices.iter().map(|v| v.position).collect(),
                fixed_size: [max_x - min_x, max_y - min_y],
                center_ratio: [center_x / 600.0, center_y / 300.0], // Assuming initial 600x300 window
                instance_buffer,
                instance_allocation,
                instance_count: initial_instance_count,
                instance_buffer_capacity,
            });
        }

        // Sort by depth after processing all objects
        renderables.sort_by(|a, b| a.depth.partial_cmp(&b.depth).unwrap_or(std::cmp::Ordering::Equal));

        Self {
            uniform_buffer,
            uniform_allocation,
            renderables,
            descriptor_set_layout,
            descriptor_pool,
        }
    }

    // Update main object offset UBO
    pub fn update_offset(renderables: &mut Vec<Renderable>, _device: &ash::Device, allocator: &vk_mem::Allocator, index: usize, offset: [f32; 2]) {
         if index >= renderables.len() { return; } // Bounds check
        let renderable = &mut renderables[index];
        unsafe {
            let data_ptr = allocator.map_memory(&mut renderable.offset_allocation).unwrap().cast::<f32>();
            data_ptr.copy_from_nonoverlapping(offset.as_ptr(), 2);
            allocator.unmap_memory(&mut renderable.offset_allocation);
            // No need to update descriptor set here, the buffer content changed but the binding is the same.
        }
    }

    // Update an existing instance's offset in the instance buffer (for dragging)
    pub fn update_instance_offset(renderables: &mut Vec<Renderable>, _device: &ash::Device, allocator: &vk_mem::Allocator, object_index: usize, instance_id: usize, offset: [f32; 2]) {
        if object_index >= renderables.len() { return; } // Bounds check
        let renderable = &mut renderables[object_index];
        if let Some(ref mut instance_allocation) = renderable.instance_allocation {
            // Check if instance_id is within the *currently active* instances
            if instance_id < renderable.instance_count as usize {
                unsafe {
                    let data_ptr = allocator.map_memory(instance_allocation).unwrap().cast::<f32>();
                    let buffer_offset_bytes = instance_id * std::mem::size_of::<[f32; 2]>();
                    let offset_ptr = (data_ptr as *mut u8).add(buffer_offset_bytes).cast::<f32>();
                    offset_ptr.copy_from_nonoverlapping(offset.as_ptr(), 2);
                    allocator.unmap_memory(instance_allocation);
                }
            } else {
                 eprintln!("[BufferManager] Warning: update_instance_offset called for instance_id {} >= current count {} on object {}", instance_id, renderable.instance_count, object_index);
            }
        }
    }

    // Update instance buffer for a NEW instance (called via event)
    pub fn update_instance_buffer(
        renderables: &mut Vec<Renderable>,
        _device: &ash::Device, // Keep device if needed later
        allocator: &vk_mem::Allocator,
        object_id: usize,
        instance_id: usize, // ID from the event
        offset: [f32; 2],
    ) {
        if object_id >= renderables.len() {
            eprintln!("[BufferManager] Error: update_instance_buffer called with invalid object_id {}", object_id);
            return;
        }

        let renderable = &mut renderables[object_id];

        if renderable.instance_buffer.is_none() || renderable.instance_allocation.is_none() {
             eprintln!("[BufferManager] Error: update_instance_buffer called for object {} which has no instance buffer.", object_id);
             return;
        }

        // Check if the instance_id provided matches the next available slot
        if instance_id != renderable.instance_count as usize {
            eprintln!("[BufferManager] Warning: Instance event ID {} does not match expected next slot {} for object {}. Sync issue?", instance_id, renderable.instance_count, object_id);
            // Proceeding anyway for now
        }

        // Check capacity
        if renderable.instance_count >= renderable.instance_buffer_capacity {
            eprintln!("[BufferManager] Warning: Instance buffer full for object {} (capacity {}). Cannot add new instance.", object_id, renderable.instance_buffer_capacity);
            // TODO: Implement buffer resizing logic here
            return;
        }

        // Write to the next available slot
        if let Some(ref mut instance_allocation) = renderable.instance_allocation {
            unsafe {
                let data_ptr = allocator.map_memory(instance_allocation).unwrap().cast::<f32>();
                let buffer_offset_bytes = renderable.instance_count as usize * std::mem::size_of::<[f32; 2]>();
                let offset_ptr = (data_ptr as *mut u8).add(buffer_offset_bytes).cast::<f32>();
                offset_ptr.copy_from_nonoverlapping(offset.as_ptr(), 2);
                allocator.unmap_memory(instance_allocation);
            }
            // Increment active instance count for this renderable
            renderable.instance_count += 1;
            // println!("[BufferManager] Added instance {} to object {}. New count: {}", instance_id, object_id, renderable.instance_count);
        }
    }

    // Cleanup resources
    pub fn cleanup(mut self, platform: &mut VulkanContext) {
        let device = platform.device.as_ref().unwrap();
        let allocator = platform.allocator.as_ref().unwrap();
        unsafe {
            // Free descriptor sets before destroying pool
            let sets_to_free: Vec<vk::DescriptorSet> = self.renderables.iter().map(|r| r.descriptor_set).collect();
            if !sets_to_free.is_empty() {
                 // Use try_catch if FREE_DESCRIPTOR_SET flag wasn't set on pool, otherwise unwrap is fine
                 device.free_descriptor_sets(self.descriptor_pool, &sets_to_free).unwrap();
            }

            for mut renderable in self.renderables { // Takes ownership
                device.destroy_pipeline(renderable.pipeline, None);
                device.destroy_shader_module(renderable.vertex_shader, None);
                device.destroy_shader_module(renderable.fragment_shader, None);
                allocator.destroy_buffer(renderable.vertex_buffer, &mut renderable.vertex_allocation);
                allocator.destroy_buffer(renderable.offset_uniform, &mut renderable.offset_allocation);
                if let (Some(instance_buffer), Some(mut instance_allocation)) = (renderable.instance_buffer.take(), renderable.instance_allocation.take()) {
                    allocator.destroy_buffer(instance_buffer, &mut instance_allocation);
                }
            }
            allocator.destroy_buffer(self.uniform_buffer, &mut self.uniform_allocation);
            device.destroy_descriptor_pool(self.descriptor_pool, None);
            device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);
        }
    }
}