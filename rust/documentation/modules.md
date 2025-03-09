# Modules in `rusty_whip`

This document lists all files in the `rusty_whip` project, a Vulkan-based graphics application forming the base of a custom GUI system. Each entry includes a summary of its purpose, key components, and relationships to other files.

---

## 1. `application.rs`
- **Purpose**: Defines the core `App` struct, which serves as the central state container for the Vulkan application. It holds Vulkan objects, window references, buffers, shaders, and synchronization primitives.
- **Key Components**:
  - `App` struct: Contains fields like `instance`, `device`, `swapchain`, `vertex_buffer`, etc., initialized as `None` or empty via `new()`.
- **Relationships**:
  - Used by `main.rs` as the main application instance.
  - Extended via `window_management.rs` (implements `ApplicationHandler` for `App`).
  - Modified by `vulkan_core.rs` and `renderer.rs` to set up and manage Vulkan resources.

---

## 2. `lib.rs`
- **Purpose**: The library root, declaring public modules and a `Vertex` struct for shader input.
- **Key Components**:
  - Exports `application`, `vulkan_core`, `renderer`, and `window_management` modules.
  - `Vertex` struct: Defines a 2D position (`[f32; 2]`) for rendering.
- **Relationships**:
  - Ties all modules together, providing a public API for the `rusty_whip` crate.
  - `Vertex` is used in `renderer.rs` for vertex buffer creation.

---

## 3. `main.rs`
- **Purpose**: The entry point of the application, setting up the `winit` event loop and running the `App`.
- **Key Components**:
  - Creates an `EventLoop`, initializes `App`, and runs it with `run_app`.
- **Relationships**:
  - Depends on `application.rs` for the `App` struct.
  - Relies on `window_management.rs` to handle events via `ApplicationHandler`.

---

## 4. `renderer.rs`
- **Purpose**: Manages the Vulkan rendering pipeline, including swapchain setup, vertex buffers, shaders, and rendering commands.
- **Key Components**:
  - `load_shader`: Dynamically loads SPIR-V shader files from `../shaders/` at runtime.
  - `setup_renderer`: Initializes rendering resources, loading `test_shader.vert.spv` for the triangle’s vertex shader and `background.frag.spv` for the fragment shader using `load_shader`.
  - `cleanup_renderer`: Destroys rendering resources.
  - `render`: Executes the draw loop using semaphores and fences for synchronization.
- **Relationships**:
  - Operates on `App` from `application.rs`, populating its fields.
  - Uses `Vertex` from `lib.rs` for vertex data.
  - Relies on `shaders/` directory, linked via `build.rs` symlink.

---

## 5. `vulkan_core.rs`
- **Purpose**: Handles Vulkan initialization, including instance creation, surface setup, device selection, and memory allocation.
- **Key Components**:
  - `setup_vulkan`: Creates Vulkan entry, instance, surface, device, queue, and allocator.
  - `cleanup_vulkan`: Tears down Vulkan resources.
- **Relationships**:
  - Modifies `App` from `application.rs` with Vulkan essentials.
  - Called by `window_management.rs` during window creation.

---

## 6. `window_management.rs`
- **Purpose**: Implements `ApplicationHandler` for `App`, managing window lifecycle and events.
- **Key Components**:
  - `resumed`: Creates the window and triggers Vulkan and renderer setup.
  - `window_event`: Handles close requests and redraws, calling rendering and cleanup functions.
- **Relationships**:
  - Extends `App` from `application.rs`.
  - Calls `vulkan_core.rs` and `renderer.rs` functions for setup and rendering.

---

## 7. `Cargo.toml`
- **Purpose**: Project configuration file specifying dependencies, metadata, and build instructions.
- **Key Components**:
  - Defines `rusty_whip` crate with dependencies (`ash`, `vk-mem`, `winit`, `ash-window`).
  - Specifies `build = "build.rs"` to invoke the build script.
- **Relationships**:
  - Governs the build process, including `build.rs` for shader compilation and symlinking.

---

## 8. `build.rs`
- **Purpose**: Build script that compiles GLSL shaders to SPIR-V and creates a cross-platform symlink to the `shaders/` directory.
- **Key Components**:
  - `compile_shaders`: Uses `glslc` to compile `.vert` and `.frag` files in `shaders/` to `.spv` files (e.g., `test_shader.vert.spv`).
  - `create_shaders_symlink`: Links `target/debug/shaders` to `rust/shaders/` (Unix symlink on Linux, directory junction on Windows).
- **Relationships**:
  - Invoked by `Cargo.toml` during build.
  - Ensures `renderer.rs` can load shaders from `../shaders/`.

---

## 9. `shaders/` Directory
- **Purpose**: Contains GLSL shader source files (`.vert`, `.frag`) and their compiled SPIR-V binaries (`.spv`).
- **Key Components**:
  - Example files: `background.vert`, `background.vert.spv`, `background.frag`, `background.frag.spv`, `test_shader.vert`, `test_shader.vert.spv`.
  - Compiled to `.spv` by `build.rs` using `glslc`.
- **Relationships**:
  - Loaded dynamically in `renderer.rs` via `load_shader` from `../shaders/`.
  - Managed by `build.rs`, which compiles shaders and symlinks the directory to `target/debug/shaders`.

---

## Project Overview
This Rust project, named `rusty_whip`, is a Vulkan-based graphics application forming the base of a custom GUI system. It renders a simple triangle using dynamic shader loading. Key features:
- **Vulkan** (`ash`, `vk-mem`) for low-level graphics programming.
- **Winit** for cross-platform windowing and event handling.
- **Modular Design**:
  - `application.rs` holds the state.
  - `vulkan_core.rs` initializes Vulkan.
  - `renderer.rs` sets up and executes the rendering pipeline, dynamically loading shaders from `../shaders/`.
  - `window_management.rs` ties it to the windowing system.
  - `main.rs` starts the application.
  - `build.rs` automates shader compilation (GLSL to SPIR-V via `glslc`) and symlinks `shaders/` to `target/debug/shaders` for cross-platform compatibility (Linux and Windows).
- **Shaders**: GLSL source files in `shaders/` are compiled to SPIR-V at build time by `build.rs`, then loaded dynamically by `renderer.rs`.
The flow begins in `main.rs`, running the `winit` event loop with `App`. During build, `build.rs` compiles shaders and sets up the symlink. `window_management.rs` creates a window, triggering Vulkan setup (`vulkan_core.rs`) and renderer initialization (`renderer.rs`). The render loop in `renderer.rs` draws continuously until the window closes, followed by cleanup.