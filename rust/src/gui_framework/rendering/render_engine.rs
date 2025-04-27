use ash::vk;
use crate::gui_framework::context::vulkan_context::VulkanContext;
use crate::gui_framework::rendering::swapchain::{create_swapchain, create_framebuffers};
// Removed direct import of cleanup_swapchain_resources, it's called by ResizeHandler
use crate::gui_framework::rendering::command_buffers::record_command_buffers;
use crate::gui_framework::rendering::pipeline_manager::PipelineManager;
use crate::gui_framework::rendering::buffer_manager::BufferManager;
use crate::gui_framework::rendering::resize_handler::ResizeHandler;
use bevy_math::Mat4;
use bevy_log::{info, warn, error};
use crate::RenderCommandData; // from lib.rs
use crate::{TextVertex, TextRenderCommandData}; // Added TextVertex and TextRenderCommandData
use vk_mem::{Allocation, Alloc}; // Added for text vertex buffer allocation
use crate::GlyphAtlasResource; // Added for render signature

pub struct Renderer {
    buffer_manager: BufferManager,
    // Store pool and layouts needed for cleanup
    descriptor_pool: vk::DescriptorPool,
    descriptor_set_layout: vk::DescriptorSetLayout, // For shapes
    text_descriptor_set_layout: vk::DescriptorSetLayout,

    // --- Text Rendering Resources ---
    text_vertex_buffer: vk::Buffer,
    text_vertex_allocation: Option<Allocation>,
    text_vertex_buffer_capacity: u32, // Max number of TextVertex this buffer can hold
    glyph_atlas_descriptor_set: vk::DescriptorSet, // Single set pointing to the atlas texture/sampler
}

