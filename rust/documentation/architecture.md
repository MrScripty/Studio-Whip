# Architecture Overview for `rusty_whip` (March 19, 2025)

## Purpose
`rusty_whip` is an advanced 2D and 3D content generation application for digital entertainment production, leveraging GPU-accelerated AI diffusion/inference, multimedia creation (2D video, stills, animation, audio), and story-driven workflows, with plans for quantum-resistant P2P networking. Current focus: 2D GUI with Vulkan-based rendering, click-and-drag functionality, instancing, logical grouping, and an event bus for decoupled communication.

## Core Components
1.  **Vulkan Context Management (`context/`)**
    *   **Role**: Initializes and manages core Vulkan resources (instance, device, queue, allocator, swapchain, etc.).
    *   **Key Modules**: `vulkan_context.rs`, `vulkan_setup.rs`.
2.  **Rendering Engine (`rendering/`)**
    *   **Role**: Orchestrates the Vulkan rendering pipeline, managing resources via sub-modules and handling event-driven updates (e.g., instance additions). Implements `EventHandler`.
    *   **Key Modules**: `render_engine.rs`, `pipeline_manager.rs`, `buffer_manager.rs`, `resize_handler.rs`, `command_buffers.rs`, `renderable.rs`, `swapchain.rs`, `shader_utils.rs`.
3.  **Pipeline Manager (`rendering/pipeline_manager.rs`)**
    *   **Role**: Manages Vulkan `PipelineLayout`, `DescriptorSetLayout`, `DescriptorPool`, and the global projection `DescriptorSet`.
4.  **Buffer Manager (`rendering/buffer_manager.rs`)**
    *   **Role**: Manages Vulkan buffers (uniform, vertex, instance, offset), allocations, per-object `Pipelines`, `ShaderModules`, and allocates/updates/frees per-object `DescriptorSets`. Handles instance buffer creation and updates.
5.  **Scene Management (`scene/`)**
    *   **Role**: Manages renderable objects (`RenderObject`) via pooling (`ElementPool`), handles object/instance state updates based on events (via `SceneEventHandler`), publishes instance creation events, and manages logical grouping (`GroupManager`).
    *   **Key Modules**: `scene.rs`, `group.rs`.
6.  **Interaction Handling (`interaction/`)**
    *   **Role**: Processes user input (e.g., mouse events) and publishes corresponding events (`ObjectMoved`, `ObjectPicked`) via the `EventBus`.
    *   **Key Module**: `controller.rs`.
7.  **Event Bus (`event_bus/`)**
    *   **Role**: Decouples components using a publish-subscribe pattern for events like `ObjectMoved`, `InstanceAdded`, etc. Manages `EventHandler` subscriptions.
    *   **Key Module**: `event_bus.rs`.
8.  **Window and Event Loop (`window/`)**
    *   **Role**: Manages window creation (`winit`), the main event loop, Vulkan setup/cleanup orchestration, event dispatching to `InteractionController`, and managing shared state (`Scene`, `Renderer`) via `Arc<Mutex<>>`. Subscribes handlers to the `EventBus`.
    *   **Key Module**: `window_handler.rs`.
9.  **Application Entry (`main.rs`)**
    *   **Role**: Bootstraps the application, initializes `EventBus`, `Scene`, `VulkanContext`, populates initial test objects/instances, and runs the `VulkanContextHandler` event loop.

## Data Flow
1.  `main.rs` initializes `EventBus`, `VulkanContext`, `Scene` (populating objects/instances), wraps `Scene` in `Arc<Mutex<>>`, creates `VulkanContextHandler`, and launches the `winit` event loop.
2.  `VulkanContextHandler::resumed` creates the window, sets up Vulkan (`vulkan_setup.rs`), creates `Renderer`, wraps it in `Arc<Mutex<>>`, and subscribes `SceneEventHandler` and `Renderer` to the `EventBus`.
3.  User input events (`winit`) are processed by `VulkanContextHandler`. Mouse events relevant to the canvas are passed to `InteractionController`.
4.  `InteractionController` interprets input and publishes events (e.g., `ObjectMoved`, `ObjectPicked`) to the `EventBus`.
5.  `EventBus` notifies subscribers:
    *   `SceneEventHandler` handles `ObjectMoved`, locks the `Scene` mutex, and calls `Scene::translate_object`.
    *   `Renderer` handles `InstanceAdded`, locks its internal queue, and adds the update task.
