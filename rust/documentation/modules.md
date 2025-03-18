# Modules Documentation for `rusty_whip` (March 17, 2025)

## Project Overview: `rusty_whip`

### Purpose
`rusty_whip` is an advanced 2D & 3D content generation application for digital entertainment production, emphasizing GPU-accelerated AI diffusion/inference, multimedia creation (2D video, stills, animation, audio), and story-driven workflows. It features a client-side, quantum-resistant, P2P networking system for real-time multi-user editing and targets Linux/Windows with unofficial Mac/BSD support. The current implementation focuses on a 2D GUI with Vulkan-based rendering and click-and-drag functionality.

### Current State
- **2D GUI**: Depth-sorted render objects (background `21292a`, triangle `ff9800`, square `42c922`) in pixel coordinates with orthographic projection, supporting window resizing and dragging.
- **Click-and-Drag**: Implemented with shader-based offsets for triangle and square; background is static.
- **Features Implemented**:
  - Mouse event detection and logging (Step 1-2).
  - Shader-based positioning with per-object descriptor sets (Step 4).
  - Object picking and dragging (Step 5).
- **Features Skipped**: Resize conflict handling, context switching (Step 6), and undo with double buffering (Step 7).

---

## Module Structure

### Directory Structure

rusty_whip/
├── src/
│   ├── gui_framework/
│   │   ├── context/
│   │   │   ├── vulkan_context.rs
│   │   │   └── vulkan_setup.rs
│   │   ├── interaction/
│   │   │   ├── mod.rs
│   │   │   └── controller.rs
│   │   ├── rendering/
│   │   │   ├── command_buffers.rs
│   │   │   ├── renderable.rs
│   │   │   ├── render_engine.rs
│   │   │   ├── shader_utils.rs
│   │   │   └── swapchain.rs
│   │   ├── scene/
│   │   │   └── scene.rs
│   │   ├── window/
│   │   │   └── window_handler.rs
│   │   └── mod.rs
│   ├── main.rs
│   └── lib.rs
├── shaders/
│   ├── background.vert.spv
│   ├── background.frag.spv
│   ├── triangle.vert.spv
│   ├── triangle.frag.spv
│   ├── square.vert.spv
│   └── square.frag.spv
├── Cargo.toml
└── build.rs


---

## Modules and Their Functions

### `src/lib.rs`
- **Purpose**: Root library file defining public exports.
- **Contents**:
  - `pub struct Vertex { position: [f32; 2] }`
  - `pub mod gui_framework;`
- **Notes**: provides `Vertex` type used across modules.

### `src/main.rs`
- **Purpose**: Application entry point, initializes Vulkan context, scene, and event loop.
- **Key Functions**:
  - `main()`: Creates `EventLoop`, `VulkanContext`, `Scene`, adds three `RenderObject`s (background, triangle, square), and runs `VulkanContextHandler`.
- **Notes**: Sets initial window size (600x300), defines object properties (vertices, shaders, depth, resize behavior, offset, draggable flag).

### `src/gui_framework/mod.rs`
- **Purpose**: Top-level module for GUI framework, re-exports submodules.
- **Contents**:
  - `pub mod context;`
  - `pub mod interaction;`
  - `pub mod rendering;`
  - `pub mod scene;`
  - `pub mod window;`
  - `pub use context::vulkan_context::VulkanContext;`
  - `pub use scene::scene::{Scene, RenderObject};`
  - `pub use window::window_handler::VulkanContextHandler;`
- **Notes**: provides access to core types.

### `src/gui_framework/context/vulkan_context.rs`
- **Purpose**: Manages Vulkan instance, device, and swapchain resources.
- **Key Struct**: `VulkanContext`
  - Fields: `instance`, `device`, `allocator`, `swapchain`, `swapchain_loader`, `queue`, `surface`, `surface_loader`, `render_pass`, `command_pool`, `command_buffers`, `framebuffers`, `image_views`, `images`, `current_image`, `image_available_semaphore`, `render_finished_semaphore`, `fence`, `window`.
  - Methods: `new()` (assumed).
- **Notes**: Assumed unchanged; provides Vulkan setup for rendering.

### `src/gui_framework/context/vulkan_setup.rs`
- **Purpose**: Utility functions for Vulkan initialization and cleanup.
- **Key Functions**:
  - `setup_vulkan(vulkan_context: &mut VulkanContext, window: Arc<Window>)`
  - `cleanup_vulkan(vulkan_context: &mut VulkanContext)`
- **Notes**: Assumed unchanged; used in `window_handler.rs`.

### `src/gui_framework/interaction/mod.rs`
- **Purpose**: Re-exports interaction-related modules.
- **Contents**:
  - `pub mod controller;`
  - `pub use controller::InteractionController;`
- **Notes**: Assumed unchanged.

### `src/gui_framework/interaction/controller.rs`
- **Purpose**: Handles mouse input for dragging objects.
- **Key Structs**:
  - `MouseState`: `is_dragging: bool`, `last_position: Option<[f32; 2]>`, `dragged_object: Option<usize>`.
  - `CursorContext`: Enum (`Canvas`, `Other`).
  - `InteractionController`: `mouse_state: MouseState`, `context: CursorContext`.
- **Key Methods**:
  - `new() -> Self`: Initializes with `Canvas` context.
  - `handle_event(&mut self, event: &Event<()>, scene: Option<&mut Scene>, _renderer: Option<&mut Renderer>, window: &Window)`: Processes mouse press, move, release; picks and translates objects, requests redraws.
