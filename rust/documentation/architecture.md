# Architecture Overview for `rusty_whip`

## Purpose
`rusty_whip` is an advanced 2D and 3D content generation application for digital entertainment production, leveraging GPU-accelerated AI diffusion/inference, multimedia creation (2D video, stills, animation, audio), and story-driven workflows, with plans for quantum-resistant P2P networking. **It uses the Bevy game engine (v0.15) for its core application structure, input handling, and ECS (Entity Component System), while strictly avoiding Bevy's rendering stack (`bevy_render`, `wgpu`, etc.) in favor of its custom Vulkan backend.**

## Core Components
1.  **Vulkan Context Management (`context/`)**
    *   Role: Initializes and manages core Vulkan resources (instance, physical device, logical device, queue, allocator, shape/text pipeline layouts, current swap extent). Accessed via `VulkanContextResource`. Holds resources needed globally or across different rendering stages.
    *   Key Modules: `vulkan_context.rs`, `vulkan_setup.rs`.
2.  **Rendering Engine (`rendering/`)**
    *   Role: Orchestrates the custom Vulkan rendering pipeline per frame. Manages `BufferManager` (for shapes), sync objects, and **text rendering resources (dynamic vertex buffer, atlas descriptor set, text pipeline)**. Handles resize via `ResizeHandler`. Calculates projection matrix. Calls `BufferManager` (shapes) and `command_buffers` (shapes, text) to record draw calls based on ECS data. Accessed via `RendererResource`.
    *   Key Modules: `render_engine.rs`, `buffer_manager.rs`, `glyph_atlas.rs`, `resize_handler.rs`, `command_buffers.rs`, `swapchain.rs`, `shader_utils.rs`.
3.  **Pipeline Manager (`rendering/pipeline_manager.rs`)**
    *   Role: **Initialization helper.** Creates Vulkan `PipelineLayout`s (shape, text), `DescriptorSetLayout`s (shape, text), and a shared `DescriptorPool` during application setup. These resources are then transferred to `VulkanContext` (layouts) or `Renderer` (pool, set layouts) for operational use and cleanup.
    *   Key Modules: `pipeline_manager.rs`.
4.  **Buffer Manager (`rendering/buffer_manager.rs`)**
    *   Role: **Shape resource manager.** Manages the global projection uniform buffer and per-entity Vulkan resources (vertex buffers, transform UBOs, descriptor sets) for shapes based on ECS `RenderCommandData`. Uses shape layout/pool provided during initialization. Caches shape pipelines and shader modules. Updates vertex buffers based on `vertices_changed` flag. Updates descriptor sets immediately per-entity. **Lacks optimization for resource removal (despawned entities).**
    *   Key Modules: `buffer_manager.rs`.
5.  **Text Handling (`rendering/font_server.rs`, `rendering/glyph_atlas.rs`, `components/text_data.rs`, `components/text_layout.rs`)**
    *   Role: Manages font loading (`font_server`), text layout/shaping (`text_layout_system`), glyph caching/packing/upload (`glyph_atlas` using `rectangle-pack`), and prepares text data (`Text`, `TextLayoutOutput`) for rendering.
    *   Key Modules: `font_server.rs`, `glyph_atlas.rs`, `text_data.rs`, `text_layout.rs`. Systems in `plugins/core.rs` orchestrate layout and vertex generation.
6.  **Bevy ECS Components (`components/`)**
    *   Role: Defines the data associated with entities (e.g., shape, text, visibility, interaction properties). Used alongside `bevy_transform::components::Transform`. Implements `bevy_reflect::Reflect` where possible.
    *   Key Modules: `shape_data.rs`, `visibility.rs`, `interaction.rs`, `text_data.rs`, `text_layout.rs`.
7.  **Bevy Events (`events/`)**
    *   Role: Defines event types for communication between systems (e.g., user interactions, hotkey triggers). Implements `bevy_reflect::Reflect`.
    *   Key Modules: `interaction_events.rs`.
8.  **Hotkey Loading (`interaction/hotkeys.rs`)**
    *   Role: Defines logic for loading hotkey configurations from a TOML file (`HotkeyConfig`).
    *   Key Modules: `hotkeys.rs`.
9.  **Framework Plugins (`plugins/`)**
    *   Role: Encapsulate core framework logic into modular Bevy plugins for better organization and reusability. Define `SystemSet`s (`CoreSet`, `InteractionSet`, etc.) to manage execution order.
    *   Key Modules: `core.rs` (Vulkan/Renderer/Text setup, text layout, **text vertex generation**, rendering, resize, cleanup), `interaction.rs` (Input processing, hotkey loading/dispatch, window close), `movement.rs` (Default drag movement), `bindings.rs` (Default hotkey actions).
