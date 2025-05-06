# Architecture Overview for `rusty_whip`

## Purpose
`rusty_whip` is an advanced 2D and 3D content generation application for digital entertainment production, leveraging GPU-accelerated AI diffusion/inference, multimedia creation (2D video, stills, animation, audio), and story-driven workflows, with plans for quantum-resistant P2P networking. **It uses the Bevy game engine (v0.15) for its core application structure, input handling, ECS (Entity Component System), and hierarchy, while strictly avoiding Bevy's rendering stack (`bevy_render`, `wgpu`, etc.) in favor of its custom Vulkan backend.**

## Core Components
1.  **Vulkan Context Management (`context/`)**
    *   Role: Initializes and manages core Vulkan resources (instance, physical device, logical device, queue, allocator, shape/text pipeline layouts, current swap extent, depth buffer resources including `depth_image_allocation`, debug messenger). Accessed via `VulkanContextResource`. Holds resources needed globally or across different rendering stages.
    *   Key Modules: `vulkan_context.rs`, `vulkan_setup.rs`.
2.  **Rendering Engine (`rendering/render_engine.rs`)**
    *   Role: Orchestrates the custom Vulkan rendering pipeline per frame. Manages `BufferManager` (for shapes/cursors) and sync objects. **Internally manages a cache of per-entity text Vulkan resources (`HashMap<Entity, TextRenderData>`) based on `TextLayoutInfo` received from the ECS.** Handles resize via `ResizeHandler`. Calls `BufferManager` (shapes/cursors) and `command_buffers` (shapes, text, cursor) to record draw calls. Accessed via `RendererResource`.
    *   Key Modules: `render_engine.rs`.
3.  **Pipeline Manager (`rendering/pipeline_manager.rs`)**
    *   Role: **Initialization helper.** Creates Vulkan `PipelineLayout`s (shape, text), `DescriptorSetLayout`s (per-entity, atlas), and a shared `DescriptorPool` during application setup. **Shape pipeline layout includes a push constant range for color.** These resources are then transferred to `VulkanContext` (layouts) or `Renderer` (pool, set layouts) for operational use and cleanup.
    *   Key Modules: `pipeline_manager.rs`.
4.  **Buffer Manager (`rendering/buffer_manager.rs`)**
    *   Role: **Shape/Cursor resource manager.** Manages per-entity Vulkan resources (vertex buffers, transform UBOs, descriptor sets) for shapes and cursor visuals based on ECS `RenderCommandData`. Uses per-entity layout/pool provided during initialization. **Caches the single shape pipeline (including depth state).** Updates vertex buffers based on `vertices_changed` flag. Updates transform UBOs and descriptor sets every frame for existing entities. **Lacks optimization for resource removal (despawned entities).**
    *   Key Modules: `buffer_manager.rs`.
5.  **Text Handling (`rendering/font_server.rs`, `rendering/glyph_atlas.rs`, `components/text_data.rs`, `components/text_layout.rs`, `plugins/core.rs`, `plugins/interaction.rs`, `rendering/render_engine.rs`)**
    *   Role: Manages font loading (`font_server`), text layout/shaping (`text_layout_system`), glyph caching/packing/upload (`glyph_atlas` using `rectangle-pack`), **per-entity text Vulkan resource management (internally by `Renderer` using its `TextRenderData` cache)**, **shared text rendering resource management** (`TextRenderingResources`), **text rendering preparation (by `Renderer`)**, **caching of layout results** (`TextBufferCache`), **visual cursor management** (`manage_cursor_visual_system`, `update_cursor_transform_system`), **text editing logic** (`text_editing_system`), and **text drag selection logic** (`text_drag_selection_system`).
    *   Key Modules: `font_server.rs`, `glyph_atlas.rs`, `text_data.rs`, `text_layout.rs`, `render_engine.rs`. Systems in `plugins/core.rs` and `plugins/interaction.rs` orchestrate these aspects.