- **Notes**: Inverts Y-delta for Vulkan coordinates; no undo.
### `src/gui_framework/rendering/command_buffers.rs`
- **Purpose**: Records Vulkan command buffers for rendering.
- **Key Function**:
  - `record_command_buffers(platform: &mut VulkanContext, renderables: &[Renderable], pipeline_layout: vk::PipelineLayout, _projection_descriptor_set: vk::DescriptorSet, extent: vk::Extent2D)`: Binds per-object descriptor sets, sets up viewports/scissors, draws objects.
- **Notes**: Uses `renderable.descriptor_set` for projection and offset uniforms.

### `src/gui_framework/rendering/renderable.rs`
- **Purpose**: Defines renderable object properties for Vulkan.
- **Key Struct**: `Renderable`
  - Fields: `vertex_buffer`, `vertex_allocation`, `vertex_shader`, `fragment_shader`, `pipeline`, `offset_uniform`, `offset_allocation`, `descriptor_set`, `vertex_count`, `depth`, `on_window_resize_scale`, `on_window_resize_move`, `original_positions`, `fixed_size`, `center_ratio`.
- **Notes**: Supports offset-based positioning.

### `src/gui_framework/rendering/render_engine.rs`
- **Purpose**: Manages Vulkan rendering pipeline and object updates.
- **Key Struct**: `Renderer`
  - Fields: `vulkan_renderables`, `pipeline_layout`, `uniform_buffer`, `uniform_allocation`, `descriptor_set_layout`, `descriptor_pool`, `descriptor_set`.
- **Key Methods**:
  - `new(platform: &mut VulkanContext, extent: vk::Extent2D, scene: &Scene) -> Self`: Initializes renderer, sets up descriptor sets for projection and offsets.
  - `update_offset(&mut self, device: &Device, allocator: &Allocator, index: usize, offset: [f32; 2])`: Updates `offset_uniform` and descriptor set.
  - `resize_renderer(&mut self, vulkan_context: &mut VulkanContext, scene: &mut Scene, width: u32, height: u32)`: Updates swapchain, framebuffers, and vertex positions.
  - `render(&mut self, platform: &mut VulkanContext, scene: &Scene)`: Syncs offsets, submits command buffers, presents frame.
  - `cleanup(mut self, platform: &mut VulkanContext)`: Frees Vulkan resources.
- **Notes**: Syncs `offset` (not `pending_offset`) in `render`

### `src/gui_framework/rendering/shader_utils.rs`
- **Purpose**: Loads shader modules.
- **Key Function**:
  - `load_shader(device: &Device, filename: &str) -> vk::ShaderModule`: Assumed to load `.spv` files.
- **Notes**: 

### `src/gui_framework/rendering/swapchain.rs`
- **Purpose**: Manages Vulkan swapchain and framebuffers.
- **Key Functions**:
  - `create_swapchain(platform: &mut VulkanContext, extent: vk::Extent2D) -> vk::SurfaceFormatKHR`
  - `create_framebuffers(platform: &mut VulkanContext, extent: vk::Extent2D, surface_format: vk::SurfaceFormatKHR)`
- **Notes**:

### `src/gui_framework/scene/scene.rs`
- **Purpose**: Defines scene and renderable objects with dragging support.
- **Key Structs**:
  - `RenderObject`: `vertices`, `vertex_shader_filename`, `fragment_shader_filename`, `depth`, `on_window_resize_scale`, `on_window_resize_move`, `offset`, `is_draggable`.
  - `Scene`: `render_objects`, `width`, `height`.
- **Key Trait**: `HitTestable`
  - `contains(&self, x: f32, y: f32, window_height: f32) -> bool`: Bounding box check with offset.
- **Key Methods**:
  - `new() -> Self`: Initializes with 600x300 dimensions.
  - `add_object(&mut self, object: RenderObject)`
  - `pick_object_at(&self, x: f32, y: f32) -> Option<usize>`: Picks topmost draggable object.
  - `translate_object(&mut self, index: usize, dx: f32, dy: f32)`: Updates `offset`.
  - `update_dimensions(&mut self, width: u32, height: u32)`: Updates scene size.
- **Notes**: 

### `src/gui_framework/window/window_handler.rs`
- **Purpose**: Handles window events and rendering loop.
- **Key Struct**: `VulkanContextHandler`
  - Fields: `vulkan_context`, `scene`, `renderer`, `resizing`, `controller`.
- **Key Methods**:
  - `new(platform: VulkanContext, scene: Scene) -> Self`
  - `resumed(&mut self, event_loop: &ActiveEventLoop)`: Initializes window and renderer.
  - `window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent)`: Handles close, redraw, resize, mouse input.
- **Notes**: Passes `scene` to `render` and `controller`; no resize blocking beyond `resizing` flag.

---

## Shaders
- **Location**: `shaders/`
- **Files**:
  - `background.vert.spv`, `triangle.vert.spv`, `square.vert.spv`: Vertex shaders with projection (binding 0) and offset (binding 1) uniforms.
  - `background.frag.spv`, `triangle.frag.spv`, `square.frag.spv`: Fragment shaders for colors (`21292a`, `ff9800`, `42c922`).
- **Notes**: Compiled from GLSL 460; assumed unchanged from Step 4.

---

## Dependencies
- **Cargo.toml** (assumed):
  ```toml
  [dependencies]
  ash = "0.37"
  vk-mem = "0.2"
  winit = "0.28"
  glam = "0.22"