# Modules in `rusty_whip`

This document lists all files in the `rusty_whip` project, a Vulkan-based graphics application forming the foundation of an advanced 2D/3D GUI system for digital entertainment production. Each entry summarizes its purpose, key components, and relationships, reflecting the state after implementing depth sorting, orthographic projection, window resizing, and GUI behaviors as of March 11, 2025.

---

## 1. `platform.rs`
- **Purpose**: Defines the `Platform` struct, the central state container for Vulkan and window management, supporting a resizable 600x300 window with Vulkan resources.
- **Key Components**:
  - `Platform` struct: Holds Vulkan objects (`instance`, `device`, `swapchain`), window (`Arc<Window>`), buffers, shaders, and synchronization primitives, initialized via `new()`.
- **Relationships**:
  - Used by `main.rs` as the core platform instance.
  - Modified by `vulkan_core.rs` for Vulkan setup and `renderer.rs` for rendering resources.
  - Interacts with `window_management.rs` for event-driven resizing.

---

## 2. `lib.rs`
- **Purpose**: The library root, declaring public modules and the `Vertex` struct for 2D rendering in pixel coordinates.
- **Key Components**:
  - Exports `platform`, `vulkan_core`, `renderer`, `window_management`, and `scene`.
  - `Vertex` struct: Defines a 2D position (`[f32; 2]`) in pixel space for GUI elements.
- **Relationships**:
  - Provides the public API for `rusty_whip`.
  - `Vertex` is used in `renderer.rs` and `scene.rs`.

---

## 3. `main.rs`
- **Purpose**: The entry point, initializing a 600x300 `winit` window, setting up `Platform` and `Scene` with a background, triangle, and square, and running them with dynamic resizing.
- **Key Components**:
  - Sets up `EventLoop`, `Platform`, and `Scene` with a background quad (`depth: 0.0`, `21292a`, `Move`), triangle (`depth: 1.0`, `ff9800`, `Move`), and square (`depth: 2.0`, `42c922`, `Move`).
  - Runs via `PlatformHandler` with `run_app`.
- **Relationships**:
  - Depends on `platform.rs` for `Platform`, `scene.rs` for `Scene`, and `window_management.rs` for event handling.

---

## 4. `renderer.rs`
- **Purpose**: Manages Vulkan rendering with depth-sorted 2D objects in pixel coordinates, using an orthographic projection and uniform buffer, supporting window resizing.
- **Key Components**:
  - `load_shader`: Loads SPIR-V shaders from `./shaders/`.
  - `Renderable` struct: Represents objects with vertex buffers, shaders, pipelines, and vertex count.
  - `Renderer::new`: Initializes resources, sorts `renderables` by `depth: f32`, sets up uniform buffer with `ortho(0, width, height, 0, -1, 1)`.
  - `resize`: Updates swapchain, framebuffers, and uniform buffer on window resize.
  - `render`: Draws depth-sorted objects with background color `21292a`.
  - Helper functions: `create_swapchain`, `create_framebuffers`, `record_command_buffers`.
- **Relationships**:
  - Operates on `Platform` from `platform.rs`.
  - Uses `Vertex` from `lib.rs` and `Scene` from `scene.rs`.

---

## 5. `vulkan_core.rs`
- **Purpose**: Initializes and cleans up Vulkan resources for `Platform`, supporting a resizable window.
- **Key Components**:
  - `setup_vulkan`: Configures Vulkan instance, surface, device, and allocator.
  - `cleanup_vulkan`: Destroys Vulkan resources.
- **Relationships**:
  - Modifies `Platform` from `platform.rs`.

---

## 6. `window_management.rs`
- **Purpose**: Manages window lifecycle and events via `PlatformHandler`, enabling resizing with GUI updates.
- **Key Components**:
  - `PlatformHandler`: Wraps `Platform`, `Scene`, and `Renderer`, with a `resizing: bool` flag.
  - `resumed`: Sets up the 600x300 window and Vulkan.
  - `window_event`: Handles `Resized` (triggers `renderer.resize`), `CloseRequested`, and `RedrawRequested`.
- **Relationships**:
  - Uses `Platform` from `platform.rs`, `Scene` from `scene.rs`, and `Renderer` from `renderer.rs`.

---

## 7. `scene.rs`
- **Purpose**: Manages `Scene` and `RenderObject` with depth for 2D layering, using pixel coordinates.
- **Key Components**:
  - `RenderObject`: Stores `vertices`, `vertex_shader_filename`, `fragment_shader_filename`, `depth: f32`, and `behavior: RenderBehavior`.
  - `Scene`: Holds a `Vec<RenderObject>` for rendering.
- **Relationships**:
  - Initialized in `main.rs`, consumed by `renderer.rs`.

---

## 8. `Cargo.toml`
- **Purpose**: Configures the project with dependencies (`ash 0.38`, `vk-mem 0.4`, `winit 0.30.9`, etc.) and build script.
- **Relationships**:
  - Drives `build.rs` for shader compilation.

---

## 9. `build.rs`
- **Purpose**: Compiles `.vert` and `.frag` shaders to SPIR-V using `glslc` for runtime loading.
- **Relationships**:
  - Ensures shaders in `./shaders/` are available for `renderer.rs`.

---

## 10. `shaders/` Directory
- **Purpose**: Contains GLSL shaders (version 460) and SPIR-V binaries for rendering with specified colors.
- **Key Components**:
  - `background.vert`, `background.frag`: Full-screen quad (`21292a`, RGB: 0.129, 0.161, 0.165).
  - `triangle.vert`, `triangle.frag`: Triangle (`ff9800`, RGB: 1.0, 0.596, 0.0).
  - `square.vert`, `square.frag`: Square (`42c922`, RGB: 0.259, 0.788, 0.133).
  - Compilation scripts: `compile_shaders.sh` and `.ps1` for manual compilation.
- **Relationships**:
  - Loaded by `renderer.rs`, managed by `build.rs`.

---

## Project Overview
`rusty_whip` is a Vulkan-based graphics application evolving into a 2D/3D content creation tool. After completing the plan, it features:
- A 600x300 resizable window with a `21292a` background.
- Depth-sorted 2D GUI elements (background: 0.0, triangle: 1.0, square: 2.0) in pixel coordinates via orthographic projection.
- Dynamic resizing: Background fills the window, elements (triangle, square) move proportionately (e.g., triangle at center, square in top-left quadrant), scaling deferred via `RenderBehavior`.
- Flow: `main.rs` sets up `Platform` and `Scene`, `window_management.rs` handles events (including resizing), `vulkan_core.rs` initializes Vulkan, and `renderer.rs` renders depth-sorted objects with updated uniforms.

This foundation supports future 3D viewports and advanced GUI features, targeting Linux and Windows with unofficial compiling for Mac and BSD.