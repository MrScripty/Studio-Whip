# Architecture Overview for `rusty_whip`

## Purpose
`rusty_whip` is an advanced 2D and 3D content generation application for digital entertainment production, leveraging GPU-accelerated AI diffusion/inference, multimedia creation (2D video, stills, animation, audio, screen writing), and story-driven workflows, with plans for quantum-resistant P2P networking. **It uses the Bevy game engine (v0.15) for its core application structure, input handling, ECS (Entity Component System), and hierarchy, while strictly avoiding Bevy's rendering stack (`bevy_render`, `wgpu`, etc.) in favor of its custom Vulkan backend.**

## Core Components
1.  **Vulkan Context Management (`context/`)**
    *   Role: Initializes and manages core Vulkan resources (instance, physical device, logical device, queue, allocator, shape/text pipeline layouts, current swap extent, depth buffer resources including `depth_image_allocation`, command pool, debug messenger, **`debug_utils::Device` extension struct**). Accessed via `VulkanContextResource`. Holds resources needed globally or across different rendering stages. **Includes helper (`set_debug_object_name`) for naming Vulkan objects.**
    *   Key Modules: `vulkan_context.rs`, `vulkan_setup.rs`.
2.  **Rendering Engine (`rendering/render_engine.rs`)**
    *   Role: Orchestrates the custom Vulkan rendering pipeline per frame. Accesses `BufferManagerResource` (for shapes/cursors) and manages `TextRenderer` (for text). Manages sync objects (semaphores, fence). Handles resize via `ResizeHandler`. Calls `BufferManager` (via resource) and `TextRenderer` (direct member) to prepare draw data, then calls `command_buffers` to record draw calls. Resets the active command buffer each frame. **Correctly waits on `image_available_semaphore` before queue submission.** Accessed via `RendererResource`.
    *   Key Modules: `render_engine.rs`.
3.  **Text Renderer (`rendering/text_renderer.rs`)**
    *   Role: Manages per-entity Vulkan resources for text rendering (`HashMap<Entity, TextRenderData>`) based on `TextLayoutInfo` received from the ECS. Prepares `PreparedTextDrawData` for the `Renderer`. **Receives debug utils handle for naming created resources.**
    *   Key Modules: `text_renderer.rs`.
4.  **Pipeline Manager (`rendering/pipeline_manager.rs`)**
    *   Role: **Initialization helper.** Creates Vulkan `PipelineLayout`s (shape, text), `DescriptorSetLayout`s (per-entity, atlas), and a shared `DescriptorPool` during application setup. Shape pipeline layout includes a push constant range for color. These resources are then transferred to `VulkanContext` (layouts) or `Renderer` (pool, set layouts) for operational use and cleanup.
    *   Key Modules: `pipeline_manager.rs`.
5.  **Buffer Manager (`rendering/buffer_manager.rs`)**
    *   Role: **Shape/Cursor resource manager.** Manages per-entity Vulkan resources (vertex buffers, transform UBOs, descriptor sets) for shapes and cursor visuals based on ECS `RenderCommandData`. Uses per-entity layout/pool provided during initialization. Caches the single shape pipeline (including depth state). Updates vertex buffers based on `vertices_changed` flag. Updates transform UBOs and descriptor sets every frame for existing entities. Implements resource removal for despawned entities via `remove_entity_resources` method, triggered by `buffer_manager_despawn_cleanup_system`. **Correctly creates vertex buffers with appropriate size and usage flags. Names created resources using debug utils.** Accessed via `BufferManagerResource`.
    *   Key Modules: `buffer_manager.rs`.
