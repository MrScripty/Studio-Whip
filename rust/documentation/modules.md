# Modules Documentation for `rusty_whip` (March 19, 2025)

## Project Overview: `rusty_whip`
### Purpose
`rusty_whip` is an advanced 2D & 3D content generation application with GPU-accelerated AI diffusion/inference, multimedia creation, and story-driven workflows, targeting P2P networking. Current focus: 2D GUI with Vulkan rendering, click-and-drag, instancing via an event bus, and logical grouping.
### Current State
- 2D GUI: Depth-sorted objects, orthographic projection, resizing, event-driven dragging/instancing, logical grouping.
- Features Implemented: Event bus, mouse picking/dragging via events, instancing via events (fixed capacity buffer), group management (logical-only), Vulkan resource management refactored (`PipelineManager`, `BufferManager`).
- Features Skipped: Instance buffer resizing, resize conflict handling, context switching, undo; batch group operations pending.
- Task 1 (Event Bus) is **Complete**.

## Module Structure
Studio_Whip/
    LICENSE
    README.md
    .gitignore
    Rust/
        src/
            gui_framework/
                context/
                    vulkan_context.rs
                    vulkan_setup.rs
                    mod.rs
                event_bus.rs # New
                interaction/
                    controller.rs
                    mod.rs
                rendering/
                    buffer_manager.rs
                    command_buffers.rs
                    pipeline_manager.rs
                    render_engine.rs
                    renderable.rs
                    resize_handler.rs
                    shader_utils.rs
                    swapchain.rs
                    mod.rs
                scene/
                    group.rs
                    scene.rs
                    mod.rs
                window/
                    window_handler.rs
                    mod.rs
                mod.rs
            lib.rs
            main.rs
        shaders/
            background.frag
            background.vert
            square.frag
            square.vert
            triangle.frag
            triangle.vert
        Documentation/
            # ... docs ...
        utilities/
            # ... utils ...
        Cargo.toml
        build.rs
        compile_shaders.ps1
        compile_shaders.sh


## Modules and Their Functions

### `src/lib.rs`
- **Purpose**: Defines the vertex type and re-exports `gui_framework` contents.
- **Key Structs**: `Vertex { position: [f32; 2] }`.
- **Notes**: Public exports include `Renderer`, `Scene`, `EventBus`, etc.

### `src/main.rs`
- **Purpose**: Entry point, initializes `EventBus`, Vulkan, `Scene` with test objects/instances, wraps state in `Arc<Mutex<>>`, runs `VulkanContextHandler`.
- **Key Methods**: `main() -> ()`.
- **Notes**: Initializes and passes `Arc<EventBus>`. Tests logical grouping.

### `src/gui_framework/mod.rs`
- **Purpose**: Re-exports submodules and key types (e.g., `Renderer`, `Scene`, `EventBus`).

### `src/gui_framework/context/mod.rs`
- **Purpose**: Re-exports Vulkan context modules.

### `src/gui_framework/context/vulkan_context.rs`
- **Purpose**: Holds core Vulkan resources and related state.
- **Key Structs**: `VulkanContext { window, entry, instance, surface_loader, surface, device, queue, queue_family_index: Option<u32>, allocator, swapchain_loader, swapchain, images, image_views, render_pass, framebuffers, command_pool, command_buffers, image_available_semaphore, render_finished_semaphore, fence, current_image }`.
- **Key Methods**: `new() -> Self`.

### `src/gui_framework/context/vulkan_setup.rs`
- **Purpose**: Sets up and tears down core Vulkan resources managed by `VulkanContext`.
- **Key Methods**:
  - `setup_vulkan(&mut VulkanContext, Arc<Window>) -> ()` - Initializes Vulkan instance, device, queue, allocator, surface. Stores queue family index.
  - `cleanup_vulkan(&mut VulkanContext) -> ()` - Destroys allocator, device, surface, instance.
- **Notes**: Used by `window_handler.rs`.

