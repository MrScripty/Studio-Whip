# Modules in `rusty_whip`

This document lists all files in the `rusty_whip` project, a Vulkan-based graphics application forming the base of a custom GUI system. Each entry includes a summary of its purpose, key components, and relationships to other files.

---

## 1. `platform.rs` (Previously `application.rs`)
- **Purpose**: Defines the core `Platform` struct, the central state container for Vulkan and window management. It holds Vulkan objects, window references, buffers, shaders, and synchronization primitives.
- **Key Components**:
  - `Platform` struct: Contains fields like `instance`, `device`, `swapchain`, `vertex_buffer`, etc., initialized as `None` or empty via `new()`.
- **Relationships**:
  - Used by `main.rs` as the main platform instance.
  - Extended via `window_management.rs` (via `PlatformHandler` implementing `ApplicationHandler`).
  - Modified by `vulkan_core.rs` and `renderer.rs` to set up and manage Vulkan resources.

---

## 2. `lib.rs`
- **Purpose**: The library root, declaring public modules and a `Vertex` struct for shader input.
- **Key Components**:
  - Exports `platform`, `vulkan_core`, `renderer`, `window_management`, and `scene` modules.
  - `Vertex` struct: Defines a 2D position (`[f32; 2]`) for rendering.
- **Relationships**:
  - Ties all modules together, providing a public API for the `rusty_whip` crate.
  - `Vertex` is used in `renderer.rs` and `scene.rs` for vertex data.

---

## 3. `main.rs`
- **Purpose**: The entry point, setting up the `winit` event loop, initializing `Platform` and `Scene`, and running them via `winit`’s `run_app`.
- **Key Components**:
  - Creates an `EventLoop`, initializes `Platform` and `Scene` (with a triangle `RenderObject`), and runs them with a `PlatformHandler` via `winit run_app`.
- **Relationships**:
  - Depends on `platform.rs` for `Platform` and `scene.rs` for `Scene`.
  - Uses `window_management.rs` to handle events via `PlatformHandler`.

---

## 4. `renderer.rs`
- **Purpose**: Manages the Vulkan rendering pipeline, including swapchain setup, vertex buffers, shaders, and rendering commands, using data from `Scene`.
- **Key Components**:
  - `load_shader`: Loads SPIR-V shaders from `../shaders/` at runtime.
  - `setup_renderer`: Initializes rendering resources using `Scene`’s `RenderObject` data (e.g., vertices, shaders).
  - `cleanup_renderer`: Destroys rendering resources.
  - `render`: Executes the draw loop.
- **Relationships**:
  - Operates on `Platform` from `platform.rs`, populating its fields.
  - Uses `Vertex` from `lib.rs` and `Scene` from `scene.rs`.

---

## 5. `vulkan_core.rs`
- **Purpose**: Handles Vulkan initialization (instance, surface, device, etc.) for `Platform`.
- **Key Components**:
  - `setup_vulkan`: Sets up Vulkan essentials.
  - `cleanup_vulkan`: Tears down Vulkan resources.
- **Relationships**:
  - Modifies `Platform` from `platform.rs`.

---

## 6. `window_management.rs`
- **Purpose**: Defines `PlatformHandler` to manage window lifecycle and events, integrating `Platform` and `Scene`.
- **Key Components**:
  - `PlatformHandler`: Wraps `Platform` and `Scene`, implementing `ApplicationHandler`.
  - `resumed`: Creates the window, triggers Vulkan and renderer setup.
  - `window_event`: Handles close requests and redraws.
- **Relationships**:
  - Uses `Platform` from `platform.rs` and `Scene` from `scene.rs`.
  - Calls `vulkan_core.rs` and `renderer.rs` functions.

---

## 7. `scene.rs`
- **Purpose**: Defines `Scene` and `RenderObject` for scene management, holding renderable object data.
- **Key Components**:
  - `RenderObject`: Stores vertices and shader filenames (e.g., `test_shader.vert.spv`).
  - `Scene`: Manages a collection of `RenderObject`s.
- **Relationships**:
  - Initialized in `main.rs`.
  - Used by `renderer.rs` via `window_management.rs`.

---

## 8. `Cargo.toml`
- **Purpose**: Project configuration file (unchanged).
- **Relationships**:
  - Governs the build process, including `build.rs`.

---

## 9. `build.rs`
- **Purpose**: Build script for shader compilation and symlinking (unchanged).
- **Relationships**:
  - Ensures `renderer.rs` can load shaders from `../shaders/`.

---

## 10. `shaders/` Directory
- **Purpose**: Contains GLSL shaders and compiled SPIR-V binaries (unchanged).
- **Relationships**:
  - Loaded by `renderer.rs`, managed by `build.rs`.

---

## Project Overview
`rusty_whip` is a Vulkan-based graphics application forming a GUI framework. It renders a triangle using dynamic shader loading, with a layered architecture:
- **Platform** (`platform.rs`): Manages Vulkan and window state.
- **Scene** (`scene.rs`): Handles renderable objects (e.g., a triangle).
- **Renderer** (`renderer.rs`): Executes Vulkan rendering.
- Flow: `main.rs` initializes `Platform` and `Scene`, runs them via `PlatformHandler` in `window_management.rs`, which triggers Vulkan setup (`vulkan_core.rs`) and rendering (`renderer.rs`).