6.  **Text Handling (`rendering/font_server.rs`, `rendering/glyph_atlas.rs`, `components/text_data.rs`, `components/text_layout.rs`, `plugins/core.rs`, `plugins/interaction.rs`, `rendering/text_renderer.rs`)**
    *   Role: Manages font loading (`font_server`), text layout/shaping (`text_layout_system`), glyph caching/packing/upload (`glyph_atlas` using `rectangle-pack`, **now with robust staging buffer cleanup**), per-entity text Vulkan resource management (by `TextRenderer`), shared text rendering resource management (`TextRenderingResources`), text rendering preparation (by `TextRenderer`), caching of layout results (`TextBufferCache`), visual cursor management (`manage_cursor_visual_system`, `update_cursor_transform_system`), text editing logic (`text_editing_system`), and text drag selection logic (`text_drag_selection_system`). **Names created resources (atlas image) using debug utils.**
    *   Key Modules: `font_server.rs`, `glyph_atlas.rs`, `text_data.rs`, `text_layout.rs`, `text_renderer.rs`. Systems in `plugins/core.rs` and `plugins/interaction.rs` orchestrate these aspects.
7.  **Bevy ECS Components (`components/`)**
    *   Role: Defines the data associated with entities (e.g., shape, text, visibility, interaction properties, text editing state (`Focus`, `CursorState`, `TextSelection`), cursor visual marker (`CursorVisual`), layout cache (`TextBufferCache`)). `TextRenderData` is a component definition but used internally by `TextRenderer`'s cache. Used alongside `bevy_transform::components::Transform` and `bevy_hierarchy` components (`Parent`, `Children`). Implements `bevy_reflect::Reflect` where possible.
    *   Key Modules: `shape_data.rs`, `visibility.rs`, `interaction.rs`, `text_data.rs` (defines `Text`, `EditableText`, `Focus`, `CursorState`, `CursorVisual`, `TextSelection`), `text_layout.rs` (defines `TextLayoutOutput`, `TextRenderData`, `TextBufferCache`).
8.  **Bevy Events (`events/`)**
    *   Role: Defines event types for communication between systems (e.g., user interactions, hotkey triggers, text focus changes, YRS text changes). Implements `bevy_reflect::Reflect`.
    *   Key Modules: `interaction_events.rs` (defines `EntityClicked`, `EntityDragged`, `HotkeyActionTriggered`, `YrsTextChanged`, `TextFocusChanged`).
9.  **Interaction Logic (`interaction/`)**
    *   Role: Defines hotkey loading (`hotkeys.rs`), text interaction utilities (`utils.rs`), text drag selection logic (`text_drag.rs`), and text editing logic (`text_editing.rs`).
    *   Key Modules: `hotkeys.rs`, `utils.rs`, `text_drag.rs`, `text_editing.rs`.
10. **Framework Plugins (`plugins/`)**
    *   Role: Encapsulate core framework logic into modular Bevy plugins for better organization and reusability. Define `SystemSet`s (`CoreSet`, `InteractionSet`, etc.) to manage execution order.
    *   Key Modules:
        *   `core.rs`: Vulkan/Renderer/Text/BufferManager setup, text layout & caching, cursor management, rendering system, resize, cleanup. Includes `buffer_manager_despawn_cleanup_system`. **Names created resources (Global UBO, Shared Text VB) using debug utils.**
        *   `interaction.rs`: Input processing (mouse clicks/drags, keyboard), hotkey loading/dispatch, window close. Manages text focus/state components.
        *   `movement.rs`: Default drag movement.
        *   `bindings.rs`: Default hotkey actions.
11. **Bevy App Core (`main.rs`)**
    *   Role: Bootstraps the application using `bevy_app::App`. Initializes core Bevy plugins (including `HierarchyPlugin`) and framework plugins. Inserts initial `VulkanContextResource`, `YrsDocResource`. Defines and schedules application-specific systems. Spawns initial entities.
12. **Build Script (`build.rs`)**
    *   Role: Compiles GLSL shaders to SPIR-V, copies assets.
    *   Key Modules: `build.rs`.
13. **Yrs Integration (`lib.rs`, `main.rs`, `plugins/core.rs`, `plugins/interaction.rs`)**
    *   Role: Manages shared text data using `yrs`.
    *   Key Modules: `lib.rs`, `main.rs`, `plugins/core.rs`, `plugins/interaction.rs`.
14. **Swapchain Management (`rendering/swapchain.rs`)**
    *   Role: Manages Vulkan swapchain creation/recreation, associated image views, depth buffer resources. **Calls `create_framebuffers` internally after setup.** Cleans up these resources. **Names created depth image using debug utils.**
    *   Key Modules: `swapchain.rs` (via `create_swapchain` function).
