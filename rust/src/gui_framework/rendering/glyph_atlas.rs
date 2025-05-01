use crate::gui_framework::context::vulkan_context::VulkanContext;
use ash::vk;
use bevy_ecs::system::Resource;
use bevy_log::{info, error};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use vk_mem::{Alloc, AllocationCreateInfo, Allocation};
use bevy_reflect::Reflect;
use cosmic_text::CacheKey;
use rectangle_pack::{RectToInsert, GroupedRectsToPlace, TargetBin, pack_rects, volume_heuristic, contains_smallest_box, RectanglePackError};
use std::collections::BTreeMap;
use swash::scale::ScaleContext;

// Represents the location and UV coordinates of a single glyph within the atlas
#[derive(Debug, Clone, Copy, Reflect)]
pub struct GlyphInfo {
    // Store pixel coordinates directly later when packing is implemented
    pub pixel_x: u32,
    pub pixel_y: u32,
    pub pixel_width: u32,
    pub pixel_height: u32,
    pub uv_min: [f32; 2], // Top-left UV coordinate
    pub uv_max: [f32; 2], // Bottom-right UV coordinate
    // Add placement info needed by cosmic-text if required later
}

// Manages the Vulkan texture atlas for glyphs
pub struct GlyphAtlas {
    pub image: vk::Image,
    pub allocation: Option<Allocation>,
    pub image_view: vk::ImageView,
    pub sampler: vk::Sampler,
    pub extent: vk::Extent2D,
    pub format: vk::Format,
    target_bins: BTreeMap<u32, TargetBin>,
    padding: u32, // Padding between glyphs
    glyph_cache: HashMap<CacheKey, GlyphInfo>, // Maps glyph key to its info
    scale_context: ScaleContext,
}

impl GlyphAtlas {
    pub fn new(vk_context: &mut VulkanContext, initial_extent: vk::Extent2D) -> Result<Self, String> {
        let device = vk_context.device.as_ref().ok_or("Device not available")?.clone();
        let allocator = vk_context.allocator.as_ref().ok_or("Allocator not available")?.clone();

        let format = vk::Format::R8_UNORM; // Grayscale, 8-bit unsigned normalized (common for alpha masks)

        // --- Create Vulkan Image ---
        let image_create_info = vk::ImageCreateInfo {
            s_type: vk::StructureType::IMAGE_CREATE_INFO,
            image_type: vk::ImageType::TYPE_2D,
            format,
            extent: vk::Extent3D {
                width: initial_extent.width,
                height: initial_extent.height,
                depth: 1,
            },
            mip_levels: 1,
            array_layers: 1,
            samples: vk::SampleCountFlags::TYPE_1,
            tiling: vk::ImageTiling::OPTIMAL,
            // TRANSFER_DST: To copy rasterized glyphs into it
            // SAMPLED: To be read by shaders
            usage: vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            initial_layout: vk::ImageLayout::UNDEFINED, // Will transition layout before use
            ..Default::default()
        };

        let allocation_create_info = vk_mem::AllocationCreateInfo {
            usage: vk_mem::MemoryUsage::AutoPreferDevice, // Let VMA decide, likely GPU local
            ..Default::default()
        };

        let (image, allocation) = unsafe {
            allocator.create_image(&image_create_info, &allocation_create_info)
        }.map_err(|e| format!("Failed to create glyph atlas image: {:?}", e))?;

        // --- Create Image View ---
        let image_view_create_info = vk::ImageViewCreateInfo {
            s_type: vk::StructureType::IMAGE_VIEW_CREATE_INFO,
            image,
            view_type: vk::ImageViewType::TYPE_2D,
            format,
            subresource_range: vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            },
            ..Default::default()
        };
        let image_view = unsafe {
            device.create_image_view(&image_view_create_info, None)
        }.map_err(|e| format!("Failed to create glyph atlas image view: {:?}", e))?;