### `src/gui_framework/event_bus.rs`
- **Purpose**: Decouples components via a publish-subscribe event system.
- **Key Structs**: `EventBus { subscribers: Arc<Mutex<Vec<Arc<Mutex<dyn EventHandler>>>>> }`.
- **Key Enums**: `BusEvent { ObjectMoved(usize, [f32; 2], Option<usize>), InstanceAdded(usize, usize, [f32; 2]), ObjectPicked(usize, Option<usize>), RedrawRequested }`.
- **Key Traits**: `EventHandler: Send + Sync { handle(&mut self, event: &BusEvent), as_any(&self) -> &dyn Any }`.
- **Key Methods**: `new() -> Self`, `subscribe_handler<H: EventHandler + 'static>(&self, H)`, `subscribe_arc<T: EventHandler + 'static>(&self, Arc<Mutex<T>>)`, `publish(&self, BusEvent)`, `clear(&self)`.
- **Notes**: Thread-safe design using `Arc<Mutex<>>`. `clear()` used during shutdown.

### `src/gui_framework/interaction/mod.rs`
- **Purpose**: Re-exports interaction controller components.

### `src/gui_framework/interaction/controller.rs`
- **Purpose**: Handles mouse input and publishes interaction events via the `EventBus`.
- **Key Structs**: `MouseState { is_dragging, last_position, dragged_object }`, `CursorContext { Canvas, Other }`, `InteractionController { mouse_state, context }`.
- **Key Methods**:
  - `new() -> Self`.
  - `handle_event(&mut self, event: &Event<()>, scene: Option<&Scene>, _renderer: Option<&mut Renderer>, window: &Window, event_bus: &Arc<EventBus>) -> ()` - Processes `winit` events, calls `Scene::pick_object_at`, publishes `ObjectMoved`/`ObjectPicked` to `event_bus`.
- **Notes**: No longer directly modifies `Scene`. Depends on `Scene` for picking and `EventBus` for publishing.

### `src/gui_framework/rendering/mod.rs`
- **Purpose**: Re-exports rendering sub-modules and key types (`Renderer`, `Renderable`).

### `src/gui_framework/rendering/command_buffers.rs`
- **Purpose**: Records Vulkan command buffers for drawing `Renderable` objects, handling instancing correctly.
- **Key Methods**: `record_command_buffers(&mut VulkanContext, &[Renderable], vk::PipelineLayout, vk::DescriptorSet, vk::Extent2D) -> ()`.
- **Notes**: Uses `queue_family_index` from `VulkanContext`. Uses `renderable.instance_count + 1` for instanced draw calls. Called by `render_engine.rs` and `resize_handler.rs`.

### `src/gui_framework/rendering/renderable.rs`
- **Purpose**: Defines properties of a Vulkan-renderable object, including instance buffer state.
- **Key Structs**: `Renderable { vertex_buffer, vertex_allocation, vertex_shader, fragment_shader, pipeline, vertex_count, depth, ..., offset_uniform, offset_allocation, descriptor_set, instance_buffer: Option<vk::Buffer>, instance_allocation: Option<vk_mem::Allocation>, instance_count: u32, instance_buffer_capacity: u32 }`.

### `src/gui_framework/rendering/render_engine.rs`
- **Purpose**: Orchestrates Vulkan rendering, manages sub-component lifecycles (`PipelineManager`, `BufferManager`), handles `InstanceAdded` events to queue buffer updates, and manages swapchain presentation.
- **Key Structs**: `Renderer { pipeline_manager: PipelineManager, buffer_manager: BufferManager, pending_instance_updates: Arc<Mutex<Vec<(usize, usize, [f32; 2])>>> }`.
- **Key Methods**:
  - `new(&mut VulkanContext, vk::Extent2D, &Scene) -> Self` - Initializes swapchain, framebuffers, `PipelineManager`, `BufferManager`, sync objects. Updates global projection UBO.
  - `resize_renderer(&mut self, vulkan_context: &mut VulkanContext, scene: &mut Scene, width: u32, height: u32) -> ()` - Delegates to `ResizeHandler`.
  - `render(&mut self, platform: &mut VulkanContext, scene: &Scene) -> ()` - Processes `pending_instance_updates` queue (calling `BufferManager::update_instance_buffer`), updates offsets (calling `BufferManager::update_offset`/`update_instance_offset`), handles Vulkan frame submission/presentation.
  - `cleanup(self, platform: &mut VulkanContext) -> ()` - Calls `BufferManager::cleanup`, `PipelineManager::cleanup`, and cleans up swapchain-related resources (sync objects, command pool, framebuffers, etc.).