15. **Framebuffer/RenderPass/CommandBuffer Setup (`rendering/swapchain.rs`)**
    *   Role: Creates the render pass (if needed), framebuffers (with depth attachment), and allocates/frees command buffers per swapchain image. Called by `Renderer::new` (via `create_swapchain`) and `ResizeHandler::resize`.
    *   Key Modules: `swapchain.rs` (via `create_framebuffers` function).
16. **Command Buffer Recording (`rendering/command_buffers.rs`)**
    *   Role: Records draw calls into a specific command buffer provided by the `Renderer`.
    *   Key Modules: `command_buffers.rs`.

## Data Flow (Updated for Initialization Fixes & Debug Naming)
1.  `main.rs` initializes `bevy_app::App`, adds core non-rendering Bevy plugins, inserts `VulkanContextResource`, `YrsDocResource`, and adds framework plugins.
2.  Bevy `Startup` schedule runs (ordered by `CoreSet`, `InteractionSet`):
    *   `setup_vulkan_system` initializes Vulkan context (instance, device, allocator, **debug utils loader/device**).
    *   `create_renderer_system` creates `RendererResource` and `BufferManagerResource`. Calls `Renderer::new`.
        *   `Renderer::new` creates command pool. Calls `create_swapchain`.
            *   `create_swapchain` creates swapchain, images, views, depth buffer (**names depth image/mem**). Calls `create_framebuffers`.
                *   `create_framebuffers` creates render pass (if needed), framebuffers, allocates command buffers.
        *   `Renderer::new` creates `PipelineManager` (gets layouts/pool), creates/inserts `BufferManagerResource`, creates `TextRenderer`. Creates sync objects.
    *   `create_glyph_atlas_system` creates `GlyphAtlasResource` (**names atlas image/mem**).
    *   `create_font_server_system`, `create_swash_cache_system` create text foundation resources.
    *   `create_global_ubo_system` creates `GlobalProjectionUboResource` (**names UBO/mem**).
    *   `create_text_rendering_resources_system` creates shared `TextRenderingResources` (**names shared VB/mem**).
    *   `load_hotkeys_system` loads hotkeys.
    *   `setup_scene_ecs` spawns initial entities.
3.  Bevy `Update` schedule runs (ordered by `InteractionSet`, `CoreSet`):
    *   Input/Interaction systems run (`interaction_system`, `text_editing_system`, etc.).
    *   `manage_cursor_visual_system`, `apply_deferred`, `text_layout_system` (calls `GlyphAtlas::add_glyph` which **uploads via staging buffer with robust cleanup**), `update_cursor_transform_system` run.
    *   `handle_resize_system` handles `WindowResized`, updates UBO, calls `Renderer::resize_renderer` (which calls `ResizeHandler::resize` -> `cleanup_swapchain_resources` -> `create_swapchain` -> `create_framebuffers`).
    *   `buffer_manager_despawn_cleanup_system` handles despawns.
    *   Movement/Binding systems run.
4.  Bevy `Last` schedule runs:
    *   `rendering_system` queries ECS, calls `Renderer::render`.
    *   `Renderer::render`:
        *   Waits for fence.
        *   Acquires swapchain image.
        *   Locks context.
        *   Calls `BufferManager::prepare_frame_resources` (**names new entity resources**).
        *   Calls `TextRenderer::prepare_text_draws` (**names new entity resources**).
        *   Resets command buffer.
        *   Calls `record_command_buffers`.
        *   Submits queue (**waits on image available semaphore**, signals render finished semaphore, signals fence).
        *   Unlocks context.
        *   Presents image (waits on render finished semaphore).
    *   `record_command_buffers` records draw calls.
    *   `cleanup_trigger_system` runs on `AppExit`, cleaning up ECS components, then framework resources (`Renderer`, `BufferManager`, `TextRenderingResources`, `GlobalProjectionUboResource`, `GlyphAtlas`, `VulkanContext`) in **corrected order**.

