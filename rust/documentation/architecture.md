# Architecture Overview for `rusty_whip`

## Purpose
`rusty_whip` is an advanced 2D and 3D content generation application for digital entertainment production, leveraging GPU-accelerated AI diffusion/inference, multimedia creation (2D video, stills, animation, audio), and story-driven workflows, with plans for quantum-resistant P2P networking. Current focus: 2D GUI with Vulkan-based rendering, click-and-drag functionality, instancing, logical grouping, visibility toggling, a configurable hotkey system, generic click handling, and an event bus for decoupled communication.

## Core Components
1.  **Vulkan Context Management (`context/`)**
    *   **Role**: Initializes and manages core Vulkan resources (instance, device, queue, allocator, swapchain, etc.).
    *   **Key Modules**: `vulkan_context.rs`, `vulkan_setup.rs`.
2.  **Rendering Engine (`rendering/`)**
    *   **Role**: Orchestrates the Vulkan rendering pipeline, managing resources via sub-modules, handling event-driven updates (e.g., instance additions), and respecting object visibility flags. Implements `EventHandler`.
    *   **Key Modules**: `render_engine.rs`, `pipeline_manager.rs`, `buffer_manager.rs`, `resize_handler.rs`, `command_buffers.rs`, `renderable.rs`, `swapchain.rs`, `shader_utils.rs`.
3.  **Pipeline Manager (`rendering/pipeline_manager.rs`)**
    *   **Role**: Manages Vulkan `PipelineLayout`, `DescriptorSetLayout`, `DescriptorPool`, and the global projection `DescriptorSet`.
4.  **Buffer Manager (`rendering/buffer_manager.rs`)**
    *   **Role**: Manages Vulkan buffers (uniform, vertex, instance, offset), allocations, per-object `Pipelines`, `ShaderModules`, and allocates/updates/frees per-object `DescriptorSets`. Handles instance buffer creation/updates and populates `Renderable` visibility state.
5.  **Scene Management (`scene/`)**
    *   **Role**: Manages application state including renderable objects (`RenderObject` with visibility flag) via pooling (`ElementPool`), handles object/instance state updates based on events (via `SceneEventHandler`), publishes instance creation events, and manages logical grouping (`GroupManager`). Performs hit-testing.
    *   **Key Modules**: `scene.rs`, `group.rs`.
6.  **Interaction Handling (`interaction/`)**
    *   **Role**: Processes user input (mouse events, keyboard events, modifier changes), loads hotkey configurations, tracks modifier state, performs hit-testing via `Scene`, and publishes corresponding events (`ObjectMoved`, `ObjectClicked`, `HotkeyPressed`) via the `EventBus`.
    *   **Key Modules**: `controller.rs`, `hotkeys.rs`.
7.  **Event Bus (`event_bus/`)**
    *   **Role**: Decouples components using a publish-subscribe pattern for events like `ObjectMoved`, `InstanceAdded`, `ObjectClicked`, `HotkeyPressed`, etc. Manages `EventHandler` subscriptions.
    *   **Key Module**: `event_bus.rs`.
8.  **Application Entry (`main.rs`)**
    *   **Role**: Bootstraps the application using `winit`'s `EventLoop::run`. Initializes `EventBus`, `Scene`, `VulkanContext`, `EventLoopProxy`. Manages core component state (`VulkanContext`, `Renderer`, `InteractionController`, `Window`) via `Option` within the event loop closure. Handles `winit` events (`Resumed`, `WindowEvent`, `UserEvent`, `LoopExiting`, `AboutToWait`) directly in the closure. Defines and subscribes event handlers (`HotkeyActionHandler`, `Renderer`, `SceneEventHandler`, `ClickRouter`) to the `EventBus`. Coordinates application startup, rendering, updates, and cleanup. Defines `UserEvent` for proxy communication and `ClickRouter` for application-level click handling.
9.  **Build Script (`build.rs`)**
    *   **Role**: Compiles shaders using `glslc` and copies runtime assets (e.g., `user/hotkeys.toml`) from the source tree to the target build directory.

## Data Flow
1.  `main.rs` initializes `EventLoop`, `EventLoopProxy`, `EventBus`, `Scene`, `ClickRouter`, and initial state `Option`s. Subscribes `HotkeyActionHandler`, `SceneEventHandler`, and `ClickRouter`.
2.  `Event::Resumed`: Creates `Window`, `VulkanContext`, `Renderer`, `InteractionController`. Subscribes `Renderer` to `EventBus`. Stores components in `Option` fields.
3.  `Event::WindowEvent`: Relevant input events (mouse, keyboard, modifiers) are passed to `InteractionController`. Core window events (resize, close request) are handled directly.
4.  `InteractionController`: Processes input, performs hit-testing via `Scene`, checks `hotkey_config`, tracks `current_modifiers`, and publishes `ObjectMoved`, `ObjectClicked`, or `HotkeyPressed` to `EventBus`.
5.  `EventBus`: Notifies subscribers:
    *   `HotkeyActionHandler` receives `HotkeyPressed`. If action is "CloseRequested", sends `UserEvent::Exit` via `EventLoopProxy`.
    *   `Renderer` receives `InstanceAdded`, queues buffer update.
    *   `SceneEventHandler` receives `ObjectMoved`, updates `Scene` state.
    *   `ClickRouter` (in `main.rs`) receives `ObjectClicked`, looks up the object ID, and executes the registered callback if found.
