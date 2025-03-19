# Architecture Overview for `rusty_whip` (March 19, 2025)

## Purpose
`rusty_whip` is an advanced 2D and 3D content generation application for digital entertainment production, leveraging GPU-accelerated AI diffusion/inference, multimedia creation (2D video, stills, animation, audio), and story-driven workflows. It aims for a client-side, quantum-resistant P2P networking system, targeting Linux/Windows with unofficial Mac/BSD support. Current focus: 2D GUI with Vulkan rendering and click-and-drag.

## Core Components
1. **Vulkan Context Management (`context/`)**
   - **Role**: Initializes and manages Vulkan resources.
   - **Key Modules**: `vulkan_context.rs`, `vulkan_setup.rs`.
2. **Rendering Engine (`rendering/`)**
   - **Role**: Executes Vulkan rendering pipeline.
   - **Key Modules**: `render_engine.rs`, `command_buffers.rs`, `renderable.rs`, `swapchain.rs`, `shader_utils.rs`.
3. **Scene Management (`scene/`)**
   - **Role**: Stores and manipulates renderable objects.
   - **Key Module**: `scene.rs`.
4. **Interaction Handling (`interaction/`)**
   - **Role**: Processes user input for object manipulation.
   - **Key Module**: `controller.rs`.
5. **Window and Event Loop (`window/`)**
   - **Role**: Manages window events and rendering loop.
   - **Key Module**: `window_handler.rs`.
6. **Application Entry (`main.rs`)**
   - **Role**: Bootstraps the application.

## Data Flow
1. `main.rs` initializes `VulkanContext`, `Scene` with three objects, and `EventLoop`.
2. `window_handler.rs` sets up Vulkan via `vulkan_setup.rs` and `Renderer`.
3. Mouse events flow to `controller.rs`, updating `Scene` offsets.
4. `render_engine.rs` syncs offsets and renders via `command_buffers.rs` and `swapchain.rs`.

## Key Interactions
- `Scene` ↔ `Renderer`: `Scene` provides objects; `Renderer` renders them.
- `InteractionController` ↔ `Scene`: Updates offsets based on input.
- `VulkanContext` ↔ `Renderer`: Supplies Vulkan resources.
- `WindowHandler` ↔ All: Coordinates events and rendering.

## Current Capabilities
- 2D GUI with depth-sorted objects.
- Click-and-drag for RenderObject via shader offsets.
- Window resizing with scaling/movement adjustments.

## Future Extensions
- Undo, P2P networking, 3D rendering, AI integration.

## Dependencies
- `ash = "0.38"`, `vk-mem = "0.4"`, `winit = "0.30.9"`, `ash-window = "0.13"`, `glam = "0.30"`, `raw-window-handle = "0.6"`.
- Shaders: `.spv` files in `shaders/`.

## Notes
- Vulkan uses inverted Y-axis, adjusted in `controller.rs`.
- Prioritizes modularity and GPU performance.