## Key Interactions (Updated for Debug Naming & Cleanup)
- **Plugins <-> Bevy App**: Standard Bevy plugin integration.
- **Plugins <-> System Sets**: Standard Bevy system set ordering.
- **App Systems (`main.rs`) <-> Plugin Sets**: Standard Bevy system set ordering.
- **Interaction Plugin -> Events/ECS/YRS**: Standard interaction logic.
- **Movement/Bindings Plugins <- Events**: Standard event handling.
- **Core Plugin (Text Layout)**: `text_layout_system` uses `VulkanContextResource` (briefly for handles), `GlyphAtlasResource` (longer lock), `FontServerResource`, `SwashCacheResource`. Calls `GlyphAtlas::add_glyph` (passing device, queue, pool, allocator). Writes `TextLayoutOutput`, `TextBufferCache`.
- **Core Plugin (Cursor Management)**: Standard cursor logic.
- **Core Plugin (BufferManager Despawn Cleanup)**: Standard despawn handling.
- **Core Plugin (Rendering)**: `rendering_system` reads ECS, uses resources, calls `Renderer::render`.
- **Core Plugin (Cleanup)**: `cleanup_trigger_system` takes resources, calls cleanup methods in specific order, calls `cleanup_vulkan`.
- **Renderer <-> BufferManagerResource**: `Renderer::render` uses resource. `Renderer::new` creates/inserts resource.
- **Renderer <-> TextRenderer**: Renderer calls `prepare_text_draws` (passing debug handle), `cleanup`.
- **Renderer <-> Swapchain/Resize**: `Renderer::new` calls `create_swapchain`. `Renderer::resize_renderer` calls `ResizeHandler::resize`.
- **ResizeHandler <-> Swapchain**: `ResizeHandler::resize` calls `cleanup_swapchain_resources`, `create_swapchain`.
- **Swapchain Module <-> VulkanContext**: `create_swapchain` creates swapchain/images/views/depth. `create_framebuffers` creates renderpass/framebuffers/command buffers. `cleanup_swapchain_resources` cleans these. **Debug naming calls use `debug_utils_device`.**
- **BufferManager <-> VulkanContext**: Uses context for device, allocator, layouts, render pass, **debug utils device**.
- **TextRenderer <-> VulkanContext**: Uses device, allocator, **debug utils device** (passed via `prepare_text_draws`).
- **GlyphAtlas <-> VulkanContext**: Uses device, allocator, queue, command pool, **debug utils device** (passed via `add_glyph` or used in `new`/`cleanup`).
- **VulkanContext <-> Vulkan Components**: Provides core Vulkan resources and debug handles.
- **Build Script -> Target Directory**: Standard asset/shader handling.
- **Debug Naming**: `set_debug_object_name` helper uses `debug_utils::Device`. Various creation systems/methods call the helper.

## Current Capabilities (Updated for Stability Fixes & Debug Naming)
- **Bevy Integration**: Application runs within `bevy_app` (v0.15), using core non-rendering plugins (including `HierarchyPlugin`).
- **Modular Framework**: Core rendering, interaction, text foundation, and default behaviors encapsulated in Bevy Plugins.
- **System Set Ordering**: Explicit execution order defined using Bevy `SystemSet`s.
- **Bevy Math**: Uses `bevy_math` types (`Vec2`, `Mat4`).
- **Bevy ECS**: Defines and uses custom components (`ShapeData`, `Visibility`, `Interaction`, `BackgroundQuad`, `Text`, `TextLayoutOutput`, `EditableText`, `Focus`, `CursorState`, `CursorVisual`, `TextBufferCache`, `TextSelection`) and `bevy_transform::components::Transform`. Uses `bevy_hierarchy` (`Parent`, `Children`). `TextRenderData` is a component definition used internally by `TextRenderer`.
- **Bevy Events**: Defines and uses custom events (`EntityClicked`, `EntityDragged`, `HotkeyActionTriggered`, `YrsTextChanged`, `TextFocusChanged`) for system communication.
- **Bevy Reflection**: Core framework components/events/resources implement `Reflect` where feasible and are registered.
- **Input Processing**: Uses `bevy_input` via `GuiFrameworkInteractionPlugin`.
    - Basic click detection (shapes), dragging (shapes), hotkey dispatching implemented.
    - **Text Interaction**: Overall bounding box hit detection using `get_cursor_at_position`, ECS-based focus management (`Focus` component), cursor state update (`CursorState` component), and basic selection state management (`TextSelection` component - click sets collapsed selection).
    - **Text Drag Selection**: Mouse dragging with left button down on `EditableText` updates `TextSelection.end`.
    - **Text Editing**: Keyboard input (character typing, backspace, delete) modifies YRS text content for focused `EditableText`. Arrow keys navigate cursor and modify selection (with Shift).
    - Hit detection uses **Y-up world coordinates**. Dragging updates `Transform` correctly (**Y-axis inversion fixed**).
    - `MouseContext` resource tracks high-level mouse interaction state.
