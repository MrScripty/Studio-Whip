use ash::vk;
use crate::gui_framework::context::vulkan_context::VulkanContext;
use crate::gui_framework::rendering::renderable::Renderable;
use std::marker::PhantomData;

pub fn record_command_buffers(
    platform: &mut VulkanContext,
    renderables: &[Renderable],
    pipeline_layout: vk::PipelineLayout,
    _projection_descriptor_set: vk::DescriptorSet, // No longer used directly
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

    if let Some(command_pool) = platform.command_pool.take() {
        unsafe { device.destroy_command_pool(command_pool, None) };
    }

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
            device.cmd_set_viewport(command_buffer, 0, &[viewport]);
            device.cmd_set_scissor(command_buffer, 0, &[scissor]);

            for renderable in renderables {
                device.cmd_bind_pipeline(command_buffer, vk::PipelineBindPoint::GRAPHICS, renderable.pipeline);
                device.cmd_bind_descriptor_sets(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    pipeline_layout,
                    0,
                    &[renderable.descriptor_set], // Use per-object descriptor set
                    &[],
                );
                device.cmd_bind_vertex_buffers(command_buffer, 0, &[renderable.vertex_buffer], &[0]);
                device.cmd_draw(command_buffer, renderable.vertex_count, 1, 0, 0);
            }

            device.cmd_end_render_pass(command_buffer);
            device.end_command_buffer(command_buffer).unwrap();
        }
    }
}