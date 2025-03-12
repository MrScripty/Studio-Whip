use ash::vk;
use ash::khr::swapchain;
use std::marker::PhantomData;
use vk_mem::Alloc;
use crate::{Vertex, Platform, Scene};
use std::fs;
use glam::Mat4;

fn load_shader(device: &ash::Device, filename: &str) -> vk::ShaderModule {
    let shader_path = format!("./shaders/{}", filename);
    let shader_code = fs::read(&shader_path).expect(&format!("Failed to read shader file: {}", shader_path));
    let shader_module_info = vk::ShaderModuleCreateInfo {
        s_type: vk::StructureType::SHADER_MODULE_CREATE_INFO,
        p_next: std::ptr::null(),
        flags: vk::ShaderModuleCreateFlags::empty(),
        code_size: shader_code.len(),
        p_code: shader_code.as_ptr() as *const u32,
        _marker: PhantomData,
    };
    unsafe { device.create_shader_module(&shader_module_info, None) }
        .expect(&format!("Failed to create shader module from: {}", filename))
}

fn create_swapchain(platform: &mut Platform, extent: vk::Extent2D) -> vk::SurfaceFormatKHR {
    let instance = platform.instance.as_ref().unwrap();
    let device = platform.device.as_ref().unwrap();
    let surface = platform.surface.unwrap();
    let surface_loader = platform.surface_loader.as_ref().unwrap();
    let physical_device = unsafe { instance.enumerate_physical_devices().unwrap() }[0];

    let swapchain_loader = swapchain::Device::new(instance, device);
    platform.swapchain_loader = Some(swapchain_loader.clone());

    let surface_formats = unsafe {
        surface_loader
            .get_physical_device_surface_formats(physical_device, surface)
            .unwrap()
    };
    let surface_format = surface_formats[0];
    let (swapchain, images) = {
        let swapchain_create_info = vk::SwapchainCreateInfoKHR {
            s_type: vk::StructureType::SWAPCHAIN_CREATE_INFO_KHR,
            p_next: std::ptr::null(),
            flags: vk::SwapchainCreateFlagsKHR::empty(),
            surface,
            min_image_count: 2,
            image_format: surface_format.format,
            image_color_space: surface_format.color_space,
            image_extent: extent,
            image_array_layers: 1,
            image_usage: vk::ImageUsageFlags::COLOR_ATTACHMENT,
            image_sharing_mode: vk::SharingMode::EXCLUSIVE,
            queue_family_index_count: 0,
            p_queue_family_indices: std::ptr::null(),
            pre_transform: vk::SurfaceTransformFlagsKHR::IDENTITY,
            composite_alpha: vk::CompositeAlphaFlagsKHR::OPAQUE,
            present_mode: vk::PresentModeKHR::FIFO,
            clipped: vk::TRUE,
            old_swapchain: vk::SwapchainKHR::null(),
            _marker: PhantomData,
        };
        let swapchain = unsafe { swapchain_loader.create_swapchain(&swapchain_create_info, None) }.unwrap();
        let images = unsafe { swapchain_loader.get_swapchain_images(swapchain).unwrap() };
        (swapchain, images)
    };
    platform.swapchain = Some(swapchain);
    platform.images = images;

    platform.image_views = platform
        .images
        .iter()
        .map(|&image| {
            let view_create_info = vk::ImageViewCreateInfo {
                s_type: vk::StructureType::IMAGE_VIEW_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: vk::ImageViewCreateFlags::empty(),
                image,
                view_type: vk::ImageViewType::TYPE_2D,
                format: surface_format.format,
                components: vk::ComponentMapping::default(),
                subresource_range: vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                _marker: PhantomData,
            };
            unsafe { device.create_image_view(&view_create_info, None) }.unwrap()
        })
        .collect();

    surface_format
}

