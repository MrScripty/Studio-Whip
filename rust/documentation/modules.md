# Modules in `rusty_whip`

This document lists all files in the `rusty_whip` project, a Vulkan-based graphics application forming the foundation of an advanced 2D/3D GUI system for digital entertainment production. Each entry summarizes its purpose, key components, and relationships, reflecting the state after implementing depth sorting, orthographic projection, window resizing, GUI behaviors, and a restructured directory layout as of March 13, 2025. Notably, `render_engine.rs` has been split into smaller modules within `rendering/` for improved modularity and maintainability.

---

## 1. `src/gui_framework/context/vulkan_context.rs`
- **Purpose**: Defines the `VulkanContext` struct, the central state container for Vulkan and window management, supporting a resizable 600x300 window with Vulkan resources.
- **Key Components**:
  - `VulkanContext` struct: Holds Vulkan objects (`instance`, `device`, `swapchain`), window (`Arc<Window>`), buffers, shaders, and synchronization primitives, initialized via `new()`.
- **Relationships**:
  - Used by `main.rs` as the core Vulkan context instance.
  - Modified by `vulkan_setup.rs` for Vulkan setup and `render_engine.rs` for rendering resources.
  - Interacts with `window_handler.rs` for event-driven resizing.

---

## 2. `src/lib.rs`
- **Purpose**: The library root, declaring the `gui_framework` module and the `Vertex` struct for 2D rendering in pixel coordinates.
- **Key Components**:
  - Exports `gui_framework` and all its public items via `pub use gui_framework::*`.
  - `Vertex` struct: Defines a 2D position (`[f32; 2]`) in pixel space for GUI elements.
- **Relationships**:
  - Provides the public API for `rusty_whip`.
  - `Vertex` is used in `render_engine.rs` and `scene.rs`.

---

## 3. `src/main.rs`
- **Purpose**: The entry point, initializing a 600x300 `winit` window, setting up `VulkanContext` and `Scene` with a background, triangle, and square, and running them with dynamic resizing.
- **Key Components**:
  - Sets up `EventLoop`, `VulkanContext`, and `Scene` with a background quad (`depth: 0.0`, `21292a`, `on_window_resize_scale: true`), triangle (`depth: 1.0`, `ff9800`, `on_window_resize_move: true`), and square (`depth: 2.0`, `42c922`, `on_window_resize_move: true`).
  - Runs via `VulkanContextHandler` with `run_app`.
- **Relationships**:
  - Depends on `vulkan_context.rs` for `VulkanContext`, `scene.rs` for `Scene`, and `window_handler.rs` for event handling.

---

## 4. `src/gui_framework/rendering/render_engine.rs`
- **Purpose**: Orchestrates Vulkan rendering by managing `Renderer`, leveraging split modules for shader loading, swapchain setup, command buffers, and renderable objects, supporting depth-sorted 2D rendering with window resizing.
- **Key Components**:
  - `Renderer` struct: Holds `vulkan_renderables: Vec<Renderable>`, `pipeline_layout`, `uniform_buffer` (orthographic projection), `uniform_allocation`, `descriptor_set_layout`, `descriptor_pool`, and `descriptor_set`.
  - `Renderer::new`: Initializes rendering resources, creates renderables from `Scene`, sorts by depth, and sets up Vulkan state using split modules.
  - `resize_renderer`: Updates resources on window resize, adjusting orthographic projection and vertex buffers via split modules.
  - `render`: Executes the rendering pipeline, submitting command buffers and presenting the swapchain.
  - `cleanup`: Frees Vulkan resources.
- **Relationships**:
  - Depends on `vulkan_context.rs` for `VulkanContext`, `scene.rs` for `Scene`, and split modules: `shader_utils.rs`, `renderable.rs`, `swapchain.rs`, `command_buffers.rs`.
  - Used by `window_handler.rs` for rendering lifecycle.

---

