# Architecture Overview for `rusty_whip`

## Purpose
`rusty_whip` is an advanced 2D and 3D content generation application for digital entertainment production, leveraging GPU-accelerated AI diffusion/inference, multimedia creation (2D video, stills, animation, audio), and story-driven workflows, with plans for quantum-resistant P2P networking. **It uses the Bevy game engine (v0.15) for its core application structure, input handling, and ECS (Entity Component System), while strictly avoiding Bevy's rendering stack (`bevy_render`, `wgpu`, etc.) in favor of its custom Vulkan backend.**

## Core Components
1.  **Vulkan Context Management (`context/`)**
    *   Role: Initializes and manages core Vulkan resources (instance, physical device, logical device, queue, allocator, pipeline layout, current swap extent). Accessed via `VulkanContextResource`. Holds resources needed globally or across different rendering stages.
    *   Key Modules: `vulkan_context.rs`, `vulkan_setup.rs`.
2.  **Rendering Engine (`rendering/`)**
    *   Role: Orchestrates the custom Vulkan rendering pipeline per frame. Manages `BufferManager`, descriptor pool/layout handles, and Vulkan sync objects (fences, semaphores). Calls `BufferManager` to prepare resources based on ECS data and `command_buffers` to record draw calls. Handles swapchain recreation on resize, calculating projection matrix based on **logical window size with Y-flip** for Vulkan NDC. Accessed via `RendererResource`.
    *   Key Modules: `render_engine.rs`, `buffer_manager.rs`, `resize_handler.rs`, `command_buffers.rs`, `swapchain.rs`, `shader_utils.rs`.
3.  **Pipeline Manager (`rendering/pipeline_manager.rs`)**
    *   Role: **Initialization helper.** Creates Vulkan `PipelineLayout`, `DescriptorSetLayout` (defining bindings for global and per-object data), and `DescriptorPool` during application setup. These resources are then transferred to `VulkanContext` (layout) or `Renderer` (pool, set layout) for operational use and cleanup.
    *   Key Modules: `pipeline_manager.rs`.
4.  **Buffer Manager (`rendering/buffer_manager.rs`)**
    *   Role: **Core resource manager.** Manages the global projection uniform buffer and per-entity Vulkan resources (vertex buffers, transform UBOs, descriptor sets) based on ECS `RenderCommandData`. Uses layout/pool provided during initialization to allocate per-entity descriptor sets. **Caches pipelines and shader modules** based on shader paths. Updates vertex buffers based on `vertices_changed` flag in `RenderCommandData`. Updates descriptor sets **immediately per-entity** to prevent state bleed. **Lacks optimization for resource removal (despawned entities).**
    *   Key Modules: `buffer_manager.rs`.
5.  **Bevy ECS Components (`components/` and `main.rs`)**
    *   Role: Defines the data associated with entities (e.g., shape, visibility, interaction properties, background marker). Used alongside `bevy_transform::components::Transform`. Implements `bevy_reflect::Reflect`.
    *   Key Modules: `shape_data.rs` (uses `Arc<Vec<Vertex>>`), `visibility.rs` (custom), `interaction.rs`, `main.rs` (defines `BackgroundQuad`).
6.  **Bevy Events (`events/`)**
    *   Role: Defines event types for communication between systems (e.g., user interactions, hotkey triggers). Implements `bevy_reflect::Reflect`.
    *   Key Modules: `interaction_events.rs`.
7.  **Hotkey Loading (`interaction/hotkeys.rs`)**
    *   Role: Loads hotkey configurations from a TOML file. The configuration (`HotkeyConfig`) and wrapper resource (`HotkeyResource`) implement `bevy_reflect::Reflect`.
    *   Key Modules: `hotkeys.rs`.
8.  **Bevy App Core (`main.rs`)**
    *   Role: Bootstraps the application using `bevy_app::App`. Initializes Bevy plugins (`bevy_winit`, `bevy_input`, `bevy_log`, `bevy_transform`, etc., *excluding rendering plugins*). Registers reflectable types. Defines Bevy systems (`Startup`, `Update`, `Last`) for lifecycle management, ECS setup (`setup_scene_ecs`), input handling (`interaction_system`, `hotkey_system`), state updates (`movement_system`, `background_resize_system`), and application control (`app_control_system`). Spawns initial entities with components. Manages the Vulkan context (`VulkanContextResource`) and the custom renderer (`RendererResource`). Orchestrates setup, updates, rendering triggers (calling `Renderer::render`), and **synchronous cleanup on `AppExit` via `cleanup_trigger_system`**. Handles Bevy events (`WindowResized`, `WindowCloseRequested`, `AppExit`, custom events).
