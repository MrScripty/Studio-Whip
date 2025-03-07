use ash::vk;
use ash::khr::swapchain;
use std::marker::PhantomData;
use vk_mem::Alloc;
use crate::{Vertex, application::App};

pub fn setup_renderer(app: &mut App, extent: vk::Extent2D) {
    let instance = app.instance.as_ref().unwrap();
    let device = app.device.as_ref().unwrap();
    let surface = app.surface.unwrap();
    let surface_loader = app.surface_loader.as_ref().unwrap();
    let physical_device = unsafe { instance.enumerate_physical_devices().unwrap() }[0]; // Simplified

    let swapchain_loader = swapchain::Device::new(instance, device);
    app.swapchain_loader = Some(swapchain_loader.clone());

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
    app.swapchain = Some(swapchain);
    app.images = images;

    app.image_views = app
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

    let vertices = vec![
        Vertex { position: [-0.5, -0.5] },
        Vertex { position: [0.0, 0.5] },
        Vertex { position: [0.5, -0.25] },
    ];
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
        let (buffer, mut allocation) = unsafe {
            app.allocator.as_ref().unwrap().create_buffer(&buffer_info, &allocation_info)
        }
        .unwrap();
        let data_ptr = unsafe { app.allocator.as_ref().unwrap().map_memory(&mut allocation) }
            .unwrap()
            .cast::<Vertex>();
        unsafe { data_ptr.copy_from_nonoverlapping(vertices.as_ptr(), vertices.len()) };
        unsafe { app.allocator.as_ref().unwrap().unmap_memory(&mut allocation) };
        (buffer, allocation)
    };
    app.vertex_buffer = Some(vertex_buffer);
    app.vertex_allocation = Some(vertex_allocation);

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
    app.render_pass = Some(render_pass);

    app.framebuffers = app
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

    let vertex_shader_spirv = include_bytes!("../shaders/background.vert.spv");
    let fragment_shader_spirv = include_bytes!("../shaders/background.frag.spv");

    let vertex_shader = unsafe {
        device.create_shader_module(
            &vk::ShaderModuleCreateInfo {
                s_type: vk::StructureType::SHADER_MODULE_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: vk::ShaderModuleCreateFlags::empty(),
                code_size: vertex_shader_spirv.len(),
                p_code: vertex_shader_spirv.as_ptr() as *const u32,
                _marker: PhantomData,
            },
            None,
        )
    }
    .unwrap();
    app.vertex_shader = Some(vertex_shader);

    let fragment_shader = unsafe {
        device.create_shader_module(
            &vk::ShaderModuleCreateInfo {
                s_type: vk::StructureType::SHADER_MODULE_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: vk::ShaderModuleCreateFlags::empty(),
                code_size: fragment_shader_spirv.len(),
                p_code: fragment_shader_spirv.as_ptr() as *const u32,
                _marker: PhantomData,
            },
            None,
        )
    }
    .unwrap();
    app.fragment_shader = Some(fragment_shader);

    let pipeline_layout = unsafe {
        device.create_pipeline_layout(&vk::PipelineLayoutCreateInfo::default(), None)
    }
    .unwrap();
    app.pipeline_layout = Some(pipeline_layout);

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
            topology: vk::PrimitiveTopology::TRIANGLE_LIST,
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
            render_pass,
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
    app.pipeline = Some(pipeline);

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
            .unwrap() // Simplified
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
    app.command_pool = Some(command_pool);

    app.command_buffers = {
        let alloc_info = vk::CommandBufferAllocateInfo {
            s_type: vk::StructureType::COMMAND_BUFFER_ALLOCATE_INFO, // Fixed to correct StructureType
            p_next: std::ptr::null(),
            command_pool,
            level: vk::CommandBufferLevel::PRIMARY,
            command_buffer_count: app.framebuffers.len() as u32,
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
        color: vk::ClearColorValue { float32: [0.0, 0.0, 1.0, 1.0] },
    }];
    for (&command_buffer, &framebuffer) in app.command_buffers.iter().zip(app.framebuffers.iter()) {
        let render_pass_begin_info = vk::RenderPassBeginInfo {
            s_type: vk::StructureType::RENDER_PASS_BEGIN_INFO,
            p_next: std::ptr::null(),
            render_pass,
            framebuffer,
            render_area: vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent,
            },
            clear_value_count: clear_values.len() as u32,
            p_clear_values: clear_values.as_ptr(),
            _marker: PhantomData,
        };

        unsafe {
            device.begin_command_buffer(command_buffer, &begin_info).unwrap();
            device.cmd_begin_render_pass(command_buffer, &render_pass_begin_info, vk::SubpassContents::INLINE);
            device.cmd_bind_pipeline(command_buffer, vk::PipelineBindPoint::GRAPHICS, pipeline);
            device.cmd_bind_vertex_buffers(command_buffer, 0, &[vertex_buffer], &[0]);
            device.cmd_draw(command_buffer, 3, 1, 0, 0);
            device.cmd_end_render_pass(command_buffer);
            device.end_command_buffer(command_buffer).unwrap();
        }
    }

    app.image_available_semaphore = Some(unsafe { device.create_semaphore(&vk::SemaphoreCreateInfo::default(), None) }.unwrap());
    app.render_finished_semaphore = Some(unsafe { device.create_semaphore(&vk::SemaphoreCreateInfo::default(), None) }.unwrap());
    app.fence = Some(unsafe {
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
}

pub fn cleanup_renderer(app: &mut App) {
    let device = app.device.as_ref().unwrap();
    let swapchain_loader = app.swapchain_loader.take().unwrap();

    unsafe {
        device.destroy_semaphore(app.image_available_semaphore.take().unwrap(), None);
        device.destroy_semaphore(app.render_finished_semaphore.take().unwrap(), None);
        device.destroy_fence(app.fence.take().unwrap(), None);

        let command_pool = app.command_pool.take().unwrap();
        device.free_command_buffers(command_pool, &app.command_buffers);
        device.destroy_command_pool(command_pool, None);

        device.destroy_pipeline(app.pipeline.take().unwrap(), None);
        device.destroy_pipeline_layout(app.pipeline_layout.take().unwrap(), None);
        device.destroy_shader_module(app.vertex_shader.take().unwrap(), None);
        device.destroy_shader_module(app.fragment_shader.take().unwrap(), None);

        for &framebuffer in &app.framebuffers {
            device.destroy_framebuffer(framebuffer, None);
        }
        device.destroy_render_pass(app.render_pass.take().unwrap(), None);

        app.allocator
            .as_ref()
            .unwrap()
            .destroy_buffer(app.vertex_buffer.take().unwrap(), &mut app.vertex_allocation.take().unwrap());

        swapchain_loader.destroy_swapchain(app.swapchain.take().unwrap(), None);
        for &view in &app.image_views {
            device.destroy_image_view(view, None);
        }
    }
}

pub fn render(app: &mut App) {
    if let (
        Some(device),
        Some(queue),
        Some(swapchain_loader),
        Some(swapchain),
    ) = (
        app.device.as_ref(),
        app.queue,
        app.swapchain_loader.as_ref(),
        app.swapchain,
    ) {
        let image_available_semaphore = app.image_available_semaphore.unwrap();
        let render_finished_semaphore = app.render_finished_semaphore.unwrap();
        let fence = app.fence.unwrap();

        unsafe { device.wait_for_fences(&[fence], true, u64::MAX) }.unwrap();
        unsafe { device.reset_fences(&[fence]) }.unwrap();

        let (image_index, _) = unsafe {
            swapchain_loader.acquire_next_image(swapchain, u64::MAX, image_available_semaphore, vk::Fence::null())
        }
        .unwrap();
        app.current_image = image_index as usize;

        let submit_info = vk::SubmitInfo {
            s_type: vk::StructureType::SUBMIT_INFO,
            p_next: std::ptr::null(),
            wait_semaphore_count: 1,
            p_wait_semaphores: &image_available_semaphore,
            p_wait_dst_stage_mask: &vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT as *const _,
            command_buffer_count: 1,
            p_command_buffers: &app.command_buffers[app.current_image],
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
            p_image_indices: &(app.current_image as u32),
            p_results: std::ptr::null_mut(),
            _marker: PhantomData,
        };
        unsafe { swapchain_loader.queue_present(queue, &present_info) }.unwrap();
    }
}