## 5. `src/gui_framework/rendering/shader_utils.rs`
- **Purpose**: Provides utility functions for loading SPIR-V shaders from the filesystem.
- **Key Components**:
  - `load_shader`: Loads a shader file from `./shaders/` and creates a `vk::ShaderModule`.
- **Relationships**:
  - Called by `render_engine.rs` to load vertex and fragment shaders for `Renderable` objects.

---

## 6. `src/gui_framework/rendering/renderable.rs`
- **Purpose**: Defines the `Renderable` struct, representing depth-sorted 2D objects with Vulkan resources and resize behavior.
- **Key Components**:
  - `Renderable` struct: Contains `vertex_buffer`, `vertex_allocation`, `vertex_shader`, `fragment_shader`, `pipeline`, `vertex_count`, `depth: f32`, `on_window_resize_scale: bool`, `on_window_resize_move: bool`, `original_positions: Vec<[f32; 2]>`, `fixed_size: [f32; 2]`, and `center_ratio: [f32; 2]` for fixed sizes and proportional movement.
- **Relationships**:
  - Instantiated in `render_engine.rs` from `Scene` data.
  - Used by `command_buffers.rs` for drawing commands.

---

## 7. `src/gui_framework/rendering/swapchain.rs`
- **Purpose**: Manages Vulkan swapchain and framebuffer creation for presentation, supporting dynamic window resizing.
- **Key Components**:
  - `create_swapchain`: Initializes the swapchain, images, and image views, returning `vk::SurfaceFormatKHR`.
  - `create_framebuffers`: Sets up render pass and framebuffers for rendering.
- **Relationships**:
  - Called by `render_engine.rs` in `Renderer::new` and `resize_renderer` to manage presentation resources.
  - Modifies `VulkanContext` fields like `swapchain`, `images`, `image_views`, `render_pass`, and `framebuffers`.

---

## 8. `src/gui_framework/rendering/command_buffers.rs`
- **Purpose**: Handles Vulkan command buffer recording for drawing depth-sorted 2D objects.
- **Key Components**:
  - `record_command_buffers`: Creates a command pool, allocates command buffers, and records drawing commands for `Renderable` objects with a specified extent.
- **Relationships**:
  - Called by `render_engine.rs` in `Renderer::new` and `resize_renderer` to prepare command buffers.
  - Uses `Renderable` from `renderable.rs` for vertex and pipeline data.
  - Modifies `VulkanContext` fields like `command_pool` and `command_buffers`.

---

## 9. `src/gui_framework/rendering/mod.rs`
- **Purpose**: Declares the `rendering` submodule hierarchy and re-exports key types for external use.
- **Key Components**:
  - Declares submodules: `render_engine`, `shader_utils`, `renderable`, `swapchain`, `command_buffers`.
  - Re-exports: `Renderer` from `render_engine.rs` and `Renderable` from `renderable.rs` via `pub use`.
- **Relationships**:
  - Part of the `gui_framework` module, enabling access to rendering components in `lib.rs`.

---

## 10. `src/gui_framework/context/vulkan_setup.rs`
- **Purpose**: Initializes and cleans up Vulkan resources for `VulkanContext`, supporting a resizable window.
- **Key Components**:
  - `setup_vulkan`: Configures Vulkan instance, surface, device, and allocator.
  - `cleanup_vulkan`: Destroys Vulkan resources.
- **Relationships**:
  - Modifies `VulkanContext` from `vulkan_context.rs`.

---

## 11. `src/gui_framework/window/window_handler.rs`
- **Purpose**: Manages window lifecycle and events via `VulkanContextHandler`, enabling resizing with GUI updates.
- **Key Components**:
  - `VulkanContextHandler`: Wraps `VulkanContext`, `Scene`, and `Renderer`, with a `resizing: bool` flag.
  - `resumed`: Sets up the 600x300 window and Vulkan.
  - `window_event`: Handles `Resized` (triggers `resize_renderer`), `CloseRequested`, and `RedrawRequested`.