6.  **Bevy ECS Components (`components/`)**
    *   Role: Defines the data associated with entities (e.g., shape, text, visibility, interaction properties, **text editing state (`Focus`, `CursorState`, `TextSelection`)**, **cursor visual marker (`CursorVisual`)**, **layout cache (`TextBufferCache`)**). `TextRenderData` is a component definition but used internally by `Renderer`'s cache. Used alongside `bevy_transform::components::Transform` and `bevy_hierarchy` components (`Parent`, `Children`). Implements `bevy_reflect::Reflect` where possible.
    *   Key Modules: `shape_data.rs`, `visibility.rs`, `interaction.rs`, `text_data.rs` (defines `Text`, `EditableText`, `Focus`, `CursorState`, `CursorVisual`, `TextSelection`), `text_layout.rs` (defines `TextLayoutOutput`, `TextRenderData`, `TextBufferCache`).
7.  **Bevy Events (`events/`)**
    *   Role: Defines event types for communication between systems (e.g., user interactions, hotkey triggers, **text focus changes**, **YRS text changes**). Implements `bevy_reflect::Reflect`.
    *   Key Modules: `interaction_events.rs` (defines `EntityClicked`, `EntityDragged`, `HotkeyActionTriggered`, `YrsTextChanged`, `TextFocusChanged`).
8.  **Interaction Logic (`interaction/`)**
    *   Role: Defines hotkey loading (`hotkeys.rs`), text interaction utilities (`utils.rs`), **text drag selection logic (`text_drag.rs`)**, and **text editing logic (`text_editing.rs`)**.
    *   Key Modules: `hotkeys.rs`, `utils.rs`, `text_drag.rs`, `text_editing.rs`.
9.  **Framework Plugins (`plugins/`)**
    *   Role: Encapsulate core framework logic into modular Bevy plugins for better organization and reusability. Define `SystemSet`s (`CoreSet`, `InteractionSet`, etc.) to manage execution order.
    *   Key Modules:
        *   `core.rs`: Vulkan/Renderer/Text setup, text layout & caching, **cursor management (including visibility based on `TextSelection` and precise positioning using `FontServerResource`)**, rendering system (collects `TextLayoutInfo`), resize, cleanup.
        *   `interaction.rs`: Input processing (mouse clicks/drags, keyboard), hotkey loading/dispatch, window close. **Manages text focus (`Focus` component), cursor state (`CursorState` component), and selection state (`TextSelection` component) via `Commands`. Manages local `MouseContext` resource. Performs hit detection on `EditableText` using `get_cursor_at_position` utility. Includes systems for text drag selection (`text_drag_selection_system`) and keyboard-based text editing (`text_editing_system`).**
        *   `movement.rs`: Default drag movement. **Includes fix for Y-axis inversion.**
        *   `bindings.rs`: Default hotkey actions.
10. **Bevy App Core (`main.rs`)**
    *   Role: Bootstraps the application using `bevy_app::App`. Initializes core Bevy plugins (including `HierarchyPlugin`) and framework plugins. Inserts initial `VulkanContextResource`, `YrsDocResource`. Defines and schedules **application-specific** systems (e.g., `setup_scene_ecs`, `background_resize_system`). Spawns initial entities with components (including sample text with `EditableText`).
11. **Build Script (`build.rs`)**
    *   Role: Compiles GLSL shaders (`.vert`, `.frag`) to SPIR-V (`.spv`) using `glslc`, copies compiled shaders and runtime assets (e.g., `user/hotkeys.toml`) to the target build directory. Tracks shader source files for recompilation.
    *   Key Modules: `build.rs`.
12. **Yrs Integration (`lib.rs`, `main.rs`, `plugins/core.rs`, `plugins/interaction.rs`)**
    *   Role: Manages shared text data using `yrs` CRDT library. `YrsDocResource` holds the document and entity-to-`TextRef` mapping. `text_layout_system` reads content from Yrs. `text_editing_system` modifies Yrs content. `YrsTextChanged` event triggers layout updates.
    *   Key Modules: `lib.rs`, `main.rs`, `plugins/core.rs`, `plugins/interaction.rs`.

