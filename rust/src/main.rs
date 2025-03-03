use ash::vk;
use ash::{Entry, Instance};
use ash::khr::surface;
use ash::khr::swapchain;
use ash_window;
use std::ffi::CStr;
use std::marker::PhantomData;
use std::sync::Arc;
use vk_mem::{Alloc, Allocator};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

#[repr(C)]
struct Vertex {
    position: [f32; 2],
}

struct App {
    window: Option<Arc<Window>>,
    entry: Option<Entry>,
    instance: Option<Instance>,
    surface_loader: Option<surface::Instance>,
    surface: Option<vk::SurfaceKHR>,
    device: Option<ash::Device>,
    queue: Option<vk::Queue>,
    allocator: Option<Arc<Allocator>>,
    swapchain_loader: Option<swapchain::Device>,
    swapchain: Option<vk::SwapchainKHR>,
    images: Vec<vk::Image>,
    image_views: Vec<vk::ImageView>,
    vertex_buffer: Option<vk::Buffer>,
    vertex_allocation: Option<vk_mem::Allocation>,
    render_pass: Option<vk::RenderPass>,
    framebuffers: Vec<vk::Framebuffer>,
    vertex_shader: Option<vk::ShaderModule>,
    fragment_shader: Option<vk::ShaderModule>,
    pipeline_layout: Option<vk::PipelineLayout>,
    pipeline: Option<vk::Pipeline>,
    command_pool: Option<vk::CommandPool>,
    command_buffers: Vec<vk::CommandBuffer>,
    image_available_semaphore: Option<vk::Semaphore>,
    render_finished_semaphore: Option<vk::Semaphore>,
    fence: Option<vk::Fence>,
    current_image: usize,
}