- **Event Handling**: `impl EventHandler for Renderer { handle(&mut self, event: &BusEvent), as_any(&self) -> &dyn Any }` - Handles `InstanceAdded` by pushing to `pending_instance_updates`.
- **Notes**: Relies heavily on delegation. Manages shared state via `Arc<Mutex<>>` in `WindowHandler`. Cleanup order is critical.

### `src/gui_framework/rendering/pipeline_manager.rs`
- **Purpose**: Manages Vulkan `PipelineLayout`, `DescriptorSetLayout`, `DescriptorPool`, and the global projection `DescriptorSet`.
- **Key Structs**: `PipelineManager { pipeline_layout, descriptor_set_layout, descriptor_pool, descriptor_set }`.
- **Key Methods**:
  - `new(&mut VulkanContext, &Scene) -> Self` - Creates layout, pool (sized generously), and allocates the global descriptor set.
  - `cleanup(self, device: &ash::Device) -> ()` - Destroys layout, pool, and set layout.
- **Notes**: Provides resources to `BufferManager`. Pool allows freeing individual sets.

### `src/gui_framework/rendering/buffer_manager.rs`
- **Purpose**: Manages Vulkan buffers (uniform, vertex, instance, offset), allocations, per-object pipelines, shaders, and descriptor sets. Handles instance buffer creation and updates.
- **Key Structs**: `BufferManager { uniform_buffer, uniform_allocation, renderables: Vec<Renderable> }`.
- **Key Methods**:
  - `new(&mut VulkanContext, &Scene, vk::PipelineLayout, vk::DescriptorSetLayout, vk::DescriptorPool) -> Self` - Creates buffers (including instance buffers with capacity), pipelines, shaders, allocates per-object descriptor sets using provided layout/pool.
  - `update_offset(&mut Vec<Renderable>, &ash::Device, &vk_mem::Allocator, usize, [f32; 2]) -> ()`.
  - `update_instance_offset(&mut Vec<Renderable>, &ash::Device, &vk_mem::Allocator, usize, usize, [f32; 2]) -> ()` - Updates existing instance offset.
  - `update_instance_buffer(&mut Vec<Renderable>, &ash::Device, &vk_mem::Allocator, usize, usize, [f32; 2]) -> ()` - Writes new instance data to buffer if capacity allows, increments `renderable.instance_count`.
  - `cleanup(mut self, device: &ash::Device, allocator: &vk_mem::Allocator, descriptor_pool: vk::DescriptorPool) -> ()` - Frees descriptor sets, destroys pipelines, shaders, buffers, allocations.
- **Notes**: Takes layout/pool from `PipelineManager`. Instance buffers created proactively for draggable objects with fixed capacity.

### `src/gui_framework/rendering/resize_handler.rs`
- **Purpose**: Handles window resizing, updating swapchain, framebuffers, projection matrix, and renderable vertices.
- **Key Structs**: `ResizeHandler` (stateless).
- **Key Methods**: `resize(&mut VulkanContext, &mut Scene, &mut Vec<Renderable>, vk::PipelineLayout, vk::DescriptorSet, u32, u32, &mut vk_mem::Allocation) -> ()`.
- **Notes**: Called by `Renderer::resize_renderer`. Updates `BufferManager`'s `uniform_allocation` and `renderables`.

### `src/gui_framework/rendering/shader_utils.rs`
- **Purpose**: Loads compiled SPIR-V shader modules from files.
- **Key Methods**: `load_shader(device: &ash::Device, filename: &str) -> vk::ShaderModule`.

### `src/gui_framework/rendering/swapchain.rs`
- **Purpose**: Manages Vulkan swapchain creation/recreation and associated image views and framebuffers.
- **Key Methods**:
  - `create_swapchain(&mut VulkanContext, vk::Extent2D) -> vk::SurfaceFormatKHR`.
  - `create_framebuffers(&mut VulkanContext, vk::Extent2D, vk::SurfaceFormatKHR) -> ()`.
- **Notes**: Called by `Renderer::new` and `ResizeHandler::resize`.

### `src/gui_framework/scene/mod.rs`
- **Purpose**: Re-exports scene management modules.