## Data Flow (Updated for Text Editing & Rendering Refactor)
1.  `main.rs` initializes `bevy_app::App`, adds core non-rendering Bevy plugins (including `HierarchyPlugin`), inserts `VulkanContextResource`, `YrsDocResource`, and adds framework plugins.
2.  Bevy `Startup` schedule runs (ordered by `CoreSet`, `InteractionSet`):
    *   `setup_vulkan_system` initializes Vulkan context.
    *   `create_renderer_system` creates `RendererResource`.
    *   `create_glyph_atlas_system`, `create_font_server_system`, `create_swash_cache_system` create text foundation resources.
    *   `create_global_ubo_system` creates `GlobalProjectionUboResource`.
    *   `create_text_rendering_resources_system` creates shared `TextRenderingResources`.
    *   `load_hotkeys_system` (Interaction Plugin) loads hotkeys, initializes `MouseContext`.
    *   `setup_scene_ecs` (main.rs) spawns initial entities (shapes with color, text with `EditableText`), populates `YrsDocResource`.
3.  Bevy `Update` schedule runs (ordered by `InteractionSet`, `CoreSet`):
    *   `interaction_system` processes input (mouse clicks/drags, keyboard), performs hit testing (shapes, **text using `get_cursor_at_position`**), sends events, **manages `Focus`, `CursorState`, `TextSelection` components via `Commands`, updates `MouseContext`**.
    *   **`text_editing_system` (Interaction Plugin) processes keyboard input for focused `EditableText`, modifies `YrsDocResource`, updates `CursorState`/`TextSelection`, sends `YrsTextChanged`.**
    *   **`text_drag_selection_system` (Interaction Plugin) processes mouse drag for `TextSelection` if `MouseContext` is `TextInteraction`.**
    *   `manage_cursor_visual_system` (Core Plugin) reacts to `Added<Focus>` / `RemovedComponents<Focus>`, adds/removes `CursorState`, spawns/despawns `CursorVisual` entity (as child of focused text), **reads `TextSelection` to set `CursorVisual` visibility**.
    *   `apply_deferred` runs in `CoreSet::ApplyInputCommands` to flush commands from focus/input systems.
    *   `text_layout_system` (Core Plugin) runs on `YrsTextChanged` or `Added<Text>`, reads content from `YrsDocResource`, uses `FontServer`, `SwashCache`, calls `GlyphAtlas::add_glyph`, updates `TextLayoutOutput`, and **updates/inserts `TextBufferCache`**.
    *   `update_cursor_transform_system` (Core Plugin) reads `CursorState`, `TextBufferCache`, `FontServerResource`, calculates visual position using layout runs, updates `Transform` of `CursorVisual` entity.
    *   `handle_resize_system` (Core Plugin) handles `WindowResized`, updates `GlobalProjectionUboResource`, and calls `Renderer::resize_renderer`.
    *   `movement_system` (Movement Plugin) reads `EntityDragged` and updates `Transform` (**Y-axis corrected**).
    *   `app_control_system` (Bindings Plugin) reads `HotkeyActionTriggered`.
    *   `background_resize_system` (main.rs) handles `WindowResized` for the background quad.
4.  Bevy `Last` schedule runs:
    *   `rendering_system` (Core Plugin) queries ECS for shapes (`ShapeData`), **collects `TextLayoutInfo` for text entities (from `TextLayoutOutput`, `GlobalTransform`, `Visibility`)**, and cursor visuals (`ShapeData`, `CursorVisual`). Calls `Renderer::render`.
    *   `Renderer::render` locks resources, waits for fence, calls `BufferManager::prepare_frame_resources` (updates shape/cursor buffers/UBOs/descriptors), **internally processes `TextLayoutInfo` to create/update its per-entity `TextRenderData` cache and collects `PreparedTextDrawData`**, acquires swapchain image, calls `record_command_buffers`, submits commands, and presents.
    *   `record_command_buffers` records draw calls for shapes (using **push constants for color**), text, and cursor visuals using prepared data. Includes depth clear.
    *   `cleanup_trigger_system` (Core Plugin) runs on `AppExit`, cleaning up **`TextBufferCache` components**, then framework resources (`Renderer` (which cleans its internal `TextRenderData` cache), `TextRenderingResources`, `GlobalProjectionUboResource`, `GlyphAtlas`, `VulkanContext`).

