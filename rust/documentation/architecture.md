# Architecture Overview for `rusty_whip`

## Purpose
`rusty_whip` is an advanced 2D and 3D content generation application for digital entertainment production, leveraging GPU-accelerated AI diffusion/inference, multimedia creation (2D video, stills, animation, audio), and story-driven workflows, with plans for quantum-resistant P2P networking. **It uses the Bevy game engine (v0.15) for its core application structure, input handling, and ECS (Entity Component System), while strictly avoiding Bevy's rendering stack (`bevy_render`, `wgpu`, etc.) in favor of its custom Vulkan backend.**

## Core Components
1.  **Vulkan Context Management (`context/`)**
    *   Role: Initializes and manages core Vulkan resources (instance, physical device, logical device, queue, allocator, shape/text pipeline layouts, current swap extent, **depth buffer resources**, **debug messenger**). Accessed via `VulkanContextResource`. Holds resources needed globally or across different rendering stages.
    *   Key Modules: `vulkan_context.rs`, `vulkan_setup.rs`.
2.  **Rendering Engine (`rendering/render_engine.rs`)**
    *   Role: Orchestrates the custom Vulkan rendering pipeline per frame. Manages `BufferManager` (for shapes) and sync objects. Handles resize via `ResizeHandler`. Calls `BufferManager` (shapes) and `command_buffers` (shapes, text) to record draw calls based on ECS data and prepared draw data. Accessed via `RendererResource`.
    *   Key Modules: `render_engine.rs`.
3.  **Pipeline Manager (`rendering/pipeline_manager.rs`)**
    *   Role: **Initialization helper.** Creates Vulkan `PipelineLayout`s (shape, text), `DescriptorSetLayout`s (per-entity, atlas), and a shared `DescriptorPool` during application setup. These resources are then transferred to `VulkanContext` (layouts) or `Renderer` (pool, set layouts) for operational use and cleanup.
    *   Key Modules: `pipeline_manager.rs`.
4.  **Buffer Manager (`rendering/buffer_manager.rs`)**
    *   Role: **Shape resource manager.** Manages per-entity Vulkan resources (vertex buffers, transform UBOs, descriptor sets) for shapes based on ECS `RenderCommandData`. Uses per-entity layout/pool provided during initialization. Caches shape pipelines (including depth state) and shader modules. Updates vertex buffers based on `vertices_changed` flag. Updates transform UBOs and descriptor sets every frame for existing entities. **Lacks optimization for resource removal (despawned entities).**
    *   Key Modules: `buffer_manager.rs`.
5.  **Text Handling (`rendering/font_server.rs`, `rendering/glyph_atlas.rs`, `components/text_data.rs`, `components/text_layout.rs`, `plugins/core.rs`)**
    *   Role: Manages font loading (`font_server`), text layout/shaping (`text_layout_system`), glyph caching/packing/upload (`glyph_atlas` using `rectangle-pack`), **per-entity text Vulkan resource management** (`TextRenderData` component holding vertex buffer, transform UBO, descriptor set 0, managed by `prepare_text_rendering_system`), **shared text resource management** (`TextRenderingResources` holding shared pipeline and global atlas descriptor set 1), and **text rendering preparation** (`rendering_system` collects data from `TextRenderData`).
    *   Key Modules: `font_server.rs`, `glyph_atlas.rs`, `text_data.rs`, `text_layout.rs`. Systems in `plugins/core.rs` orchestrate layout, resource creation/management, and rendering preparation.
6.  **Bevy ECS Components (`components/`)**
    *   Role: Defines the data associated with entities (e.g., shape, text, visibility, interaction properties, **text render resources**). Used alongside `bevy_transform::components::Transform`. Implements `bevy_reflect::Reflect` where possible.
    *   Key Modules: `shape_data.rs`, `visibility.rs`, `interaction.rs`, `text_data.rs`, `text_layout.rs` (defines `TextLayoutOutput` and `TextRenderData`).
