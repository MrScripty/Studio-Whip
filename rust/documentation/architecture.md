# Architecture Overview for `rusty_whip`

## Purpose
`rusty_whip` is an advanced 2D and 3D content generation application for digital entertainment production, leveraging GPU-accelerated AI diffusion/inference, multimedia creation (2D video, stills, animation, audio), and story-driven workflows, with plans for quantum-resistant P2P networking. **It uses the Bevy game engine (v0.15) for its core application structure, input handling, and ECS (Entity Component System), while strictly avoiding Bevy's rendering stack (`bevy_render`, `wgpu`, etc.) in favor of its custom Vulkan backend.**

## Core Components
1.  **Vulkan Context Management (`context/`)**
    *   Role: Initializes and manages core Vulkan resources (instance, physical device, logical device, queue, allocator, pipeline layout, current swap extent). Accessed via `VulkanContextResource`. Holds resources needed globally or across different rendering stages.
    *   Key Modules: `vulkan_context.rs`, `vulkan_setup.rs`.
2.  **Rendering Engine (`rendering/`)**
    *   Role: Orchestrates the custom Vulkan rendering pipeline per frame. Manages `BufferManager` (for shapes), `GlyphAtlas` (for text), descriptor pool/layout handles, and Vulkan sync objects (fences, semaphores). Calls `BufferManager` and potentially text rendering logic to prepare resources based on ECS data and `command_buffers` to record draw calls. Handles swapchain recreation on resize, calculating projection matrix based on **logical window size with Y-flip** for Vulkan NDC. Accessed via `RendererResource`.
    *   Key Modules: `render_engine.rs`, `buffer_manager.rs`, `glyph_atlas.rs`, `resize_handler.rs`, `command_buffers.rs`, `swapchain.rs`, `shader_utils.rs`.
3.  **Pipeline Manager (`rendering/pipeline_manager.rs`)**
    *   Role: **Initialization helper.** Creates Vulkan `PipelineLayout`, `DescriptorSetLayout`, and `DescriptorPool` during application setup (currently for shapes). These resources are then transferred to `VulkanContext` (layout) or `Renderer` (pool, set layout) for operational use and cleanup.
    *   Key Modules: `pipeline_manager.rs`.
4.  **Buffer Manager (`rendering/buffer_manager.rs`)**
    *   Role: **Shape resource manager.** Manages the global projection uniform buffer and per-entity Vulkan resources (vertex buffers, transform UBOs, descriptor sets) for shapes based on ECS `RenderCommandData`. Uses layout/pool provided during initialization to allocate per-entity descriptor sets. **Caches pipelines and shader modules** based on shader paths. Updates vertex buffers based on `vertices_changed` flag in `RenderCommandData`. Updates descriptor sets **immediately per-entity**. **Lacks optimization for resource removal (despawned entities).**
    *   Key Modules: `buffer_manager.rs`.
5.  **Text Handling (`rendering/font_server.rs`, `rendering/glyph_atlas.rs`, `components/text_data.rs`, `components/text_layout.rs`)**
    *   Role: Manages font loading, text layout/shaping, glyph caching/rasterization, and prepares text data for rendering.
    *   Key Modules: `font_server.rs` (loads fonts via `fontdb`, provides `cosmic_text::FontSystem`), `glyph_atlas.rs` (manages Vulkan texture atlas for glyphs), `text_data.rs` (`Text` component), `text_layout.rs` (`TextLayoutOutput` component). Systems in `plugins/core.rs` orchestrate layout.
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
    *   Key Modules: `core.rs` (Vulkan/Renderer/Text setup, text layout, rendering, resize, cleanup), `interaction.rs` (Input processing, hotkey loading/dispatch, window close), `movement.rs` (Default drag movement), `bindings.rs` (Default hotkey actions).
10. **Bevy App Core (`main.rs`)**
    *   Role: Bootstraps the application using `bevy_app::App`. Initializes core Bevy plugins and framework plugins. Inserts initial `VulkanContextResource`. Defines and schedules **application-specific** systems (e.g., `setup_scene_ecs`, `background_resize_system`). Spawns initial entities with components.