impl Renderer {
    pub fn new(platform: &mut VulkanContext, extent: vk::Extent2D) -> Self {
        let surface_format = create_swapchain(platform, extent);
        create_framebuffers(platform, surface_format);
        info!("[Renderer::new] Framebuffers created");

        // Create PipelineManager temporarily to get layout/pool
        let pipeline_mgr = PipelineManager::new(platform);
        info!("[Renderer::new] PipelineManager created (temporarily)");

        // Store layouts in VulkanContext for BufferManager access
        platform.shape_pipeline_layout = Some(pipeline_mgr.shape_pipeline_layout);
        platform.text_pipeline_layout = Some(pipeline_mgr.text_pipeline_layout);
        info!("[Renderer::new] Shape and Text PipelineLayouts stored in VulkanContext");

        let buffer_mgr = BufferManager::new(
            platform, // Pass &mut VulkanContext
            // Pass the shape-specific layouts needed by BufferManager for shapes
            pipeline_mgr.shape_pipeline_layout,
            pipeline_mgr.shape_descriptor_set_layout,
            pipeline_mgr.descriptor_pool,
        );
        info!("[Renderer::new] BufferManager created");

        // Store pool and set_layout in Renderer for cleanup
        let descriptor_pool = pipeline_mgr.descriptor_pool;
        // Store the shape layout, as BufferManager uses it
        let descriptor_set_layout = pipeline_mgr.shape_descriptor_set_layout;
        // Also store the text layout for potential future cleanup needs? Or let VulkanContext own it?
        // Let's store both shape and text layouts in Renderer for cleanup for now.
        let text_descriptor_set_layout = pipeline_mgr.text_descriptor_set_layout;
        // pipeline_mgr goes out of scope here, its layouts are moved to platform/Renderer

        // Update global projection UBO (BufferManager owns the buffer/allocation)
        let initial_logical_width = extent.width as f32; // Use the extent passed to Renderer::new
        let initial_logical_height = extent.height as f32;
        unsafe {
            let proj = Mat4::orthographic_rh(0.0, initial_logical_width, 0.0, initial_logical_height, -1.0, 1.0);
            // We are flipping Y beacuse Bevy coord space uses +Y and Vulkan uses -Y
            let flip_y = Mat4::from_scale(bevy_math::Vec3::new(1.0, -1.0, 1.0));
            let proj_matrix = flip_y * proj;

            let allocator = platform.allocator.as_ref().unwrap();
            let info = allocator.get_allocation_info(&buffer_mgr.uniform_allocation);
            bevy_log::info!("Renderer::new: Writing initial projection for logical extent {}x{}, Matrix:\n{:?}", initial_logical_width, initial_logical_height, proj_matrix);
            // Use get_allocation_info for persistently mapped buffer
            if !info.mapped_data.is_null() {
                let data_ptr = info.mapped_data.cast::<f32>();
                data_ptr.copy_from_nonoverlapping(proj_matrix.to_cols_array().as_ptr(), 16);
                // No need to unmap
            } else {
                error!("[Renderer::new] Failed to get mapped pointer for initial uniform buffer update.");
                // Attempt map/unmap as fallback? Or panic?
                // For now, log error.
            }
        }
        info!("[Renderer::new] Global projection UBO buffer updated (Descriptor set update deferred to BufferManager)");

        // --- Create Command Pool (Once) ---
        // Command buffers will be allocated later in record_command_buffers if needed
        platform.command_pool = Some(unsafe {
            let queue_family_index = platform.queue_family_index
                .expect("Queue family index not set in VulkanContext");
            platform.device.as_ref().unwrap().create_command_pool(
                &vk::CommandPoolCreateInfo {
                    s_type: vk::StructureType::COMMAND_POOL_CREATE_INFO,
                    // Allow resetting individual command buffers or the whole pool
                    flags: vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
                    queue_family_index,
                    ..Default::default()
                },
                None,
            )
        }.expect("Failed to create command pool"));
        info!("[Renderer::new] Command pool created");


        // Create sync objects
        platform.image_available_semaphore = Some(unsafe {
            platform.device.as_ref().unwrap().create_semaphore(&vk::SemaphoreCreateInfo::default(), None).expect("Failed to create image available semaphore")
        });
        platform.render_finished_semaphore = Some(unsafe {
             platform.device.as_ref().unwrap().create_semaphore(&vk::SemaphoreCreateInfo::default(), None).expect("Failed to create render finished semaphore")
        });
        platform.fence = Some(unsafe {
            platform.device.as_ref().unwrap().create_fence(
                &vk::FenceCreateInfo {
                    s_type: vk::StructureType::FENCE_CREATE_INFO,
                    flags: vk::FenceCreateFlags::SIGNALED, // Start signaled
                    ..Default::default()
                }, None).expect("Failed to create fence")
        });
        info!("[Renderer::new] Sync objects created");
        info!("[Renderer::new] Finished");

        let mut renderer = Self {
            buffer_manager: buffer_mgr,
            descriptor_pool, // Store for cleanup
            descriptor_set_layout, // Store shape layout for cleanup
            text_descriptor_set_layout, // Store text layout for cleanup

            // --- Initialize Text Rendering Resources ---
            text_vertex_buffer: vk::Buffer::null(), // Placeholder, will be created below
            text_vertex_allocation: None,
            text_vertex_buffer_capacity: 0, // Placeholder
            glyph_atlas_descriptor_set: vk::DescriptorSet::null(), // Placeholder
        };

        // --- Create Initial Dynamic Text Vertex Buffer ---
        // Start with a reasonable capacity, e.g., 1024 vertices
        let initial_text_capacity = 1024 * 6; // Enough for ~1024 glyphs (6 vertices per quad)
        let buffer_size = (std::mem::size_of::<TextVertex>() * initial_text_capacity as usize) as vk::DeviceSize;
        let (buffer, allocation) = unsafe {
            let buffer_info = vk::BufferCreateInfo {
                s_type: vk::StructureType::BUFFER_CREATE_INFO,
                size: buffer_size,
                usage: vk::BufferUsageFlags::VERTEX_BUFFER,
                sharing_mode: vk::SharingMode::EXCLUSIVE,
                ..Default::default()
            };
            let allocation_info = vk_mem::AllocationCreateInfo {
                flags: vk_mem::AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE | vk_mem::AllocationCreateFlags::MAPPED,
                usage: vk_mem::MemoryUsage::AutoPreferDevice,
                ..Default::default()
            };
            let allocator = platform.allocator.as_ref().unwrap();
            allocator.create_buffer(&buffer_info, &allocation_info)
                     .expect("Failed to create initial text vertex buffer")
        };
        info!("[Renderer::new] Initial text vertex buffer created (Capacity: {} vertices, Size: {} bytes)", initial_text_capacity, buffer_size);
        renderer.text_vertex_buffer = buffer;
        renderer.text_vertex_allocation = Some(allocation);
        renderer.text_vertex_buffer_capacity = initial_text_capacity;


        // --- Allocate Glyph Atlas Descriptor Set ---
        // This set points to the atlas texture/sampler. It's allocated once here.
        // We need the GlyphAtlasResource to get the image view and sampler handles.
        // This requires access to the resource system, which isn't ideal in Renderer::new.
        // Alternative: Allocate it lazily in the first render call or pass GlyphAtlasResource here.
        // Let's allocate it here, assuming GlyphAtlasResource is available *before* RendererResource.
        // This implies create_glyph_atlas_system runs before create_renderer_system.
        // *** Correction: Renderer is created *after* Atlas. We need to get the resource from the world or pass it. ***
        // *** Simplification: Let's allocate it in the first `render` call if it's null. ***

        renderer // Return the modified renderer instance
    }