6.  `Scene::add_instance` publishes `InstanceAdded` event.
7.  On `RedrawRequested`, `VulkanContextHandler` locks the `Renderer` mutex and calls `Renderer::render`.
8.  `Renderer::render` processes its `pending_instance_updates` queue, calling `BufferManager::update_instance_buffer`. It then iterates through the scene (via locked `Scene` ref) to update object/instance offsets by calling `BufferManager::update_offset`/`update_instance_offset`. Finally, it executes the Vulkan rendering commands via `command_buffers.rs` and `swapchain.rs`.

## Key Interactions
- **`InteractionController` -> `EventBus`**: Publishes user interaction events.
- **`Scene` -> `EventBus`**: Publishes instance creation events.
- **`EventBus` -> `SceneEventHandler` -> `Scene`**: Handles `ObjectMoved` events to update scene state.
- **`EventBus` -> `Renderer`**: Handles `InstanceAdded` events to queue buffer updates.
- **`Renderer` -> `BufferManager`**: Delegates buffer creation, updates (offset, instance), and provides renderables.
- **`Renderer` -> `PipelineManager`**: Uses pipeline layout and global descriptor set.
- **`Renderer` -> `command_buffers.rs`**: Provides data needed to record draw commands.
- **`PipelineManager` -> `BufferManager`**: Provides `DescriptorSetLayout` and `DescriptorPool` during initialization.
- **`WindowHandler` <-> `EventBus`**: Manages bus lifecycle and subscriptions.
- **`WindowHandler` <-> `Renderer`/`Scene`**: Manages state via `Arc<Mutex<>>`, orchestrates rendering and cleanup.
- **`VulkanContext` <-> All Vulkan Components**: Provides core Vulkan resources.

## Current Capabilities
- 2D GUI with depth-sorted objects using orthographic projection.
- Event bus (`EventBus`) for decoupled communication (dragging, instancing).
- Click-and-drag support for objects and instances via events and shader offsets.
- Window resizing with scaling or repositioning of objects.
- Object pooling (`ElementPool`) for efficient memory management.
- Instancing for rendering multiple object copies efficiently, with event-driven buffer updates.
- Logical grouping (`GroupManager`) for organizing objects, independent of rendering.
- Clear separation of Vulkan resource management (`PipelineManager`, `BufferManager`).

## Future Extensions
- Batch operations for groups (e.g., translate all objects in a group via events).
- Optional visibility toggling for objects or groups.
- P2P networking.
- 3D rendering and AI-driven content generation.
- Performance optimization for 256+ elements.
- Robust instance buffer resizing.

## Error Handling
- Vulkan errors checked via `ash` results. `vk-mem` assertions check memory leaks.
- Logical errors (e.g., `GroupError`) use `Result`.
- Event bus mutex poisoning handled with logs. `Arc::try_unwrap` used for safe cleanup.

## Shader Integration
- Per-object shaders (`triangle.vert.spv`, etc.) managed by `BufferManager`.
- Shaders support orthographic projection (binding 0), object offset (binding 1), and instance offset (vertex attribute 1, binding 1).

## Dependencies
- External crates: `ash = "0.38"`, `vk-mem = "0.4"`, `winit = "0.30.9"`, `ash-window = "0.13"`, `glam = "0.30"`, `raw-window-handle = "0.6"`.
- Shaders: Precompiled `.spv` files in `shaders/`.

## Notes
- Vulkan’s inverted Y-axis adjusted in `controller.rs` and `scene.rs`.
- `Renderer` delegates heavily to `PipelineManager` and `BufferManager`.
- `main.rs` serves as a functional testbed.
- State (`Scene`, `Renderer`) managed via `Arc<Mutex<>>` in `WindowHandler`.
- Cleanup order (`EventBus::clear`, `Renderer::cleanup`, `cleanup_vulkan`) is critical to avoid leaks/assertions.