# Architecture Overview for `rusty_whip`

## Purpose
`rusty_whip` is an advanced 2D and 3D content generation application for digital entertainment production, leveraging GPU-accelerated AI diffusion/inference, multimedia creation (2D video, stills, animation, audio), and story-driven workflows, with plans for quantum-resistant P2P networking. **It is currently migrating its core logic (excluding rendering) to the Bevy game engine ecosystem (v0.15), strictly avoiding Bevy's rendering stack (`bevy_render`, `wgpu`, etc.) in favor of its custom Vulkan backend.**

## Core Components
1.  **Vulkan Context Management (`context/`)**
    *   Role: Initializes and manages core Vulkan resources (instance, device, queue, allocator). Accessed via `VulkanContextResource`.
    *   Key Modules: `vulkan_context.rs`, `vulkan_setup.rs`.
2.  **Rendering Engine (`rendering/`)**
    *   Role: Orchestrates the custom Vulkan rendering pipeline. Manages Vulkan resources via sub-modules. **Signatures and data structures partially adapted for ECS data flow, but core logic (resource creation/updates in `BufferManager`, draw calls in `command_buffers`) still uses placeholder implementations.** Uses `bevy_math` for matrix operations. Accessed via `RendererResource`.
    *   Key Modules: `render_engine.rs`, `pipeline_manager.rs`, `buffer_manager.rs`, `resize_handler.rs`, `command_buffers.rs`, `swapchain.rs`, `shader_utils.rs`. (Removed `renderable.rs`).
3.  **Pipeline Manager (`rendering/pipeline_manager.rs`)**
    *   Role: Manages Vulkan `PipelineLayout`, `DescriptorSetLayout` (defining bindings for global and per-object data), and `DescriptorPool`. **No longer manages a global descriptor set.** Provides layout/pool to `BufferManager`.
    *   Key Modules: `pipeline_manager.rs`.
4.  **Buffer Manager (`rendering/buffer_manager.rs`)**
    *   Role: **Component undergoing major rework.** Responsible for managing the global projection uniform buffer and, eventually, per-entity Vulkan resources (vertex buffers, offset UBOs, pipelines, shaders, descriptor sets) based on ECS data. Uses layout/pool from `PipelineManager` to allocate per-entity descriptor sets. **Currently contains placeholder logic for resource creation/updates.**
    *   Key Modules: `buffer_manager.rs`.
5.  **Bevy ECS Components (`components/`)**
    *   Role: Defines the data associated with entities (e.g., shape, visibility, interaction properties). Replaces the old `RenderObject`/`ElementPool`.
    *   Key Modules: `shape_data.rs` (uses `Arc<Vec<Vertex>>`), `visibility.rs` (custom), `interaction.rs`. Uses `bevy_transform::components::Transform`.
6.  **Bevy Events (`events/`)**
    *   Role: Defines event types for communication between systems (e.g., user interactions, hotkey triggers). Replaces the old `EventBus`.
    *   Key Modules: `interaction_events.rs`.
7.  **Hotkey Loading (`interaction/hotkeys.rs`)**
    *   Role: Loads hotkey configurations from a TOML file. The configuration is stored in the `HotkeyResource`.
    *   Key Modules: `hotkeys.rs`.
8.  **Bevy App Core (`main.rs`)**
    *   Role: Bootstraps the application using `bevy_app::App`. Initializes Bevy plugins (`bevy_winit`, `bevy_input`, `bevy_log`, `bevy_transform`, etc., *excluding rendering plugins*). Defines Bevy systems (`Startup`, `Update`, `Last`) for lifecycle management, input handling (`interaction_system`, `hotkey_system`), state updates (`movement_system`), and application control (`app_control_system`). Spawns initial entities with components. Manages the Vulkan context (`VulkanContextResource`) and the custom renderer (`RendererResource`). Orchestrates setup, updates, rendering triggers (calling `Renderer::render`), and cleanup via systems. Handles Bevy events (`WindowResized`, `WindowCloseRequested`, `AppExit`, custom events).
9.  **Build Script (`build.rs`)**
    *   Role: Compiles shaders using `glslc` and copies runtime assets (e.g., `user/hotkeys.toml`) to the target build directory.