    // Accept &mut VulkanContext
    pub fn resize_renderer(&mut self, vulkan_context: &mut VulkanContext, width: u32, height: u32) {
        info!("[Renderer::resize_renderer] Called with width: {}, height: {}", width, height);
        // Prevent resizing to 0x0 which causes Vulkan errors
        if width == 0 || height == 0 {
            warn!("[Renderer::resize_renderer] Ignoring resize to zero dimensions.");
            return;
        }
        let logical_extent = vk::Extent2D { width, height };
        ResizeHandler::resize(
            vulkan_context,
            logical_extent,
            &mut self.buffer_manager.uniform_allocation, // Pass the allocation for the UBO update
        );
        // Note: Command buffers will be re-allocated inside record_command_buffers
        // if the framebuffer count changed, which it shouldn't during typical resize.
        // If swapchain image count changes, this needs more handling.
    }

    // Accept &mut VulkanContext and both shape and text render commands
    pub fn render(
        &mut self,
        platform: &mut VulkanContext,
        shape_commands: &[RenderCommandData],
        text_vertices: &[TextVertex], // Pass collected vertices directly
        glyph_atlas_resource: &GlyphAtlasResource, // Pass resource to access atlas handles
    ) {
        // --- Clone handles needed *after* the mutable borrow ---
        // Clone the ash::Device handle (cheap)
        let device = platform.device.as_ref().unwrap().clone();
        // Clone other handles (cheap) - Add check for queue
        let Some(queue) = platform.queue else {
            warn!("[Renderer::render] Queue is None, likely during cleanup. Skipping frame.");
            return;
        };
        // Check if swapchain resources are still valid, might be None during cleanup
        let Some(swapchain_loader) = platform.swapchain_loader.as_ref().cloned() else {
            warn!("[Renderer::render] Swapchain loader is None, likely during cleanup. Skipping frame.");
            return;
        };
        let Some(swapchain) = platform.swapchain else {
            warn!("[Renderer::render] Swapchain is None, likely during cleanup. Skipping frame.");
            return;
        };
        let Some(image_available_semaphore) = platform.image_available_semaphore else {
            warn!("[Renderer::render] Image available semaphore is None, likely during cleanup. Skipping frame.");
            return;
        };
        let Some(render_finished_semaphore) = platform.render_finished_semaphore else {
            warn!("[Renderer::render] Render finished semaphore is None, likely during cleanup. Skipping frame.");
            return;
        };
        let Some(fence) = platform.fence else {
            warn!("[Renderer::render] Fence is None, likely during cleanup. Skipping frame.");
            return;
        };
        // Prefix unused allocator
        let _allocator = platform.allocator.as_ref().unwrap(); // Needed for buffer updates

        // --- Wait for previous frame's fence ---
        // This ensures the GPU is finished with the command buffer and resources
        // from the *last* time this image index was used before we reset/reuse them.
        unsafe { device.wait_for_fences(&[fence], true, u64::MAX) }.unwrap();
        // Reset the fence *before* submitting new work that will signal it
        unsafe { device.reset_fences(&[fence]) }.unwrap();


        // --- Prepare Shape Buffers/Descriptors (Call BufferManager) ---
        let prepared_shape_draws = self.buffer_manager.prepare_frame_resources(
            platform, // Pass mutable platform here
            shape_commands, // Pass the shape commands
        );
        // Mutable borrow of platform for buffer manager ends here

        // --- Prepare Text Vertex Buffer ---
        let num_text_vertices = text_vertices.len() as u32;
        if num_text_vertices > 0 {
            // Check if buffer needs resizing
            if num_text_vertices > self.text_vertex_buffer_capacity {
                let new_capacity = (num_text_vertices * 2).max(self.text_vertex_buffer_capacity * 2); // Double capacity or more if needed
                info!("[Renderer::render] Resizing text vertex buffer from {} to {} vertices", self.text_vertex_buffer_capacity, new_capacity);
                let new_size = (std::mem::size_of::<TextVertex>() * new_capacity as usize) as vk::DeviceSize;
                let allocator = platform.allocator.as_ref().unwrap();
                // Destroy old buffer/allocation
                // Take the allocation out of the Option before destroying
                if let Some(mut alloc) = self.text_vertex_allocation.take() {
                    unsafe { allocator.destroy_buffer(self.text_vertex_buffer, &mut alloc); }
                }
                // Create new buffer/allocation
                let (new_buffer, new_alloc) = unsafe {
                    let buffer_info = vk::BufferCreateInfo {
                        s_type: vk::StructureType::BUFFER_CREATE_INFO,
                        size: new_size,
                        usage: vk::BufferUsageFlags::VERTEX_BUFFER,
                        sharing_mode: vk::SharingMode::EXCLUSIVE,
                        ..Default::default()
                    };
                    let allocation_info = vk_mem::AllocationCreateInfo {
                        flags: vk_mem::AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE | vk_mem::AllocationCreateFlags::MAPPED,
                        usage: vk_mem::MemoryUsage::AutoPreferDevice,
                        ..Default::default()
                    };
                    allocator.create_buffer(&buffer_info, &allocation_info)
                             .expect("Failed to resize text vertex buffer")
                };
                self.text_vertex_buffer = new_buffer;
                self.text_vertex_allocation = Some(new_alloc);
                self.text_vertex_buffer_capacity = new_capacity;
            }

            // Copy data to text vertex buffer, ensuring allocation exists
            if let Some(alloc) = &self.text_vertex_allocation {
                unsafe {
                    let allocator = platform.allocator.as_ref().unwrap();
                    let info = allocator.get_allocation_info(alloc); // Use the allocation from Option
                    if !info.mapped_data.is_null() {
                        let data_ptr = info.mapped_data.cast::<TextVertex>();
                        data_ptr.copy_from_nonoverlapping(text_vertices.as_ptr(), num_text_vertices as usize);
                        // Optional flush if not HOST_COHERENT (safer to include)
                        // allocator.flush_allocation(alloc, 0, vk::WHOLE_SIZE).expect("Failed to flush text vertex buffer");
                    } else {
                        error!("[Renderer::render] Text vertex buffer allocation not mapped during update!");
                    }
                }
            } else {
                 error!("[Renderer::render] Text vertex buffer allocation is None during update!");
            }
        }

        // --- Ensure Glyph Atlas Descriptor Set Exists ---
        if self.glyph_atlas_descriptor_set == vk::DescriptorSet::null() {
            info!("[Renderer::render] Allocating glyph atlas descriptor set.");
            let Ok(atlas_guard) = glyph_atlas_resource.0.lock() else {
                error!("[Renderer::render] Failed to lock GlyphAtlasResource to get handles for descriptor set allocation.");
                return; // Cannot proceed without atlas handles
            };
            let set_layouts = [self.text_descriptor_set_layout];
            let alloc_info = vk::DescriptorSetAllocateInfo {
                s_type: vk::StructureType::DESCRIPTOR_SET_ALLOCATE_INFO,
                descriptor_pool: self.descriptor_pool,
                descriptor_set_count: 1,
                p_set_layouts: set_layouts.as_ptr(),
                ..Default::default()
            };
            self.glyph_atlas_descriptor_set = unsafe {
                device.allocate_descriptor_sets(&alloc_info)
                    .expect("Failed to allocate glyph atlas descriptor set")
                    .remove(0)
            };

            // Update the descriptor set to point to the atlas image view and sampler
            let image_info = vk::DescriptorImageInfo {
                sampler: atlas_guard.sampler,
                image_view: atlas_guard.image_view,
                image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL, // Layout should be this after upload
            };
            let write_set = vk::WriteDescriptorSet {
                s_type: vk::StructureType::WRITE_DESCRIPTOR_SET,
                dst_set: self.glyph_atlas_descriptor_set,
                dst_binding: 0, // Binding 0 in text_descriptor_set_layout
                dst_array_element: 0,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                p_image_info: &image_info,
                ..Default::default()
            };
            unsafe { device.update_descriptor_sets(&[write_set], &[]); }
            info!("[Renderer::render] Glyph atlas descriptor set allocated and updated.");
        }


        // --- Acquire Swapchain Image ---
        let acquire_result = unsafe {
            swapchain_loader.acquire_next_image(swapchain, u64::MAX, image_available_semaphore, vk::Fence::null())
        };

        let image_index = match acquire_result {
            Ok((index, suboptimal)) => {
                if suboptimal {
                    warn!("[Renderer::render] Swapchain suboptimal during acquire.");
                    // TODO: Trigger resize handling here? Or just continue?
                }
                index
            },
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                warn!("[Renderer::render] Swapchain out of date during acquire. Triggering resize.");
                // Trigger resize explicitly
                self.resize_renderer(platform, platform.current_swap_extent.width, platform.current_swap_extent.height);
                return; // Skip rest of the frame, resize will handle recreation
            }
            Err(e) => panic!("Failed to acquire swapchain image: {:?}", e),
        };
        // We still need mutable access to platform to update current_image
        // This is okay because the mutable borrow for buffer manager ended.
        platform.current_image = image_index as usize;