7.  **Bevy Events (`events/`)**
    *   Role: Defines event types for communication between systems (e.g., user interactions, hotkey triggers). Implements `bevy_reflect::Reflect`.
    *   Key Modules: `interaction_events.rs`.
8.  **Hotkey Loading (`interaction/hotkeys.rs`)**
    *   Role: Defines logic for loading hotkey configurations from a TOML file (`HotkeyConfig`).
    *   Key Modules: `hotkeys.rs`.
9.  **Framework Plugins (`plugins/`)**
    *   Role: Encapsulate core framework logic into modular Bevy plugins for better organization and reusability. Define `SystemSet`s (`CoreSet`, `InteractionSet`, etc.) to manage execution order.
    *   Key Modules: `core.rs` (Vulkan/Renderer/Text setup, text layout, **text resource management (shared & per-entity)**, rendering system, resize, cleanup), `interaction.rs` (Input processing, hotkey loading/dispatch, window close), `movement.rs` (Default drag movement), `bindings.rs` (Default hotkey actions).
10. **Bevy App Core (`main.rs`)**
    *   Role: Bootstraps the application using `bevy_app::App`. Initializes core Bevy plugins and framework plugins. Inserts initial `VulkanContextResource`. Defines and schedules **application-specific** systems (e.g., `setup_scene_ecs`, `background_resize_system`). Spawns initial entities with components (including sample text).
11. **Build Script (`build.rs`)**
    *   Role: Compiles GLSL shaders (`.vert`, `.frag`) to SPIR-V (`.spv`) using `glslc`, copies compiled shaders and runtime assets (e.g., `user/hotkeys.toml`) to the target build directory. Tracks shader source files for recompilation.
    *   Key Modules: `build.rs`.

## Data Flow (Post Per-Entity Text Resource Refactor)
1.  `main.rs` initializes `bevy_app::App`, adds core non-rendering Bevy plugins, inserts `VulkanContextResource`, and adds framework plugins (`GuiFrameworkCorePlugin`, `GuiFrameworkInteractionPlugin`, etc.).
2.  Bevy `Startup` schedule runs (ordered by `CoreSet`):
    *   `setup_vulkan_system` initializes Vulkan context (including debug messenger).
    *   `create_renderer_system` creates `RendererResource`.
    *   `create_glyph_atlas_system`, `create_font_server_system`, `create_swash_cache_system` create text foundation resources.
    *   `create_global_ubo_system` creates `GlobalProjectionUboResource`.
    *   `create_text_rendering_resources_system` creates shared `TextRenderingResources` (pipeline, atlas descriptor set).
    *   `load_hotkeys_system` (Interaction Plugin) loads hotkeys.
    *   `setup_scene_ecs` (main.rs) spawns initial entities (shapes and text).
3.  Bevy `Update` schedule runs (ordered by `CoreSet`):
    *   `text_layout_system` queries `Text`, uses `FontServer`, `SwashCache`, calls `GlyphAtlas::add_glyph`, and updates `TextLayoutOutput`.
    *   `prepare_text_rendering_system` queries `Changed<TextLayoutOutput>`, creates/updates per-entity `TextRenderData` components (vertex buffer, transform UBO, descriptor set 0).
    *   `handle_resize_system` handles `WindowResized`, updates `GlobalProjectionUboResource`, and calls `Renderer::resize_renderer`.
    *   `interaction_system` processes input, sends `EntityClicked`/`EntityDragged`/`HotkeyActionTriggered` events.
    *   `movement_system` reads `EntityDragged` and updates `Transform`.
    *   `app_control_system` reads `HotkeyActionTriggered`.
    *   `background_resize_system` handles `WindowResized` for the background quad.