9.  **Build Script (`build.rs`)**
    *   Role: Compiles GLSL shaders (`.vert`, `.frag`) to SPIR-V (`.spv`) using `glslc`, copies compiled shaders and runtime assets (e.g., `user/hotkeys.toml`) to the target build directory. Tracks shader source files for recompilation.
    *   Key Modules: `build.rs`.

## Data Flow (Post Rendering Fixes)
1.  `main.rs` initializes `bevy_app::App`, adds core non-rendering plugins, registers custom components/events/resources for reflection, and inserts initial resources (`VulkanContextResource`).
2.  Bevy `Startup` schedule runs:
    *   `setup_vulkan_system` initializes core Vulkan via `VulkanContextResource`.
    *   `create_renderer_system` creates the `Renderer` (which initializes `BufferManager`, swapchain, framebuffers, command pool, sync objects, and writes **initial projection matrix with Y-flip**), wraps it in `RendererResource`. Stores `pipeline_layout` in `VulkanContextResource`.
    *   `setup_scene_ecs` loads hotkeys into `HotkeyResource` and spawns initial entities (including background with `BackgroundQuad` marker) with `Transform`, `ShapeData`, `Visibility`, `Interaction` components. Background vertices use **initial logical window size**.
3.  Bevy `Update` schedule runs:
    *   `InputPlugin` populates `ButtonInput<T>` resources.
    *   `interaction_system` reads input resources, queries interactive entities, performs hit-testing (using **Y-up world coordinates**), and writes `EntityClicked` / `EntityDragged` events.
    *   `hotkey_system` reads input resources and `HotkeyResource`, writes `HotkeyActionTriggered` events.
    *   `movement_system` reads `EntityDragged` events and updates `Transform` components (using **Y-up world coordinates**).
    *   `handle_resize_system` reads `WindowResized` events, calls `Renderer::resize_renderer` (which triggers swapchain recreation via `ResizeHandler` and updates projection matrix using **new logical size**).
    *   `background_resize_system` reads `WindowResized` events, updates `ShapeData.vertices` for the `BackgroundQuad` entity using **new logical size**.
    *   `handle_close_request` reads `WindowCloseRequested` events, sends `AppExit` event.
    *   `app_control_system` reads `HotkeyActionTriggered` events (e.g., "CloseRequested"), sends `AppExit` event.
    *   **If `AppExit` event occurred:** `cleanup_trigger_system` runs, takes ownership of `VulkanContextResource` and `RendererResource` via `world.remove_resource()`, calls `Renderer::cleanup` and `cleanup_vulkan`, then drops the owned resources.
4.  Bevy `Last` schedule runs:
    *   **If `AppExit` event did *not* occur:** `rendering_system` queries ECS for renderable entities (`GlobalTransform`, `ShapeData`, `Visibility`), checks **`Changed<ShapeData>` to set `vertices_changed` flag**, collects data into `Vec<RenderCommandData>`, sorts by depth, and calls `Renderer::render` via `RendererResource`, passing the collected data.
    *   `Renderer::render`: Waits for fence, calls `BufferManager::prepare_frame_resources` (which updates UBOs, potentially **updates/recreates vertex buffers based on `vertices_changed`**, and **updates descriptor sets immediately**), acquires image, records commands, submits, presents.