10. **Bevy App Core (`main.rs`)**
    *   Role: Bootstraps the application using `bevy_app::App`. Initializes core Bevy plugins and framework plugins. Inserts initial `VulkanContextResource`. Defines and schedules **application-specific** systems (e.g., `setup_scene_ecs`, `background_resize_system`). Spawns initial entities with components (including sample text).
11. **Build Script (`build.rs`)**
    *   Role: Compiles GLSL shaders (`.vert`, `.frag`) to SPIR-V (`.spv`) using `glslc`, copies compiled shaders and runtime assets (e.g., `user/hotkeys.toml`) to the target build directory. Tracks shader source files for recompilation.
    *   Key Modules: `build.rs`.

## Data Flow (Post Text Rendering Implementation)
1.  `main.rs` initializes `bevy_app::App`, adds core non-rendering Bevy plugins, inserts `VulkanContextResource`, and adds framework plugins (`GuiFrameworkCorePlugin`, `GuiFrameworkInteractionPlugin`, etc.).
2.  Bevy `Startup` schedule runs:
    *   `GuiFrameworkCorePlugin` systems initialize Vulkan, create `RendererResource` (which creates text pipeline/buffer), `GlyphAtlasResource`, `FontServerResource`, `SwashCacheResource`.
    *   `GuiFrameworkInteractionPlugin` system loads hotkeys.
    *   `main.rs` system `setup_scene_ecs` spawns initial entities (shapes and text).
3.  Bevy `Update` schedule runs:
    *   Input processing, interaction events, movement, hotkey actions handled by respective plugins.
    *   `GuiFrameworkCorePlugin` system `handle_resize_system` handles `WindowResized` for the renderer.
    *   `main.rs` system `background_resize_system` handles `WindowResized` for the background quad.
    *   `GuiFrameworkCorePlugin` system `text_layout_system` queries `Text`, uses `FontServer`, `SwashCache`, calls `GlyphAtlas::add_glyph` (packing/uploading glyphs), and updates `TextLayoutOutput`.
4.  Bevy `Last` schedule runs:
    *   `GuiFrameworkCorePlugin` system `rendering_system` queries ECS for shapes (`ShapeData`) and text (`TextLayoutOutput`). It prepares `RenderCommandData` for shapes. It **generates world-space `TextVertex` data** for text (with baseline alignment and flipped UVs) and collects it into a `Vec`. It calls `Renderer::render`, passing shape commands, the text vertex `Vec`, and `GlyphAtlasResource`.
    *   `Renderer::render` updates shape resources via `BufferManager`, **updates the dynamic text vertex buffer**, ensures the atlas descriptor set exists, acquires swapchain image, calls `record_command_buffers`, submits commands, and presents.
    *   `record_command_buffers` records draw calls for shapes and **text (binding text pipeline, text vertex buffer, atlas descriptor set)**.
    *   `GuiFrameworkCorePlugin` system `cleanup_trigger_system` runs on `AppExit`, cleaning up `Renderer` (including text resources), `GlyphAtlas`, and `VulkanContext`.

## Key Interactions (Post Text Rendering Implementation)
- **Plugins <-> Bevy App**: Plugins register components/resources/events and add systems to schedules.
- **Plugins <-> System Sets**: Plugins define and use `SystemSet`s (`CoreSet`, `InteractionSet`, etc.) to configure execution order.
- **App Systems (`main.rs`) <-> Plugin Sets**: Application systems order themselves relative to plugin sets.
- **Interaction Plugin -> Events**: Writes interaction/hotkey/exit events.
- **Movement Plugin <- Events**: Reads `EntityDragged`.
- **Bindings Plugin <- Events**: Reads `HotkeyActionTriggered`.
- **Core Plugin (Text Layout)**: `text_layout_system` reads `Text`, `Transform`; uses `FontServerResource`, `SwashCacheResource`; reads `VulkanContextResource`; reads/writes `GlyphAtlasResource`; writes `TextLayoutOutput`.
- **Core Plugin (Rendering)**: `rendering_system` reads `ShapeData`, `TextLayoutOutput`, `GlobalTransform`; reads `GlyphAtlasResource`; **generates `TextVertex` data**; calls `Renderer::render`.
- **Core Plugin (Cleanup)**: `cleanup_trigger_system` calls `Renderer::cleanup`, `GlyphAtlas::cleanup`, `vulkan_setup::cleanup_vulkan`.
- **Renderer <-> BufferManager**: Renderer owns and calls BufferManager for **shape** resource prep/cleanup.
- **Renderer <-> GlyphAtlas**: Renderer uses `GlyphAtlasResource` to get handles for atlas descriptor set update.
- **Renderer <-> PipelineManager**: Renderer uses layouts/pool created by PipelineManager during initialization.
- **Renderer <-> Command Buffers**: `Renderer::render` calls `record_command_buffers`, passing shape data and **text rendering resources/metadata**.
- **Renderer <-> Shader Utils**: `Renderer::new` uses `shader_utils` to load text shaders for pipeline creation.
- **BufferManager <-> VulkanContext**: BufferManager uses `shape_pipeline_layout` and other context info.
- **GlyphAtlas <-> VulkanContext**: GlyphAtlas uses context for image/sampler creation and upload command execution.
- **Core Plugin <-> ResizeHandler**: `handle_resize_system` calls `Renderer::resize_renderer` which uses `ResizeHandler`.
- **App Systems (`main.rs`) <-> Components**: `background_resize_system` modifies `ShapeData`.
- **VulkanContext <-> Vulkan Components**: Provides core Vulkan resources, including separate pipeline layouts for shapes and text.
- **Build Script -> Target Directory**: Compiles shaders, copies assets.