        // --- Create Sampler ---
        let sampler_create_info = vk::SamplerCreateInfo {
            s_type: vk::StructureType::SAMPLER_CREATE_INFO,
            mag_filter: vk::Filter::LINEAR, // Linear filtering for smoother scaling
            min_filter: vk::Filter::LINEAR, // Nearerst helps remove some sampling artifacts around text 
            mipmap_mode: vk::SamplerMipmapMode::LINEAR,
            address_mode_u: vk::SamplerAddressMode::CLAMP_TO_EDGE,
            address_mode_v: vk::SamplerAddressMode::CLAMP_TO_EDGE,
            address_mode_w: vk::SamplerAddressMode::CLAMP_TO_EDGE,
            mip_lod_bias: 0.0,
            anisotropy_enable: vk::FALSE, // Basic sampler
            max_anisotropy: 1.0,
            compare_enable: vk::FALSE,
            compare_op: vk::CompareOp::ALWAYS,
            min_lod: 0.0,
            max_lod: 1.0, // Only one mip level
            border_color: vk::BorderColor::FLOAT_TRANSPARENT_BLACK,
            unnormalized_coordinates: vk::FALSE, // Use normalized UVs [0, 1]
            ..Default::default()
        };
        let sampler = unsafe {
            device.create_sampler(&sampler_create_info, None)
        }.map_err(|e| format!("Failed to create glyph atlas sampler: {:?}", e))?;

        // Initialize swash scale context
        let scale_context = ScaleContext::new();
        // Initialize the target bin map
        let mut target_bins = BTreeMap::new();
        target_bins.insert(0, TargetBin::new(initial_extent.width, initial_extent.height, 1)); // Bin ID 0
        
        let padding = 1; // Define padding value needed for struct init

