use ash::vk;
use crate::gui_framework::context::vulkan_context::VulkanContext;
use crate::{PreparedDrawData, TextRenderCommandData};
use bevy_log::info; // Added for logging

pub fn record_command_buffers(
    platform: &mut VulkanContext,
    prepared_shape_draws: &[PreparedDrawData],
    text_vertex_buffer: vk::Buffer,
    glyph_atlas_descriptor_set: vk::DescriptorSet,
    text_command: Option<&TextRenderCommandData>, // Pass optional text draw info
    extent: vk::Extent2D,
    text_pipeline: vk::Pipeline,
) {
    let device = platform.device.as_ref().expect("Device not available for command buffer recording");
    let command_pool = platform.command_pool.expect("Command pool missing for recording");

    // --- Reset Command Pool (Instead of Recreating) ---
    // This implicitly resets all command buffers allocated from it.
    // This MUST happen only after the fence associated with the *last* submission
    // using this pool/buffers has been signaled and waited upon.
    // The waiting happens in Renderer::render *before* calling this function.
    unsafe {
        device.reset_command_pool(command_pool, vk::CommandPoolResetFlags::empty())
            .expect("Failed to reset command pool");
    }

    // --- Allocate Command Buffers (If needed, usually only once) ---
    // If command buffers haven't been allocated yet (e.g., first frame or after resize)
    if platform.command_buffers.is_empty() || platform.command_buffers.len() != platform.framebuffers.len() {
        // Free existing buffers if the count is wrong (e.g., after resize changed framebuffer count)
        if !platform.command_buffers.is_empty() {
             unsafe { device.free_command_buffers(command_pool, &platform.command_buffers); }
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
        info!("[record_command_buffers] Allocated {} command buffers.", platform.command_buffers.len());
    }


    // --- Command Buffer Recording Loop ---
    // Get the command buffer for the *current* frame being rendered
    // The index is determined by acquire_next_image in Renderer::render
    let command_buffer = platform.command_buffers[platform.current_image];

    let begin_info = vk::CommandBufferBeginInfo {
        s_type: vk::StructureType::COMMAND_BUFFER_BEGIN_INFO,
        // Consider VK_COMMAND_BUFFER_USAGE_SIMULTANEOUS_USE_BIT if command buffers might be resubmitted
        // while pending execution, but ONE_TIME_SUBMIT is usually fine for simple render loops.
        flags: vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
        ..Default::default()
    };
    let clear_values = [vk::ClearValue {
        // Use opaque black background
        color: vk::ClearColorValue { float32: [1.0, 0.0, 1.0, 1.0] },
    }];

    let framebuffer = platform.framebuffers[platform.current_image]; // Use current image index

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

        // --- Draw Shapes ---
        if !prepared_shape_draws.is_empty() { // Check if there are shapes to draw
            // Get shape pipeline layout from context
            let shape_pipeline_layout = platform.shape_pipeline_layout.expect("Shape pipeline layout missing");
            let mut current_pipeline = vk::Pipeline::null(); // Track bound pipeline

            for draw_data in prepared_shape_draws { // Iterate using the correct variable name
                // Bind pipeline only if it changed
                if draw_data.pipeline != current_pipeline {
                    device.cmd_bind_pipeline(command_buffer, vk::PipelineBindPoint::GRAPHICS, draw_data.pipeline);
                    current_pipeline = draw_data.pipeline;
                }

                // Bind shape descriptor set (Set 0)
                device.cmd_bind_descriptor_sets(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    shape_pipeline_layout, // Use the fetched layout
                    0, // firstSet index
                    &[draw_data.descriptor_set], // The specific set for this entity
                    &[], // No dynamic offsets
                );

                // Bind the vertex buffer to binding point 0
                device.cmd_bind_vertex_buffers(command_buffer, 0, &[draw_data.vertex_buffer], &[0]); // offset 0

                // --- Draw Call (Non-instanced for now) ---
                device.cmd_draw(
                    command_buffer,
                    draw_data.vertex_count,    // vertexCount
                    1,                         // instanceCount (Hardcoded to 1 for now)
                    0,                         // firstVertex
                    0,                         // firstInstance
                );
            }
        }

        // --- Draw Text ---
        if let Some(text_cmd) = text_command {
            if text_cmd.vertex_count > 0 {
                // Get text pipeline layout from context
                let text_pipeline_layout = platform.text_pipeline_layout.expect("Text pipeline layout missing");

                // Bind the text pipeline (passed as argument)
                device.cmd_bind_pipeline(command_buffer, vk::PipelineBindPoint::GRAPHICS, text_pipeline);

                // Bind text descriptor sets
                // Set 0: Global UBO (reuse shape descriptor set? Or need a dedicated one?)
                //      Let's assume text vertex shader only uses binding 0 from set 0.
                //      We need the *global* projection UBO descriptor set.
                //      This is currently part of the per-shape descriptor sets.
                //      Refactor needed: Separate global UBO descriptor set.
                //      *** Temporary Workaround: Bind the *first* shape's descriptor set for Set 0 ***
                //      *** This is INCORRECT but allows compilation. Needs fixing. ***
                let set0_binding = if let Some(first_shape) = prepared_shape_draws.first() {
                    first_shape.descriptor_set
                } else {
                    // Cannot bind Set 0 if no shapes exist. Text won't render correctly.
                    bevy_log::warn!("[record_command_buffers] No shape descriptor set available to bind for text Set 0 (Projection UBO). Text rendering will likely fail.");
                    vk::DescriptorSet::null() // Or skip text rendering entirely
                };

                if set0_binding != vk::DescriptorSet::null() {
                    // Bind Set 0 (Projection UBO - using workaround) and Set 1 (Atlas Sampler)
                    device.cmd_bind_descriptor_sets(
                        command_buffer, vk::PipelineBindPoint::GRAPHICS, text_pipeline_layout,
                        0, // firstSet
                        &[set0_binding, glyph_atlas_descriptor_set], // Bind Set 0 and Set 1
                        &[], // No dynamic offsets
                    );

                    // Bind text vertex buffer
                    device.cmd_bind_vertex_buffers(command_buffer, 0, &[text_vertex_buffer], &[0]);

                    // Draw text vertices
                    device.cmd_draw(
                        command_buffer,
                        text_cmd.vertex_count,
                        1, // instanceCount
                        text_cmd.vertex_buffer_offset, // firstVertex
                        0, // firstInstance
                    );
                }
            }
        }

        // End the render pass
        device.cmd_end_render_pass(command_buffer);

        // End command buffer recording
        device.end_command_buffer(command_buffer)
            .expect("Failed to end command buffer recording");
    }
}