fn create_framebuffers(platform: &mut Platform, extent: vk::Extent2D, surface_format: vk::SurfaceFormatKHR) {
    let device = platform.device.as_ref().unwrap();
    let render_pass = {
        let attachment = vk::AttachmentDescription {
            flags: vk::AttachmentDescriptionFlags::empty(),
            format: surface_format.format,
            samples: vk::SampleCountFlags::TYPE_1,
            load_op: vk::AttachmentLoadOp::CLEAR,
            store_op: vk::AttachmentStoreOp::STORE,
            stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
            stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
            initial_layout: vk::ImageLayout::UNDEFINED,
            final_layout: vk::ImageLayout::PRESENT_SRC_KHR,
        };
        let color_attachment_ref = vk::AttachmentReference {
            attachment: 0,
            layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        };
        let subpass = vk::SubpassDescription {
            flags: vk::SubpassDescriptionFlags::empty(),
            pipeline_bind_point: vk::PipelineBindPoint::GRAPHICS,
            input_attachment_count: 0,
            p_input_attachments: std::ptr::null(),
            color_attachment_count: 1,
            p_color_attachments: &color_attachment_ref,
            p_resolve_attachments: std::ptr::null(),
            p_depth_stencil_attachment: std::ptr::null(),
            preserve_attachment_count: 0,
            p_preserve_attachments: std::ptr::null(),
            _marker: PhantomData,
        };
        let render_pass_info = vk::RenderPassCreateInfo {
            s_type: vk::StructureType::RENDER_PASS_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: vk::RenderPassCreateFlags::empty(),
            attachment_count: 1,
            p_attachments: &attachment,
            subpass_count: 1,
            p_subpasses: &subpass,
            dependency_count: 0,
            p_dependencies: std::ptr::null(),
            _marker: PhantomData,
        };
        unsafe { device.create_render_pass(&render_pass_info, None) }.unwrap()
    };
    platform.render_pass = Some(render_pass);

    platform.framebuffers = platform
        .image_views
        .iter()
        .map(|&view| {
            let framebuffer_info = vk::FramebufferCreateInfo {
                s_type: vk::StructureType::FRAMEBUFFER_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: vk::FramebufferCreateFlags::empty(),
                render_pass,
                attachment_count: 1,
                p_attachments: &view,
                width: extent.width,
                height: extent.height,
                layers: 1,
                _marker: PhantomData,
            };
            unsafe { device.create_framebuffer(&framebuffer_info, None) }.unwrap()
        })
        .collect();
}

fn record_command_buffers(
    platform: &mut Platform,
    renderables: &[Renderable],
    pipeline_layout: vk::PipelineLayout,
    descriptor_set: vk::DescriptorSet,
    extent: vk::Extent2D,
) {
    let device = platform.device.as_ref().unwrap();
    let instance = platform.instance.as_ref().unwrap();
    let queue_family_index = unsafe {
        instance
            .enumerate_physical_devices()
            .unwrap()
            .into_iter()
            .find_map(|pd| {
                instance
                    .get_physical_device_queue_family_properties(pd)
                    .iter()
                    .position(|qf| qf.queue_flags.contains(vk::QueueFlags::GRAPHICS))
                    .map(|index| index as u32)
            })
            .unwrap()
    };
    let command_pool = unsafe {
        device.create_command_pool(
            &vk::CommandPoolCreateInfo {
                s_type: vk::StructureType::COMMAND_POOL_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
                queue_family_index,
                _marker: PhantomData,
            },
            None,
        )
    }
    .unwrap();
    platform.command_pool = Some(command_pool);

    platform.command_buffers = {
        let alloc_info = vk::CommandBufferAllocateInfo {
            s_type: vk::StructureType::COMMAND_BUFFER_ALLOCATE_INFO,
            p_next: std::ptr::null(),
            command_pool,
            level: vk::CommandBufferLevel::PRIMARY,
            command_buffer_count: platform.framebuffers.len() as u32,
            _marker: PhantomData,
        };
        unsafe { device.allocate_command_buffers(&alloc_info) }.unwrap()
    };

    let begin_info = vk::CommandBufferBeginInfo {
        s_type: vk::StructureType::COMMAND_BUFFER_BEGIN_INFO,
        p_next: std::ptr::null(),
        flags: vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
        p_inheritance_info: std::ptr::null(),
        _marker: PhantomData,
    };
    let clear_values = [vk::ClearValue {
        color: vk::ClearColorValue { float32: [0.0, 0.0, 0.0, 0.0] },
    }];
    for (&command_buffer, &framebuffer) in platform.command_buffers.iter().zip(platform.framebuffers.iter()) {
        unsafe {
            device.begin_command_buffer(command_buffer, &begin_info).unwrap();
            device.cmd_begin_render_pass(command_buffer, &vk::RenderPassBeginInfo {
                s_type: vk::StructureType::RENDER_PASS_BEGIN_INFO,
                p_next: std::ptr::null(),
                render_pass: platform.render_pass.unwrap(),
                framebuffer,
                render_area: vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent,
                },
                clear_value_count: clear_values.len() as u32,
                p_clear_values: clear_values.as_ptr(),
                _marker: PhantomData,
            }, vk::SubpassContents::INLINE);

            device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline_layout,
                0,
                &[descriptor_set],
                &[],
            );

            for renderable in renderables {
                device.cmd_bind_pipeline(command_buffer, vk::PipelineBindPoint::GRAPHICS, renderable.pipeline);
                device.cmd_bind_vertex_buffers(command_buffer, 0, &[renderable.vertex_buffer], &[0]);
                device.cmd_draw(command_buffer, renderable.vertex_count, 1, 0, 0);
            }

            device.cmd_end_render_pass(command_buffer);
            device.end_command_buffer(command_buffer).unwrap();
        }
    }
}