## Key Interactions (Updated for Text Editing & Rendering Refactor)
- **Plugins <-> Bevy App**: Plugins register components/resources/events and add systems to schedules.
- **Plugins <-> System Sets**: Plugins define and use `SystemSet`s (`CoreSet`, `InteractionSet`, etc.). **InteractionSet now runs before `CoreSet::ManageCursorVisual`. `CoreSet` includes `ApplyInputCommands` and `ApplyUpdateCommands` for command flushing.**
- **App Systems (`main.rs`) <-> Plugin Sets**: Application systems order themselves relative to plugin sets.
- **Interaction Plugin -> Events**: Writes `EntityClicked`, `EntityDragged`, `HotkeyActionTriggered`, `TextFocusChanged`, `YrsTextChanged`, `AppExit`.
- **Interaction Plugin -> ECS**: **Adds/Removes `Focus`, `TextSelection` components using `Commands`. Writes `CursorState` component. Writes `MouseContext` resource.** Reads `TextBufferCache` for hit detection.
- **Interaction Plugin -> YRS**: `text_editing_system` writes to `YrsDocResource`.
- **Movement Plugin <- Events**: Reads `EntityDragged`, writes `Transform`.
- **Bindings Plugin <- Events**: Reads `HotkeyActionTriggered`.
- **Core Plugin (Text Layout)**: `text_layout_system` reads `YrsTextChanged`/`Added<Text>`, `Text`, `Transform`, `Visibility`; uses `YrsDocResource`, `FontServerResource`, `SwashCacheResource`, `VulkanContextResource`; reads/writes `GlyphAtlasResource`; writes `TextLayoutOutput`, **writes `TextBufferCache`**.
- **Core Plugin (Cursor Management)**: `manage_cursor_visual_system` reads `Added<Focus>`/`RemovedComponents<Focus>`, **reads `TextSelection`**, writes `CursorState`, spawns/despawns `CursorVisual` (using `Commands`, `BuildChildren`, `DespawnRecursiveExt`), **writes `Visibility` for `CursorVisual`**. `update_cursor_transform_system` reads `CursorState`, `TextBufferCache`, `Children`, `FontServerResource`; writes `Transform` (for `CursorVisual`).
- **Core Plugin (Rendering)**: `rendering_system` reads `ShapeData`, `TextLayoutOutput`, `CursorVisual`, `GlobalTransform`; uses `TextRenderingResources`; calls `Renderer::render` with `TextLayoutInfo`.
- **Core Plugin (Cleanup)**: `cleanup_trigger_system` queries/cleans up `TextBufferCache`, then cleans up `Renderer` (and its internal cache), `TextRenderingResources`, `GlobalProjectionUboResource`, `GlyphAtlas`, `VulkanContext`.
- **Renderer <-> BufferManager**: Renderer calls BufferManager for **shape/cursor** resource prep/cleanup.
- **Renderer <-> Command Buffers**: `Renderer::render` calls `record_command_buffers`, passing prepared shape, cursor, and text draw data.
- **Renderer <-> Internal Text Cache**: `Renderer::render` manages an internal `HashMap<Entity, TextRenderData>`.
- **Renderer <-> TextLayoutInfo**: `Renderer::render` consumes `TextLayoutInfo` (containing `TextLayoutOutput`, `GlobalTransform`, `Visibility`) to generate text draw data.
- **Renderer <-> ResizeHandler**: `Renderer::resize_renderer` calls `ResizeHandler::resize`.
- **BufferManager <-> VulkanContext**: Uses context for device, allocator, layouts, render pass.
- **BufferManager <-> GlobalProjectionUboResource**: Reads UBO handle for descriptor updates.
- **GlyphAtlas <-> VulkanContext**: Uses context for image/sampler creation and upload command execution.
- **App Systems (`main.rs`) <-> Components**: `background_resize_system` modifies `ShapeData`. `setup_scene_ecs` adds `EditableText`.
- **VulkanContext <-> Vulkan Components**: Provides core Vulkan resources, including depth buffer and debug messenger.
- **Build Script -> Target Directory**: Compiles shaders, copies assets.