- **Vulkan Setup**: Core context initialized via `GuiFrameworkCorePlugin`. Separate pipeline layouts for shapes (with **push constant range**) and text created. Depth buffer, single command pool (managed by `Renderer`), and per-swapchain-image command buffers created. **Debug messenger and `debug_utils::Device` created (debug builds).**
- **Rendering Bridge**: Custom Vulkan context (`VulkanContextResource`) and renderer (`RendererResource`) managed as Bevy resources. `BufferManager` is also a Bevy resource (`BufferManagerResource`).
- **ECS-Driven Resource Management**:
    - `BufferManager` (via `BufferManagerResource`) creates/updates/cleans Vulkan resources for **shapes and cursor visuals**, including despawn handling. **Names resources.**
    - `GlyphAtlas` manages Vulkan texture for glyphs. **Names resources.** **Robust staging buffer cleanup.**
    - `TextRenderer` internally manages per-entity text Vulkan resources. **Names resources.**
    - `TextRenderingResources` holds shared text pipeline and global atlas descriptor set. **Names resources.** Managed by `create_text_rendering_resources_system`.
- **Resource Caching**: `BufferManager` caches the shape pipeline. `GlyphAtlas` caches glyph locations/UVs. `TextBufferCache` caches `cosmic-text` layout results. `TextRenderer` caches `TextRenderData`.
- **Rendering Path**: Data flows from ECS -> `Renderer` (prep via `BufferManagerResource`, `TextRenderer`) -> `Renderer::render` (resets command buffer) -> `record_command_buffers` (records draws). **Corrected synchronization (semaphore wait).** Projection matrix uses logical window size, Y-flip, wide depth range. Depth testing enabled.
- **Configurable Hotkeys**: Loads from TOML file into `HotkeyResource`.
- **Build Script**: Compiles GLSL shaders to SPIR-V, copies assets.
- **Resize Handling**: Correctly handles window resizing, swapchain/framebuffer/depth buffer recreation, command buffer re-allocation/cleanup, projection matrix updates, and dynamic background vertex updates.
- **Visual Output**: Functional 2D rendering of **shapes (with color)**, **editable static text**, and **visual text cursor** based on ECS data. Background dynamically resizes. Objects positioned and layered correctly according to **Y-up world coordinates and Z-depth**. Alpha blending enabled. Cursor visibility/position correct.
- **Text Foundation**: Functional components, font loading, glyph atlas, layout system, layout caching, text shaders/pipeline, visual cursor.
- **Yrs Integration**: Basic setup with `YrsDocResource`, text content read/modified by systems.
- **Robust Shutdown**: **Improved cleanup sequence.** Most resources cleaned correctly via `cleanup_trigger_system`.
- **Debug Naming**: **Implemented `VK_EXT_debug_utils` object naming** via helper function and calls in resource creation paths.

## Future Extensions
- **Text Handling**: Implement advanced selection (word/line selection via double/triple click), clipboard operations (copy/paste), context menus (Task 10). Improve text rendering quality (e.g., SDF). Improve cursor positioning logic with complex wrapping and BiDi text.
- **Rendering Optimization**: Optimize text vertex buffer updates in `TextRenderer`.
- **Hit Detection**: Improve Z-sorting/picking logic.
- **Bevy State Integration**.
- Instancing via ECS.
- Batch operations for groups.
- P2P networking (Yrs integration).
- 3D rendering and AI-driven content generation.
- Divider system (Task 11).