impl App {
    fn new() -> Self {
        Self {
            window: None,
            entry: None,
            instance: None,
            surface_loader: None,
            surface: None,
            device: None,
            queue: None,
            allocator: None,
            swapchain_loader: None,
            swapchain: None,
            images: Vec::new(),
            image_views: Vec::new(),
            vertex_buffer: None,
            vertex_allocation: None,
            render_pass: None,
            framebuffers: Vec::new(),
            vertex_shader: None,
            fragment_shader: None,
            pipeline_layout: None,
            pipeline: None,
            command_pool: None,
            command_buffers: Vec::new(),
            image_available_semaphore: None,
            render_finished_semaphore: None,
            fence: None,
            current_image: 0,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window = Arc::new(event_loop.create_window(Window::default_attributes()).unwrap());
            self.window = Some(window.clone());
            let window_inner_size = window.inner_size();
            let extent = vk::Extent2D {
                width: window_inner_size.width,
                height: window_inner_size.height,
            };

            let entry = unsafe { Entry::load() }.unwrap();
            self.entry = Some(entry.clone());

            let surface_extensions = ash_window::enumerate_required_extensions(
                window.display_handle().unwrap().as_raw(),
            )
            .unwrap();
            let instance_desc = vk::InstanceCreateInfo::default()
                .enabled_extension_names(surface_extensions);
            let instance = unsafe { entry.create_instance(&instance_desc, None) }.unwrap();
            self.instance = Some(instance.clone());

            let surface_loader = surface::Instance::new(&entry, &instance);
            self.surface_loader = Some(surface_loader.clone());

            let surface = unsafe {
                ash_window::create_surface(
                    &entry,
                    &instance,
                    window.display_handle().unwrap().as_raw(),
                    window.window_handle().unwrap().as_raw(),
                    None,
                )
            }
            .unwrap();
            self.surface = Some(surface);

            let (physical_device, queue_family_index) = unsafe {
                instance.enumerate_physical_devices().unwrap()
            }
            .into_iter()
            .find_map(|pd| {
                let props = unsafe { instance.get_physical_device_queue_family_properties(pd) };
                props.iter().position(|qf| {
                    qf.queue_flags.contains(vk::QueueFlags::GRAPHICS)
                        && unsafe {
                            surface_loader
                                .get_physical_device_surface_support(pd, 0, surface)
                                .unwrap_or(false)
                        }
                })
                .map(|index| (pd, index as u32))
            })
            .unwrap();

            println!(
                "Selected GPU: {}",
                unsafe {
                    CStr::from_ptr(
                        instance
                            .get_physical_device_properties(physical_device)
                            .device_name
                            .as_ptr(),
                    )
                }
                .to_str()
                .unwrap()
            );

            let (device, queue) = {
                let queue_create_info = vk::DeviceQueueCreateInfo {
                    s_type: vk::StructureType::DEVICE_QUEUE_CREATE_INFO,
                    p_next: std::ptr::null(),
                    flags: vk::DeviceQueueCreateFlags::empty(),
                    queue_family_index,
                    queue_count: 1,
                    p_queue_priorities: [1.0].as_ptr(),
                    _marker: PhantomData,
                };
                let device_extensions = [swapchain::NAME.as_ptr()];
                let device_create_info = vk::DeviceCreateInfo {
                    s_type: vk::StructureType::DEVICE_CREATE_INFO,
                    p_next: std::ptr::null(),
                    flags: vk::DeviceCreateFlags::empty(),
                    queue_create_info_count: 1,
                    p_queue_create_infos: &queue_create_info,
                    enabled_extension_count: 1,
                    pp_enabled_extension_names: device_extensions.as_ptr(),
                    ..Default::default()
                };
                let device = unsafe {
                    instance.create_device(physical_device, &device_create_info, None)
                }
                .unwrap();
                let queue = unsafe { device.get_device_queue(queue_family_index, 0) };
                (device, queue)
            };
            self.device = Some(device.clone());
            self.queue = Some(queue);

            let allocator = Arc::new(unsafe {
                Allocator::new(vk_mem::AllocatorCreateInfo::new(
                    &instance,
                    &device,
                    physical_device,
                ))
            }
            .unwrap());
            self.allocator = Some(allocator.clone());

            let swapchain_loader = swapchain::Device::new(&instance, &device);
            self.swapchain_loader = Some(swapchain_loader.clone());

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
                let swapchain = unsafe {
                    swapchain_loader.create_swapchain(&swapchain_create_info, None)
                }
                .unwrap();
                let images = unsafe { swapchain_loader.get_swapchain_images(swapchain).unwrap() };
                (swapchain, images)
            };
            self.swapchain = Some(swapchain);
            self.images = images;

            self.image_views = self
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
                Vertex {
                    position: [-0.5, -0.5],
                },
                Vertex {
                    position: [0.0, 0.5],
                },
                Vertex {
                    position: [0.5, -0.25],
                },
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
                    flags: vk_mem::AllocationCreateFlags::MAPPED
                        | vk_mem::AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE,
                    ..Default::default()
                };
                let (buffer, mut allocation) = unsafe {
                    allocator.create_buffer(&buffer_info, &allocation_info)
                }
                .unwrap();
                let data_ptr = unsafe { allocator.map_memory(&mut allocation) }
                    .unwrap()
                    .cast::<Vertex>();
                unsafe { data_ptr.copy_from_nonoverlapping(vertices.as_ptr(), vertices.len()) };
                unsafe { allocator.unmap_memory(&mut allocation) };
                (buffer, allocation)
            };
            self.vertex_buffer = Some(vertex_buffer);
            self.vertex_allocation = Some(vertex_allocation);

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
            self.render_pass = Some(render_pass);

            self.framebuffers = self
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
            self.vertex_shader = Some(vertex_shader);

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
            self.fragment_shader = Some(fragment_shader);

            let pipeline_layout = unsafe {
                device.create_pipeline_layout(&vk::PipelineLayoutCreateInfo::default(), None)
            }
            .unwrap();
            self.pipeline_layout = Some(pipeline_layout);

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
                    flags: vk::PipelineInputAssemblyStateCreateFlags::empty(), // Fixed here
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
            self.pipeline = Some(pipeline);

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
            self.command_pool = Some(command_pool);

            self.command_buffers = {
                let alloc_info = vk::CommandBufferAllocateInfo {
                    s_type: vk::StructureType::COMMAND_BUFFER_ALLOCATE_INFO,
                    p_next: std::ptr::null(),
                    command_pool: self.command_pool.unwrap(),
                    level: vk::CommandBufferLevel::PRIMARY,
                    command_buffer_count: self.framebuffers.len() as u32,
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
                color: vk::ClearColorValue {
                    float32: [0.0, 0.0, 1.0, 1.0], // Blue background
                },
            }];
            for (&command_buffer, &framebuffer) in self.command_buffers.iter().zip(self.framebuffers.iter()) {
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
                    device.cmd_begin_render_pass(
                        command_buffer,
                        &render_pass_begin_info,
                        vk::SubpassContents::INLINE,
                    );
                    device.cmd_bind_pipeline(command_buffer, vk::PipelineBindPoint::GRAPHICS, pipeline);
                    device.cmd_bind_vertex_buffers(command_buffer, 0, &[vertex_buffer], &[0]);
                    device.cmd_draw(command_buffer, 3, 1, 0, 0);
                    device.cmd_end_render_pass(command_buffer);
                    device.end_command_buffer(command_buffer).unwrap();
                }
            }

            self.image_available_semaphore = Some(unsafe {
                device.create_semaphore(&vk::SemaphoreCreateInfo::default(), None)
            }
            .unwrap());
            self.render_finished_semaphore = Some(unsafe {
                device.create_semaphore(&vk::SemaphoreCreateInfo::default(), None)
            }
            .unwrap());
            self.fence = Some(unsafe {
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
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                println!("The close button was pressed; stopping");
                event_loop.exit();

                let device = self.device.take().unwrap();
                let swapchain_loader = self.swapchain_loader.take().unwrap();
                let surface_loader = self.surface_loader.take().unwrap();
                let allocator = self.allocator.take().unwrap();

                unsafe {
                    device.device_wait_idle().unwrap();

                    device.destroy_semaphore(self.image_available_semaphore.take().unwrap(), None);
                    device.destroy_semaphore(self.render_finished_semaphore.take().unwrap(), None);
                    device.destroy_fence(self.fence.take().unwrap(), None);

                    let command_pool = self.command_pool.take().unwrap();
                    device.free_command_buffers(command_pool, &self.command_buffers);
                    device.destroy_command_pool(command_pool, None);

                    device.destroy_pipeline(self.pipeline.take().unwrap(), None);
                    device.destroy_pipeline_layout(self.pipeline_layout.take().unwrap(), None);
                    device.destroy_shader_module(self.vertex_shader.take().unwrap(), None);
                    device.destroy_shader_module(self.fragment_shader.take().unwrap(), None);

                    for &framebuffer in &self.framebuffers {
                        device.destroy_framebuffer(framebuffer, None);
                    }
                    device.destroy_render_pass(self.render_pass.take().unwrap(), None);

                    allocator.destroy_buffer(
                        self.vertex_buffer.take().unwrap(),
                        &mut self.vertex_allocation.take().unwrap(),
                    );

                    swapchain_loader.destroy_swapchain(self.swapchain.take().unwrap(), None);
                    for &view in &self.image_views {
                        device.destroy_image_view(view, None);
                    }

                    drop(allocator); // Explicitly drop allocator before device

                    device.destroy_device(None);
                    surface_loader.destroy_surface(self.surface.take().unwrap(), None);
                    self.instance.take().unwrap().destroy_instance(None);
                }
            }
            WindowEvent::RedrawRequested => {
                if let (
                    Some(device),
                    Some(queue),
                    Some(swapchain_loader),
                    Some(swapchain),
                ) = (
                    self.device.as_ref(),
                    self.queue,
                    self.swapchain_loader.as_ref(),
                    self.swapchain,
                ) {
                    let image_available_semaphore = self.image_available_semaphore.unwrap();
                    let render_finished_semaphore = self.render_finished_semaphore.unwrap();
                    let fence = self.fence.unwrap();

                    unsafe { device.wait_for_fences(&[fence], true, u64::MAX) }.unwrap();
                    unsafe { device.reset_fences(&[fence]) }.unwrap();

                    let (image_index, _) = unsafe {
                        swapchain_loader.acquire_next_image(
                            swapchain,
                            u64::MAX,
                            image_available_semaphore,
                            vk::Fence::null(),
                        )
                    }
                    .unwrap();
                    self.current_image = image_index as usize;

                    let submit_info = vk::SubmitInfo {
                        s_type: vk::StructureType::SUBMIT_INFO,
                        p_next: std::ptr::null(),
                        wait_semaphore_count: 1,
                        p_wait_semaphores: &image_available_semaphore,
                        p_wait_dst_stage_mask: &vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT
                            as *const _,
                        command_buffer_count: 1,
                        p_command_buffers: &self.command_buffers[self.current_image],
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
                        p_image_indices: &(self.current_image as u32),
                        p_results: std::ptr::null_mut(),
                        _marker: PhantomData,
                    };
                    unsafe { swapchain_loader.queue_present(queue, &present_info) }.unwrap();

                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                }
            }
            _ => (),
        }
    }
}

#[allow(deprecated)]
fn main() {
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App::new();
    event_loop.run_app(&mut app).unwrap();
}