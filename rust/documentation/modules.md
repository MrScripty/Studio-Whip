# Modules Documentation for `rusty_whip` (March 27, 2025)

## Project Overview: `rusty_whip`
### Purpose
`rusty_whip` is an advanced 2D & 3D content generation application with GPU-accelerated AI diffusion/inference, multimedia creation, and story-driven workflows, targeting P2P networking. Current focus: 2D GUI with Vulkan rendering, click-and-drag, instancing, and grouping (under redesign).
### Current State
- 2D GUI: Depth-sorted objects with orthographic projection, resizing, dragging, instancing, and grouping.
- Features Implemented: Mouse detection, shader offsets, object picking/dragging, instancing, group management (rendering-affecting).
- Features Skipped: Resize conflict handling, context switching, undo; grouping redesign in progress.

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
                interaction/
                    mod.rs
                    controller.rs
                rendering/
                    command_buffers.rs
                    renderable.rs
                    render_engine.rs
                    shader_utils.rs
                    swapchain.rs
                scene/
                    scene.rs
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
            documentation_prompt.d
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
- **Purpose**: Defines public exports and `Vertex` type.
- **Key Structs**: `Vertex { position: [f32; 2] }`.
- **Contents**: `pub mod gui_framework;`, `pub use gui_framework::*;`.

### `src/main.rs`
- **Purpose**: Application entry point, sets up Vulkan, scene, and event loop.
- **Key Functions**: `main() -> ()`: Creates `EventLoop`, `VulkanContext`, `Scene`, adds objects with instances and one group, runs `VulkanContextHandler`.
- **Notes**: Initial window 600x300; objects: background, triangle (2 instances), square (1 instance), group (small square, vertical rectangle).

### `src/gui_framework/mod.rs`
- **Purpose**: Re-exports GUI framework submodules and types.
- **Contents**: `pub mod` for submodules, `pub use` for `Renderer`, `VulkanContext`, `Scene`, `RenderObject`, `InstanceData`, `InteractionController`.

### `src/gui_framework/context/mod.rs`
- **Purpose**: Re-exports Vulkan context modules.
- **Contents**: `pub mod vulkan_context;`, `pub mod vulkan_setup;`.

### `src/gui_framework/context/vulkan_context.rs`
- **Purpose**: Manages Vulkan resources.
- **Key Struct**: `VulkanContext`
  - Fields: `window`, `entry`, `instance`, `surface_loader`, `surface`, `device`, `queue`, `allocator`, `swapchain_loader`, `swapchain`, `images`, `image_views`, `vertex_buffer`, `vertex_allocation`, `render_pass`, `framebuffers`, `vertex_shader`, `fragment_shader`, `pipeline_layout`, `pipeline`, `command_pool`, `command_buffers`, `image_available_semaphore`, `render_finished_semaphore`, `fence`, `current_image`.
  - Methods: `new() -> Self`: Initializes empty context.
- **Notes**: Core Vulkan setup for rendering.

### `src/gui_framework/context/vulkan_setup.rs`
- **Purpose**: Initializes and cleans up Vulkan resources.
- **Key Functions**:
  - `setup_vulkan(app: &mut VulkanContext, window: Arc<Window>) -> ()`: Sets up Vulkan instance, device, etc.
  - `cleanup_vulkan(app: &mut VulkanContext) -> ()`: Frees resources.
- **Notes**: Called by `window_handler.rs`; selects GPU with graphics and surface support.

### `src/gui_framework/interaction/mod.rs`
- **Purpose**: Re-exports interaction modules.
- **Contents**: `pub mod controller;`, `pub use controller::InteractionController;`.

### `src/gui_framework/interaction/controller.rs`
- **Purpose**: Handles mouse input for dragging objects and instances.
- **Key Structs**:
  - `MouseState`: `is_dragging: bool`, `last_position: Option<[f32; 2]>`, `dragged_object: Option<(usize, Option<usize>)>`.
  - `CursorContext`: Enum (`Canvas`, `Other`).
  - `InteractionController`: `mouse_state: MouseState`, `context: CursorContext`.
- **Key Methods**:
  - `new() -> Self`: Initializes with `Canvas` context.
  - `handle_event(&mut self, event: &Event<()>, scene: Option<&mut Scene>, renderer: Option<&mut Renderer>, window: &Window) -> ()`: Processes mouse events, updates `Scene`.
- **Notes**: Supports instance dragging; does not yet handle keyboard events; used by `window_handler.rs`.

### `src/gui_framework/rendering/mod.rs`
- **Purpose**: Re-exports rendering modules.
- **Contents**: `pub mod` for submodules, `pub use` for `Renderer`, `Renderable`.

### `src/gui_framework/rendering/command_buffers.rs`
- **Purpose**: Records Vulkan command buffers for rendering with instancing.
- **Key Functions**: `record_command_buffers(platform: &mut VulkanContext, renderables: &[Renderable], pipeline_layout: vk::PipelineLayout, _projection_descriptor_set: vk::DescriptorSet, extent: vk::Extent2D) -> ()`: Sets up draw commands.
- **Notes**: Uses `vkCmdDraw` with instance counts; recreates command pool each call; supports instancing via `instance_buffer`.

### `src/gui_framework/rendering/renderable.rs`
- **Purpose**: Defines Vulkan renderable object properties with instancing.
- **Key Struct**: `Renderable`
  - Fields: `vertex_buffer`, `vertex_allocation`, `vertex_shader`, `fragment_shader`, `pipeline`, `vertex_count`, `depth`, `on_window_resize_scale`, `on_window_resize_move`, `original_positions`, `fixed_size`, `center_ratio`, `offset_uniform`, `offset_allocation`, `descriptor_set`, `instance_buffer`, `instance_allocation`, `instance_count`.

