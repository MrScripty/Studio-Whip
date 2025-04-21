use ash::vk;
use crate::gui_framework::context::vulkan_context::VulkanContext;
use crate::PreparedDrawData;

pub fn record_command_buffers(
    platform: &mut VulkanContext,
    prepared_draws: &[PreparedDrawData], // Use PreparedDrawData
    pipeline_layout: vk::PipelineLayout, // Still need layout for binding
    // Removed projection_descriptor_set parameter
    extent: vk::Extent2D,
) {
    let device = platform.device.as_ref().expect("Device not available for command buffer recording");

    // --- Recreate Command Pool ---
    // Destroy existing pool if it exists
    if let Some(command_pool) = platform.command_pool.take() {
        unsafe { device.destroy_command_pool(command_pool, None) };
    }

    // Create new command pool
    let command_pool = unsafe {
        // TODO: Properly determine the graphics queue family index.
        // This might involve querying during setup and storing it in VulkanContext,
        // or re-querying here if necessary. Using 0 as a placeholder.
        let queue_family_index = platform.queue_family_index
            .expect("Queue family index not set in VulkanContext");
        device.create_command_pool(
            &vk::CommandPoolCreateInfo {
                s_type: vk::StructureType::COMMAND_POOL_CREATE_INFO,
                flags: vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
                queue_family_index, // Use the retrieved index
                ..Default::default()
            },
            None,
        )
    }.expect("Failed to create command pool");
    platform.command_pool = Some(command_pool);

    // --- Allocate Command Buffers ---
    // Free existing buffers if necessary (though destroying the pool should handle this)
    if !platform.command_buffers.is_empty() {
        // This might be redundant if pool is always destroyed, but safer if pool isn't always destroyed.
        // unsafe { device.free_command_buffers(command_pool, &platform.command_buffers); }
        platform.command_buffers.clear();
    }

    platform.command_buffers = {
        let alloc_info = vk::CommandBufferAllocateInfo {
            s_type: vk::StructureType::COMMAND_BUFFER_ALLOCATE_INFO,
            command_pool,
            level: vk::CommandBufferLevel::PRIMARY,
            command_buffer_count: platform.framebuffers.len() as u32, // One per framebuffer
            ..Default::default()
        };
        unsafe { device.allocate_command_buffers(&alloc_info) }.expect("Failed to allocate command buffers")
    };

    // --- Command Buffer Recording Loop ---
    let begin_info = vk::CommandBufferBeginInfo {
        s_type: vk::StructureType::COMMAND_BUFFER_BEGIN_INFO,
        // Consider VK_COMMAND_BUFFER_USAGE_SIMULTANEOUS_USE_BIT if command buffers might be resubmitted
        // while pending execution, but ONE_TIME_SUBMIT is usually fine for simple render loops.
        flags: vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
        ..Default::default()
    };
    let clear_values = [vk::ClearValue {
        // Use opaque black background
        color: vk::ClearColorValue { float32: [0.0, 0.0, 0.0, 1.0] },
    }];

    for (i, &command_buffer) in platform.command_buffers.iter().enumerate() {
        let framebuffer = platform.framebuffers[i]; // Get corresponding framebuffer

        unsafe {
            // Begin command buffer recording
            device.begin_command_buffer(command_buffer, &begin_info)
                .expect("Failed to begin command buffer recording");

            // Begin render pass
            let render_pass_begin_info = vk::RenderPassBeginInfo {
                s_type: vk::StructureType::RENDER_PASS_BEGIN_INFO,
                render_pass: platform.render_pass.expect("Render pass not available for command buffer recording"),
                framebuffer,
                render_area: vk::Rect2D { offset: vk::Offset2D { x: 0, y: 0 }, extent },
                clear_value_count: clear_values.len() as u32,
                p_clear_values: clear_values.as_ptr(),
                ..Default::default()
            };
            device.cmd_begin_render_pass(command_buffer, &render_pass_begin_info, vk::SubpassContents::INLINE);

            // Set dynamic viewport and scissor state
            let viewport = vk::Viewport {
                x: 0.0,
                y: 0.0, // Vulkan's Y is typically down from top-left
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

            // --- Draw Prepared Data ---
            for draw_data in prepared_draws {
                // Visibility check is now done in rendering_system before creating PreparedDrawData

                // Bind the pipeline for this draw
                device.cmd_bind_pipeline(command_buffer, vk::PipelineBindPoint::GRAPHICS, draw_data.pipeline);

                // Bind the per-entity descriptor set (contains projection and offset UBOs)
                device.cmd_bind_descriptor_sets(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    pipeline_layout, // Use the common pipeline layout
                    0, // firstSet index
                    &[draw_data.descriptor_set], // The specific set for this entity
                    &[], // No dynamic offsets
                );

                // Bind the vertex buffer to binding point 0
                device.cmd_bind_vertex_buffers(command_buffer, 0, &[draw_data.vertex_buffer], &[0]); // offset 0

                // --- Draw Call (Non-instanced for now) ---
                // TODO: Add instancing support later by checking PreparedDrawData
                device.cmd_draw(
                    command_buffer,
                    draw_data.vertex_count,    // vertexCount
                    1,                         // instanceCount (Hardcoded to 1 for now)
                    0,                         // firstVertex
                    0,                         // firstInstance
                );
            }

            // End the render pass
            device.cmd_end_render_pass(command_buffer);

            // End command buffer recording
            device.end_command_buffer(command_buffer)
                .expect("Failed to end command buffer recording");
        }
    }
}