## Key Interactions (Post Rendering Fixes)
- **Bevy Systems (`main.rs`) <-> ECS (Components, Events, Resources)**: Systems query/modify components, read/write events, and access resources (`VulkanContextResource`, `RendererResource`, `HotkeyResource`, `ButtonInput`).
- **`setup_vulkan_system` -> `vulkan_setup::setup_vulkan`**: Initializes core Vulkan.
- **`create_renderer_system` -> `Renderer::new`**: Initializes custom renderer state, swapchain, framebuffers, sync objects, command pool, `BufferManager`, and **initial projection UBO (with Y-flip)**. Stores `pipeline_layout` in `VulkanContext`.
- **`rendering_system` -> `Renderer::render`**: Triggers frame rendering, passing `RenderCommandData` (including **`vertices_changed` flag**). (Conditional on `AppExit`).
- **`Renderer::render` -> `BufferManager::prepare_frame_resources`**: Prepares Vulkan resources based on ECS data, using caches, **updating vertex buffers if needed**, **updating descriptor sets immediately**.
- **`Renderer::render` -> `record_command_buffers`**: Records draw commands using `PreparedDrawData`.
- **`Renderer` <-> `BufferManager`**: Renderer owns and calls BufferManager for resource prep/cleanup.
- **`BufferManager` <-> `VulkanContext`**: BufferManager uses `pipeline_layout` and other context info.
- **`Renderer::resize_renderer` -> `ResizeHandler::resize`**: Handles window resize logic, passing **logical extent**.
- **`ResizeHandler::resize` -> `swapchain::cleanup_swapchain_resources`, `swapchain::create_swapchain`, `swapchain::create_framebuffers`**: Recreates swapchain and related resources using physical extent. Updates projection UBO using **logical extent (with Y-flip)**.
- **`background_resize_system` <- `WindowResized` Event**: Reads resize event.
- **`background_resize_system` -> `ShapeData` Component**: Modifies vertices of the background entity.
- **`cleanup_trigger_system` (on `AppExit`) -> `World::remove_resource`, `Renderer::cleanup`, `vulkan_setup::cleanup_vulkan`**: Takes ownership of resources and triggers synchronous cleanup on exit.
- **`interaction_system` -> `EntityClicked`, `EntityDragged` Events**: Publishes interaction events.
- **`hotkey_system` -> `HotkeyActionTriggered` Events**: Publishes hotkey events.
- **`movement_system` <- `EntityDragged` Events**: Updates `Transform` based on drag events.
- **`app_control_system` <- `HotkeyActionTriggered` Events**: Sends `AppExit`.
- **`VulkanContext` <-> Vulkan Components**: Provides core Vulkan resources (instance, device, allocator, pipeline layout, swap extent, etc.).
- **`build.rs` -> Target Directory**: Compiles shaders, copies `.spv` files and configuration files.

## Current Capabilities (Post Rendering Fixes)
- **Bevy Integration**: Application runs within `bevy_app`, using core non-rendering plugins (`bevy_log`, `bevy_input`, `bevy_transform`, `bevy_window`, `bevy_winit`, etc.).
- **Bevy Math**: Uses `bevy_math` types (`Vec2`, `Mat4`).
- **Bevy ECS**: Defines and uses custom components (`ShapeData` [using `Arc<Vec<Vertex>>`], `Visibility`, `Interaction`, `BackgroundQuad`) and `bevy_transform::components::Transform`. Entities spawned at startup.
- **Bevy Events**: Defines and uses custom events (`EntityClicked`, `EntityDragged`, `HotkeyActionTriggered`) for system communication.
- **Bevy Reflection**: Core components, events, and hotkey resources implement `Reflect` and are registered.
- **Bevy Change Detection**: Uses `Changed<ShapeData>` filter to trigger vertex buffer updates.
- **Input Processing**: Uses `bevy_input` (`ButtonInput<T>`, `CursorMoved`) via Bevy systems (`interaction_system`, `hotkey_system`). Basic click detection, dragging (**correct Y-axis handling for Y-up world**), and hotkey dispatching implemented. Hit detection uses **Y-up world coordinates**.
- **Vulkan Setup**: Core context initialized via Bevy system.
- **Rendering Bridge**: Custom Vulkan context (`VulkanContextResource`) and actual custom renderer (`RendererResource`) managed as Bevy resources.
- **ECS-Driven Resource Management**: `BufferManager` creates/updates Vulkan buffers (including **vertex buffer updates via change detection**) and descriptor sets based on `RenderCommandData` from the ECS. Descriptor sets updated **immediately per-entity**.
- **Resource Caching**: `BufferManager` caches Vulkan `Pipeline` and `ShaderModule` resources.
- **Rendering Path**: Data flows from ECS (`rendering_system`) through `Renderer` to `BufferManager` (resource prep) and `command_buffers` (draw recording). Synchronization corrected using fences. **Projection matrix uses logical window size and Y-flip.**
- **Configurable Hotkeys**: Loads from TOML file into `HotkeyResource`, processed by `hotkey_system`.
- **Build Script**: Compiles GLSL shaders to SPIR-V, copies assets, tracks shader changes.
- **Resize Handling**: Correctly handles window resizing, swapchain recreation, framebuffer updates, **projection matrix updates (using logical size)**, and **dynamic background vertex updates**.
- **Visual Output**: Functional 2D rendering of shapes based on ECS data. Background dynamically resizes. Objects positioned correctly according to **Y-up world coordinates**.
- **Robust Shutdown**: Application exit sequence correctly cleans up Vulkan resources via `cleanup_trigger_system`, preventing leaks and assertions.