11. **Build Script (`build.rs`)**
    *   Role: Compiles GLSL shaders (`.vert`, `.frag`) to SPIR-V (`.spv`) using `glslc`, copies compiled shaders and runtime assets (e.g., `user/hotkeys.toml`) to the target build directory. Tracks shader source files for recompilation.
    *   Key Modules: `build.rs`.

## Data Flow (Post Plugin Refactor & Text Foundation)
1.  `main.rs` initializes `bevy_app::App`, adds core non-rendering Bevy plugins, inserts `VulkanContextResource`, and adds framework plugins (`GuiFrameworkCorePlugin`, `GuiFrameworkInteractionPlugin`, etc.).
2.  Bevy `Startup` schedule runs:
    *   `GuiFrameworkCorePlugin` systems (`CoreSet::SetupVulkan`, `CoreSet::CreateRenderer`, `CoreSet::CreateGlyphAtlas`, `CoreSet::CreateFontServer`, `CoreSet::CreateSwashCache`) initialize Vulkan, create the `RendererResource`, `GlyphAtlasResource`, `FontServerResource`, `SwashCacheResource`.
    *   `GuiFrameworkInteractionPlugin` system (`InteractionSet::LoadHotkeys`) loads hotkeys into `HotkeyResource`.
    *   `main.rs` system `setup_scene_ecs` (ordered `.after(CoreSet::CreateRenderer)`) spawns initial application entities (shapes, potentially text later).
3.  Bevy `Update` schedule runs:
    *   `InputPlugin` populates `ButtonInput<T>` resources.
    *   `GuiFrameworkInteractionPlugin` systems (`InteractionSet::InputHandling`, `InteractionSet::WindowClose`) process input, send interaction/hotkey events, and handle window close requests.
    *   `GuiFrameworkDefaultMovementPlugin` system (`MovementSet::ApplyMovement`, ordered `.after(InteractionSet::InputHandling)`) updates `Transform` based on `EntityDragged` events.
    *   `GuiFrameworkDefaultBindingsPlugin` system (`BindingsSet::HandleActions`, ordered `.after(InteractionSet::InputHandling)`) handles default hotkey actions (e.g., sending `AppExit`).
    *   `GuiFrameworkCorePlugin` system (`CoreSet::HandleResize`) handles `WindowResized` events for the renderer.
    *   `main.rs` system `background_resize_system` handles `WindowResized` events for the background quad.
    *   `GuiFrameworkCorePlugin` system `text_layout_system` (`CoreSet::TextLayout`, ordered `.after(CoreSet::HandleResize)`) queries `Text` components, uses `FontServerResource`, `GlyphAtlasResource`, `SwashCacheResource` to perform layout, and inserts/updates `TextLayoutOutput` components.
4.  Bevy `Last` schedule runs:
    *   `GuiFrameworkCorePlugin` system `rendering_system` (`CoreSet::Render`, ordered `.after(CoreSet::TextLayout)`, runs if `AppExit` not sent) queries ECS for shapes (`RenderCommandData`) and text (`TextLayoutOutput`), collects render data, potentially updates glyph atlas via `GlyphAtlas::add_glyph`, and calls `Renderer::render`.
    *   `GuiFrameworkCorePlugin` system `cleanup_trigger_system` (`CoreSet::Cleanup`, ordered `.after(CoreSet::Render)`, runs if `AppExit` *is* sent) takes ownership of resources (`Renderer`, `GlyphAtlas`, `VulkanContext`) and performs synchronous Vulkan cleanup.