4.  Bevy `Last` schedule runs:
    *   `rendering_system` queries ECS for shapes (`ShapeData`) and text (`TextRenderData`), collects `RenderCommandData` (shapes) and `PreparedTextDrawData` (text). Calls `Renderer::render`.
    *   `Renderer::render` locks resources, waits for fence, calls `BufferManager::prepare_frame_resources` (updates shape buffers/UBOs/descriptors), acquires swapchain image, calls `record_command_buffers`, submits commands, and presents.
    *   `record_command_buffers` records draw calls for shapes and text using prepared data (binding correct pipelines, vertex buffers, descriptor sets). Includes depth clear.
    *   `cleanup_trigger_system` runs on `AppExit`, cleaning up **per-entity `TextRenderData` resources**, then framework resources (`Renderer`, `TextRenderingResources`, `GlobalProjectionUboResource`, `GlyphAtlas`, `VulkanContext`).

## Key Interactions (Post Per-Entity Text Resource Refactor)
- **Plugins <-> Bevy App**: Plugins register components/resources/events and add systems to schedules.
- **Plugins <-> System Sets**: Plugins define and use `SystemSet`s (`CoreSet`, `InteractionSet`, etc.) to configure execution order.
- **App Systems (`main.rs`) <-> Plugin Sets**: Application systems order themselves relative to plugin sets.
- **Interaction Plugin -> Events**: Writes interaction/hotkey/exit events.
- **Movement Plugin <- Events**: Reads `EntityDragged`, writes `Transform`.
- **Bindings Plugin <- Events**: Reads `HotkeyActionTriggered`.
- **Core Plugin (Text Layout)**: `text_layout_system` reads `Text`, `Transform`; uses `FontServerResource`, `SwashCacheResource`, `VulkanContextResource`; reads/writes `GlyphAtlasResource`; writes `TextLayoutOutput`.
- **Core Plugin (Text Prep)**: `prepare_text_rendering_system` reads `Changed<TextLayoutOutput>`, `GlobalTransform`; uses `GlobalProjectionUboResource`, `VulkanContextResource`, `RendererResource`; creates/updates `TextRenderData` components.
- **Core Plugin (Rendering)**: `rendering_system` reads `ShapeData`, `TextRenderData`, `GlobalTransform`; uses `TextRenderingResources`; calls `Renderer::render`.
- **Core Plugin (Cleanup)**: `cleanup_trigger_system` queries/cleans up `TextRenderData`, then cleans up `Renderer`, `TextRenderingResources`, `GlobalProjectionUboResource`, `GlyphAtlas`, `VulkanContext`.
- **Renderer <-> BufferManager**: Renderer calls BufferManager for **shape** resource prep/cleanup.
- **Renderer <-> Command Buffers**: `Renderer::render` calls `record_command_buffers`, passing prepared shape and text draw data.
- **Renderer <-> ResizeHandler**: `Renderer::resize_renderer` calls `ResizeHandler::resize`.
- **BufferManager <-> VulkanContext**: Uses context for device, allocator, layouts, render pass.
- **BufferManager <-> GlobalProjectionUboResource**: Reads UBO handle for descriptor updates.
- **GlyphAtlas <-> VulkanContext**: Uses context for image/sampler creation and upload command execution.
- **Core Plugin <-> ResizeHandler**: `handle_resize_system` calls `Renderer::resize_renderer`.
- **App Systems (`main.rs`) <-> Components**: `background_resize_system` modifies `ShapeData`.
- **VulkanContext <-> Vulkan Components**: Provides core Vulkan resources, including depth buffer and debug messenger.
- **Build Script -> Target Directory**: Compiles shaders, copies assets.