## Current Capabilities (Updated for Text Editing & Rendering Refactor)
- **Bevy Integration**: Application runs within `bevy_app` (v0.15), using core non-rendering plugins (including `HierarchyPlugin`).
- **Modular Framework**: Core rendering, interaction, text foundation, and default behaviors encapsulated in Bevy Plugins.
- **System Set Ordering**: Explicit execution order defined using Bevy `SystemSet`s.
- **Bevy Math**: Uses `bevy_math` types (`Vec2`, `Mat4`).
- **Bevy ECS**: Defines and uses custom components (`ShapeData`, `Visibility`, `Interaction`, `BackgroundQuad`, `Text`, `TextLayoutOutput`, `EditableText`, `Focus`, `CursorState`, `CursorVisual`, `TextBufferCache`, `TextSelection`) and `bevy_transform::components::Transform`. Uses `bevy_hierarchy` (`Parent`, `Children`). `TextRenderData` is a component definition used internally by `Renderer`.
- **Bevy Events**: Defines and uses custom events (`EntityClicked`, `EntityDragged`, `HotkeyActionTriggered`, `YrsTextChanged`, `TextFocusChanged`) for system communication.
- **Bevy Reflection**: Core framework components/events/resources implement `Reflect` where feasible and are registered.
- **Input Processing**: Uses `bevy_input` via `GuiFrameworkInteractionPlugin`.
    - Basic click detection (shapes), dragging (shapes), hotkey dispatching implemented.
    - **Text Interaction**: Overall bounding box hit detection using `get_cursor_at_position`, ECS-based focus management (`Focus` component), cursor state update (`CursorState` component), and basic selection state management (`TextSelection` component - click sets collapsed selection).
    - **Text Drag Selection**: Mouse dragging with left button down on `EditableText` updates `TextSelection.end`.
    - **Text Editing**: Keyboard input (character typing, backspace, delete) modifies YRS text content for focused `EditableText`. Arrow keys navigate cursor and modify selection (with Shift).
    - Hit detection uses **Y-up world coordinates**. Dragging updates `Transform` correctly (**Y-axis inversion fixed**).
    - `MouseContext` resource tracks high-level mouse interaction state.
- **Vulkan Setup**: Core context initialized via `GuiFrameworkCorePlugin`. Separate pipeline layouts for shapes (with **push constant range**) and text created. **Depth buffer created. Debug messenger enabled (debug builds).**
- **Rendering Bridge**: Custom Vulkan context (`VulkanContextResource`) and renderer (`RendererResource`) managed as Bevy resources.
- **ECS-Driven Resource Management**:
    - `BufferManager` creates/updates Vulkan resources for **shapes and cursor visuals**.
    - `GlyphAtlas` manages Vulkan texture for glyphs.
    - **`Renderer` internally manages per-entity text Vulkan resources (in its `TextRenderData` cache) based on `TextLayoutInfo`.**
    - **`TextRenderingResources` holds shared text pipeline and global atlas descriptor set.** Managed by `create_text_rendering_resources_system`.