        // --- Re-Record Command Buffer for the acquired image index ---
        // This now happens *after* acquiring the image index and *after* waiting on the fence.
        // It also resets the command pool/buffer internally.
        let text_command = if num_text_vertices > 0 {
            Some(TextRenderCommandData {
                vertex_buffer_offset: 0, // Start from the beginning of the buffer for now
                vertex_count: num_text_vertices,
            })
        } else {
            None
        };

        record_command_buffers(
            platform, // Pass mutable platform here again
            &prepared_shape_draws, // Pass the prepared shape data
            // Pass text rendering info
            self.text_vertex_buffer,
            self.glyph_atlas_descriptor_set,
            text_command.as_ref(), // Pass Option<&TextRenderCommandData>
            platform.current_swap_extent,
        );
        // Mutable borrow for command buffer recording ends here.


        // Ensure command buffer exists for the acquired image index
        if platform.current_image >= platform.command_buffers.len() {
             error!(
                 "[Renderer::render] Image index {} out of bounds for command buffers (len {}). Skipping submit.",
                 platform.current_image,
                 platform.command_buffers.len()
             );
             // This might happen if resize occurred but command buffers weren't recreated yet.
             // The allocation logic in record_command_buffers should handle this now.
             return; // Avoid panic
        }

        // --- Submit Queue ---
        let submit_info = vk::SubmitInfo {
            s_type: vk::StructureType::SUBMIT_INFO,
            wait_semaphore_count: 1,
            p_wait_semaphores: &image_available_semaphore,
            p_wait_dst_stage_mask: &vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            command_buffer_count: 1,
            // Use the command buffer for the current image index, which was just recorded
            p_command_buffers: &platform.command_buffers[platform.current_image],
            signal_semaphore_count: 1,
            p_signal_semaphores: &render_finished_semaphore,
            ..Default::default()
        };
        // Use cloned device handle
        if let Err(e) = unsafe { device.queue_submit(queue, &[submit_info], fence) } {
             error!("[Renderer::render] Failed to submit queue: {:?}", e);
             // Don't panic here, let present handle potential OOD
             // return; // Optionally return early
        }


