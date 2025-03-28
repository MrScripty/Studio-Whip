use ash::vk;
use std::marker::PhantomData;
use vk_mem::Alloc;
use crate::Vertex;
use crate::gui_framework::context::vulkan_context::VulkanContext;
use crate::gui_framework::scene::scene::Scene;
use glam::Mat4;
use crate::gui_framework::rendering::shader_utils::load_shader;
use crate::gui_framework::rendering::renderable::Renderable;
use crate::gui_framework::rendering::swapchain::{create_swapchain, create_framebuffers};
use crate::gui_framework::rendering::command_buffers::record_command_buffers;

pub struct Renderer {
    vulkan_renderables: Vec<Renderable>,
    pipeline_layout: vk::PipelineLayout,
    uniform_buffer: vk::Buffer,
    uniform_allocation: vk_mem::Allocation,
    descriptor_set_layout: vk::DescriptorSetLayout,
    descriptor_pool: vk::DescriptorPool,
    descriptor_set: vk::DescriptorSet,
}

impl Renderer {
    pub fn new(platform: &mut VulkanContext, extent: vk::Extent2D, scene: &Scene) -> Self {
        let surface_format = create_swapchain(platform, extent);
        create_framebuffers(platform, extent, surface_format);

        let ortho = Mat4::orthographic_rh(0.0, 600.0, 300.0, 0.0, -1.0, 1.0).to_cols_array();
        let (uniform_buffer, uniform_allocation) = {
            let buffer_info = vk::BufferCreateInfo {
                s_type: vk::StructureType::BUFFER_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: vk::BufferCreateFlags::empty(),
                size: std::mem::size_of_val(&ortho) as u64,
                usage: vk::BufferUsageFlags::UNIFORM_BUFFER,
                sharing_mode: vk::SharingMode::EXCLUSIVE,
                queue_family_index_count: 0,
                p_queue_family_indices: std::ptr::null(),
                _marker: PhantomData,
            };
            let allocation_info = vk_mem::AllocationCreateInfo {
                usage: vk_mem::MemoryUsage::AutoPreferDevice,
                flags: vk_mem::AllocationCreateFlags::MAPPED | vk_mem::AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE,
                ..Default::default()
            };
            unsafe {
                match platform.allocator.as_ref().unwrap().create_buffer(&buffer_info, &allocation_info) {
                    Ok((buffer, mut allocation)) => {
                        let data_ptr = platform.allocator.as_ref().unwrap().map_memory(&mut allocation)
                            .unwrap()
                            .cast::<f32>();
                        data_ptr.copy_from_nonoverlapping(ortho.as_ptr(), ortho.len());
                        platform.allocator.as_ref().unwrap().unmap_memory(&mut allocation);
                        (buffer, allocation)
                    }
                    Err(e) => {
                        println!("Failed to create uniform buffer: {:?}", e);
                        panic!("Uniform buffer creation failed");
                    }
                }
            }
        };

        let descriptor_set_layout = unsafe {
            let bindings = [
                vk::DescriptorSetLayoutBinding {
                    binding: 0,
                    descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                    descriptor_count: 1,
                    stage_flags: vk::ShaderStageFlags::VERTEX,
                    p_immutable_samplers: std::ptr::null(),
                    _marker: PhantomData,
                },
                vk::DescriptorSetLayoutBinding {
                    binding: 1,
                    descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                    descriptor_count: 1,
                    stage_flags: vk::ShaderStageFlags::VERTEX,
                    p_immutable_samplers: std::ptr::null(),
                    _marker: PhantomData,
                },
            ];
            match platform.device.as_ref().unwrap().create_descriptor_set_layout(&vk::DescriptorSetLayoutCreateInfo {
                s_type: vk::StructureType::DESCRIPTOR_SET_LAYOUT_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: vk::DescriptorSetLayoutCreateFlags::empty(),
                binding_count: bindings.len() as u32,
                p_bindings: bindings.as_ptr(),
                _marker: PhantomData,
            }, None) {
                Ok(layout) => layout,
                Err(e) => {
                    println!("Failed to create descriptor set layout: {:?}", e);
                    panic!("Descriptor set layout creation failed");
                }
            }
        };

        let descriptor_pool = unsafe {
            let pool_sizes = [
                vk::DescriptorPoolSize {
                    ty: vk::DescriptorType::UNIFORM_BUFFER,
                    descriptor_count: 2 * (1 + scene.pool.len() as u32),
                },
            ];
            match platform.device.as_ref().unwrap().create_descriptor_pool(&vk::DescriptorPoolCreateInfo {
                s_type: vk::StructureType::DESCRIPTOR_POOL_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: vk::DescriptorPoolCreateFlags::empty(),
                max_sets: 1 + scene.pool.len() as u32,
                pool_size_count: pool_sizes.len() as u32,
                p_pool_sizes: pool_sizes.as_ptr(),
                _marker: PhantomData,
            }, None) {
                Ok(pool) => pool,
                Err(e) => {
                    println!("Failed to create descriptor pool: {:?}", e);
                    panic!("Descriptor pool creation failed");
                }
            }
        };

        let descriptor_sets = unsafe {
            let layouts = vec![descriptor_set_layout; 1 + scene.pool.len()];
            match platform.device.as_ref().unwrap().allocate_descriptor_sets(&vk::DescriptorSetAllocateInfo {
                s_type: vk::StructureType::DESCRIPTOR_SET_ALLOCATE_INFO,
                p_next: std::ptr::null(),
                descriptor_pool,
                descriptor_set_count: layouts.len() as u32,
                p_set_layouts: layouts.as_ptr(),
                _marker: PhantomData,
            }) {
                Ok(sets) => sets,
                Err(e) => {
                    println!("Failed to allocate descriptor sets: {:?}", e);
                    panic!("Descriptor set allocation failed");
                }
            }
        };

        let descriptor_set = descriptor_sets[0];
        unsafe {
            let buffer_info = vk::DescriptorBufferInfo {
                buffer: uniform_buffer,
                offset: 0,
                range: std::mem::size_of_val(&ortho) as u64,
            };
            platform.device.as_ref().unwrap().update_descriptor_sets(&[vk::WriteDescriptorSet {
                s_type: vk::StructureType::WRITE_DESCRIPTOR_SET,
                p_next: std::ptr::null(),
                dst_set: descriptor_set,
                dst_binding: 0,
                dst_array_element: 0,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                p_image_info: std::ptr::null(),
                p_buffer_info: &buffer_info,
                p_texel_buffer_view: std::ptr::null(),
                _marker: PhantomData,
            }], &[]);
        }

        let pipeline_layout = unsafe {
            match platform.device.as_ref().unwrap().create_pipeline_layout(&vk::PipelineLayoutCreateInfo {
                s_type: vk::StructureType::PIPELINE_LAYOUT_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: vk::PipelineLayoutCreateFlags::empty(),
                set_layout_count: 1,
                p_set_layouts: &descriptor_set_layout,
                push_constant_range_count: 0,
                p_push_constant_ranges: std::ptr::null(),
                _marker: PhantomData,
            }, None) {
                Ok(layout) => layout,
                Err(e) => {
                    println!("Failed to create pipeline layout: {:?}", e);
                    panic!("Pipeline layout creation failed");
                }
            }
        };

        let mut renderables = Vec::new();
        for (i, obj) in scene.pool.iter().enumerate() {
            let vertices = &obj.vertices;
            let (vertex_buffer, vertex_allocation) = {
                let buffer_info = vk::BufferCreateInfo {
                    s_type: vk::StructureType::BUFFER_CREATE_INFO,
                    p_next: std::ptr::null(),
                    flags: vk::BufferCreateFlags::empty(),
                    size: (vertices.len() * std::mem::size_of::<Vertex>()) as u64,
                    usage: vk::BufferUsageFlags::VERTEX_BUFFER,
                    sharing_mode: vk::SharingMode::EXCLUSIVE,
                    queue_family_index_count: 0,
                    p_queue_family_indices: std::ptr::null(),
                    _marker: PhantomData,
                };
                let allocation_info = vk_mem::AllocationCreateInfo {
                    usage: vk_mem::MemoryUsage::AutoPreferDevice,
                    flags: vk_mem::AllocationCreateFlags::MAPPED | vk_mem::AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE,
                    ..Default::default()
                };
                unsafe {
                    match platform.allocator.as_ref().unwrap().create_buffer(&buffer_info, &allocation_info) {
                        Ok((buffer, mut allocation)) => {
                            let data_ptr = platform.allocator.as_ref().unwrap().map_memory(&mut allocation)
                                .unwrap()
                                .cast::<Vertex>();
                            data_ptr.copy_from_nonoverlapping(vertices.as_ptr(), vertices.len());
                            platform.allocator.as_ref().unwrap().unmap_memory(&mut allocation);
                            (buffer, allocation)
                        }
                        Err(e) => {
                            println!("Failed to create vertex buffer: {:?}", e);
                            panic!("Vertex buffer creation failed");
                        }
                    }
                }
            };

            let (offset_uniform, offset_allocation) = {
                let buffer_info = vk::BufferCreateInfo {
                    s_type: vk::StructureType::BUFFER_CREATE_INFO,
                    p_next: std::ptr::null(),
                    flags: vk::BufferCreateFlags::empty(),
                    size: std::mem::size_of::<[f32; 2]>() as u64,
                    usage: vk::BufferUsageFlags::UNIFORM_BUFFER,
                    sharing_mode: vk::SharingMode::EXCLUSIVE,
                    queue_family_index_count: 0,
                    p_queue_family_indices: std::ptr::null(),
                    _marker: PhantomData,
                };
                let allocation_info = vk_mem::AllocationCreateInfo {
                    usage: vk_mem::MemoryUsage::AutoPreferDevice,
                    flags: vk_mem::AllocationCreateFlags::MAPPED | vk_mem::AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE,
                    ..Default::default()
                };
                unsafe {
                    match platform.allocator.as_ref().unwrap().create_buffer(&buffer_info, &allocation_info) {
                        Ok((buffer, mut allocation)) => {
                            let data_ptr = platform.allocator.as_ref().unwrap().map_memory(&mut allocation)
                                .unwrap()
                                .cast::<f32>();
                            data_ptr.copy_from_nonoverlapping(obj.offset.as_ptr(), 2);
                            platform.allocator.as_ref().unwrap().unmap_memory(&mut allocation);
                            (buffer, allocation)
                        }
                        Err(e) => {
                            println!("Failed to create offset uniform buffer: {:?}", e);
                            panic!("Offset uniform buffer creation failed");
                        }
                    }
                }
            };

            // New: Create instance buffer if there are instances
            let (instance_buffer, instance_allocation, instance_count) = if !obj.instances.is_empty() {
                let instance_data: Vec<[f32; 2]> = obj.instances.iter().map(|i| i.offset).collect();
                let buffer_info = vk::BufferCreateInfo {
                    s_type: vk::StructureType::BUFFER_CREATE_INFO,
                    p_next: std::ptr::null(),
                    flags: vk::BufferCreateFlags::empty(),
                    size: (instance_data.len() * std::mem::size_of::<[f32; 2]>()) as u64,
                    usage: vk::BufferUsageFlags::VERTEX_BUFFER,
                    sharing_mode: vk::SharingMode::EXCLUSIVE,
                    queue_family_index_count: 0,
                    p_queue_family_indices: std::ptr::null(),
                    _marker: PhantomData,
                };
                let allocation_info = vk_mem::AllocationCreateInfo {
                    usage: vk_mem::MemoryUsage::AutoPreferDevice,
                    flags: vk_mem::AllocationCreateFlags::MAPPED | vk_mem::AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE,
                    ..Default::default()
                };
                unsafe {
                    match platform.allocator.as_ref().unwrap().create_buffer(&buffer_info, &allocation_info) {
                        Ok((buffer, mut allocation)) => {
                            let data_ptr = platform.allocator.as_ref().unwrap().map_memory(&mut allocation)
                                .unwrap()
                                .cast::<f32>();
                            data_ptr.copy_from_nonoverlapping(instance_data.as_ptr() as *const f32, instance_data.len() * 2);
                            platform.allocator.as_ref().unwrap().unmap_memory(&mut allocation);
                            (Some(buffer), Some(allocation), instance_data.len() as u32)
                        }
                        Err(e) => {
                            println!("Failed to create instance buffer: {:?}", e);
                            panic!("Instance buffer creation failed");
                        }
                    }
                }
            } else {
                (None, None, 0)
            };

            let descriptor_set = descriptor_sets[i + 1];
            unsafe {
                let buffer_infos = [
                    vk::DescriptorBufferInfo {
                        buffer: uniform_buffer,
                        offset: 0,
                        range: std::mem::size_of_val(&ortho) as u64,
                    },
                    vk::DescriptorBufferInfo {
                        buffer: offset_uniform,
                        offset: 0,
                        range: std::mem::size_of::<[f32; 2]>() as u64,
                    },
                ];
                let write_sets = [
                    vk::WriteDescriptorSet {
                        s_type: vk::StructureType::WRITE_DESCRIPTOR_SET,
                        p_next: std::ptr::null(),
                        dst_set: descriptor_set,
                        dst_binding: 0,
                        dst_array_element: 0,
                        descriptor_count: 1,
                        descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                        p_image_info: std::ptr::null(),
                        p_buffer_info: &buffer_infos[0],
                        p_texel_buffer_view: std::ptr::null(),
                        _marker: PhantomData,
                    },
                    vk::WriteDescriptorSet {
                        s_type: vk::StructureType::WRITE_DESCRIPTOR_SET,
                        p_next: std::ptr::null(),
                        dst_set: descriptor_set,
                        dst_binding: 1,
                        dst_array_element: 0,
                        descriptor_count: 1,
                        descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                        p_image_info: std::ptr::null(),
                        p_buffer_info: &buffer_infos[1],
                        p_texel_buffer_view: std::ptr::null(),
                        _marker: PhantomData,
                    },
                ];
                platform.device.as_ref().unwrap().update_descriptor_sets(&write_sets, &[]);
            }

            let vertex_shader = load_shader(platform.device.as_ref().unwrap(), &obj.vertex_shader_filename);
            let fragment_shader = load_shader(platform.device.as_ref().unwrap(), &obj.fragment_shader_filename);

            let pipeline = {
                let vertex_stage = vk::PipelineShaderStageCreateInfo {
                    s_type: vk::StructureType::PIPELINE_SHADER_STAGE_CREATE_INFO,
                    p_next: std::ptr::null(),
                    flags: vk::PipelineShaderStageCreateFlags::empty(),
                    stage: vk::ShaderStageFlags::VERTEX,
                    module: vertex_shader,
                    p_name: c"main".as_ptr(),
                    p_specialization_info: std::ptr::null(),
                    _marker: PhantomData,
                };
                let fragment_stage = vk::PipelineShaderStageCreateInfo {
                    s_type: vk::StructureType::PIPELINE_SHADER_STAGE_CREATE_INFO,
                    p_next: std::ptr::null(),
                    flags: vk::PipelineShaderStageCreateFlags::empty(),
                    stage: vk::ShaderStageFlags::FRAGMENT,
                    module: fragment_shader,
                    p_name: c"main".as_ptr(),
                    p_specialization_info: std::ptr::null(),
                    _marker: PhantomData,
                };
                let stages = [vertex_stage, fragment_stage];

                let (vertex_attributes, vertex_bindings) = if obj.instances.is_empty() {
                    (
                        vec![vk::VertexInputAttributeDescription {
                            location: 0,
                            binding: 0,
                            format: vk::Format::R32G32_SFLOAT,
                            offset: 0,
                        }],
                        vec![vk::VertexInputBindingDescription {
                            binding: 0,
                            stride: std::mem::size_of::<Vertex>() as u32,
                            input_rate: vk::VertexInputRate::VERTEX,
                        }],
                    )
                } else {
                    (
                        vec![
                            vk::VertexInputAttributeDescription {
                                location: 0,
                                binding: 0,
                                format: vk::Format::R32G32_SFLOAT,
                                offset: 0,
                            },
                            vk::VertexInputAttributeDescription {
                                location: 1,
                                binding: 1,
                                format: vk::Format::R32G32_SFLOAT,
                                offset: 0,
                            },
                        ],
                        vec![
                            vk::VertexInputBindingDescription {
                                binding: 0,
                                stride: std::mem::size_of::<Vertex>() as u32,
                                input_rate: vk::VertexInputRate::VERTEX,
                            },
                            vk::VertexInputBindingDescription {
                                binding: 1,
                                stride: std::mem::size_of::<[f32; 2]>() as u32,
                                input_rate: vk::VertexInputRate::INSTANCE,
                            },
                        ],
                    )
                };
                let vertex_input = vk::PipelineVertexInputStateCreateInfo {
                    s_type: vk::StructureType::PIPELINE_VERTEX_INPUT_STATE_CREATE_INFO,
                    p_next: std::ptr::null(),
                    flags: vk::PipelineVertexInputStateCreateFlags::empty(),
                    vertex_binding_description_count: vertex_bindings.len() as u32,
                    p_vertex_binding_descriptions: vertex_bindings.as_ptr(),
                    vertex_attribute_description_count: vertex_attributes.len() as u32,
                    p_vertex_attribute_descriptions: vertex_attributes.as_ptr(),
                    _marker: PhantomData,
                };

                let input_assembly = vk::PipelineInputAssemblyStateCreateInfo {
                    s_type: vk::StructureType::PIPELINE_INPUT_ASSEMBLY_STATE_CREATE_INFO,
                    p_next: std::ptr::null(),
                    flags: vk::PipelineInputAssemblyStateCreateFlags::empty(),
                    topology: if vertices.len() == 3 { vk::PrimitiveTopology::TRIANGLE_LIST } else { vk::PrimitiveTopology::TRIANGLE_FAN },
                    primitive_restart_enable: vk::FALSE,
                    _marker: PhantomData,
                };

                let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
                let dynamic_state = vk::PipelineDynamicStateCreateInfo {
                    s_type: vk::StructureType::PIPELINE_DYNAMIC_STATE_CREATE_INFO,
                    p_next: std::ptr::null(),
                    flags: vk::PipelineDynamicStateCreateFlags::empty(),
                    dynamic_state_count: dynamic_states.len() as u32,
                    p_dynamic_states: dynamic_states.as_ptr(),
                    _marker: PhantomData,
                };
                let viewport_state = vk::PipelineViewportStateCreateInfo {
                    s_type: vk::StructureType::PIPELINE_VIEWPORT_STATE_CREATE_INFO,
                    p_next: std::ptr::null(),
                    flags: vk::PipelineViewportStateCreateFlags::empty(),
                    viewport_count: 1,
                    p_viewports: std::ptr::null(),
                    scissor_count: 1,
                    p_scissors: std::ptr::null(),
                    _marker: PhantomData,
                };

                let rasterization_state = vk::PipelineRasterizationStateCreateInfo {
                    s_type: vk::StructureType::PIPELINE_RASTERIZATION_STATE_CREATE_INFO,
                    p_next: std::ptr::null(),
                    flags: vk::PipelineRasterizationStateCreateFlags::empty(),
                    depth_clamp_enable: vk::FALSE,
                    rasterizer_discard_enable: vk::FALSE,
                    polygon_mode: vk::PolygonMode::FILL,
                    cull_mode: vk::CullModeFlags::NONE,
                    front_face: vk::FrontFace::COUNTER_CLOCKWISE,
                    depth_bias_enable: vk::FALSE,
                    depth_bias_constant_factor: 0.0,
                    depth_bias_clamp: 0.0,
                    depth_bias_slope_factor: 0.0,
                    line_width: 1.0,
                    _marker: PhantomData,
                };

                let multisample_state = vk::PipelineMultisampleStateCreateInfo {
                    s_type: vk::StructureType::PIPELINE_MULTISAMPLE_STATE_CREATE_INFO,
                    p_next: std::ptr::null(),
                    flags: vk::PipelineMultisampleStateCreateFlags::empty(),
                    rasterization_samples: vk::SampleCountFlags::TYPE_1,
                    sample_shading_enable: vk::FALSE,
                    min_sample_shading: 0.0,
                    p_sample_mask: std::ptr::null(),
                    alpha_to_coverage_enable: vk::FALSE,
                    alpha_to_one_enable: vk::FALSE,
                    _marker: PhantomData,
                };

                let color_blend_attachment = vk::PipelineColorBlendAttachmentState {
                    blend_enable: vk::FALSE,
                    src_color_blend_factor: vk::BlendFactor::ONE,
                    dst_color_blend_factor: vk::BlendFactor::ZERO,
                    color_blend_op: vk::BlendOp::ADD,
                    src_alpha_blend_factor: vk::BlendFactor::ONE,
                    dst_alpha_blend_factor: vk::BlendFactor::ZERO,
                    alpha_blend_op: vk::BlendOp::ADD,
                    color_write_mask: vk::ColorComponentFlags::RGBA,
                };
                let color_blend_state = vk::PipelineColorBlendStateCreateInfo {
                    s_type: vk::StructureType::PIPELINE_COLOR_BLEND_STATE_CREATE_INFO,
                    p_next: std::ptr::null(),
                    flags: vk::PipelineColorBlendStateCreateFlags::empty(),
                    logic_op_enable: vk::FALSE,
                    logic_op: vk::LogicOp::COPY,
                    attachment_count: 1,
                    p_attachments: &color_blend_attachment,
                    blend_constants: [0.0; 4],
                    _marker: PhantomData,
                };

                let pipeline_info = vk::GraphicsPipelineCreateInfo {
                    s_type: vk::StructureType::GRAPHICS_PIPELINE_CREATE_INFO,
                    p_next: std::ptr::null(),
                    flags: vk::PipelineCreateFlags::empty(),
                    stage_count: 2,
                    p_stages: stages.as_ptr(),
                    p_vertex_input_state: &vertex_input,
                    p_input_assembly_state: &input_assembly,
                    p_tessellation_state: std::ptr::null(),
                    p_viewport_state: &viewport_state,
                    p_rasterization_state: &rasterization_state,
                    p_multisample_state: &multisample_state,
                    p_depth_stencil_state: std::ptr::null(),
                    p_color_blend_state: &color_blend_state,
                    p_dynamic_state: &dynamic_state,
                    layout: pipeline_layout,
                    render_pass: platform.render_pass.unwrap(),
                    subpass: 0,
                    base_pipeline_handle: vk::Pipeline::null(),
                    base_pipeline_index: -1,
                    _marker: PhantomData,
                };

                unsafe {
                    match platform.device.as_ref().unwrap().create_graphics_pipelines(vk::PipelineCache::null(), &[pipeline_info], None) {
                        Ok(pipelines) => pipelines[0],
                        Err(e) => {
                            println!("Failed to create graphics pipeline: {:?}", e);
                            panic!("Graphics pipeline creation failed");
                        }
                    }
                }
            };

            let min_x = vertices.iter().map(|v| v.position[0]).fold(f32::INFINITY, f32::min);
            let max_x = vertices.iter().map(|v| v.position[0]).fold(f32::NEG_INFINITY, f32::max);
            let min_y = vertices.iter().map(|v| v.position[1]).fold(f32::INFINITY, f32::min);
            let max_y = vertices.iter().map(|v| v.position[1]).fold(f32::NEG_INFINITY, f32::max);
            let center_x = (min_x + max_x) / 2.0;
            let center_y = (min_y + max_y) / 2.0;

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
                center_ratio: [center_x / 600.0, center_y / 300.0],
                instance_buffer,        // New
                instance_allocation,    // New
                instance_count,         // New
            });
        }

        renderables.sort_by(|a, b| a.depth.partial_cmp(&b.depth).unwrap());

        record_command_buffers(platform, &renderables, pipeline_layout, descriptor_set, extent);

        platform.image_available_semaphore = Some(unsafe {
            match platform.device.as_ref().unwrap().create_semaphore(&vk::SemaphoreCreateInfo::default(), None) {
                Ok(semaphore) => semaphore,
                Err(e) => {
                    println!("Failed to create image available semaphore: {:?}", e);
                    panic!("Semaphore creation failed");
                }
            }
        });
        platform.render_finished_semaphore = Some(unsafe {
            match platform.device.as_ref().unwrap().create_semaphore(&vk::SemaphoreCreateInfo::default(), None) {
                Ok(semaphore) => semaphore,
                Err(e) => {
                    println!("Failed to create render finished semaphore: {:?}", e);
                    panic!("Semaphore creation failed");
                }
            }
        });
        platform.fence = Some(unsafe {
            match platform.device.as_ref().unwrap().create_fence(
                &vk::FenceCreateInfo {
                    s_type: vk::StructureType::FENCE_CREATE_INFO,
                    p_next: std::ptr::null(),
                    flags: vk::FenceCreateFlags::SIGNALED,
                    _marker: PhantomData,
                },
                None,
            ) {
                Ok(fence) => fence,
                Err(e) => {
                    println!("Failed to create fence: {:?}", e);
                    panic!("Fence creation failed");
                }
            }
        });

        Self {
            vulkan_renderables: renderables,
            pipeline_layout,
            uniform_buffer,
            uniform_allocation,
            descriptor_set_layout,
            descriptor_pool,
            descriptor_set,
        }
    }

    pub fn update_offset(&mut self, device: &ash::Device, allocator: &vk_mem::Allocator, index: usize, offset: [f32; 2]) {
        let renderable = &mut self.vulkan_renderables[index];
        unsafe {
            let data_ptr = allocator
                .map_memory(&mut renderable.offset_allocation)
                .unwrap()
                .cast::<f32>();
            data_ptr.copy_from_nonoverlapping(offset.as_ptr(), 2);
            allocator.unmap_memory(&mut renderable.offset_allocation);

            let buffer_info = vk::DescriptorBufferInfo {
                buffer: renderable.offset_uniform,
                offset: 0,
                range: std::mem::size_of::<[f32; 2]>() as u64,
            };
            device.update_descriptor_sets(&[vk::WriteDescriptorSet {
                s_type: vk::StructureType::WRITE_DESCRIPTOR_SET,
                p_next: std::ptr::null(),
                dst_set: renderable.descriptor_set,
                dst_binding: 1,
                dst_array_element: 0,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                p_image_info: std::ptr::null(),
                p_buffer_info: &buffer_info,
                p_texel_buffer_view: std::ptr::null(),
                _marker: PhantomData,
            }], &[]);
        }
    }

    pub fn resize_renderer(&mut self, vulkan_context: &mut VulkanContext, scene: &mut Scene, width: u32, height: u32) {
        let device = vulkan_context.device.as_ref().unwrap();
        unsafe { device.device_wait_idle().unwrap() };

        for &framebuffer in &vulkan_context.framebuffers {
            unsafe { device.destroy_framebuffer(framebuffer, None) };
        }
        unsafe { device.destroy_render_pass(vulkan_context.render_pass.take().unwrap(), None) };
        for &view in &vulkan_context.image_views {
            unsafe { device.destroy_image_view(view, None) };
        }
        if let Some(swapchain) = vulkan_context.swapchain.take() {
            unsafe { vulkan_context.swapchain_loader.as_ref().unwrap().destroy_swapchain(swapchain, None) };
        }

        let extent = vk::Extent2D { width, height };
        let surface_format = create_swapchain(vulkan_context, extent);
        create_framebuffers(vulkan_context, extent, surface_format);
        println!("New extent: {:?}", extent);

        let ortho = Mat4::orthographic_rh(0.0, width as f32, height as f32, 0.0, -1.0, 1.0).to_cols_array();
        let data_ptr = unsafe { vulkan_context.allocator.as_ref().unwrap().map_memory(&mut self.uniform_allocation) }
            .unwrap()
            .cast::<f32>();
        unsafe { data_ptr.copy_from_nonoverlapping(ortho.as_ptr(), ortho.len()) };
        unsafe { vulkan_context.allocator.as_ref().unwrap().unmap_memory(&mut self.uniform_allocation) };

        for (renderable, obj) in self.vulkan_renderables.iter_mut().zip(scene.pool.iter_mut()) {
            let mut new_vertices = Vec::new();
            if renderable.on_window_resize_scale {
                new_vertices = vec![
                    Vertex { position: [0.0, 0.0] },
                    Vertex { position: [0.0, height as f32] },
                    Vertex { position: [width as f32, height as f32] },
                    Vertex { position: [width as f32, 0.0] },
                ];
            } else if renderable.on_window_resize_move {
                let new_center_x = renderable.center_ratio[0] * width as f32;
                let new_center_y = renderable.center_ratio[1] * height as f32;
                let half_width = renderable.fixed_size[0] / 2.0;
                let half_height = renderable.fixed_size[1] / 2.0;

                if renderable.vertex_count == 4 {
                    new_vertices = vec![
                        Vertex { position: [new_center_x - half_width, new_center_y - half_height] },
                        Vertex { position: [new_center_x - half_width, new_center_y + half_height] },
                        Vertex { position: [new_center_x + half_width, new_center_y + half_height] },
                        Vertex { position: [new_center_x + half_width, new_center_y - half_height] },
                    ];
                } else if renderable.vertex_count == 3 {
                    new_vertices = vec![
                        Vertex { position: [new_center_x - half_width, new_center_y - half_height] },
                        Vertex { position: [new_center_x, new_center_y + half_height] },
                        Vertex { position: [new_center_x + half_width, new_center_y - half_height] },
                    ];
                }
            }

            let data_ptr = unsafe { vulkan_context.allocator.as_ref().unwrap().map_memory(&mut renderable.vertex_allocation) }
                .unwrap()
                .cast::<Vertex>();
            unsafe { data_ptr.copy_from_nonoverlapping(new_vertices.as_ptr(), new_vertices.len()) };
            unsafe { vulkan_context.allocator.as_ref().unwrap().unmap_memory(&mut renderable.vertex_allocation) };
            obj.vertices = new_vertices; // Sync Scene vertices
        }

        scene.update_dimensions(width, height); // Update Scene dimensions
        record_command_buffers(vulkan_context, &self.vulkan_renderables, self.pipeline_layout, self.descriptor_set, extent);
    }

    pub fn update_instance_offset(&mut self, _device: &ash::Device, allocator: &vk_mem::Allocator, object_index: usize, instance_id: usize, offset: [f32; 2]) {
        let renderable = &mut self.vulkan_renderables[object_index];
        if let Some(ref mut instance_allocation) = renderable.instance_allocation {
            unsafe {
                let data_ptr = allocator.map_memory(instance_allocation).unwrap().cast::<f32>();
                let offset_ptr = data_ptr.add(instance_id * 2); // Each instance offset is 2 f32s
                offset_ptr.copy_from_nonoverlapping(offset.as_ptr(), 2);
                allocator.unmap_memory(instance_allocation);
            }
        }
    }

    pub fn render(&mut self, platform: &mut VulkanContext, scene: &Scene) {
        let device = platform.device.as_ref().unwrap();
        let queue = platform.queue.unwrap();
        let swapchain_loader = platform.swapchain_loader.as_ref().unwrap();
        let swapchain = platform.swapchain.unwrap();
        let image_available_semaphore = platform.image_available_semaphore.unwrap();
        let render_finished_semaphore = platform.render_finished_semaphore.unwrap();
        let fence = platform.fence.unwrap();
        let allocator = platform.allocator.as_ref().unwrap();

        // Sync all offsets before rendering
        for (i, obj) in scene.pool.iter().enumerate() {
            self.update_offset(device, allocator, i, obj.offset);
            for (j, instance) in obj.instances.iter().enumerate() {
                self.update_instance_offset(device, allocator, i, j, instance.offset);
            }
        }

        unsafe { device.wait_for_fences(&[fence], true, u64::MAX) }.unwrap();
        unsafe { device.reset_fences(&[fence]) }.unwrap();

        let (image_index, _) = unsafe {
            swapchain_loader.acquire_next_image(swapchain, u64::MAX, image_available_semaphore, vk::Fence::null())
        }.unwrap();
        platform.current_image = image_index as usize;

        let submit_info = vk::SubmitInfo {
            s_type: vk::StructureType::SUBMIT_INFO,
            p_next: std::ptr::null(),
            wait_semaphore_count: 1,
            p_wait_semaphores: &image_available_semaphore,
            p_wait_dst_stage_mask: &vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT as *const _,
            command_buffer_count: 1,
            p_command_buffers: &platform.command_buffers[platform.current_image],
            signal_semaphore_count: 1,
            p_signal_semaphores: &render_finished_semaphore,
            _marker: PhantomData,
        };
        unsafe { device.queue_submit(queue, &[submit_info], fence) }.unwrap();

        let present_info = vk::PresentInfoKHR {
            s_type: vk::StructureType::PRESENT_INFO_KHR,
            p_next: std::ptr::null(),
            wait_semaphore_count: 1,
            p_wait_semaphores: &render_finished_semaphore,
            swapchain_count: 1,
            p_swapchains: &swapchain,
            p_image_indices: &(platform.current_image as u32),
            p_results: std::ptr::null_mut(),
            _marker: PhantomData,
        };
        unsafe { swapchain_loader.queue_present(queue, &present_info) }.unwrap();
    }

    pub fn cleanup(mut self, platform: &mut VulkanContext) {
        let device = platform.device.as_ref().unwrap();
        let swapchain_loader = platform.swapchain_loader.take().unwrap();
    
        unsafe {
            device.device_wait_idle().unwrap();
    
            device.destroy_semaphore(platform.image_available_semaphore.take().unwrap(), None);
            device.destroy_semaphore(platform.render_finished_semaphore.take().unwrap(), None);
            device.destroy_fence(platform.fence.take().unwrap(), None);
    
            let command_pool = platform.command_pool.take().unwrap();
            device.free_command_buffers(command_pool, &platform.command_buffers);
            device.destroy_command_pool(command_pool, None);
    
            for mut renderable in self.vulkan_renderables {
                device.destroy_pipeline(renderable.pipeline, None);
                device.destroy_shader_module(renderable.vertex_shader, None);
                device.destroy_shader_module(renderable.fragment_shader, None);
                platform.allocator.as_ref().unwrap()
                    .destroy_buffer(renderable.vertex_buffer, &mut renderable.vertex_allocation);
                platform.allocator.as_ref().unwrap()
                    .destroy_buffer(renderable.offset_uniform, &mut renderable.offset_allocation);
                if let (Some(instance_buffer), Some(mut instance_allocation)) = (renderable.instance_buffer, renderable.instance_allocation) {
                    platform.allocator.as_ref().unwrap().destroy_buffer(instance_buffer, &mut instance_allocation);
                }
            }
    
            platform.allocator.as_ref().unwrap()
                .destroy_buffer(self.uniform_buffer, &mut self.uniform_allocation);
            device.destroy_descriptor_pool(self.descriptor_pool, None);
            device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);
    
            for &framebuffer in &platform.framebuffers {
                device.destroy_framebuffer(framebuffer, None);
            }
            device.destroy_render_pass(platform.render_pass.take().unwrap(), None);
    
            swapchain_loader.destroy_swapchain(platform.swapchain.take().unwrap(), None);
            for &view in &platform.image_views {
                device.destroy_image_view(view, None);
            }
    
            device.destroy_pipeline_layout(self.pipeline_layout, None);
        }
    }
}