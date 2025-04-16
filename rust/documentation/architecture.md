# Architecture Overview for `rusty_whip` (March 19, 2025)

## Purpose
`rusty_whip` is an advanced 2D and 3D content generation application for digital entertainment production, leveraging GPU-accelerated AI diffusion/inference, multimedia creation (2D video, stills, animation, audio), and story-driven workflows, with plans for quantum-resistant P2P networking. **It is currently undergoing a migration to the Bevy game engine ecosystem.** Current focus (pre-migration): 2D GUI with Vulkan-based rendering, click-and-drag functionality, instancing, logical grouping, visibility toggling, a configurable hotkey system, generic click handling, and an event bus for decoupled communication. **Post-migration (Task 6.2): Application runs under Bevy, windowing handled by Bevy, legacy components bridged via Resources, math types migrated to `bevy_math`.**

## Core Components
1.  **Vulkan Context Management (`context/`)**
    *   Role: Initializes and manages core Vulkan resources (instance, device, queue, allocator). **Does not manage swapchain/framebuffers directly (handled by Renderer).**
    *   Key Modules: `vulkan_context.rs`, `vulkan_setup.rs`.
2.  **Rendering Engine (`rendering/`)**
    *   Role: Orchestrates the Vulkan rendering pipeline, managing resources via sub-modules (swapchain, framebuffers, pipelines, buffers), handling event-driven updates (e.g., instance additions via legacy `EventBus`), and respecting object visibility flags. Implements `EventHandler`. Uses `bevy_math` for matrix operations. **Currently invoked via placeholder logic in Bevy systems (`main.rs`) due to integration challenges.**
    *   Key Modules: `render_engine.rs`, `pipeline_manager.rs`, `buffer_manager.rs`, `resize_handler.rs`, `command_buffers.rs`, `renderable.rs`, `swapchain.rs`, `shader_utils.rs`.
3.  **Pipeline Manager (`rendering/pipeline_manager.rs`)**
    *   Role: Manages Vulkan `PipelineLayout`, `DescriptorSetLayout`, `DescriptorPool`, and the global projection `DescriptorSet`.
4.  **Buffer Manager (`rendering/buffer_manager.rs`)**
    *   Role: Manages Vulkan buffers (uniform, vertex, instance, offset), allocations, per-object `Pipelines`, `ShaderModules`, and allocates/updates/frees per-object `DescriptorSets`. Handles instance buffer creation/updates and populates `Renderable` visibility state. Uses `bevy_math` for matrix operations.
5.  **Scene Management (`scene/`)**
    *   Role: Manages application state including renderable objects (`RenderObject` with visibility flag) via pooling (`ElementPool`), handles object/instance state updates based on events (via legacy `SceneEventHandler`), publishes instance creation events to legacy `EventBus`, and manages logical grouping (`GroupManager`). Performs hit-testing. Uses `[f32; 2]` for positions/offsets. **State is accessed via `SceneResource` in `main.rs`.**
    *   Key Modules: `scene.rs`, `group.rs`.
6.  **Interaction Handling (`interaction/`)**
    *   Role: **Legacy component.** Processes user input (mouse events, keyboard events, modifier changes), loads hotkey configurations, tracks modifier state, performs hit-testing via `Scene`, and publishes corresponding events (`ObjectMoved`, `ObjectClicked`, `HotkeyPressed`) via the legacy `EventBus`. **Currently inactive; input handling will be migrated to Bevy Input systems.**
    *   Key Modules: `controller.rs`, `hotkeys.rs`.
7.  **Event Bus (`event_bus/`)**
    *   Role: **Legacy component.** Decouples components using a publish-subscribe pattern for events like `ObjectMoved`, `InstanceAdded`, `ObjectClicked`, `HotkeyPressed`, etc. Manages `EventHandler` subscriptions. **Still exists as a Resource but is not actively driven by input events in the current state.**
    *   Key Module: `event_bus.rs`.