6.  `Event::UserEvent::Exit`: Received by the main loop closure, triggers `elwt.exit()`.
7.  `Event::RedrawRequested` / `Event::AboutToWait`: Triggers `Renderer::render` within the main loop closure.
8.  `Renderer::render`: Processes pending updates, updates buffers via `BufferManager`, records/submits commands via `command_buffers.rs` (skipping non-visible renderables).
9.  `Event::LoopExiting`: Main loop closure performs cleanup: `EventBus::clear`, `Renderer::cleanup`, `cleanup_vulkan`.

## Key Interactions
- **`InteractionController` -> `EventBus`**: Publishes `ObjectMoved`, `ObjectClicked`, `HotkeyPressed`.
- **`InteractionController` -> `Scene`**: Calls `pick_object_at`.
- **`InteractionController` <- `hotkeys.rs`**: Uses `HotkeyConfig` for mapping.
- **`Scene` -> `EventBus`**: Publishes `InstanceAdded`.
- **`EventBus` -> `HotkeyActionHandler` -> `EventLoopProxy` -> `main.rs` (Closure)**: Handles hotkey actions, triggers exit via user event.
- **`EventBus` -> `Renderer`**: Handles `InstanceAdded`.
- **`EventBus` -> `SceneEventHandler` -> `Scene`**: Handles `ObjectMoved`.
- **`EventBus` -> `ClickRouter` (in `main.rs`)**: Handles `ObjectClicked`, triggers application logic via callbacks.
- **`Renderer` -> `BufferManager`**: Delegates buffer creation/updates, provides renderables (incl. visibility).
- **`Renderer` -> `PipelineManager`**: Uses pipeline layout/descriptor set.
- **`Renderer` -> `command_buffers.rs`**: Provides data for draw commands.
- **`PipelineManager` -> `BufferManager`**: Provides layout/pool.
- **`main.rs` (Closure) <-> `Renderer`/`Scene`/`Controller`/`VulkanContext`/`ClickRouter`**: Manages state via `Option` and `Arc<Mutex<>>`, orchestrates setup, rendering, updates, cleanup.
- **`VulkanContext` <-> All Vulkan Components**: Provides core Vulkan resources.
- **`build.rs` -> Target Directory**: Copies configuration files.

## Current Capabilities
- 2D GUI with depth-sorted objects using orthographic projection.
- Event bus (`EventBus`) for decoupled communication.
- Click-and-drag support for objects and instances via events.
- Generic object click detection via `ObjectClicked` event.
- Application-level click routing mechanism (example `ClickRouter` in `main.rs`).
- Window resizing with scaling or repositioning of objects.
- Object pooling (`ElementPool`).
- Instancing with event-driven buffer updates (fixed capacity).
- Logical grouping (`GroupManager`).
- Object visibility toggle support (via state flag and renderer check).
- Configurable hotkey system via TOML file (loaded relative to executable).
- Application exit via hotkey (`Escape` -> `CloseRequested` action).
- Refactored Vulkan resource management (`PipelineManager`, `BufferManager`).
- Build script for shader compilation and asset copying.

## Future Extensions
- Batch operations for groups (e.g., translate all objects in a group via events).
- P2P networking.
- 3D rendering and AI-driven content generation.
- Performance optimization (e.g., instance buffer resizing).
- Text rendering and editing.
- Context menus.
- Divider system.

## Error Handling
- Vulkan errors checked via `ash` results. `vk-mem` assertions check memory leaks.
- Logical errors (`GroupError`, `HotkeyError`) use `Result` or `thiserror`.
- Event bus mutex poisoning handled with logs. `Arc::try_unwrap` used for safe cleanup. Callback mutex poisoning handled with logs.
- Hotkey file loading/parsing errors handled gracefully with defaults.

## Shader Integration
- Per-object shaders (`triangle.vert.spv`, etc.) managed by `BufferManager`. Compiled by `build.rs`.
- Shaders support orthographic projection (binding 0), object offset (binding 1), and instance offset (vertex attribute 1, binding 1).

## Dependencies
- External crates: `ash = "0.38"`, `vk-mem = "0.4"`, `winit = "0.30.9"`, `ash-window = "0.13"`, `glam = "0.30"`, `raw-window-handle = "0.6"`, `toml = "0.8"`, `thiserror = "2.0"`.
- Shaders: Precompiled `.spv` files in `shaders/`.

## Notes
- Vulkan’s inverted Y-axis potentially needs adjustment in interaction/scene logic.
- `Renderer` delegates heavily to `PipelineManager` and `BufferManager`.
- `main.rs` uses `EventLoop::run` closure for event handling and state management.
- State (`Scene`, `Renderer`, `ClickRouter` etc.) managed via `Arc<Mutex<>>` for shared access and `Option` for lifecycle within `main.rs`.
- Cleanup order in `Event::LoopExiting` is critical.
- `build.rs` copies `user/hotkeys.toml` to target directory; runtime loads from there.