- **Resource Caching**: `BufferManager` caches the **single shape pipeline**. `GlyphAtlas` caches glyph locations/UVs. **`TextBufferCache` caches `cosmic-text` layout results.** `Renderer` caches `TextRenderData`.
- **Rendering Path**: Data flows from ECS (`rendering_system` collects `RenderCommandData` for shapes/cursors and `TextLayoutInfo` for text) -> `Renderer` (shape/cursor prep via `BufferManager`, text prep internally using `TextRenderData` cache) -> `record_command_buffers` (shape/cursor draw recording **using push constants for color**, text draw recording). Synchronization corrected. Projection matrix uses logical window size, Y-flip, and wide depth range. **Depth testing enabled.**
- **Configurable Hotkeys**: Loads from TOML file into `HotkeyResource`.
- **Build Script**: Compiles GLSL shaders to SPIR-V, copies assets.
- **Resize Handling**: Correctly handles window resizing, swapchain/framebuffer/depth buffer recreation, projection matrix updates, and dynamic background vertex updates.
- **Visual Output**: Functional 2D rendering of **shapes (with color)**, **editable static text**, and **visual text cursor** based on ECS data. Background dynamically resizes. Objects positioned and layered correctly according to **Y-up world coordinates and Z-depth**. Alpha blending enabled for shapes, text, and cursor. **Cursor visibility correctly reflects collapsed/active selection state. Cursor position accurately reflects logical cursor state within text layout.**
- **Text Foundation**:
    - Text component definition (`Text`, `TextVertex`, `EditableText`, `Focus`, `CursorState`, `CursorVisual`, `TextSelection`).
    - Font loading and management (`FontServer`, `FontServerResource`).
    - Glyph atlas resource management (`GlyphAtlas`, `GlyphAtlasResource`), including packing/upload.
    - CPU-side text layout system (`text_layout_system` using `cosmic-text`, triggered by `YrsTextChanged`/`Added<Text>`).
    - Intermediate text layout component (`TextLayoutOutput`).
    - **Per-entity text rendering resource management (by `Renderer` using its internal `TextRenderData` cache).**
    - **Text layout result caching (`text_layout_system`, `TextBufferCache`).**
    - Text shaders (`text.vert`, `text.frag`) created and used by text pipeline.
    - Text-specific Vulkan layouts and pipeline created.
    - **Visual text cursor display and positioning.**
- **Yrs Integration**: Basic setup with `YrsDocResource`, text content read from Yrs for layout by `text_layout_system`, modified by `text_editing_system`.
- **Robust Shutdown**: Application exit sequence correctly cleans up **`TextBufferCache` components**, then `Renderer` (which cleans its internal `TextRenderData` cache), and then shared Vulkan resources via `cleanup_trigger_system`.

## Future Extensions
- **Text Handling**: Implement advanced selection (word/line selection via double/triple click), clipboard operations (copy/paste), context menus (Task 10). Improve text rendering quality (e.g., SDF). Improve cursor positioning logic with complex wrapping and BiDi text.
- **Rendering Optimization**: Implement resource removal for despawned entities in `BufferManager` (using `RemovedComponents`). Optimize text vertex buffer updates in `Renderer` (e.g., smarter resizing, partial updates).
- **Hit Detection**: Improve Z-sorting/picking logic in `interaction_system` for overlapping non-text objects.
- **Bevy State Integration**: Consider Bevy State for managing application modes.
- Instancing via ECS.
- Batch operations for groups (using ECS queries/components).
- P2P networking (Yrs integration).
- 3D rendering and AI-driven content generation.
- Divider system (Task 11).

## Error Handling
- Vulkan errors checked via `ash` results. **Validation layers enabled in debug builds via debug messenger.** `vk-mem` assertions check memory leaks. `map_memory` errors handled. Persistent map access uses `get_allocation_info`.
- Logical errors (`HotkeyError`) use `Result` or `thiserror`.
- Hotkey file loading/parsing errors handled gracefully with defaults and logs.
- Bevy logging integrated via `LogPlugin`. Mutex poisoning handled with logs.
- Renderer/Atlas checks for `None` resources during shutdown to prevent panics. Startup systems now `panic!` on critical setup errors.
- **Cleanup of `TextBufferCache` components by `cleanup_trigger_system`. Cleanup of `TextRenderData` resources (buffers, allocations, descriptor sets) handled by `Renderer::cleanup()` when its internal cache is cleared.**