8.  **Bevy App Core (`main.rs`)**
    *   Role: **Replaces winit `EventLoop`.** Bootstraps the application using `bevy_app::App`. Initializes Bevy plugins (`bevy_winit`, `bevy_input`, `bevy_log`, `bevy_math`, etc.). Defines Bevy systems (`Startup`, `Update`, `Last`) to manage the application lifecycle. Holds legacy framework components (`VulkanContext`, `Scene`, `Renderer`, `EventBus`, `InteractionController`, `ClickRouter`) as Bevy `Resource`s using a temporary `Arc<Mutex<>>` bridge pattern. Orchestrates setup, updates, rendering triggers, and cleanup via systems. Defines placeholder logic where integration issues exist (e.g., Renderer creation/calls). Handles Bevy events (`WindowResized`, `WindowCloseRequested`, `AppExit`).
9.  **Build Script (`build.rs`)**
    *   Role: Compiles shaders using `glslc` and copies runtime assets (e.g., `user/hotkeys.toml`) from the source tree to the target build directory.

## Data Flow (Post Task 6.2)
1.  `main.rs` initializes `bevy_app::App`, adds plugins, and inserts legacy framework components wrapped in `Arc<Mutex<>>` as Bevy `Resources`. Subscribes legacy handlers (`SceneEventHandler`, `ClickRouter`, `HotkeyActionHandler`) to the legacy `EventBusResource`.
2.  Bevy `Startup` schedule runs:
    *   `setup_vulkan_system` gets the `winit::Window` handle via `WinitWindows`, locks `VulkanContextResource`, calls `setup_vulkan`.
    *   `create_renderer_system` (piped from previous) locks resources, creates a *placeholder* `Renderer`, wraps it in `RendererResource`, and subscribes it to the legacy `EventBusResource`.
3.  Bevy `Update` schedule runs:
    *   `InputPlugin` populates Bevy input resources (e.g., `Res<Input<KeyCode>>`).
    *   `winit_event_bridge_system` (currently inactive) was intended to bridge input to the legacy `InteractionController`.
    *   `handle_resize_system` reads `WindowResized` events, locks resources, calls *placeholder* resize logic (updates `Scene` dimensions).
    *   `handle_close_request` reads `WindowCloseRequested` events, sends `AppExit` event.
4.  Bevy `Last` schedule runs:
    *   `render_trigger_system` locks resources, calls *placeholder* rendering logic.
    *   `cleanup_system` (runs if `AppExit` event occurred) locks resources, calls `Renderer::cleanup` (placeholder) and `cleanup_vulkan`.

## Key Interactions (Post Task 6.2)
- **Bevy Systems (`main.rs`) <-> Framework Resources (`Arc<Mutex<T>>`)**: Systems access and modify the state of legacy components via Bevy's resource mechanism.
- **`setup_vulkan_system` -> `vulkan_setup::setup_vulkan`**: Initializes core Vulkan.
- **`create_renderer_system` -> `Renderer::new` (Placeholder)**: Initializes renderer state.
- **`render_trigger_system` -> `Renderer::render` (Placeholder)**: Triggers frame rendering.
- **`cleanup_system` -> `Renderer::cleanup` (Placeholder) / `vulkan_setup::cleanup_vulkan`**: Triggers cleanup on exit.
- **Legacy `EventBus` Interactions**: Still technically exist but are largely dormant as the `InteractionController` isn't publishing events. `InstanceAdded` events from `Scene` might still trigger `Renderer` updates if instances are added programmatically.
- **`Renderer` -> `BufferManager`**: Delegates buffer creation/updates, provides renderables (incl. visibility).
- **`Renderer` -> `PipelineManager`**: Uses pipeline layout/descriptor set.
- **`Renderer` -> `command_buffers.rs`**: Provides data for draw commands.
- **`PipelineManager` -> `BufferManager`**: Provides layout/pool.
- **`VulkanContext` <-> Vulkan Components**: Provides core Vulkan resources (device, allocator, etc.).
- **`build.rs` -> Target Directory**: Copies configuration files.