## Error Handling
- Vulkan errors checked via `ash` results. **Validation layers enabled and provide named object info on errors.** `vk-mem` assertions check memory leaks. `map_memory` errors handled.
- Logical errors (`HotkeyError`) use `Result` or `thiserror`.
- Hotkey file loading/parsing errors handled gracefully.
- Bevy logging integrated. Mutex poisoning handled.
- Startup systems `panic!` on critical setup errors.
- **Improved resource cleanup on exit.** **Robust staging buffer cleanup in `GlyphAtlas`.**

## Shader Integration
- GLSL shaders (`.vert`, `.frag`) in `shaders/` compiled to SPIR-V (`.spv`) by `build.rs`.
- **Shape Shaders (`shape.vert`, `shape.frag`)**: Single pipeline using push constants for color.
- **Text Shaders (`text.vert`, `text.frag`)**: Uses UBOs (Set 0) and atlas sampler (Set 1).

## Dependencies
- External crates: `ash = "0.38"`, `vk-mem = "0.4"`, `ash-window = "0.13"`, `raw-window-handle = "0.6"`, `toml = "0.8"`, `thiserror = "2.0"`, `bevy_app = "0.15"`, `bevy_core = "0.15"`, `bevy_ecs = "0.15"`, `bevy_log = "0.15"`, `bevy_utils = "0.15"`, `bevy_window = "0.15"`, `bevy_winit = "0.15"`, `bevy_reflect = "0.15"`, `bevy_input = "0.15"`, `bevy_time = "0.15"`, `bevy_diagnostic = "0.15"`, `bevy_a11y = "0.15"`, `bevy_math = "0.15"`, `bevy_transform = "0.15"`, `bevy_color = "0.15"`, `bevy_hierarchy = "0.15"`, `winit = "0.30.9"`, `cosmic-text = "0.14"`, `fontdb = "0.23"`, `swash = "0.2"`, `rectangle-pack = "0.4"`, `yrs = "0.23"`.
- Build dependencies: `walkdir = "2"`.
- Shaders: Precompiled `.spv` files in target directory.

## Notes
- **Framework Structure**: Core logic refactored into Bevy Plugins using System Sets for ordering. Application logic resides in `main.rs`.
- **Coordinate System**: Uses a **Y-up world coordinate system** (origin bottom-left). Projection matrix includes a Y-flip to match Vulkan's default Y-down NDC space. Input and movement systems handle coordinate conversions correctly. Text vertex generation correctly transforms relative glyph coordinates to world space. Text hit detection handles conversion between Bevy Y-up and cosmic-text Y-down local coordinates. Cursor positioning calculated relative to text layout runs.
- **Depth**: Depth testing is enabled (`CompareOp::LESS` for shapes, `CompareOp::LESS_OR_EQUAL` for text). Entities with lower Z values are rendered "on top". Projection matrix uses a wide depth range (`1024.0` to `0.0`). Cursor visual rendered slightly in front of text.
- **Known Issues**:
    - `vulkan_setup` still uses `winit::window::Window` directly.
    - Cursor positioning logic needs refinement for complex text wrapping and BiDi text.
    - **Persistent `VkDeviceMemory` leak on exit (VUID-vkDestroyDevice-device-05137), despite improved cleanup. Leaking objects are now named (e.g., "DepthImage_Mem", "TextTransformUBO_Entity..._Mem"), indicating issues likely within `TextRenderer` or `swapchain` cleanup.**
- **Cleanup Logic**: Synchronous cleanup on `AppExit` is handled by `cleanup_trigger_system`. **Order refined, most resources cleaned correctly.** `Renderer::cleanup()` cleans its internals (TextRenderer, pool, layouts, sync objects, command pool). `BufferManagerResource` cleanup triggered. `cleanup_vulkan` handles final context destruction.
- **Mouse Context**: `MouseContext` resource is local to the `interaction` plugin.
- **Command Buffer Management**: Per-image buffers allocated/freed by `create_framebuffers`/`cleanup_swapchain_resources`. Reset per frame by `Renderer::render`. Command pool created with `RESET_COMMAND_BUFFER` flag.