## Future Extensions
- **Rendering Optimization**: Implement resource removal for despawned entities (using `RemovedComponents`).
- **Hit Detection**: Improve Z-sorting/picking logic in `interaction_system` for overlapping objects. Consider more robust bounding box checks if rotation/scaling is added.
- **Complete Bevy Migration**: **Refactor framework logic into Bevy Plugins.** Consider Bevy State for managing application modes.
- Instancing via ECS.
- Batch operations for groups (using ECS queries/components).
- P2P networking.
- 3D rendering and AI-driven content generation.
- Text rendering and editing (Task 7+).
- Context menus (Task 9).
- Divider system (Task 10).

## Error Handling
- Vulkan errors checked via `ash` results. Validation layers used for debugging. `vk-mem` assertions check memory leaks (resolved shutdown assertion). `map_memory` errors handled. Persistent map access uses `get_allocation_info`.
- Logical errors (`HotkeyError`) use `Result` or `thiserror`.
- Hotkey file loading/parsing errors handled gracefully with defaults and logs.
- Bevy logging integrated via `LogPlugin`. Mutex poisoning handled with logs.
- Renderer checks for `None` resources during shutdown to prevent panics.

## Shader Integration
- GLSL shaders (`.vert`, `.frag`) in `shaders/` compiled to SPIR-V (`.spv`) by `build.rs`. Compiled `.spv` files copied to target directory.
- Shaders loaded and cached by `BufferManager` based on `ShapeData` component paths.
- Shaders support orthographic projection (binding 0) and object transformation matrix (binding 1). Only vertex position (location 0) is currently used as input. **`background.vert` ignores object transform.**

## Dependencies
- External crates: `ash = "0.38"`, `vk-mem = "0.4"`, `ash-window = "0.13"`, `raw-window-handle = "0.6"`, `toml = "0.8"`, `thiserror = "2.0"`, `bevy_app = "0.15"`, `bevy_core = "0.15"`, `bevy_ecs = "0.15"`, `bevy_log = "0.15"`, `bevy_utils = "0.15"`, `bevy_window = "0.15"`, `bevy_winit = "0.15"`, `bevy_reflect = "0.15"`, `bevy_input = "0.15"`, `bevy_time = "0.15"`, `bevy_diagnostic = "0.15"`, `bevy_a11y = "0.15"`, `bevy_math = "0.15"`, `bevy_transform = "0.15"`, `winit = "0.30.9"`.
- Build dependencies: `walkdir = "2"`.
- Shaders: Precompiled `.spv` files in target directory.

## Notes
- **Migration State**: Application core logic uses Bevy ECS/Input/Events/Reflection. Custom Vulkan rendering backend implements ECS-driven resource management with pipeline/shader caching, corrected synchronization, and **dynamic vertex buffer updates**. Strict avoidance of `bevy_render`.
- **Coordinate System**: Uses a **Y-up world coordinate system** (origin bottom-left). Projection matrix includes a Y-flip to match Vulkan's default Y-down NDC space. Input and movement systems handle coordinate conversions correctly.
- **Known Issues**: `BufferManager` lacks resource removal for despawned entities. Hit detection Z-sorting needs review for overlapping objects. `vulkan_setup` still uses `winit::window::Window` directly.
- **Cleanup Logic**: Synchronous cleanup on `AppExit` is handled by `cleanup_trigger_system`.