## Shader Integration
- GLSL shaders (`.vert`, `.frag`) in `shaders/` compiled to SPIR-V (`.spv`) by `build.rs`. Compiled `.spv` files copied to target directory.
- **Shape Shaders (`shape.vert`, `shape.frag`)**: Loaded and pipeline cached by `BufferManager`. Used for shapes and cursor visuals. Support orthographic projection (Set 0, Binding 0) and object transformation matrix (Set 0, Binding 1). Use vertex position (location 0). **`shape.frag` receives color via push constants.**
- **Text Shaders (`text.vert`, `text.frag`)**: Loaded by `create_text_rendering_resources_system` for text pipeline creation. `text.vert` uses Set 0 (Binding 0: projection, Binding 1: transform). `text.frag` uses Set 1 (Binding 0: glyph atlas sampler).

## Dependencies
- External crates: `ash = "0.38"`, `vk-mem = "0.4"`, `ash-window = "0.13"`, `raw-window-handle = "0.6"`, `toml = "0.8"`, `thiserror = "2.0"`, `bevy_app = "0.15"`, `bevy_core = "0.15"`, `bevy_ecs = "0.15"`, `bevy_log = "0.15"`, `bevy_utils = "0.15"`, `bevy_window = "0.15"`, `bevy_winit = "0.15"`, `bevy_reflect = "0.15"`, `bevy_input = "0.15"`, `bevy_time = "0.15"`, `bevy_diagnostic = "0.15"`, `bevy_a11y = "0.15"`, `bevy_math = "0.15"`, `bevy_transform = "0.15"`, `bevy_color = "0.15"`, `bevy_hierarchy = "0.15"`, `winit = "0.30.9"`, `cosmic-text = "0.14"`, `fontdb = "0.23"`, `swash = "0.2"`, `rectangle-pack = "0.4"`, `yrs = "0.23"`.
- Build dependencies: `walkdir = "2"`.
- Shaders: Precompiled `.spv` files in target directory.

## Notes
- **Framework Structure**: Core logic refactored into Bevy Plugins using System Sets for ordering. Application logic resides in `main.rs`.
- **Coordinate System**: Uses a **Y-up world coordinate system** (origin bottom-left). Projection matrix includes a Y-flip to match Vulkan's default Y-down NDC space. Input and movement systems handle coordinate conversions correctly. Text vertex generation correctly transforms relative glyph coordinates to world space. **Text hit detection handles conversion between Bevy Y-up and cosmic-text Y-down local coordinates.** Cursor positioning calculated relative to text layout runs.
- **Depth**: Depth testing is enabled (`CompareOp::LESS` for shapes, `CompareOp::LESS_OR_EQUAL` for text). Entities with lower Z values are rendered "on top". Projection matrix uses a wide depth range (`1024.0` to `0.0`). Cursor visual rendered slightly in front of text.
- **Known Issues**:
    - `BufferManager` lacks resource removal for despawned entities.
    - `vulkan_setup` still uses `winit::window::Window` directly.
    - Cursor positioning logic needs refinement for complex text wrapping and BiDi text.
- **Cleanup Logic**: Synchronous cleanup on `AppExit` is handled by `cleanup_trigger_system` within the `GuiFrameworkCorePlugin`, running in the `Last` schedule. It now cleans up `TextBufferCache` components. `Renderer::cleanup()` is called, which cleans its internal `TextRenderData` cache. Then shared framework resources are cleaned. `CursorVisual` cleanup handled via `DespawnRecursiveExt`.
- **Mouse Context**: `MouseContext` resource is local to the `interaction` plugin, used to coordinate mouse-down behavior between `interaction_system` and `text_drag_selection_system`.