        Ok(Self {
            image,
            allocation: Some(allocation),
            image_view,
            sampler,
            extent: initial_extent,
            format,
            target_bins,
            padding: padding,
            glyph_cache: HashMap::new(),
            scale_context,
        })
    }

    // Adds a glyph if not present, rasterizing and uploading it.
    // Takes the swash::Image which contains the key, data, and placement info.
    pub fn add_glyph(
        &mut self,
        vk_context: &VulkanContext, // Needed for upload
        cache_key: CacheKey, // Pass the key separately
        swash_image: &swash::scale::image::Image, // Pass the image data
    ) -> Result<&GlyphInfo, String> {

        // 1. Check cache using the passed-in key
        if self.glyph_cache.contains_key(&cache_key) {
            // Key exists, so we can safely get and return the immutable reference now.
            // The unwrap is safe because we just checked contains_key.
            let existing_info = self.glyph_cache.get(&cache_key).unwrap();
            return Ok(existing_info);
        }
        // --- Key not found, proceed with rasterization, packing, and insertion ---

        // --- 2. If not present: Get data from swash_image ---
        let placement = swash_image.placement;
        let width = placement.width;
        let height = placement.height;

        // Skip empty glyphs (like spaces) - they don't need packing or rendering
        if width == 0 || height == 0 {
            // Cache an empty GlyphInfo? Or handle this upstream?
            // For now, let's cache a zero-sized entry.
            let empty_info = GlyphInfo {
                pixel_x: 0, pixel_y: 0, pixel_width: 0, pixel_height: 0,
                uv_min: [0.0, 0.0], uv_max: [0.0, 0.0],
            };
            // Use entry API to insert and return reference
            let inserted_info = self.glyph_cache.entry(cache_key).or_insert(empty_info);
            // Return an immutable reference derived from the mutable one.
            return Ok(inserted_info);
        }

        let bitmap_data = &swash_image.data; // Get data from swash_image
        
        // --- 3. Attempt to Pack using rectangle-pack ---
        // RectToInsert takes dimensions (w, h, depth=1 for 2D)
        let rect_data = RectToInsert::new(width, height, 1);
        let mut rects_to_place: GroupedRectsToPlace<CacheKey, u32> = GroupedRectsToPlace::new();
        // Associate the CacheKey ID when pushing the rect data
        rects_to_place.push_rect(cache_key, Some(vec![0]), rect_data);

       // Call pack_rects - requires BTreeMap, heuristic, and custom data (&() is fine if unused)
       match pack_rects(&rects_to_place, &mut self.target_bins, &volume_heuristic, &contains_smallest_box) {
           Ok(pack_result) => {
               // packed_locations maps RectId (CacheKey) -> (BinId, PackedLocation)
               if let Some((_bin_id, packed_location)) = pack_result.packed_locations().get(&cache_key) {
                    // Successfully packed! Get coordinates directly from PackedLocation methods.
                    let packed_x_i32 = packed_location.x();
                    let packed_y_i32 = packed_location.y();
                    // Log the raw coordinates returned by the packer
                    let pixel_x = packed_location.x() as u32;
                    let pixel_y = packed_location.y() as u32;

                    // --- 4. Upload Bitmap ---
                    self.upload_glyph_bitmap(
                        vk_context,
                        pixel_x,
                        pixel_y,
                        width,
                        height,
                        &bitmap_data,
                    )?; // Propagate upload errors

                    // --- 5. Calculate UVs ---
                    let atlas_width = self.extent.width as f32;
                    let atlas_height = self.extent.height as f32;
                    let uv_min = [
                        pixel_x as f32 / atlas_width,
                        pixel_y as f32 / atlas_height,
                    ];
                    let uv_max = [
                        (pixel_x + width) as f32 / atlas_width,
                        (pixel_y + height) as f32 / atlas_height,
                    ];

                    // --- 6. Store GlyphInfo in Cache ---
                    let glyph_info = GlyphInfo {
                        pixel_x,
                        pixel_y,
                        pixel_width: width,
                        pixel_height: height,
                        uv_min,
                        uv_max,
                    };
                    info!("[GlyphAtlas] Caching GlyphInfo for key {:?}: px={}, py={}, w={}, h={}, uv_min={:?}, uv_max={:?}",
                           cache_key, pixel_x, pixel_y, width, height, uv_min, uv_max);

                    // Use entry API to insert and return reference
                    let inserted_info = self.glyph_cache.entry(cache_key).or_insert(glyph_info);
                    Ok(inserted_info)

                } else {
                    // This case *shouldn't* happen if pack_rects returned Ok, but handle defensively.
                    error!("[GlyphAtlas::add_glyph] Packing reported success, but location not found for key: {:?}", cache_key);
                    Err("Internal packing error: location not found after successful pack.".to_string())
                }
            }
            Err(RectanglePackError::NotEnoughBinSpace) => {
                // Atlas is full
                error!("[GlyphAtlas::add_glyph] Atlas full! Cannot pack glyph ({}x{}). Key: {:?}", width, height, cache_key);
                Err("Glyph atlas is full".to_string())
                // TODO: Implement atlas resizing or eviction strategy
            }
        }
    }

    // Helper function to upload glyph data using a staging buffer
    fn upload_glyph_bitmap(
        &self, // Needs self only for image handle and extent
        vk_context: &VulkanContext,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        bitmap_data: &[u8],
    ) -> Result<(), String> {
        let device = vk_context.device.as_ref().ok_or("Device not available for upload")?;
        let allocator = vk_context.allocator.as_ref().ok_or("Allocator not available for upload")?;
        let queue = vk_context.queue.ok_or("Queue not available for upload")?;
        let command_pool = vk_context.command_pool.ok_or("Command pool not available for upload")?;

        let buffer_size = (width * height) as vk::DeviceSize;
        if buffer_size == 0 { return Ok(()); } // Nothing to upload

        // --- Create Staging Buffer ---
        let staging_buffer_create_info = vk::BufferCreateInfo {
            s_type: vk::StructureType::BUFFER_CREATE_INFO,
            size: buffer_size,
            usage: vk::BufferUsageFlags::TRANSFER_SRC,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            ..Default::default()
        };
        let staging_allocation_create_info = AllocationCreateInfo {
            usage: vk_mem::MemoryUsage::AutoPreferHost, // Host visible for mapping
            flags: vk_mem::AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE | vk_mem::AllocationCreateFlags::MAPPED, // Create mapped
            ..Default::default()
        };

        let (staging_buffer, mut staging_allocation) = unsafe {
            allocator.create_buffer(&staging_buffer_create_info, &staging_allocation_create_info)
        }.map_err(|e| format!("Failed to create staging buffer: {:?}", e))?;

        // --- Copy data to staging buffer ---
        let allocation_info = allocator.get_allocation_info(&staging_allocation);
        let mapped_data_ptr = allocation_info.mapped_data;
        if !mapped_data_ptr.is_null() {
            unsafe {
                // Ensure data size matches buffer size
                if bitmap_data.len() as vk::DeviceSize != buffer_size {
                    // Cleanup before erroring
                    allocator.destroy_buffer(staging_buffer, &mut staging_allocation);
                    return Err(format!(
                        "Bitmap data size ({}) does not match staging buffer size ({})",
                        bitmap_data.len(), buffer_size
                    ));
                }
                // Cast the void pointer to a byte pointer before copying
                std::ptr::copy_nonoverlapping(bitmap_data.as_ptr(), mapped_data_ptr as *mut u8, bitmap_data.len());
            }
            // No need to flush if using HOST_COHERENT memory, but VMA might not guarantee that with AutoPreferHost.
            // Explicit flush is safer.
            allocator.flush_allocation(&staging_allocation, 0, vk::WHOLE_SIZE)
                .map_err(|e| format!("Failed to flush staging buffer allocation: {:?}", e))?;
        } else {
            // Cleanup before erroring
            unsafe { allocator.destroy_buffer(staging_buffer, &mut staging_allocation); }
            return Err("Failed to get mapped pointer for staging buffer".to_string());
        }

        // --- Record and Submit Command Buffer ---
        let cmd_buffer = unsafe {
            let alloc_info = vk::CommandBufferAllocateInfo {
                s_type: vk::StructureType::COMMAND_BUFFER_ALLOCATE_INFO,
                command_pool,
                level: vk::CommandBufferLevel::PRIMARY,
                command_buffer_count: 1,
                ..Default::default()
            };
            device.allocate_command_buffers(&alloc_info)
                .map_err(|e| format!("Failed to allocate command buffer: {:?}", e))?
                .remove(0) // Get the first allocated buffer
        };

        unsafe {
            let begin_info = vk::CommandBufferBeginInfo {
                s_type: vk::StructureType::COMMAND_BUFFER_BEGIN_INFO,
                flags: vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
                ..Default::default()
            };
            device.begin_command_buffer(cmd_buffer, &begin_info)
                .map_err(|e| format!("Failed to begin command buffer: {:?}", e))?;

            // 1. Transition Image Layout: Undefined -> TransferDstOptimal
            let barrier_undefined_to_dst = vk::ImageMemoryBarrier {
                s_type: vk::StructureType::IMAGE_MEMORY_BARRIER,
                src_access_mask: vk::AccessFlags::NONE, // No access needed before
                dst_access_mask: vk::AccessFlags::TRANSFER_WRITE, // Write access needed for copy
                old_layout: vk::ImageLayout::UNDEFINED,
                new_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                image: self.image,
                subresource_range: vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                ..Default::default()
            };
            device.cmd_pipeline_barrier(
                cmd_buffer,
                vk::PipelineStageFlags::TOP_OF_PIPE, // Before any operation
                vk::PipelineStageFlags::TRANSFER,    // Before the transfer stage
                vk::DependencyFlags::empty(),
                &[], // No memory barriers
                &[], // No buffer barriers
                &[barrier_undefined_to_dst],
            );

            // 2. Copy Buffer to Image
            let region = vk::BufferImageCopy {
                buffer_offset: 0,
                buffer_row_length: 0, // Tightly packed
                buffer_image_height: 0, // Tightly packed
                image_subresource: vk::ImageSubresourceLayers {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    mip_level: 0,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                image_offset: vk::Offset3D { x: x as i32, y: y as i32, z: 0 },
                image_extent: vk::Extent3D { width, height, depth: 1 },
            };
            device.cmd_copy_buffer_to_image(
                cmd_buffer,
                staging_buffer,
                self.image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL, // Layout must match barrier dest
                &[region],
            );

            // 3. Transition Image Layout: TransferDstOptimal -> ShaderReadOnlyOptimal
            let barrier_dst_to_shader = vk::ImageMemoryBarrier {
                s_type: vk::StructureType::IMAGE_MEMORY_BARRIER,
                src_access_mask: vk::AccessFlags::TRANSFER_WRITE, // Write access needed before
                dst_access_mask: vk::AccessFlags::SHADER_READ,    // Read access needed by shader
                old_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                new_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL, // Ready for sampling
                src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                image: self.image,
                subresource_range: vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                ..Default::default()
            };
            device.cmd_pipeline_barrier(
                cmd_buffer,
                vk::PipelineStageFlags::TRANSFER,          // After the transfer stage
                vk::PipelineStageFlags::FRAGMENT_SHADER, // Before the fragment shader reads it
                vk::DependencyFlags::empty(),
                &[], // No memory barriers
                &[], // No buffer barriers
                &[barrier_dst_to_shader],
            );

            device.end_command_buffer(cmd_buffer)
                .map_err(|e| format!("Failed to end command buffer: {:?}", e))?;

            // Submit command buffer
            let submit_info = vk::SubmitInfo {
                s_type: vk::StructureType::SUBMIT_INFO,
                command_buffer_count: 1,
                p_command_buffers: &cmd_buffer,
                ..Default::default()
            };
            // Use a fence to wait for completion
            let fence = device.create_fence(&vk::FenceCreateInfo::default(), None)
                .map_err(|e| format!("Failed to create fence: {:?}", e))?;

            device.queue_submit(queue, &[submit_info], fence)
                .map_err(|e| format!("Failed to submit command buffer: {:?}", e))?;

            // Wait for the fence (blocking)
            device.wait_for_fences(&[fence], true, u64::MAX)
                .map_err(|e| format!("Failed to wait for fence: {:?}", e))?;

            // Cleanup
            device.destroy_fence(fence, None);
            device.free_command_buffers(command_pool, &[cmd_buffer]);
            allocator.destroy_buffer(staging_buffer, &mut staging_allocation);
        }
        Ok(())
    }

    // Cleanup Vulkan resources
    pub fn cleanup(&mut self, vk_context: &VulkanContext) {
        let Some(device) = vk_context.device.as_ref() else {
            error!("[GlyphAtlas::cleanup] Device not available for cleanup.");
            return;
        };
        let Some(allocator) = vk_context.allocator.as_ref() else {
            error!("[GlyphAtlas::cleanup] Allocator not available for cleanup.");
            // Need allocator to destroy image+allocation safely
            return;
        };

        unsafe {
            device.destroy_sampler(self.sampler, None);
            device.destroy_image_view(self.image_view, None);
            // Take ownership of allocation before destroying image
            if let Some(mut alloc) = self.allocation.take() {
                allocator.destroy_image(self.image, &mut alloc); // Pass &mut alloc
            } else {
                error!("[GlyphAtlas::cleanup] Allocation was already taken or None!");
                // Maybe destroy image anyway? Or just log?
                // device.destroy_image(self.image, None); // Potentially unsafe without allocator
            }
        }
    }
}

// --- Bevy Resource ---

// Using Arc<Mutex> for interior mutability, similar to VulkanContextResource
#[derive(Resource, Clone)]
pub struct GlyphAtlasResource(pub Arc<Mutex<GlyphAtlas>>);