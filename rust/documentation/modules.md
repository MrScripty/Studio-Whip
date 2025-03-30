# Modules Documentation for `rusty_whip` (March 29, 2025)

## Project Overview: `rusty_whip`
### Purpose
`rusty_whip` is an advanced 2D & 3D content generation application with GPU-accelerated AI diffusion/inference, multimedia creation, and story-driven workflows, targeting P2P networking. Current focus: 2D GUI with Vulkan rendering, click-and-drag, instancing, and logical grouping.
### Current State
- 2D GUI: Depth-sorted objects with orthographic projection, resizing, dragging, instancing, and logical grouping.
- Features Implemented: Mouse detection, shader offsets, object picking/dragging, instancing, group management (logical-only).
- Features Skipped: Resize conflict handling, context switching, undo; batch group operations pending.

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
                interaction/
                    mod.rs
                    controller.rs
                rendering/
                    command_buffers.rs
                    renderable.rs
                    render_engine.rs
                    pipeline_manager.rs
                    buffer_manager.rs
                    resize_handler.rs  // New
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
            main.rs
            lib.rs
        shaders/
            background.vert
            background.frag
            triangle.vert
            triangle.frag
            square.vert
            square.frag
        Documentation/
            architecture.md
            tasks.md
            modules.md
            roadmap.md
            documentation_prompt.md
            tasks_instruction_prompt.md
            modules_plan.md
        utilities/
            llm_prompt_tool.sh
        Cargo.toml
        build.rs
        compile_shaders.ps1
        compile_shaders.sh


## Modules and Their Functions

### `src/lib.rs`
- **Purpose**: Defines the vertex type and re-exports `gui_framework` contents.
- **Key Structs**: `Vertex { position: [f32; 2] }`.
- **Notes**: Public exports include `Renderer`, `Scene`, etc.

### `src/main.rs`
- **Purpose**: Entry point, initializes Vulkan, scene with test objects/instances, and event loop.
- **Key Methods**: `main() -> ()` - Sets up and runs `VulkanContextHandler`.
- **Notes**: Tests logical grouping via `Scene::groups()`.

### `src/gui_framework/mod.rs`
- **Purpose**: Re-exports submodules and key types (e.g., `Renderer`, `Scene`).

### `src/gui_framework/context/mod.rs`
- **Purpose**: Re-exports Vulkan context modules.

### `src/gui_framework/context/vulkan_context.rs`
- **Purpose**: Holds Vulkan resources (e.g., device, swapchain).
- **Key Structs**: `VulkanContext { window, instance, swapchain, images, framebuffers, command_buffers, ... }`.
- **Key Methods**: `new() -> Self` - Creates an empty context.

### `src/gui_framework/context/vulkan_setup.rs`
- **Purpose**: Sets up and tears down Vulkan resources.
- **Key Methods**:
  - `setup_vulkan(&mut VulkanContext, Arc<Window>) -> ()` - Initializes Vulkan.
  - `cleanup_vulkan(&mut VulkanContext) -> ()` - Frees resources.
- **Notes**: Used by `window_handler.rs`.

### `src/gui_framework/interaction/mod.rs`
- **Purpose**: Re-exports `controller.rs` and `InteractionController`.

### `src/gui_framework/interaction/controller.rs`
- **Purpose**: Handles mouse input for dragging objects and instances.
- **Key Structs**:
  - `MouseState { is_dragging, last_position, dragged_object }`.
  - `CursorContext { Canvas, Other }`.
  - `InteractionController { mouse_state, context }`.
- **Key Methods**:
  - `new() -> Self` - Initializes with `Canvas` context.
  - `handle_event(&mut self, &Event<()>, Option<&mut Scene>, Option<&mut Renderer>, &Window) -> ()` - Updates `Scene` offsets, triggers redraws.
- **Notes**: Supports instance dragging; keyboard input (e.g., Ctrl+Z for undo) pending.

