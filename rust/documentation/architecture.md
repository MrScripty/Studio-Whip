# Architecture Overview for `rusty_whip` (March 27, 2025)

## Purpose
`rusty_whip` is an advanced 2D and 3D content generation application for digital entertainment production, leveraging GPU-accelerated AI diffusion/inference, multimedia creation (2D video, stills, animation, audio), and story-driven workflows, with plans for quantum-resistant P2P networking. Current focus: 2D GUI with Vulkan rendering, click-and-drag, instancing, and grouping (under redesign).

## Core Components
1. **Vulkan Context Management (`context/`)**
   - **Role**: Initializes and manages Vulkan resources.
   - **Key Modules**: `vulkan_context.rs`, `vulkan_setup.rs`.
2. **Rendering Engine (`rendering/`)**
   - **Role**: Executes Vulkan rendering pipeline with instancing support.
   - **Key Modules**: `render_engine.rs`, `command_buffers.rs`, `renderable.rs`, `swapchain.rs`, `shader_utils.rs`.
3. **Scene Management (`scene/`)**
   - **Role**: Stores and manipulates renderable objects with pooling, instancing, and grouping (currently rendering-affecting, to be logical-only).
   - **Key Module**: `scene.rs`.
4. **Interaction Handling (`interaction/`)**
   - **Role**: Processes user input for object and instance manipulation.
   - **Key Module**: `controller.rs`.
5. **Window and Event Loop (`window/`)**
   - **Role**: Manages window events and rendering loop.
   - **Key Module**: `window_handler.rs`.
6. **Application Entry (`main.rs`)**
   - **Role**: Bootstraps the application.

## Data Flow
1. `main.rs` initializes `VulkanContext`, `Scene` with objects, instances, and one group, and `EventLoop`.
2. `window_handler.rs` sets up Vulkan via `vulkan_setup.rs` and `Renderer`.
3. Mouse events flow to `controller.rs`, updating `Scene` offsets for objects, instances, or groups.
4. `render_engine.rs` syncs offsets and renders via `command_buffers.rs` and `swapchain.rs`, using instancing where applicable.

## Key Interactions
- `Scene` ↔ `Renderer`: `Scene` provides objects and instances; `Renderer` renders them with instancing.
- `InteractionController` ↔ `Scene`: Updates offsets based on pool or group indices.
- `VulkanContext` ↔ `Renderer`: Supplies Vulkan resources.
- `WindowHandler` ↔ All: Coordinates events and rendering.

## Current Capabilities
- 2D GUI with depth-sorted objects using orthographic projection.
- Click-and-drag for individual objects and instances via shader offsets.
- Window resizing with scaling/movement adjustments.
- Object pooling for efficient management.
- Instancing for efficient rendering of multiple object copies.
- Grouping (affects rendering, under redesign) for organizing objects.

## Future Extensions
- Redesign groups as logical containers (Task 2).
- P2P networking with event bus.
- 3D rendering and AI integration.
- Performance testing with 256+ elements.

## Dependencies
- `ash = "0.38"`, `vk-mem = "0.4"`, `winit = "0.30.9"`, `ash-window = "0.13"`, `glam = "0.30"`, `raw-window-handle = "0.6"`.
- Shaders: `.spv` files in `shaders/`.

## Notes
- Vulkan uses inverted Y-axis, adjusted in `controller.rs` and hit detection.
- Grouping currently impacts rendering (hitbox/dragging issues); redesign in progress.
- Instancing fixed for instanced objects; non-instanced grouped objects problematic.