## Current Capabilities (Post Per-Entity Text Resource Refactor)
- **Bevy Integration**: Application runs within `bevy_app`, using core non-rendering plugins.
- **Modular Framework**: Core rendering, interaction, text foundation, and default behaviors encapsulated in Bevy Plugins.
- **System Set Ordering**: Explicit execution order defined using Bevy `SystemSet`s.
- **Bevy Math**: Uses `bevy_math` types (`Vec2`, `Mat4`).
- **Bevy ECS**: Defines and uses custom components (`ShapeData`, `Visibility`, `Interaction`, `BackgroundQuad`, `Text`, `TextLayoutOutput`, `TextRenderData`) and `bevy_transform::components::Transform`.
- **Bevy Events**: Defines and uses custom events (`EntityClicked`, `EntityDragged`, `HotkeyActionTriggered`) for system communication.
- **Bevy Reflection**: Core framework components/events/resources implement `Reflect` where feasible and are registered.
- **Input Processing**: Uses `bevy_input` via `GuiFrameworkInteractionPlugin`. Basic click detection, dragging, and hotkey dispatching implemented. Hit detection uses **Y-up world coordinates**. Dragging updates `Transform` correctly.
- **Vulkan Setup**: Core context initialized via `GuiFrameworkCorePlugin`. Separate pipeline layouts for shapes and text created. **Depth buffer created. Debug messenger enabled (debug builds).**
- **Rendering Bridge**: Custom Vulkan context (`VulkanContextResource`) and renderer (`RendererResource`) managed as Bevy resources.
- **ECS-Driven Resource Management**:
    - `BufferManager` creates/updates Vulkan resources for **shapes** (vertex buffer, transform UBO, descriptor set). Correctly handles initial spawn vs. updates.
    - `GlyphAtlas` manages Vulkan texture for glyphs (packing, rasterization data input, upload).
    - **`TextRenderData` component holds per-entity text Vulkan resources (vertex buffer, transform UBO, descriptor set 0).** Managed by `prepare_text_rendering_system`.
    - **`TextRenderingResources` holds shared text pipeline and global atlas descriptor set 1.** Managed by `create_text_rendering_resources_system`.
- **Resource Caching**: `BufferManager` caches shape pipelines/shaders. `GlyphAtlas` caches glyph locations/UVs.
- **Rendering Path**: Data flows from ECS (`rendering_system` queries `ShapeData` and `TextRenderData`) -> `Renderer` (shape prep via `BufferManager`) -> `record_command_buffers` (shape and text draw recording using prepared data). Synchronization corrected. Projection matrix uses logical window size, Y-flip, and wide depth range. **Depth testing enabled.**
- **Configurable Hotkeys**: Loads from TOML file into `HotkeyResource` via `GuiFrameworkInteractionPlugin`.
- **Build Script**: Compiles GLSL shaders to SPIR-V, copies assets, tracks shader changes.
- **Resize Handling**: Correctly handles window resizing, swapchain/framebuffer/depth buffer recreation, projection matrix updates, and dynamic background vertex updates.
- **Visual Output**: Functional 2D rendering of **shapes** and **static, baseline-aligned text** based on ECS data. Background dynamically resizes. Objects positioned and layered correctly according to **Y-up world coordinates and Z-depth**. Alpha blending enabled for text.
- **Text Foundation**:
    - Text component definition (`Text`, `TextVertex`).
    - Font loading and management (`FontServer`, `FontServerResource`).
    - Glyph atlas resource management (`GlyphAtlas`, `GlyphAtlasResource`), including packing/upload.
    - CPU-side text layout system (`text_layout_system` using `cosmic-text`).
    - Intermediate text layout component (`TextLayoutOutput`).
    - **Per-entity text rendering resource management (`prepare_text_rendering_system`, `TextRenderData`).**
    - Text shaders (`text.vert`, `text.frag`) created and used by text pipeline.
    - Text-specific Vulkan layouts and pipeline created (including compatible depth state).
- **Robust Shutdown**: Application exit sequence correctly cleans up **per-entity text resources** and then shared Vulkan resources via `cleanup_trigger_system`.

## Future Extensions
- **Text Handling**: Implement text editing (`yrs`), context menus (Task 9). Improve text rendering quality (e.g., SDF).
- **Rendering Optimization**: Implement resource removal for despawned entities (using `RemovedComponents`). Optimize text vertex buffer updates (process only changed entities).
- **Hit Detection**: Improve Z-sorting/picking logic in `interaction_system` for overlapping objects.
- **Bevy State Integration**: Consider Bevy State for managing application modes.
- Instancing via ECS.
- Batch operations for groups (using ECS queries/components).
- P2P networking.
- 3D rendering and AI-driven content generation.
- Divider system (Task 10).

