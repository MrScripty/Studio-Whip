use ash::vk;
use crate::gui_framework::context::vulkan_context::VulkanContext;
use crate::{PreparedDrawData, PreparedTextDrawData};
use bevy_log::info;

pub fn record_command_buffers(
    platform: &VulkanContext,
    prepared_shape_draws: &[PreparedDrawData],
    prepared_text_draws: &[PreparedTextDrawData],
    extent: vk::Extent2D,
    debug_buffer: Option<&mut crate::gui_framework::debug::DebugRingBuffer>,
) {
    #[cfg(feature = "debug_logging")]
    {
        let message = format!("[record_command_buffers] Entered. Shape draws: {}, Text draws: {}", prepared_shape_draws.len(), prepared_text_draws.len());
        if let Some(ref mut buffer) = debug_buffer {
            buffer.add_rendering_context(message);
        } else {
            info!("{}", message);
        }
    }

    // --- Command Buffer Recording Loop ---
    let device = platform.device.as_ref().expect("Device not available for command buffer recording");
    // Get the specific command buffer for the current image; it's already reset by the Renderer
    let command_buffer = platform.command_buffers[platform.current_image];
    let begin_info = vk::CommandBufferBeginInfo { s_type: vk::StructureType::COMMAND_BUFFER_BEGIN_INFO, flags: vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT, ..Default::default() };
    let clear_values = [ vk::ClearValue { color: vk::ClearColorValue { float32: [0.1, 0.1, 0.1, 1.0] } }, vk::ClearValue { depth_stencil: vk::ClearDepthStencilValue { depth: 1.0, stencil: 0 } }, ];
    let framebuffer = platform.framebuffers[platform.current_image];

    unsafe {
        device.begin_command_buffer(command_buffer, &begin_info).expect("Failed to begin command buffer recording");

        // Begin render pass
        let render_pass_begin_info = vk::RenderPassBeginInfo { s_type: vk::StructureType::RENDER_PASS_BEGIN_INFO, render_pass: platform.render_pass.expect("Render pass not available"), framebuffer, render_area: vk::Rect2D { offset: vk::Offset2D { x: 0, y: 0 }, extent }, clear_value_count: clear_values.len() as u32, p_clear_values: clear_values.as_ptr(), ..Default::default() };
        device.cmd_begin_render_pass(command_buffer, &render_pass_begin_info, vk::SubpassContents::INLINE);

        // Set dynamic viewport and scissor state
        let viewport = vk::Viewport { x: 0.0, y: 0.0, width: extent.width as f32, height: extent.height as f32, min_depth: 0.0, max_depth: 1.0, };
        let scissor = vk::Rect2D { offset: vk::Offset2D { x: 0, y: 0 }, extent, };
        device.cmd_set_viewport(command_buffer, 0, &[viewport]);
        device.cmd_set_scissor(command_buffer, 0, &[scissor]);

        // --- Draw Shapes ---
        if !prepared_shape_draws.is_empty() {
            // Get shape pipeline layout (includes push constant range)
            let shape_pipeline_layout = platform.shape_pipeline_layout.expect("Shape pipeline layout missing");

            // Bind the single shape pipeline *once* outside the loop
            // Assuming the first draw_data contains the correct pipeline handle
            if let Some(first_draw) = prepared_shape_draws.first() {
                 device.cmd_bind_pipeline(command_buffer, vk::PipelineBindPoint::GRAPHICS, first_draw.pipeline);
            }

            for draw_data in prepared_shape_draws {
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

                // --- Push Color Constant ---
                device.cmd_push_constants(
                    command_buffer,
                    shape_pipeline_layout,
                    vk::ShaderStageFlags::FRAGMENT, // Stage flags match range definition
                    0, // Offset matches range definition
                    std::slice::from_raw_parts(
                        draw_data.color.as_ptr() as *const u8, // Pointer to color data
                        std::mem::size_of::<[f32; 4]>(),       // Size matches range definition
                    ),
                );

                // --- Draw Call (Non-instanced for now) ---
                device.cmd_draw(
                    command_buffer,
                    draw_data.vertex_count,    // vertexCount
                    1,                         // instanceCount
                    0,                         // firstVertex
                    0,                         // firstInstance
                );
            } // End of shape draw loop
        }

        // --- Draw Text ---
        if !prepared_text_draws.is_empty() {
            #[cfg(feature = "debug_logging")]
            {
                let message = format!("[record_command_buffers] Processing {} text draws.", prepared_text_draws.len());
                if let Some(ref mut buffer) = debug_buffer {
                    buffer.add_rendering_context(message);
                } else {
                    info!("{}", message);
                }
            }
            let text_pipeline_layout = platform.text_pipeline_layout.expect("Text pipeline layout missing");
            let mut current_text_pipeline = vk::Pipeline::null();

            for (i, text_draw) in prepared_text_draws.iter().enumerate() { // Iterate with index and reference
                if text_draw.vertex_count > 0 {
                    #[cfg(feature = "trace_logging")]
                    {
                        let message = format!("[record_command_buffers] Text Draw Index {}: Attempting to bind resources. VB: {:?}, Vertices: {}, DS0: {:?}, DS1: {:?}",
                            i,
                            text_draw.vertex_buffer,
                            text_draw.vertex_count,
                            text_draw.projection_descriptor_set,
                            text_draw.atlas_descriptor_set
                        );
                        if let Some(ref mut buffer) = debug_buffer {
                            buffer.add_rendering_context(message);
                        } else {
                            info!("{}", message);
                        }
                    }
                    if text_draw.pipeline != current_text_pipeline {
                        device.cmd_bind_pipeline(command_buffer, vk::PipelineBindPoint::GRAPHICS, text_draw.pipeline);
                        #[cfg(feature = "trace_logging")]
                        {
                            let message = format!("[record_command_buffers] Text Draw Index {}: Bound NEW pipeline.", i);
                            if let Some(ref mut buffer) = debug_buffer {
                                buffer.add_rendering_context(message);
                            } else {
                                info!("{}", message);
                            }
                        }
                        current_text_pipeline = text_draw.pipeline;
                    } else {
                        #[cfg(feature = "trace_logging")]
                        {
                            let message = format!("[record_command_buffers] Text Draw Index {}: Reusing current pipeline.", i);
                            if let Some(ref mut buffer) = debug_buffer {
                                buffer.add_rendering_context(message);
                            } else {
                                info!("{}", message);
                            }
                        }
                    }
                    device.cmd_bind_descriptor_sets(
                        command_buffer, vk::PipelineBindPoint::GRAPHICS, text_pipeline_layout,
                        0, // firstSet
                        &[text_draw.projection_descriptor_set, text_draw.atlas_descriptor_set], // Bind Set 0 and Set 1
                        &[], // No dynamic offsets
                    );
                    let offsets = [0];
                    device.cmd_bind_vertex_buffers(command_buffer, 0, &[text_draw.vertex_buffer], &offsets);
                    #[cfg(feature = "trace_logging")]
                    {
                        let message = format!("[record_command_buffers] Text Draw Index {}: Bound vertex buffer.", i);
                        if let Some(ref mut buffer) = debug_buffer {
                            buffer.add_rendering_context(message);
                        } else {
                            info!("{}", message);
                        }
                    }
                    device.cmd_draw(
                        command_buffer,
                        text_draw.vertex_count,
                        1, // instanceCount
                        0, // firstVertex
                        0, // firstInstance
                    );
                    #[cfg(feature = "trace_logging")]
                    {
                        let message = format!("[record_command_buffers] Text Draw Index {}: Draw call executed.", i);
                        if let Some(ref mut buffer) = debug_buffer {
                            buffer.add_rendering_context(message);
                        } else {
                            info!("{}", message);
                        }
                    }
                }
            }
        }
        // End the render pass
        device.cmd_end_render_pass(command_buffer);

        // End command buffer recording
        device.end_command_buffer(command_buffer).expect("Failed to end command buffer recording");
    }
    #[cfg(feature = "debug_logging")]
    {
        let message = "[record_command_buffers] Exited successfully.".to_string();
        if let Some(ref mut buffer) = debug_buffer {
            buffer.add_rendering_context(message);
        } else {
            info!("{}", message);
        }
    }
}