## Key Interactions (Post Plugin Refactor & Text Foundation)
- **Plugins <-> Bevy App**: Plugins register components/resources/events and add systems to schedules.
- **Plugins <-> System Sets**: Plugins define and use `SystemSet`s (`CoreSet`, `InteractionSet`, etc.) to configure execution order internally and relative to each other.
- **App Systems (`main.rs`) <-> Plugin Sets**: Application systems (`setup_scene_ecs`) use `.after()` to order themselves relative to plugin sets.
- **Interaction Plugin -> Events**: `interaction_system` writes `EntityClicked`/`EntityDragged`. `hotkey_system` writes `HotkeyActionTriggered`. `handle_close_request` writes `AppExit`.
- **Movement Plugin <- Events**: `movement_system` reads `EntityDragged`.
- **Bindings Plugin <- Events**: `app_control_system` reads `HotkeyActionTriggered`.
- **Core Plugin (Text Layout)**: `text_layout_system` reads `Text`, `Transform`; reads/writes `FontServerResource`, `GlyphAtlasResource`, `SwashCacheResource`; writes `TextLayoutOutput`.
- **Core Plugin (Rendering)**: `rendering_system` reads ECS data (`ShapeData`, `TextLayoutOutput`), reads `GlyphAtlasResource`, calls `Renderer::render`.
- **Core Plugin (Cleanup)**: `cleanup_trigger_system` calls `Renderer::cleanup`, `GlyphAtlas::cleanup`, `vulkan_setup::cleanup_vulkan`.
- **Renderer <-> BufferManager**: Renderer owns and calls BufferManager for shape resource prep/cleanup.
- **Renderer <-> GlyphAtlas**: Renderer will need access to the glyph atlas texture/sampler for text rendering.
- **BufferManager <-> VulkanContext**: BufferManager uses `pipeline_layout` and other context info.
- **GlyphAtlas <-> VulkanContext**: GlyphAtlas uses context for image/sampler creation and cleanup.
- **Core Plugin <-> ResizeHandler**: `handle_resize_system` calls `Renderer::resize_renderer` which uses `ResizeHandler`.
- **App Systems (`main.rs`) <-> Components**: `background_resize_system` modifies `ShapeData`.
- **VulkanContext <-> Vulkan Components**: Provides core Vulkan resources.
- **Build Script -> Target Directory**: Compiles shaders, copies assets.

## Current Capabilities (Post Plugin Refactor & Text Foundation)
- **Bevy Integration**: Application runs within `bevy_app`, using core non-rendering plugins.
- **Modular Framework**: Core rendering, interaction, text foundation, and default behaviors encapsulated in Bevy Plugins.
- **System Set Ordering**: Explicit execution order defined using Bevy `SystemSet`s.
- **Bevy Math**: Uses `bevy_math` types (`Vec2`, `Mat4`).
- **Bevy ECS**: Defines and uses custom components (`ShapeData`, `Visibility`, `Interaction`, `BackgroundQuad`, `Text`, `TextLayoutOutput`) and `bevy_transform::components::Transform`.
- **Bevy Events**: Defines and uses custom events (`EntityClicked`, `EntityDragged`, `HotkeyActionTriggered`) for system communication.
- **Bevy Reflection**: Core framework components/events/resources implement `Reflect` where feasible and are registered.
- **Bevy Change Detection**: Uses `Changed<ShapeData>` and `Changed<Text>` filters to trigger updates.
- **Input Processing**: Uses `bevy_input` via `GuiFrameworkInteractionPlugin`. Basic click detection, dragging, and hotkey dispatching implemented. Hit detection uses **Y-up world coordinates**.
- **Vulkan Setup**: Core context initialized via `GuiFrameworkCorePlugin`.
- **Rendering Bridge**: Custom Vulkan context (`VulkanContextResource`) and renderer (`RendererResource`) managed as Bevy resources.
- **ECS-Driven Resource Management**: `BufferManager` creates/updates Vulkan buffers/descriptors for shapes. `GlyphAtlas` manages Vulkan texture for glyphs.
- **Resource Caching**: `BufferManager` caches Vulkan `Pipeline` and `ShaderModule` resources. `GlyphAtlas` caches glyph locations/UVs (implementation pending).
- **Rendering Path**: Data flows from ECS (`rendering_system`) through `Renderer` to `BufferManager` (shape prep) and `command_buffers` (draw recording). Synchronization corrected using fences. **Projection matrix uses logical window size and Y-flip.** (Text rendering path partially implemented).
- **Configurable Hotkeys**: Loads from TOML file into `HotkeyResource` via `GuiFrameworkInteractionPlugin`.
- **Build Script**: Compiles GLSL shaders to SPIR-V, copies assets, tracks shader changes.
- **Resize Handling**: Correctly handles window resizing, swapchain recreation, framebuffer updates, **projection matrix updates (using logical size)**, and **dynamic background vertex updates** (via `main.rs` system).
- **Visual Output**: Functional 2D rendering of shapes based on ECS data. Background dynamically resizes. Objects positioned correctly according to **Y-up world coordinates**.
- **Text Foundation**:
    - Text component definition (`Text`).
    - Font loading and management (`FontServer`, `FontServerResource`).
    - Glyph atlas resource creation (`GlyphAtlas`, `GlyphAtlasResource`).
    - CPU-side text layout system (`text_layout_system` using `cosmic-text`).
    - Intermediate text layout component (`TextLayoutOutput`).