## Data Flow (Post Task 6.3 Step 7 Preparation)
1.  `main.rs` initializes `bevy_app::App`, adds core non-rendering plugins, registers custom components/events, and inserts initial resources (`VulkanContextResource`).
2.  Bevy `Startup` schedule runs:
    *   `setup_vulkan_system` initializes core Vulkan via `VulkanContextResource`.
    *   `create_renderer_system` creates the *actual* `Renderer`, wraps it in `RendererResource`.
    *   `setup_scene_ecs` loads hotkeys into `HotkeyResource` and spawns initial entities with `Transform`, `ShapeData`, `Visibility`, `Interaction` components.
3.  Bevy `Update` schedule runs:
    *   `InputPlugin` populates `ButtonInput<T>` resources.
    *   `interaction_system` reads input resources, queries interactive entities, performs hit-testing, and writes `EntityClicked` / `EntityDragged` events.
    *   `hotkey_system` reads input resources and `HotkeyResource`, writes `HotkeyActionTriggered` events.
    *   `movement_system` reads `EntityDragged` events and updates `Transform` components.
    *   `handle_resize_system` reads `WindowResized` events, calls `Renderer::resize_renderer`.
    *   `handle_close_request` reads `WindowCloseRequested` events, sends `AppExit` event.
    *   `app_control_system` reads `HotkeyActionTriggered` events (e.g., "CloseRequested"), sends `AppExit` event.
4.  Bevy `Last` schedule runs:
    *   `rendering_system` queries ECS for renderable entities (`GlobalTransform`, `ShapeData`, `Visibility`), collects data into `Vec<RenderCommandData>`, sorts by depth, and calls `Renderer::render` via `RendererResource`, passing the collected data.
    *   `Renderer::render` (currently):
        *   Calls `BufferManager::prepare_frame_resources` (placeholder, intended to create/update Vulkan resources based on `RenderCommandData` and return `Vec<PreparedDrawData>`).
        *   Calls `record_command_buffers`, passing the (currently empty/placeholder) `PreparedDrawData` and pipeline layout.
        *   Executes Vulkan submission/presentation logic.
    *   `cleanup_system` (runs if `AppExit` event occurred) removes `RendererResource`, calls `Renderer::cleanup` (which calls cleanup on `BufferManager` and `PipelineManager`), and `cleanup_vulkan`.

## Key Interactions (Post Task 6.3 Step 7 Preparation)
- **Bevy Systems (`main.rs`) <-> ECS (Components, Events, Resources)**: Systems query/modify components, read/write events, and access resources (`VulkanContextResource`, `RendererResource`, `HotkeyResource`, `ButtonInput`).
- **`setup_vulkan_system` -> `vulkan_setup::setup_vulkan`**: Initializes core Vulkan.
- **`create_renderer_system` -> `Renderer::new`**: Initializes custom renderer state.
- **`rendering_system` -> `Renderer::render`**: Triggers frame rendering, passing `RenderCommandData`.
- **`Renderer::render` -> `BufferManager::prepare_frame_resources`**: (Placeholder) Intended to prepare Vulkan resources.
- **`Renderer::render` -> `record_command_buffers`**: Records draw commands using `PreparedDrawData`.
- **`Renderer` <-> `BufferManager`**: Renderer owns and calls BufferManager for resource prep/cleanup.
- **`Renderer` <-> `PipelineManager`**: Renderer owns and calls PipelineManager for cleanup.
- **`BufferManager` <-> `PipelineManager`**: BufferManager uses layout/pool from PipelineManager.
- **`cleanup_system` -> `Renderer::cleanup` / `vulkan_setup::cleanup_vulkan`**: Triggers cleanup on exit.
- **`interaction_system` -> `EntityClicked`, `EntityDragged` Events**: Publishes interaction events.
- **`hotkey_system` -> `HotkeyActionTriggered` Events**: Publishes hotkey events.
- **`movement_system` <- `EntityDragged` Events**: Updates `Transform` based on drag events.
- **`app_control_system` <- `HotkeyActionTriggered` Events**: Sends `AppExit`.
- **`VulkanContext` <-> Vulkan Components**: Provides core Vulkan resources.
- **`build.rs` -> Target Directory**: Copies configuration files.