## Current Capabilities (Post Task 6.2)
- **Bevy Integration**: Application runs within `bevy_app`, windowing by `bevy_winit`, exit via `AppExit`.
- **Bevy Math**: Uses `bevy_math` types (`Mat4`) for rendering calculations.
- **Input Processing**: `bevy_input` active internally (but not connected to custom logic).
- **Vulkan Setup**: Core context initialized via Bevy system.
- **Legacy Bridge**: Framework components (`Scene`, `EventBus`, etc.) exist as Bevy `Resources` (`Arc<Mutex<>>`).
- **Placeholders**: Rendering and resize logic triggered by systems but use placeholder implementations.
- **Legacy Features (State)**:
    - 2D GUI state (depth-sorted objects via `Scene`).
    - Legacy Event bus (`EventBus`) for decoupled communication (mostly inactive).
    - Click-and-drag state management in `Scene` (triggered by inactive `InteractionController`).
    - Generic object click detection state/routing via legacy `ClickRouter` (triggered by inactive `InteractionController`).
    - Window resizing state updates in `Scene` (triggered by placeholder `handle_resize_system`).
    - Object pooling (`ElementPool`).
    - Instancing state management in `Scene` (updates published to legacy `EventBus`).
    - Logical grouping (`GroupManager`).
    - Object visibility toggle support (state stored in `RenderObject`).
    - Configurable hotkey system via TOML file (loading works, triggering inactive).
    - Refactored Vulkan resource management (`PipelineManager`, `BufferManager`).
- Build script for shader compilation and asset copying.

## Future Extensions
- **Complete Bevy Migration**: ECS components/systems for Scene/Interaction, Bevy Input integration, Bevy State.
- **Refactor Rendering Bridge**: Resolve `&mut VulkanContext` issues, remove placeholder logic.
- Implement actual rendering and resize logic within Bevy systems.
- Batch operations for groups.
- P2P networking.
- 3D rendering and AI-driven content generation.
- Performance optimization (e.g., instance buffer resizing, ECS parallelism).
- Text rendering and editing.
- Context menus.
- Divider system.

## Error Handling
- Vulkan errors checked via `ash` results. `vk-mem` assertions check memory leaks.
- Logical errors (`GroupError`, `HotkeyError`) use `Result` or `thiserror`.
- Event bus mutex poisoning handled with logs. `Arc::try_unwrap` used for safe cleanup. Callback mutex poisoning handled with logs.
- Hotkey file loading/parsing errors handled gracefully with defaults.
- **Bevy logging integrated via `LogPlugin`.**

## Shader Integration
- Per-object shaders (`triangle.vert.spv`, etc.) managed by `BufferManager`. Compiled by `build.rs`.
- Shaders support orthographic projection (binding 0), object offset (binding 1), and instance offset (vertex attribute 1, binding 1).

## Dependencies
- External crates: `ash = "0.38"`, `vk-mem = "0.4"`, `ash-window = "0.13"`, `raw-window-handle = "0.6"`, `toml = "0.8"`, `thiserror = "2.0"`, **`bevy_app = "0.15"`**, **`bevy_core = "0.15"`**, **`bevy_ecs = "0.15"`**, **`bevy_log = "0.15"`**, **`bevy_utils = "0.15"`**, **`bevy_window = "0.15"`**, **`bevy_winit = "0.15"`**, **`bevy_reflect = "0.15"`**, **`bevy_input = "0.15"`**, **`bevy_time = "0.15"`**, **`bevy_diagnostic = "0.15"`**, **`bevy_a11y = "0.15"`**, **`bevy_math = "0.15"`**. (Removed `glam`). `winit = "0.30.9"` (Still needed for legacy components).
- Shaders: Precompiled `.spv` files in `shaders/`.

## Notes
- **Migration State**: The application is in a transitional state, running under Bevy but using legacy components via a temporary `Arc<Mutex<>>` resource bridge. Math types migrated to `bevy_math`.
- **Known Issues**: Accessing `&mut VulkanContext` required by `Renderer` methods from Bevy systems is problematic due to the bridge, leading to placeholder logic for rendering/resizing. Input handling uses Bevy internally but doesn't drive the legacy `InteractionController`.
- Vulkan’s inverted Y-axis potentially needs adjustment in interaction/scene logic (once active).
- `Renderer` delegates heavily to `PipelineManager` and `BufferManager`.
- **`main.rs` uses Bevy App/Systems/Resources for event handling and state management.**
- State (`Scene`, `Renderer`, `ClickRouter` etc.) managed via `Arc<Mutex<>>` **Bevy Resources**.
- Cleanup order in `cleanup_system` is critical.
- `build.rs` copies `user/hotkeys.toml` to target directory; runtime loads from there.