## Current Capabilities (Post Text Rendering Implementation)
- **Bevy Integration**: Application runs within `bevy_app`, using core non-rendering plugins.
- **Modular Framework**: Core rendering, interaction, text foundation, and default behaviors encapsulated in Bevy Plugins.
- **System Set Ordering**: Explicit execution order defined using Bevy `SystemSet`s.
- **Bevy Math**: Uses `bevy_math` types (`Vec2`, `Mat4`).
- **Bevy ECS**: Defines and uses custom components (`ShapeData`, `Visibility`, `Interaction`, `BackgroundQuad`, `Text`, `TextLayoutOutput`) and `bevy_transform::components::Transform`.
- **Bevy Events**: Defines and uses custom events (`EntityClicked`, `EntityDragged`, `HotkeyActionTriggered`) for system communication.
- **Bevy Reflection**: Core framework components/events/resources implement `Reflect` where feasible and are registered.
- **Bevy Change Detection**: Uses `Changed<ShapeData>` and `Changed<Text>` filters to trigger updates.
- **Input Processing**: Uses `bevy_input` via `GuiFrameworkInteractionPlugin`. Basic click detection, dragging, and hotkey dispatching implemented. Hit detection uses **Y-up world coordinates**.
- **Vulkan Setup**: Core context initialized via `GuiFrameworkCorePlugin`. Separate pipeline layouts for shapes and text created.
- **Rendering Bridge**: Custom Vulkan context (`VulkanContextResource`) and renderer (`RendererResource`) managed as Bevy resources.
- **ECS-Driven Resource Management**: `BufferManager` creates/updates Vulkan resources for **shapes**. `GlyphAtlas` manages Vulkan texture for glyphs (packing, rasterization data input, upload). **`Renderer` manages dynamic vertex buffer, descriptor set, and pipeline for text.**
- **Resource Caching**: `BufferManager` caches shape pipelines/shaders. `GlyphAtlas` caches glyph locations/UVs.
- **Rendering Path**: Data flows from ECS (`rendering_system`) through `Renderer` (shape prep via `BufferManager`, **text vertex buffer update**) to `command_buffers` (shape and **text** draw recording). Synchronization corrected. Projection matrix uses logical window size and Y-flip.
- **Configurable Hotkeys**: Loads from TOML file into `HotkeyResource` via `GuiFrameworkInteractionPlugin`.
- **Build Script**: Compiles GLSL shaders (including text shaders) to SPIR-V, copies assets, tracks shader changes.
- **Resize Handling**: Correctly handles window resizing, swapchain recreation, framebuffer updates, projection matrix updates, and dynamic background vertex updates.
- **Visual Output**: Functional 2D rendering of **shapes** and **static, baseline-aligned text** based on ECS data. Background dynamically resizes. Objects positioned correctly according to **Y-up world coordinates**. Alpha blending enabled for text.
- **Text Foundation**:
    - Text component definition (`Text`, `TextVertex`).
    - Font loading and management (`FontServer`, `FontServerResource`).
    - Glyph atlas resource management (`GlyphAtlas`, `GlyphAtlasResource`), including packing via `rectangle-pack`, rasterization data input, and upload logic.
    - CPU-side text layout system (`text_layout_system` using `cosmic-text`, calculates baseline-aligned vertex positions).
    - Intermediate text layout component (`TextLayoutOutput`).
    - Text shaders (`text.vert`, `text.frag`) created and used by text pipeline.
    - Text-specific Vulkan layouts and pipeline created.
    - **Dynamic text vertex buffer management implemented in `Renderer`.**