- **Robust Shutdown**: Application exit sequence correctly cleans up Vulkan resources (`Renderer`, `GlyphAtlas`, `VulkanContext`) via `cleanup_trigger_system` in the `Last` schedule.

## Future Extensions
- **Text Handling (Task 8+)**: Implement glyph rasterization/upload (`GlyphAtlas::add_glyph`), text rendering pipeline, text editing (`yrs`), context menus (Task 9).
- **Rendering Optimization**: Implement resource removal for despawned entities (using `RemovedComponents`).
- **Hit Detection**: Improve Z-sorting/picking logic in `interaction_system` for overlapping objects.
- **Bevy State Integration**: Consider Bevy State for managing application modes (e.g., editing vs. viewing).
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
- Shaders loaded and cached by `BufferManager` based on `ShapeData` component paths.
- Shaders support orthographic projection (binding 0) and object transformation matrix (binding 1). Only vertex position (location 0) is currently used as input. **`background.vert` ignores object transform.** (Text shaders TBD).

## Dependencies
- External crates: `ash = "0.38"`, `vk-mem = "0.4"`, `ash-window = "0.13"`, `raw-window-handle = "0.6"`, `toml = "0.8"`, `thiserror = "2.0"`, `bevy_app = "0.15"`, `bevy_core = "0.15"`, `bevy_ecs = "0.15"`, `bevy_log = "0.15"`, `bevy_utils = "0.15"`, `bevy_window = "0.15"`, `bevy_winit = "0.15"`, `bevy_reflect = "0.15"`, `bevy_input = "0.15"`, `bevy_time = "0.15"`, `bevy_diagnostic = "0.15"`, `bevy_a11y = "0.15"`, `bevy_math = "0.15"`, `bevy_transform = "0.15"`, `bevy_color = "0.15"`, `winit = "0.30.9"`, `cosmic-text = "0.14"`, `fontdb = "0.23"`, `swash = "0.2"`, `rectangle-pack = "0.4"`.
- Build dependencies: `walkdir = "2"`.
- Shaders: Precompiled `.spv` files in target directory.

## Notes
- **Framework Structure**: Core logic refactored into Bevy Plugins using System Sets for ordering. Application logic resides in `main.rs`.
- **Coordinate System**: Uses a **Y-up world coordinate system** (origin bottom-left). Projection matrix includes a Y-flip to match Vulkan's default Y-down NDC space. Input and movement systems handle coordinate conversions correctly. Text layout currently assumes Y-down relative to entity origin, needs adjustment for rendering.
- **Known Issues**: `BufferManager` lacks resource removal for despawned entities. Hit detection Z-sorting needs review. `vulkan_setup` still uses `winit::window::Window` directly. Text rendering pipeline not implemented. Glyph caching/rasterization/upload not implemented. Text glyph key generation is placeholder.
- **Cleanup Logic**: Synchronous cleanup on `AppExit` is handled by `cleanup_trigger_system` within the `GuiFrameworkCorePlugin`, running in the `Last` schedule.