## Error Handling
- Vulkan errors checked via `ash` results. **Validation layers enabled in debug builds via debug messenger.** `vk-mem` assertions check memory leaks. `map_memory` errors handled. Persistent map access uses `get_allocation_info`.
- Logical errors (`HotkeyError`) use `Result` or `thiserror`.
- Hotkey file loading/parsing errors handled gracefully with defaults and logs.
- Bevy logging integrated via `LogPlugin`. Mutex poisoning handled with logs.
- Renderer/Atlas checks for `None` resources during shutdown to prevent panics. Startup systems now `panic!` on critical setup errors.
- **Cleanup of per-entity `TextRenderData` requires careful handling of Vulkan resources within ECS.**

## Shader Integration
- GLSL shaders (`.vert`, `.frag`) in `shaders/` compiled to SPIR-V (`.spv`) by `build.rs`. Compiled `.spv` files copied to target directory.
- Shape shaders loaded and cached by `BufferManager`. Support orthographic projection (Set 0, Binding 0) and object transformation matrix (Set 0, Binding 1). Use vertex position (location 0).
- Text shaders (`text.vert`, `text.frag`) loaded by `create_text_rendering_resources_system` for text pipeline creation. `text.vert` uses Set 0 (Binding 0: projection, Binding 1: transform). `text.frag` uses Set 1 (Binding 0: glyph atlas sampler).

## Dependencies
- External crates: `ash = "0.38"`, `vk-mem = "0.4"`, `ash-window = "0.13"`, `raw-window-handle = "0.6"`, `toml = "0.8"`, `thiserror = "2.0"`, `bevy_app = "0.15"`, `bevy_core = "0.15"`, `bevy_ecs = "0.15"`, `bevy_log = "0.15"`, `bevy_utils = "0.15"`, `bevy_window = "0.15"`, `bevy_winit = "0.15"`, `bevy_reflect = "0.15"`, `bevy_input = "0.15"`, `bevy_time = "0.15"`, `bevy_diagnostic = "0.15"`, `bevy_a11y = "0.15"`, `bevy_math = "0.15"`, `bevy_transform = "0.15"`, `bevy_color = "0.15"`, `winit = "0.30.9"`, `cosmic-text = "0.14"`, `fontdb = "0.23"`, `swash = "0.2"`, `rectangle-pack = "0.4"`.
- Build dependencies: `walkdir = "2"`.
- Shaders: Precompiled `.spv` files in target directory.

## Notes
- **Framework Structure**: Core logic refactored into Bevy Plugins using System Sets for ordering. Application logic resides in `main.rs`.
- **Coordinate System**: Uses a **Y-up world coordinate system** (origin bottom-left). Projection matrix includes a Y-flip to match Vulkan's default Y-down NDC space. Input and movement systems handle coordinate conversions correctly. Text vertex generation correctly transforms relative glyph coordinates to world space.
- **Depth**: Depth testing is enabled (`CompareOp::LESS`). Entities with lower Z values are rendered "on top". Projection matrix uses a wide depth range (`1024.0` to `0.0`).
- **Known Issues**:
    - `BufferManager` lacks resource removal for despawned entities.
    - Hit detection Z-sorting needs review (though depth testing helps).
    - `vulkan_setup` still uses `winit::window::Window` directly.
    - `rendering_system` temporarily updates text transform UBOs; this should be moved to a dedicated system.
- **Cleanup Logic**: Synchronous cleanup on `AppExit` is handled by `cleanup_trigger_system` within the `GuiFrameworkCorePlugin`, running in the `Last` schedule. It now cleans up per-entity `TextRenderData` before shared resources.