## Current Capabilities (Post Task 6.3 Step 7 Preparation)
- **Bevy Integration**: Application runs within `bevy_app`, using core non-rendering plugins (`bevy_log`, `bevy_input`, `bevy_transform`, `bevy_window`, `bevy_winit`, etc.).
- **Bevy Math**: Uses `bevy_math` types (`Vec2`, `Mat4`).
- **Bevy ECS**: Defines and uses custom components (`ShapeData` [using `Arc<Vec<Vertex>>`], `Visibility`, `Interaction`) and `bevy_transform::components::Transform`. Entities spawned at startup.
- **Bevy Events**: Defines and uses custom events (`EntityClicked`, `EntityDragged`, `HotkeyActionTriggered`) for system communication.
- **Input Processing**: Uses `bevy_input` (`ButtonInput<T>`, `CursorMoved`) via Bevy systems (`interaction_system`, `hotkey_system`). Basic click detection, dragging, and hotkey dispatching implemented.
- **Vulkan Setup**: Core context initialized via Bevy system.
- **Rendering Bridge**: Custom Vulkan context (`VulkanContextResource`) and actual custom renderer (`RendererResource`) managed as Bevy resources.
- **Rendering Path Refactoring**: Signatures and data structures (`RenderCommandData`, `PreparedDrawData`) updated to prepare for ECS data flow. `Renderable` struct removed.
- **Placeholders**: Core rendering logic (`BufferManager` resource creation/updates, `command_buffers` draw calls) still uses placeholder implementations and **does not yet draw the scene based on ECS data.**
- **Configurable Hotkeys**: Loads from TOML file into `HotkeyResource`, processed by `hotkey_system`.
- Build script for shader compilation and asset copying.
- Resize handling includes error checking for buffer mapping.

## Future Extensions
- **Complete Rendering Integration (Task 6.3 Step 7)**: Implement core logic in `BufferManager::prepare_frame_resources` to create/update Vulkan resources based on `RenderCommandData`. Update `record_command_buffers` to use `PreparedDrawData` correctly.
- **Complete Bevy Migration**: Consider Bevy State for managing application modes.
- Instancing via ECS.
- Batch operations for groups (using ECS queries/components).
- P2P networking.
- 3D rendering and AI-driven content generation.
- Performance optimization (ECS parallelism, Vulkan optimizations, `Changed<T>` detection).
- Text rendering and editing (Task 7+).
- Context menus (Task 9).
- Divider system (Task 10).

## Error Handling
- Vulkan errors checked via `ash` results. `vk-mem` assertions check memory leaks. `map_memory` errors handled in `ResizeHandler`.
- Logical errors (`HotkeyError`) use `Result` or `thiserror`.
- Hotkey file loading/parsing errors handled gracefully with defaults and logs.
- Bevy logging integrated via `LogPlugin`. Mutex poisoning handled with logs.

## Shader Integration
- Shaders (`triangle.vert.spv`, etc.) compiled by `build.rs`. **Integration with ECS components via `BufferManager` pending.**
- Shaders support orthographic projection (binding 0), object offset (binding 1), and instance offset (vertex attribute 1, binding 1).

## Dependencies
- External crates: `ash = "0.38"`, `vk-mem = "0.4"`, `ash-window = "0.13"`, `raw-window-handle = "0.6"`, `toml = "0.8"`, `thiserror = "2.0"`, **`bevy_app = "0.15"`**, **`bevy_core = "0.15"`**, **`bevy_ecs = "0.15"`**, **`bevy_log = "0.15"`**, **`bevy_utils = "0.15"`**, **`bevy_window = "0.15"`**, **`bevy_winit = "0.15"`**, **`bevy_reflect = "0.15"`**, **`bevy_input = "0.15"`**, **`bevy_time = "0.15"`**, **`bevy_diagnostic = "0.15"`**, **`bevy_a11y = "0.15"`**, **`bevy_math = "0.15"`**, **`bevy_transform = "0.15"`**. (Removed `glam`). `winit = "0.30.9"` (Used internally by `bevy_winit` and `vulkan_setup`).
- Shaders: Precompiled `.spv` files in `shaders/`.

## Notes
- **Migration State**: Application core logic migrated to Bevy ECS/Input/Events. Custom Vulkan rendering backend signatures/structs adapted for ECS data flow, but core resource management/drawing logic is **pending implementation in `BufferManager` and `command_buffers`**. Strict avoidance of `bevy_render`.
- **Known Issues**: Rendering does not reflect ECS state. `BufferManager` requires significant rework. `vulkan_setup` still uses `winit::window::Window`.
- Vulkan’s inverted Y-axis handled in `movement_system`.
- Cleanup logic uses `MutexGuard` due to `Arc` reference counting complexities during `AppExit`.