- **Relationships**:
  - Uses `VulkanContext` from `vulkan_context.rs`, `Scene` from `scene.rs`, and `Renderer` from `render_engine.rs`.

---

## 12. `src/gui_framework/scene/scene.rs`
- **Purpose**: Manages `Scene` and `RenderObject` with depth for 2D layering, using pixel coordinates.
- **Key Components**:
  - `RenderObject`: Stores `vertices`, `vertex_shader_filename`, `fragment_shader_filename`, `depth: f32`, `on_window_resize_scale: bool`, and `on_window_resize_move: bool`.
  - `Scene`: Holds a `Vec<RenderObject>` for rendering.
- **Relationships**:
  - Initialized in `main.rs`, consumed by `render_engine.rs`.

---

## 13. `src/gui_framework/mod.rs`
- **Purpose**: Defines the `gui_framework` module hierarchy and re-exports key types.
- **Key Components**:
  - Declares submodules: `rendering`, `context`, `window`, `scene`.
  - Re-exports: `Renderer`, `VulkanContext`, `VulkanContextHandler`, `Scene`, `RenderObject`.
- **Relationships**:
  - Ties together all `gui_framework` components for use in `lib.rs`.

---

## 14. `Cargo.toml`
- **Purpose**: Configures the project with dependencies (`ash 0.38`, `vk-mem 0.4`, `winit 0.30.9`, etc.) and build script.
- **Relationships**:
  - Drives `build.rs` for shader compilation.

---

## 15. `build.rs`
- **Purpose**: Compiles `.vert` and `.frag` shaders to SPIR-V using `glslc` for runtime loading.
- **Relationships**:
  - Ensures shaders in `./shaders/` are available for `shader_utils.rs`.

---

## 16. `shaders/` Directory
- **Purpose**: Contains GLSL shaders (version 460) and SPIR-V binaries for rendering with specified colors.
- **Key Components**:
  - `background.vert`, `background.frag`: Full-screen quad (`21292a`, RGB: 0.129, 0.161, 0.165).
  - `triangle.vert`, `triangle.frag`: Triangle (`ff9800`, RGB: 1.0, 0.596, 0.0).
  - `square.vert`, `square.frag`: Square (`42c922`, RGB: 0.259, 0.788, 0.133).
  - Compilation scripts: `compile_shaders.sh` and `.ps1` for manual compilation.
- **Relationships**:
  - Loaded by `shader_utils.rs`, managed by `build.rs`.

---

## Project Overview
`rusty_whip` is a Vulkan-based graphics application evolving into a 2D/3D content creation tool. As of March 13, 2025, it features:
- A 600x300 resizable window with a `21292a` background.
- Depth-sorted 2D GUI elements (background: 0.0, triangle: 1.0, square: 2.0) in pixel coordinates via orthographic projection.
- Dynamic resizing: Background fills the window using `on_window_resize_scale`, elements (triangle, square) move proportionately using `on_window_resize_move` (e.g., triangle at center, square in top-left quadrant) while maintaining fixed sizes (e.g., 50x50 pixels).
- Flow: `main.rs` sets up `VulkanContext` and `Scene`, `window_handler.rs` handles events (including resizing), `vulkan_setup.rs` initializes Vulkan, and `render_engine.rs` orchestrates rendering with depth-sorted objects using split modules (`shader_utils.rs`, `renderable.rs`, `swapchain.rs`, `command_buffers.rs`) and updated uniforms.
- New Structure: Organized under `gui_framework/` with subdirectories (`rendering/`, `context/`, etc.), each with a `mod.rs` for explicit module definition. The `rendering/` directory now includes `render_engine.rs`, `shader_utils.rs`, `renderable.rs`, `swapchain.rs`, and `command_buffers.rs`.

This foundation supports future 3D viewports and advanced GUI features, targeting Linux and Windows with unofficial compiling for Mac and BSD.