- **Robust Shutdown**: Application exit sequence correctly cleans up Vulkan resources (`Renderer` including text resources, `GlyphAtlas`, `VulkanContext`) via `cleanup_trigger_system`.

## Future Extensions
- **Text Handling**: Implement text editing (`yrs`), context menus (Task 9). Refactor text rendering resource management (see Notes). Improve text rendering quality (e.g., SDF).
- **Rendering Optimization**: Implement resource removal for despawned entities (using `RemovedComponents`). Optimize text vertex buffer updates (process only changed entities). Fix text descriptor set binding (Set 0).
- **Hit Detection**: Improve Z-sorting/picking logic in `interaction_system` for overlapping objects.
- **Bevy State Integration**: Consider Bevy State for managing application modes.
- Instancing via ECS.
- Batch operations for groups (using ECS queries/components).
- P2P networking.
- 3D rendering and AI-driven content generation.
- Divider system (Task 10).

## Error Handling
- Vulkan errors checked via `ash` results. Validation layers used for debugging. `vk-mem` assertions check memory leaks. `map_memory` errors handled. Persistent map access uses `get_allocation_info`.
- Logical errors (`HotkeyError`) use `Result` or `thiserror`.
- Hotkey file loading/parsing errors handled gracefully with defaults and logs.
- Bevy logging integrated via `LogPlugin`. Mutex poisoning handled with logs.
- Renderer/Atlas checks for `None` resources during shutdown to prevent panics. Startup systems now `panic!` on critical setup errors.

## Shader Integration
- GLSL shaders (`.vert`, `.frag`) in `shaders/` compiled to SPIR-V (`.spv`) by `build.rs`. Compiled `.spv` files copied to target directory.
- Shape shaders loaded and cached by `BufferManager`. Support orthographic projection (Set 0, Binding 0) and object transformation matrix (Set 0, Binding 1). `background.vert` ignores object transform.
- Text shaders (`text.vert`, `text.frag`) loaded by `Renderer` for text pipeline creation. `text.vert` uses Set 0 (Binding 0: projection). `text.frag` uses Set 1 (Binding 0: glyph atlas sampler).

## Dependencies
- External crates: `ash = "0.38"`, `vk-mem = "0.4"`, `ash-window = "0.13"`, `raw-window-handle = "0.6"`, `toml = "0.8"`, `thiserror = "2.0"`, `bevy_app = "0.15"`, `bevy_core = "0.15"`, `bevy_ecs = "0.15"`, `bevy_log = "0.15"`, `bevy_utils = "0.15"`, `bevy_window = "0.15"`, `bevy_winit = "0.15"`, `bevy_reflect = "0.15"`, `bevy_input = "0.15"`, `bevy_time = "0.15"`, `bevy_diagnostic = "0.15"`, `bevy_a11y = "0.15"`, `bevy_math = "0.15"`, `bevy_transform = "0.15"`, `bevy_color = "0.15"`, `winit = "0.30.9"`, `cosmic-text = "0.14"`, `fontdb = "0.23"`, `swash = "0.2"`, `rectangle-pack = "0.4"`.
- Build dependencies: `walkdir = "2"`.
- Shaders: Precompiled `.spv` files in target directory.

## Notes
- **Framework Structure**: Core logic refactored into Bevy Plugins using System Sets for ordering. Application logic resides in `main.rs`.
- **Coordinate System**: Uses a **Y-up world coordinate system** (origin bottom-left). Projection matrix includes a Y-flip to match Vulkan's default Y-down NDC space. Input and movement systems handle coordinate conversions correctly. Text vertex generation in `rendering_system` correctly transforms relative glyph coordinates (calculated for baseline alignment) to world space.
- **Known Issues**:
    - `BufferManager` lacks resource removal for despawned entities and only handles shapes.
    - Hit detection Z-sorting needs review.
    - `vulkan_setup` still uses `winit::window::Window` directly.
    - **Text Rendering:** Current implementation manages text Vulkan resources within `Renderer`, which could be refactored for better encapsulation/efficiency. Binding descriptor Set 0 for text uses an incorrect workaround and needs fixing. Minor visual artifacts may occur near glyph edges with linear filtering.
- **Cleanup Logic**: Synchronous cleanup on `AppExit` is handled by `cleanup_trigger_system` within the `GuiFrameworkCorePlugin`, running in the `Last` schedule.