        // --- Present Queue ---
        let present_info = vk::PresentInfoKHR {
            s_type: vk::StructureType::PRESENT_INFO_KHR,
            wait_semaphore_count: 1,
            p_wait_semaphores: &render_finished_semaphore,
            swapchain_count: 1,
            p_swapchains: &swapchain,
            p_image_indices: &(platform.current_image as u32),
            ..Default::default()
        };
        // Use cloned swapchain_loader handle
        let present_result = unsafe { swapchain_loader.queue_present(queue, &present_info) };

        match present_result {
            Ok(suboptimal) => {
                if suboptimal {
                    warn!("[Renderer::render] Swapchain suboptimal during present.");
                    // Trigger resize explicitly
                    self.resize_renderer(platform, platform.current_swap_extent.width, platform.current_swap_extent.height);
                }
            }
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                warn!("[Renderer::render] Swapchain out of date during present. Triggering resize.");
                // Trigger resize explicitly
                self.resize_renderer(platform, platform.current_swap_extent.width, platform.current_swap_extent.height);
            }
            Err(e) => panic!("Failed to present swapchain image: {:?}", e),
        }
    }

    // Accept &mut self and &mut VulkanContext
    pub fn cleanup(&mut self, platform: &mut VulkanContext) { // Changed to &mut self
        info!("[Renderer::cleanup] Called (&mut self)");
        // Clone device handle early if needed, but cleanup methods might take platform directly
        let device = platform.device.as_ref().expect("Device not available for cleanup").clone();

        // Ensure GPU is idle before destroying anything
        unsafe { device.device_wait_idle().unwrap(); }
        info!("[Renderer::cleanup] Device idle.");

        // Call cleanup on BufferManager first (destroys buffers, pipelines, shaders)
        self.buffer_manager.cleanup(
            platform, // Pass &mut VulkanContext
        );
        info!("[Renderer::cleanup] BufferManager cleanup finished.");

        // Cleanup text vertex buffer
        if let Some(mut alloc) = self.text_vertex_allocation.take() { // Take from Option
            if self.text_vertex_buffer != vk::Buffer::null() {
                 unsafe {
                    let allocator = platform.allocator.as_ref().expect("Allocator missing for text buffer cleanup");
                    allocator.destroy_buffer(self.text_vertex_buffer, &mut alloc); // Pass the taken &mut alloc
                    info!("[Renderer::cleanup] Text vertex buffer destroyed.");
                 }
            }
        }

        // Cleanup layouts stored in Renderer/Platform
        unsafe {
            // Destroy layouts stored in VulkanContext
            if let Some(layout) = platform.shape_pipeline_layout.take() {
                 device.destroy_pipeline_layout(layout, None);
                 info!("[Renderer::cleanup] Shape pipeline layout destroyed");
            }
             if let Some(layout) = platform.text_pipeline_layout.take() {
                 device.destroy_pipeline_layout(layout, None);
                 info!("[Renderer::cleanup] Text pipeline layout destroyed");
            }
            // Use pool/set_layouts stored in self
            device.destroy_descriptor_pool(self.descriptor_pool, None);
            device.destroy_descriptor_set_layout(self.descriptor_set_layout, None); // Shape layout
            device.destroy_descriptor_set_layout(self.text_descriptor_set_layout, None); // Text layout
            info!("[Renderer::cleanup] Descriptor pool and set layouts destroyed");
        }

        // Cleanup swapchain resources (Framebuffers, Views, Swapchain, RenderPass)
        // Use the dedicated cleanup function
        crate::gui_framework::rendering::swapchain::cleanup_swapchain_resources(platform);
        info!("[Renderer::cleanup] Swapchain resources cleanup finished.");


        // Cleanup remaining resources (Sync objects, Command Pool)
        unsafe {
            if let Some(sema) = platform.image_available_semaphore.take() { device.destroy_semaphore(sema, None); }
            if let Some(sema) = platform.render_finished_semaphore.take() { device.destroy_semaphore(sema, None); }
            if let Some(fen) = platform.fence.take() { device.destroy_fence(fen, None); }
            info!("[Renderer::cleanup] Sync objects destroyed.");

            // Cleanup command pool *after* waiting for idle and *before* device destroy
            if let Some(pool) = platform.command_pool.take() {
                // Command buffers should be implicitly freed by pool destruction,
                // but explicit free doesn't hurt if needed. They are empty now anyway.
                if !platform.command_buffers.is_empty() {
                    // device.free_command_buffers(pool, &platform.command_buffers); // Optional explicit free
                    platform.command_buffers.clear(); // Clear the vec
                }
                device.destroy_command_pool(pool, None); // Now destroy the pool
                info!("[Renderer::cleanup] Command pool destroyed.");
            }
        }

        // Note: VulkanContext itself (device, instance, allocator) is cleaned up
        // by the main cleanup_system calling vulkan_setup::cleanup_vulkan

        info!("[Renderer::cleanup] Finished");
    }
}