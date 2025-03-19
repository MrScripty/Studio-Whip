# Modules Documentation for `rusty_whip` (March 19, 2025)

## Project Overview: `rusty_whip`
### Purpose
`rusty_whip` is an advanced 2D & 3D content generation application for digital entertainment production, emphasizing GPU-accelerated AI diffusion/inference, multimedia creation, and story-driven workflows, with plans for P2P networking. Current focus: 2D GUI with Vulkan rendering and click-and-drag.
### Current State
- 2D GUI: Depth-sorted objects with orthographic projection, resizing, and dragging.
- Features Implemented: Mouse detection, shader-based positioning, object picking/dragging.
- Features Skipped: Resize conflict handling, context switching, undo.

## Module Structure
rusty_whip/
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
        background.vert.spv
        background.frag.spv
        triangle.vert.spv
        triangle.frag.spv
        square.vert.spv
        square.frag.spv
    Cargo.toml
    build.rs


## Modules and Their Functions

### `src/lib.rs`
- **Purpose**: Defines public exports and `Vertex` type.
- **Contents**: `pub struct Vertex { position: [f32; 2] }`, `pub mod gui_framework;`.

### `src/main.rs`
- **Purpose**: Application entry point, sets up Vulkan, scene, and event loop.
- **Key Functions**: `main() -> ()`: Creates `EventLoop`, `VulkanContext`, `Scene`, adds three `RenderObject`s, runs `VulkanContextHandler`.
- **Notes**: Initial window 600x300, objects: background (static), triangle/square (draggable).

### `src/gui_framework/mod.rs`
- **Purpose**: Re-exports GUI framework submodules and types.
- **Contents**: `pub mod` for submodules, `pub use` for `Renderer`, `VulkanContext`, etc.

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
- **Notes**: Called by `window_handler.rs`.

### `src/gui_framework/interaction/mod.rs`
- **Purpose**: Re-exports interaction modules.
- **Contents**: `pub mod controller;`, `pub use controller::InteractionController;`.

### `src/gui_framework/interaction/controller.rs`
- **Purpose**: Handles mouse input for dragging.
- **Key Structs**:
  - `MouseState`: `is_dragging: bool`, `last_position: Option<[f32; 2]>`, `dragged_object: Option<usize>`.
  - `CursorContext`: Enum (`Canvas`, `Other`).
  - `InteractionController`: `mouse_state: MouseState`, `context: CursorContext`.
- **Key Methods**:
  - `new() -> Self`: Initializes with `Canvas` context.
  - `handle_event(&mut self, event: &Event<()>, scene: Option<&mut Scene>, _renderer: Option<&mut Renderer>, window: &Window) -> ()`: Processes mouse events, updates `Scene`.
- **Notes**: Inverts Y-delta for Vulkan; depends on `scene.rs`.

### `src/gui_framework/rendering/command_buffers.rs`
- **Purpose**: Records Vulkan command buffers for rendering.
- **Key Functions**: `record_command_buffers(platform: &mut VulkanContext, renderables: &[Renderable], pipeline_layout: vk::PipelineLayout, _projection_descriptor_set: vk::DescriptorSet, extent: vk::Extent2D) -> ()`: Sets up draw commands.
- **Notes**: Uses per-object descriptor sets; recreates command pool each call.

### `src/gui_framework/rendering/renderable.rs`
- **Purpose**: Defines Vulkan renderable object properties.
- **Key Struct**: `Renderable`
  - Fields: `vertex_buffer`, `vertex_allocation`, `vertex_shader`, `fragment_shader`, `pipeline`, `offset_uniform`, `offset_allocation`, `descriptor_set`, `vertex_count`, `depth`, `on_window_resize_scale`, `on_window_resize_move`, `original_positions`, `fixed_size`, `center_ratio`.

