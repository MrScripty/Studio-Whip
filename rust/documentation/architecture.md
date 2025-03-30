# Architecture Overview for `rusty_whip` (March 19, 2025)

## Purpose
`rusty_whip` is an advanced 2D and 3D content generation application for digital entertainment production, leveraging GPU-accelerated AI diffusion/inference, multimedia creation (2D video, stills, animation, audio), and story-driven workflows, with plans for quantum-resistant P2P networking. Current focus: 2D GUI with Vulkan-based rendering, click-and-drag functionality, instancing, and logical grouping.

## Core Components
1. **Vulkan Context Management (`context/`)**
   - **Role**: Initializes and manages Vulkan resources (instance, device, swapchain, etc.).
   - **Key Modules**: `vulkan_context.rs`, `vulkan_setup.rs`.
2. **Rendering Engine (`rendering/`)**
   - **Role**: Executes the Vulkan rendering pipeline with support for instancing, split into pipeline and buffer management.
   - **Key Modules**: `render_engine.rs`, `pipeline_manager.rs`, `buffer_manager.rs`, `command_buffers.rs`, `renderable.rs`, `swapchain.rs`, `shader_utils.rs`.
3. **Scene Management (`scene/`)**
   - **Role**: Manages renderable objects with pooling, instancing, and logical grouping.
   - **Key Modules**: `scene.rs`, `group.rs`.
4. **Interaction Handling (`interaction/`)**
   - **Role**: Processes user input (e.g., mouse events) for object and instance manipulation.
   - **Key Module**: `controller.rs`.
5. **Window and Event Loop (`window/`)**
   - **Role**: Manages window creation, events, and the rendering loop.
   - **Key Module**: `window_handler.rs`.
6. **Application Entry (`main.rs`)**
   - **Role**: Bootstraps the application with initial setup and test objects.

## Data Flow
1. `main.rs` initializes `VulkanContext` and `Scene`, populates objects/instances, and launches `EventLoop`.
2. `window_handler.rs` configures Vulkan via `vulkan_setup.rs` and initializes `Renderer`.
3. Mouse events are routed to `controller.rs`, which updates `Scene` offsets for objects or instances.
4. `render_engine.rs` orchestrates rendering: `pipeline_manager.rs` sets up pipelines/descriptors, `buffer_manager.rs` manages buffers, and rendering proceeds via `command_buffers.rs` and `swapchain.rs` with instancing.

## Key Interactions
- **`Scene` ↔ `Renderer`**: `Scene` supplies objects and instances; `Renderer` renders them with depth sorting and instancing via `buffer_manager.rs`.
- **`InteractionController` ↔ `Scene`**: Updates object/instance offsets using pool indices.
- **`VulkanContext` ↔ `Renderer`**: Provides Vulkan resources, accessed by `pipeline_manager.rs` and `buffer_manager.rs`.
- **`WindowHandler` ↔ All Components**: Orchestrates event handling and rendering.
- **`Scene` ↔ `GroupManager`**: Manages logical groups for object organization.
- **`PipelineManager` ↔ `BufferManager`**: `PipelineManager` provides pipeline layout and descriptor sets; `BufferManager` uses them for buffer setup.

## Current Capabilities
- 2D GUI with depth-sorted objects using orthographic projection.
- Click-and-drag support for objects and instances via shader offsets.
- Window resizing with scaling or repositioning of objects.
- Object pooling for efficient memory management.
- Instancing for rendering multiple object copies efficiently.
- Logical grouping for organizing objects, independent of rendering.

## Future Extensions
- Batch operations for groups (e.g., translate all objects in a group).
- Optional visibility toggling for objects or groups.
- P2P networking with an event bus.
- 3D rendering and AI-driven content generation.
- Performance optimization for 256+ elements.

## Error Handling
- Vulkan errors are checked via `ash` results, with cleanup handled in `vulkan_setup.rs`.
- Logical errors (e.g., duplicate group names) use `Result` in `group.rs`.

## Shader Integration
- Per-object shaders (e.g., `triangle.vert.spv`) are managed by `renderable.rs` and integrated into `BufferManager` pipelines.

## Dependencies
- External crates: `ash = "0.38"`, `vk-mem = "0.4"`, `winit = "0.30.9"`, `ash-window = "0.13"`, `glam = "0.30"`, `raw-window-handle = "0.6"`.
- Shaders: Precompiled `.spv` files in `shaders/` (e.g., `triangle.vert.spv`, `square.frag.spv`).

## Notes
- Vulkan’s inverted Y-axis is adjusted in `controller.rs` for mouse input and in `scene.rs` for hit detection.
- Logical grouping redesign ensures no direct rendering impact, managed via `GroupManager`.
- `render_engine.rs` now delegates buffer and pipeline tasks, reducing its scope.
- `main.rs` serves as a functional testbed for GUI features (e.g., dragging, instancing).