### `src/gui_framework/rendering/render_engine.rs`
- **Purpose**: Manages Vulkan rendering pipeline with instancing.
- **Key Struct**: `Renderer`
  - Fields: `vulkan_renderables`, `pipeline_layout`, `uniform_buffer`, `uniform_allocation`, `descriptor_set_layout`, `descriptor_pool`, `descriptor_set`.
- **Key Methods**:
  - `new(platform: &mut VulkanContext, extent: vk::Extent2D, scene: &Scene) -> Self`: Initializes renderer with instancing.
  - `update_offset(&mut self, device: &Device, allocator: &Allocator, index: usize, offset: [f32; 2]) -> ()`: Updates object offset.
  - `update_instance_offset(&mut self, device: &Device, allocator: &Allocator, object_index: usize, instance_id: usize, offset: [f32; 2]) -> ()`: Updates instance offset.
  - `resize_renderer(&mut self, vulkan_context: &mut VulkanContext, scene: &mut Scene, width: u32, height: u32) -> ()`: Handles resizing with vertex sync.
  - `render(&mut self, platform: &mut VulkanContext, scene: &Scene) -> ()`: Renders frame with offset syncing.
  - `cleanup(self, platform: &mut VulkanContext) -> ()`: Frees resources.
- **Notes**: Syncs instance offsets; instancing implemented; non-instanced group rendering issues noted.

### `src/gui_framework/rendering/shader_utils.rs`
- **Purpose**: Loads shader modules.
- **Key Functions**: `load_shader(device: &Device, filename: &str) -> vk::ShaderModule`: Loads `.spv` files from `shaders/`.

### `src/gui_framework/rendering/swapchain.rs`
- **Purpose**: Manages Vulkan swapchain and framebuffers.
- **Key Functions**:
  - `create_swapchain(platform: &mut VulkanContext, extent: vk::Extent2D) -> vk::SurfaceFormatKHR`: Creates swapchain.
  - `create_framebuffers(platform: &mut VulkanContext, extent: vk::Extent2D, surface_format: vk::SurfaceFormatKHR) -> ()`: Sets up framebuffers.

### `src/gui_framework/scene/mod.rs`
- **Purpose**: Re-exports scene module.
- **Contents**: `pub mod scene;`.

### `src/gui_framework/scene/scene.rs`
- **Purpose**: Manages scene and renderable objects with pooling, instancing, and grouping (rendering-affecting, under redesign).
- **Key Structs**:
  - `RenderObject`: `vertices`, `vertex_shader_filename`, `fragment_shader_filename`, `depth`, `on_window_resize_scale`, `on_window_resize_move`, `offset`, `is_draggable`, `instances`.
  - `InstanceData`: `offset: [f32; 2]`.
  - `ElementPool`: `elements`, `free_indices`.
  - `Group`: `element_ids`, `is_draggable`.
  - `Scene`: `pool`, `groups`, `width`, `height`.
- **Key Trait**: `HitTestable`
  - `contains(&self, x: f32, y: f32, window_height: f32, offset: [f32; 2]) -> bool`: Checks object bounds with offset.
- **Key Methods**:
  - `new() -> Self`: Initializes 600x300 scene with `ElementPool`.
  - `add_object(&mut self, object: RenderObject) -> usize`: Adds object to pool.
  - `add_objects(&mut self, templates: Vec<RenderObject>) -> Vec<usize>`: Adds multiple objects.
  - `update_element(&mut self, element_id: usize, new_offset: [f32; 2]) -> ()`: Updates offset.
  - `add_group(&mut self, elements: Vec<RenderObject>, is_draggable: bool) -> usize`: Creates group with pool indices.
  - `add_to_group(&mut self, group_id: usize, elements: Vec<RenderObject>) -> ()`: Extends group.
  - `add_instance(&mut self, object_id: usize, offset: [f32; 2]) -> usize`: Adds instance.
  - `pick_object_at(&self, x: f32, y: f32) -> Option<(usize, Option<usize>)>`: Picks object/instance or group.
  - `translate_object(&mut self, index: usize, dx: f32, dy: f32, instance_id: Option<usize>) -> ()`: Updates offset (group or single).
  - `update_dimensions(&mut self, width: u32, height: u32) -> ()`: Updates size.
- **Notes**: Grouping affects rendering (hitbox/dragging issues); redesign planned for logical-only organization.

### `src/gui_framework/window/mod.rs`
- **Purpose**: Re-exports window module.
- **Contents**: `pub mod window_handler;`.

### `src/gui_framework/window/window_handler.rs`
- **Purpose**: Handles window events and rendering loop.
- **Key Struct**: `VulkanContextHandler`
  - Fields: `vulkan_context`, `scene`, `renderer`, `resizing`, `controller`.
- **Key Methods**:
  - `new(platform: VulkanContext, scene: Scene) -> Self`: Initializes handler.
  - `resumed(&mut self, event_loop: &ActiveEventLoop) -> ()`: Sets up window and renderer.
  - `window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) -> ()`: Handles events.
- **Notes**: Implements `ApplicationHandler`; supports instance dragging via `controller`.

## Shaders
- **Location**: `shaders/`
- **Files**: `background.vert.spv`, `background.frag.spv`, `triangle.vert.spv`, `triangle.frag.spv`, `square.vert.spv`, `square.frag.spv`.
- **Notes**: Compiled from GLSL; uniforms at binding 0 (projection), 1 (offset); `square.vert`, `triangle.vert` support instancing.

## Dependencies
- `ash = "0.38"`, `vk-mem = "0.4"`, `winit = "0.30.9"`, `ash-window = "0.13"`, `glam = "0.30"`, `raw-window-handle = "0.6"`.