### `src/gui_framework/rendering/render_engine.rs`
- **Purpose**: Manages Vulkan rendering pipeline.
- **Key Struct**: `Renderer`
  - Fields: `vulkan_renderables`, `pipeline_layout`, `uniform_buffer`, `uniform_allocation`, `descriptor_set_layout`, `descriptor_pool`, `descriptor_set`.
- **Key Methods**:
  - `new(platform: &mut VulkanContext, extent: vk::Extent2D, scene: &Scene) -> Self`: Initializes renderer.
  - `update_offset(&mut self, device: &Device, allocator: &Allocator, index: usize, offset: [f32; 2]) -> ()`: Updates object offset.
  - `resize_renderer(&mut self, vulkan_context: &mut VulkanContext, scene: &mut Scene, width: u32, height: u32) -> ()`: Handles resizing.
  - `render(&mut self, platform: &mut VulkanContext, scene: &Scene) -> ()`: Renders frame.
  - `cleanup(self, platform: &mut VulkanContext) -> ()`: Frees resources.
- **Notes**: Syncs offsets before rendering; depends on `scene.rs`.

### `src/gui_framework/rendering/shader_utils.rs`
- **Purpose**: Loads shader modules.
- **Key Functions**: `load_shader(device: &Device, filename: &str) -> vk::ShaderModule`: Loads `.spv` files.

### `src/gui_framework/rendering/swapchain.rs`
- **Purpose**: Manages Vulkan swapchain and framebuffers.
- **Key Functions**:
  - `create_swapchain(platform: &mut VulkanContext, extent: vk::Extent2D) -> vk::SurfaceFormatKHR`: Creates swapchain.
  - `create_framebuffers(platform: &mut VulkanContext, extent: vk::Extent2D, surface_format: vk::SurfaceFormatKHR) -> ()`: Sets up framebuffers.

### `src/gui_framework/scene/scene.rs`
- **Purpose**: Manages scene and renderable objects.
- **Key Structs**:
  - `RenderObject`: `vertices`, `vertex_shader_filename`, `fragment_shader_filename`, `depth`, `on_window_resize_scale`, `on_window_resize_move`, `offset`, `is_draggable`.
  - `Scene`: `render_objects`, `width`, `height`.
- **Key Trait**: `HitTestable`
  - `contains(&self, x: f32, y: f32, window_height: f32) -> bool`: Checks object bounds.
- **Key Methods**:
  - `new() -> Self`: Initializes 600x300 scene.
  - `add_object(&mut self, object: RenderObject) -> ()`: Adds object.
  - `pick_object_at(&self, x: f32, y: f32) -> Option<usize>`: Picks draggable object.
  - `translate_object(&mut self, index: usize, dx: f32, dy: f32) -> ()`: Updates offset.
  - `update_dimensions(&mut self, width: u32, height: u32) -> ()`: Updates size.
- **Notes**: Used by `controller.rs` and `render_engine.rs`.

### `src/gui_framework/window/window_handler.rs`
- **Purpose**: Handles window events and rendering loop.
- **Key Struct**: `VulkanContextHandler`
  - Fields: `vulkan_context`, `scene`, `renderer`, `resizing`, `controller`.
- **Key Methods**:
  - `new(platform: VulkanContext, scene: Scene) -> Self`: Initializes handler.
  - `resumed(&mut self, event_loop: &ActiveEventLoop) -> ()`: Sets up window and renderer.
  - `window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) -> ()`: Handles events.
- **Notes**: Implements `ApplicationHandler`; depends on all subsystems.

## Shaders
- **Location**: `shaders/`
- **Files**: `background.vert.spv`, `background.frag.spv`, `triangle.vert.spv`, `triangle.frag.spv`, `square.vert.spv`, `square.frag.spv`.
- **Notes**: Compiled from GLSL; uniforms at binding 0 (projection), 1 (offset).

## Dependencies
- `ash = "0.38"`, `vk-mem = "0.4"`, `winit = "0.30.9"`, `ash-window = "0.13"`, `glam = "0.30"`, `raw-window-handle = "0.6"`.