### `src/gui_framework/scene/scene.rs`
- **Purpose**: Manages application state: renderable objects (`ElementPool`), logical groups (`GroupManager`), window dimensions. Publishes `InstanceAdded` events. Handles `ObjectMoved` events via `SceneEventHandler`.
- **Key Structs**: `RenderObject { vertices, ..., offset, is_draggable, instances: Vec<InstanceData> }`, `InstanceData { offset: [f32; 2] }`, `ElementPool { elements, free_indices }`, `Scene { pool, groups, width, height, event_bus: Arc<EventBus> }`.
- **Key Traits**: `HitTestable { contains(...) -> bool }`.
- **Key Methods**:
  - `new(event_bus: Arc<EventBus>) -> Self`.
  - `groups(&mut self) -> &mut GroupManager`.
  - `add_object(&mut self, RenderObject) -> usize`.
  - `add_instance(&mut self, object_id: usize, offset: [f32; 2]) -> usize` - Pushes instance data, publishes `InstanceAdded` event.
  - `pick_object_at(&self, f32, f32) -> Option<(usize, Option<usize>)>`.
  - `translate_object(&mut self, index: usize, dx: f32, dy: f32, instance_id: Option<usize>) -> ()` - Updates internal offset state (called by `SceneEventHandler`).
  - `update_dimensions(&mut self, width: u32, height: u32) -> ()`.
- **Notes**: Relies on `EventBus` for decoupling. `SceneEventHandler` acts on its behalf for `ObjectMoved`.

### `src/gui_framework/scene/group.rs`
- **Purpose**: Manages named, logical groups of object pool indices for organization.
- **Key Structs**: `Group { name, object_ids }`, `GroupManager { groups }`, `GroupEditor<'a> { group }`, `GroupError { DuplicateName, GroupNotFound }`.
- **Key Methods**: `GroupManager::new() -> Self`, `add_group(&mut self, &str) -> Result<(), GroupError>`, `delete_group(&mut self, &str) -> Result<(), GroupError>`, `group<'a>(&'a mut self, &str) -> Result<GroupEditor<'a>, GroupError>`, `get_groups_with_object(&self, usize) -> Vec<&str>`, `GroupEditor::add_object(&mut self, usize)`, `remove_object(&mut self, usize)`, `list_objects(&self) -> &[usize]`.
- **Notes**: Purely logical organization; no direct rendering impact. Batch operations pending (Task 3).

### `src/gui_framework/window/mod.rs`
- **Purpose**: Re-exports window handling components.

### `src/gui_framework/window/window_handler.rs`
- **Purpose**: Drives the `winit` event loop, manages application lifecycle, orchestrates Vulkan setup/cleanup, manages shared state (`Scene`, `Renderer`) via `Arc<Mutex<>>`, dispatches events, subscribes handlers to `EventBus`, and handles shutdown cleanup order.
- **Key Structs**: `SceneEventHandler { scene_ref: Arc<Mutex<Scene>> }`, `VulkanContextHandler { vulkan_context, scene: Arc<Mutex<Scene>>, renderer: Option<Arc<Mutex<Renderer>>>, resizing, controller, event_bus: Arc<EventBus> }`.
- **Key Methods**: `VulkanContextHandler::new(...) -> Self`, `impl ApplicationHandler for VulkanContextHandler { resumed(...), window_event(...) }`.
- **Event Handling**: `SceneEventHandler` implements `EventHandler` for `ObjectMoved`. `Renderer` is subscribed via `subscribe_arc`.
- **Notes**: `resumed` sets up Vulkan/Renderer and subscribes handlers. `window_event` handles `RedrawRequested`, `Resized`, `CloseRequested`, and dispatches input to `InteractionController`. `CloseRequested` ensures `EventBus::clear()` and `Renderer::cleanup` are called before `cleanup_vulkan`.

## Shaders
- **Location**: `shaders/`
- **Files**: `background.vert.spv`, `background.frag.spv`, `triangle.vert.spv`, `triangle.frag.spv`, `square.vert.spv`, `square.frag.spv`.
- **Roles**: Support orthographic projection (UBO binding 0), object offset (UBO binding 1), and instance offset (vertex attribute binding 1).

## Dependencies
- `ash = "0.38"`, `vk-mem = "0.4"`, `winit = "0.30.9"`, `ash-window = "0.13"`, `glam = "0.30"`, `raw-window-handle = "0.6"`.