### `src/gui_framework/rendering/mod.rs`
- **Purpose**: Re-exports rendering modules and types (`Renderer`, `Renderable`).

### `src/gui_framework/rendering/command_buffers.rs`
- **Purpose**: Records Vulkan command buffers with instancing support.
- **Key Methods**: `record_command_buffers(&mut VulkanContext, &[Renderable], vk::PipelineLayout, vk::DescriptorSet, vk::Extent2D) -> ()` - Configures draw commands.
- **Notes**: Uses `instance_buffer` for instancing; called by `render_engine.rs`.

### `src/gui_framework/rendering/renderable.rs`
- **Purpose**: Defines properties of Vulkan-renderable objects with instancing.
- **Key Structs**: `Renderable { vertex_buffer, pipeline, depth, offset_uniform, instance_buffer, ... }`.

### `src/gui_framework/rendering/render_engine.rs`
- **Purpose**: Orchestrates Vulkan rendering by delegating to `pipeline_manager.rs`, `buffer_manager.rs`, and `resize_handler.rs`.
- **Key Structs**: `Renderer { vulkan_renderables, pipeline_layout, uniform_buffer, descriptor_set, ... }`.
- **Key Methods**:
  - `new(&mut VulkanContext, vk::Extent2D, &Scene) -> Self` - Initializes renderer with delegated setup.
  - `resize_renderer(&mut self, &mut VulkanContext, &mut Scene, u32, u32) -> ()` - Delegates to `ResizeHandler`.
  - `render(&mut self, &mut VulkanContext, &Scene) -> ()` - Renders frame with offset sync via `BufferManager`.
  - `cleanup(self, &mut VulkanContext) -> ()` - Delegates cleanup to submodules.
- **Notes**: Reduced scope; syncs offsets via `BufferManager`; depends on `pipeline_manager.rs`, `buffer_manager.rs`, `resize_handler.rs`.

### `src/gui_framework/rendering/pipeline_manager.rs`
- **Purpose**: Manages Vulkan pipeline creation and descriptor set setup.
- **Key Structs**: `PipelineManager { pipeline_layout, descriptor_set_layout, descriptor_pool, descriptor_set }`.
- **Key Methods**:
  - `new(&mut VulkanContext, &Scene) -> Self` - Creates pipeline layout and projection descriptor set.
  - `cleanup(self, &mut VulkanContext) -> ()` - Frees pipeline resources.
- **Notes**: Used by `render_engine.rs`; provides layout for `buffer_manager.rs`.

### `src/gui_framework/rendering/buffer_manager.rs`
- **Purpose**: Manages Vulkan buffers (uniform, vertex, instance) and renderable pipelines, including offset updates.
- **Key Structs**: `BufferManager { uniform_buffer, uniform_allocation, renderables, descriptor_set_layout, descriptor_pool }`.
- **Key Methods**:
  - `new(&mut VulkanContext, &Scene, vk::PipelineLayout) -> Self` - Sets up buffers and renderables.
  - `update_offset(&mut Vec<Renderable>, &Device, &Allocator, usize, [f32; 2]) -> ()` - Updates object offset buffer.
  - `update_instance_offset(&mut Vec<Renderable>, &Device, &Allocator, usize, usize, [f32; 2]) -> ()` - Updates instance offset buffer.
  - `cleanup(mut self, &mut VulkanContext) -> ()` - Frees buffer resources.
- **Notes**: Used by `render_engine.rs` for buffer management and offset syncing.

### `src/gui_framework/rendering/resize_handler.rs`
- **Purpose**: Handles window resizing, updating swapchain, framebuffers, and renderable vertices.
- **Key Structs**: `ResizeHandler` (stateless).
- **Key Methods**:
  - `resize(&mut VulkanContext, &mut Scene, &mut Vec<Renderable>, vk::PipelineLayout, vk::DescriptorSet, u32, u32, &mut Allocation) -> ()` - Adjusts rendering for new window size.