pub struct Renderable {
    vertex_buffer: vk::Buffer,
    vertex_allocation: vk_mem::Allocation,
    vertex_shader: vk::ShaderModule,
    fragment_shader: vk::ShaderModule,
    pipeline: vk::Pipeline,
    vertex_count: u32,
    depth: f32,
    on_window_resize_scale: bool,
    on_window_resize_move: bool,
}

pub struct Renderer {
    renderables: Vec<Renderable>,
    pipeline_layout: vk::PipelineLayout,
    uniform_buffer: vk::Buffer,
    uniform_allocation: vk_mem::Allocation,
    descriptor_set_layout: vk::DescriptorSetLayout,
    descriptor_pool: vk::DescriptorPool,
    descriptor_set: vk::DescriptorSet,
}

impl Renderer {
    pub fn new(platform: &mut Platform, extent: vk::Extent2D, scene: &Scene) -> Self {
        let surface_format = create_swapchain(platform, extent);
        create_framebuffers(platform, extent, surface_format);

        let ortho = Mat4::orthographic_rh(0.0, 600.0, 300.0, 0.0, -1.0, 1.0).to_cols_array();
        let (uniform_buffer, uniform_allocation) = {
            let device = platform.device.as_ref().unwrap(); // Borrow device here
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
            let (buffer, mut allocation) = unsafe {
                platform.allocator.as_ref().unwrap().create_buffer(&buffer_info, &allocation_info)
            }
            .unwrap();
            let data_ptr = unsafe { platform.allocator.as_ref().unwrap().map_memory(&mut allocation) }
                .unwrap()
                .cast::<f32>();
            unsafe { data_ptr.copy_from_nonoverlapping(ortho.as_ptr(), ortho.len()) };
            unsafe { platform.allocator.as_ref().unwrap().unmap_memory(&mut allocation) };
            (buffer, allocation)
        };

        let descriptor_set_layout = unsafe {
            let device = platform.device.as_ref().unwrap(); // Borrow device here
            let binding = vk::DescriptorSetLayoutBinding {
                binding: 0,
                descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::VERTEX,
                p_immutable_samplers: std::ptr::null(),
                _marker: PhantomData,
            };
            device.create_descriptor_set_layout(&vk::DescriptorSetLayoutCreateInfo {
                s_type: vk::StructureType::DESCRIPTOR_SET_LAYOUT_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: vk::DescriptorSetLayoutCreateFlags::empty(),
                binding_count: 1,
                p_bindings: &binding,
                _marker: PhantomData,
            }, None)
        }
        .unwrap();

        let descriptor_pool = unsafe {
            let device = platform.device.as_ref().unwrap(); // Borrow device here
            let pool_size = vk::DescriptorPoolSize {
                ty: vk::DescriptorType::UNIFORM_BUFFER,
                descriptor_count: 1,
            };
            device.create_descriptor_pool(&vk::DescriptorPoolCreateInfo {
                s_type: vk::StructureType::DESCRIPTOR_POOL_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: vk::DescriptorPoolCreateFlags::empty(),
                max_sets: 1,
                pool_size_count: 1,
                p_pool_sizes: &pool_size,
                _marker: PhantomData,
            }, None)
        }
        .unwrap();

        let descriptor_set = unsafe {
            let device = platform.device.as_ref().unwrap(); // Borrow device here
            device.allocate_descriptor_sets(&vk::DescriptorSetAllocateInfo {
                s_type: vk::StructureType::DESCRIPTOR_SET_ALLOCATE_INFO,
                p_next: std::ptr::null(),
                descriptor_pool,
                descriptor_set_count: 1,
                p_set_layouts: &descriptor_set_layout,
                _marker: PhantomData,
            })
        }
        .unwrap()[0];

        unsafe {
            let device = platform.device.as_ref().unwrap(); // Borrow device here
            let buffer_info = vk::DescriptorBufferInfo {
                buffer: uniform_buffer,
                offset: 0,
                range: std::mem::size_of_val(&ortho) as u64,
            };
            device.update_descriptor_sets(&[vk::WriteDescriptorSet {
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
            let device = platform.device.as_ref().unwrap(); // Borrow device here
            device.create_pipeline_layout(&vk::PipelineLayoutCreateInfo {
                s_type: vk::StructureType::PIPELINE_LAYOUT_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: vk::PipelineLayoutCreateFlags::empty(),
                set_layout_count: 1,
                p_set_layouts: &descriptor_set_layout,
                push_constant_range_count: 0,
                p_push_constant_ranges: std::ptr::null(),
                _marker: PhantomData,
            }, None)
        }
        .unwrap();

        let mut renderables = Vec::new();
        for obj in &scene.render_objects {
            let vertices = &obj.vertices;
            let (vertex_buffer, vertex_allocation) = {
                let device = platform.device.as_ref().unwrap(); // Borrow device here
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
                let (buffer, mut allocation) = unsafe {
                    platform.allocator.as_ref().unwrap().create_buffer(&buffer_info, &allocation_info)
                }
                .unwrap();
                let data_ptr = unsafe { platform.allocator.as_ref().unwrap().map_memory(&mut allocation) }
                    .unwrap()
                    .cast::<Vertex>();
                unsafe { data_ptr.copy_from_nonoverlapping(vertices.as_ptr(), vertices.len()) };
                unsafe { platform.allocator.as_ref().unwrap().unmap_memory(&mut allocation) };
                (buffer, allocation)
            };

            let vertex_shader = load_shader(platform.device.as_ref().unwrap(), &obj.vertex_shader_filename);
            let fragment_shader = load_shader(platform.device.as_ref().unwrap(), &obj.fragment_shader_filename);

            let pipeline = {
                let device = platform.device.as_ref().unwrap(); // Borrow device here
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

                let vertex_attributes = [vk::VertexInputAttributeDescription {
                    location: 0,
                    binding: 0,
                    format: vk::Format::R32G32_SFLOAT,
                    offset: 0,
                }];
                let vertex_bindings = [vk::VertexInputBindingDescription {
                    binding: 0,
                    stride: std::mem::size_of::<Vertex>() as u32,
                    input_rate: vk::VertexInputRate::VERTEX,
                }];
                let vertex_input = vk::PipelineVertexInputStateCreateInfo {
                    s_type: vk::StructureType::PIPELINE_VERTEX_INPUT_STATE_CREATE_INFO,
                    p_next: std::ptr::null(),
                    flags: vk::PipelineVertexInputStateCreateFlags::empty(),
                    vertex_binding_description_count: 1,
                    p_vertex_binding_descriptions: vertex_bindings.as_ptr(),
                    vertex_attribute_description_count: 1,
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

                let viewport = vk::Viewport {
                    x: 0.0,
                    y: 0.0,
                    width: extent.width as f32,
                    height: extent.height as f32,
                    min_depth: 0.0,
                    max_depth: 1.0,
                };
                let scissor = vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent,
                };
                let viewport_state = vk::PipelineViewportStateCreateInfo {
                    s_type: vk::StructureType::PIPELINE_VIEWPORT_STATE_CREATE_INFO,
                    p_next: std::ptr::null(),
                    flags: vk::PipelineViewportStateCreateFlags::empty(),
                    viewport_count: 1,
                    p_viewports: &viewport,
                    scissor_count: 1,
                    p_scissors: &scissor,
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
                    p_dynamic_state: std::ptr::null(),
                    layout: pipeline_layout,
                    render_pass: platform.render_pass.unwrap(),
                    subpass: 0,
                    base_pipeline_handle: vk::Pipeline::null(),
                    base_pipeline_index: -1,
                    _marker: PhantomData,
                };

                unsafe {
                    device
                        .create_graphics_pipelines(vk::PipelineCache::null(), &[pipeline_info], None)
                        .unwrap()[0]
                }
            };

            renderables.push(Renderable {
                vertex_buffer,
                vertex_allocation,
                vertex_shader,
                fragment_shader,
                pipeline,
                vertex_count: vertices.len() as u32,
                depth: obj.depth,
                on_window_resize_scale: obj.on_window_resize_scale,
                on_window_resize_move: obj.on_window_resize_move,
            });
        }

        renderables.sort_by(|a, b| a.depth.partial_cmp(&b.depth).unwrap());

        record_command_buffers(platform, &renderables, pipeline_layout, descriptor_set, extent);

        // Borrow device for semaphores and fence after mutable operations
        let device = platform.device.as_ref().unwrap();
        platform.image_available_semaphore = Some(unsafe { device.create_semaphore(&vk::SemaphoreCreateInfo::default(), None) }.unwrap());
        platform.render_finished_semaphore = Some(unsafe { device.create_semaphore(&vk::SemaphoreCreateInfo::default(), None) }.unwrap());
        platform.fence = Some(unsafe {
            device.create_fence(
                &vk::FenceCreateInfo {
                    s_type: vk::StructureType::FENCE_CREATE_INFO,
                    p_next: std::ptr::null(),
                    flags: vk::FenceCreateFlags::SIGNALED,
                    _marker: PhantomData,
                },
                None,
            )
        }
        .unwrap());

        Self {
            renderables,
            pipeline_layout,
            uniform_buffer,
            uniform_allocation,
            descriptor_set_layout,
            descriptor_pool,
            descriptor_set,
        }
    }

    pub fn render(&self, platform: &mut Platform) {
        if let (
            Some(device),
            Some(queue),
            Some(swapchain_loader),
            Some(swapchain),
        ) = (
            platform.device.as_ref(),
            platform.queue,
            platform.swapchain_loader.as_ref(),
            platform.swapchain,
        ) {
            let image_available_semaphore = platform.image_available_semaphore.unwrap();
            let render_finished_semaphore = platform.render_finished_semaphore.unwrap();
            let fence = platform.fence.unwrap();

            unsafe { device.wait_for_fences(&[fence], true, u64::MAX) }.unwrap();
            unsafe { device.reset_fences(&[fence]) }.unwrap();

            let (image_index, _) = unsafe {
                swapchain_loader.acquire_next_image(swapchain, u64::MAX, image_available_semaphore, vk::Fence::null())
            }
            .unwrap();
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
    }

    pub fn cleanup(mut self, platform: &mut Platform) {
        let device = platform.device.as_ref().unwrap();
        let swapchain_loader = platform.swapchain_loader.take().unwrap();

        unsafe {
            device.destroy_semaphore(platform.image_available_semaphore.take().unwrap(), None);
            device.destroy_semaphore(platform.render_finished_semaphore.take().unwrap(), None);
            device.destroy_fence(platform.fence.take().unwrap(), None);

            let command_pool = platform.command_pool.take().unwrap();
            device.free_command_buffers(command_pool, &platform.command_buffers);
            device.destroy_command_pool(command_pool, None);

            for mut renderable in self.renderables {
                device.destroy_pipeline(renderable.pipeline, None);
                device.destroy_shader_module(renderable.vertex_shader, None);
                device.destroy_shader_module(renderable.fragment_shader, None);
                platform
                    .allocator
                    .as_ref()
                    .unwrap()
                    .destroy_buffer(renderable.vertex_buffer, &mut renderable.vertex_allocation);
            }

            device.destroy_descriptor_pool(self.descriptor_pool, None);
            device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);
            platform
                .allocator
                .as_ref()
                .unwrap()
                .destroy_buffer(self.uniform_buffer, &mut self.uniform_allocation);

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