- **Notes**: Used by `render_engine.rs` for resize operations.

### `src/gui_framework/rendering/shader_utils.rs`
- **Purpose**: Loads compiled shader modules.
- **Key Methods**: `load_shader(&Device, &str) -> vk::ShaderModule` - Loads `.spv` files.

### `src/gui_framework/rendering/swapchain.rs`
- **Purpose**: Manages Vulkan swapchain and framebuffers.
- **Key Methods**:
  - `create_swapchain(&mut VulkanContext, vk::Extent2D) -> vk::SurfaceFormatKHR` - Sets up swapchain.
  - `create_framebuffers(&mut VulkanContext, vk::Extent2D, vk::SurfaceFormatKHR) -> ()` - Configures framebuffers.

### `src/gui_framework/scene/mod.rs`
- **Purpose**: Re-exports `scene.rs` and `group.rs`.

### `src/gui_framework/scene/scene.rs`
- **Purpose**: Manages renderable objects with pooling, instancing, and grouping.
- **Key Structs**:
  - `RenderObject { vertices, offset, instances, ... }`.
  - `InstanceData { offset: [f32; 2] }`.
  - `ElementPool { elements, free_indices }`.
  - `Scene { pool, groups, width, height }`.
- **Key Traits**: `HitTestable { contains(&self, x: f32, y: f32, window_height: f32, offset: [f32; 2]) -> bool }` - Enables picking.
- **Key Methods**:
  - `new() -> Self` - Initializes scene.
  - `groups(&mut self) -> &mut GroupManager` - Accesses group manager.
  - `add_object(&mut self, RenderObject) -> usize` - Adds object to pool.
  - `add_instance(&mut self, usize, [f32; 2]) -> usize` - Adds instance to object.
  - `pick_object_at(&self, f32, f32) -> Option<(usize, Option<usize>)>` - Detects clicked object/instance.
  - `translate_object(&mut self, usize, f32, f32, Option<usize>) -> ()` - Updates offset.
- **Notes**: Undo could store previous offsets here; integrates `GroupManager`.

### `src/gui_framework/scene/group.rs`
- **Purpose**: Manages logical groups of objects.
- **Key Structs**:
  - `Group { name, object_ids }`.
  - `GroupManager { groups }`.
  - `GroupEditor { group }`.
  - `GroupError { DuplicateName, GroupNotFound }`.
- **Key Methods**:
  - `GroupManager::new() -> Self` - Initializes manager.
  - `GroupManager::add_group(&mut self, &str) -> Result<(), GroupError>` - Creates group.
  - `GroupEditor::add_object(&mut self, usize) -> ()` - Adds object to group.
- **Notes**: Used by `Scene` for organization; batch operations pending.

### `src/gui_framework/window/mod.rs`
- **Purpose**: Re-exports `window_handler.rs`.

### `src/gui_framework/window/window_handler.rs`
- **Purpose**: Drives window events and rendering loop.
- **Key Structs**: `VulkanContextHandler { vulkan_context, scene, renderer, controller, ... }`.
- **Key Methods**:
  - `new(VulkanContext, Scene) -> Self` - Initializes handler.
  - `resumed(&mut self, &ActiveEventLoop) -> ()` - Sets up window and renderer.
  - `window_event(&mut self, &ActiveEventLoop, WindowId, WindowEvent) -> ()` - Handles events.

## Shaders
- **Location**: `shaders/`
- **Files**: `background.vert.spv`, `background.frag.spv`, `triangle.vert.spv`, `triangle.frag.spv`, `square.vert.spv`, `square.frag.spv`.
- **Roles**: Support orthographic projection (binding 0), offset uniforms (binding 1), and instancing (for `triangle.vert`, `square.vert`).

## Dependencies
- `ash = "0.38"`, `vk-mem = "0.4"`, `winit = "0.30.9"`, `ash-window = "0.13"`, `glam = "0.30"`, `raw-window-handle = "0.6"`.