# Modules Documentation for `rusty_whip`

## Project Overview: `rusty_whip`
### Purpose
`rusty_whip` is an advanced 2D & 3D content generation application with GPU-accelerated AI diffusion/inference, multimedia creation, and story-driven workflows, targeting P2P networking. **It uses the Bevy engine (v0.15) for its core application structure, input handling, and ECS, while strictly avoiding Bevy's rendering stack.**
### Current State (Post Basic Text Rendering Implementation)
- **Bevy Integration**: Application runs using `bevy_app::App`, `bevy_winit`, `bevy_input`, `bevy_transform`, `bevy_log`, `bevy_reflect`, `bevy_color` and other core non-rendering Bevy plugins.
- **Plugin Architecture**: Core framework logic (rendering, interaction, text foundation, default behaviors) refactored into modular Bevy plugins (`GuiFrameworkCorePlugin`, `GuiFrameworkInteractionPlugin`, etc.) using `SystemSet`s for execution ordering. Application-specific logic remains in `main.rs`.
- **ECS Core**: Application state and logic managed via Bevy ECS components (`ShapeData`, `Visibility`, `Interaction`, `Transform`, `BackgroundQuad`, `Text`, `TextLayoutOutput`). User input processed by plugin systems, triggering Bevy events (`EntityClicked`, `EntityDragged`, `HotkeyActionTriggered`).
- **Reflection**: Core framework components, events, and resources implement `Reflect` where feasible and are registered by the plugins. `TextLayoutOutput` and `PositionedGlyph` currently do not support reflection due to containing external types.
- **Math Migration**: Uses `bevy_math` types (`Vec2`, `Mat4`).
- **Rendering Bridge**: Custom Vulkan context (`VulkanContext`) managed via `VulkanContextResource`. Custom Vulkan renderer (`Renderer`) accessed via `RendererResource`. Both resources defined in `lib.rs`.
- **Rendering Status**: Rendering triggered by `rendering_system` (in `GuiFrameworkCorePlugin`). `BufferManager` creates/updates Vulkan resources for **shapes**. `Renderer` manages dynamic vertex buffer, descriptor set, and pipeline for **text**. `GlyphAtlas` manages Vulkan texture for glyphs. `command_buffers` records draw calls for **shapes and text**. Synchronization and resize handling are corrected. **Visual output functional for shapes and static text.** Optimizations needed: resource removal for despawned entities, text rendering efficiency/encapsulation, text descriptor set binding (Set 0).
- **Text Handling**: Foundation laid with `Text` component, `FontServer` (loading system fonts), `GlyphAtlas` (Vulkan texture setup, packing, rasterization via `swash`, upload), `SwashCache` resource, and `text_layout_system` (CPU-side layout using `cosmic-text`, calls `add_glyph`). Text shaders and Vulkan layouts created. **Basic text rendering pipeline implemented:** `rendering_system` generates vertices, `Renderer` manages buffer/pipeline/descriptor set, `command_buffers` draws text.
- **Shutdown**: Robust shutdown sequence implemented via `cleanup_trigger_system` (in `GuiFrameworkCorePlugin`) running on `AppExit` in the `Last` schedule, cleaning up Renderer (including text resources), GlyphAtlas, and VulkanContext.
- **Features Active**: Bevy app structure, windowing, logging, reflection (partial), input handling (click, drag, hotkeys via plugins), ECS component/event usage, `bevy_transform`, core Vulkan setup (shape/text layouts), hotkey loading, ECS-driven Vulkan resource management (shapes), dynamic vertex updates (shapes), corrected synchronization and resize handling, robust shutdown, dynamic background resizing (app-specific), text component definition, font loading, glyph atlas resource management (including packing/upload), CPU-side text layout, **basic text rendering (vertex generation, buffer management, pipeline, drawing)**, text shaders.
- Task 1-7 **Complete**. Task 8 **Mostly Complete** (Layout, Atlas, Basic Rendering complete; Editing pending). Legacy `event_bus` and `scene` modules **removed**.

## Module Structure
Studio_Whip/
    LICENSE
    README.md
    .gitignore
    Rust/
        src/
            gui_framework/
                components/
                    interaction.rs
                    mod.rs
                    shape_data.rs
                    text_data.rs
                    text_layout.rs
                    visibility.rs
                context/
                    vulkan_context.rs
                    vulkan_setup.rs
                    mod.rs
                events/
                    interaction_events.rs
                    mod.rs
                interaction/
                    hotkeys.rs
                    mod.rs
                plugins/
                    bindings.rs
                    core.rs
                    interaction.rs
                    mod.rs
                    movement.rs
                rendering/
                    buffer_manager.rs
                    command_buffers.rs
                    font_server.rs
                    glyph_atlas.rs
                    mod.rs
                    pipeline_manager.rs
                    render_engine.rs
                    resize_handler.rs
                    shader_utils.rs
                    swapchain.rs
                mod.rs
            lib.rs
            main.rs
        shaders/
            background.frag
            background.vert
            square.frag
            square.vert
            text.frag
            text.vert
            triangle.frag
            triangle.vert
        user/
            hotkeys.toml
        Documentation/
            modules.md # This file
            architecture.md
            usage.md
            # ... other docs ...
        utilities/
            # ... utils ...
        Cargo.toml
        build.rs
        # Removed compile_shaders scripts


## Modules and Their Functions

### `src/lib.rs`
- **Purpose**: Defines shared types (`Vertex`, `TextVertex`, `RenderCommandData`, `PreparedDrawData`, `TextRenderCommandData`) and shared framework resources (`VulkanContextResource`, `RendererResource`, `HotkeyResource`, `GlyphAtlasResource`, `FontServerResource`, `SwashCacheResource`).
- **Key Structs**:
    - `Vertex { position: [f32; 2] }` (Derives `Reflect`)
    - `TextVertex { position: [f32; 2], uv: [f32; 2] }` (Derives `Debug`, `Clone`, `Copy`) - **New**
    - `RenderCommandData { entity_id: Entity, transform_matrix: Mat4, vertices: Arc<Vec<Vertex>>, vertex_shader_path: String, fragment_shader_path: String, depth: f32, vertices_changed: bool }`
    - `PreparedDrawData { pipeline: vk::Pipeline, vertex_buffer: vk::Buffer, vertex_count: u32, descriptor_set: vk::DescriptorSet }`
    - `TextRenderCommandData { vertex_buffer_offset: u32, vertex_count: u32 }` (Derives `Debug`, `Clone`) - **New**
    - `VulkanContextResource(pub Arc<Mutex<VulkanContext>>)` (Derives `Resource`, `Clone`)
    - `RendererResource(pub Arc<Mutex<Renderer>>)` (Derives `Resource`, `Clone`)
    - `HotkeyResource(pub HotkeyConfig)` (Derives `Resource`, `Debug`, `Clone`, `Default`, `Reflect`)
    - `GlyphAtlasResource(pub Arc<Mutex<GlyphAtlas>>)` (Derives `Resource`, `Clone`)
    - `FontServerResource(pub Arc<Mutex<FontServer>>)` (Derives `Resource`, `Clone`)
    - `SwashCacheResource(pub Mutex<SwashCache>)` (Derives `Resource`)
- **Notes**: Uses `ash::vk`, `bevy_ecs::Entity`, `bevy_math::Mat4`, `std::sync::Arc`, `bevy_reflect::Reflect`, `cosmic_text::SwashCache`. Resources defined here for easy import across app and plugins.

### `src/main.rs`
- **Purpose**: Entry point; sets up `bevy_app::App`, core Bevy plugins, framework plugins (`GuiFrameworkCorePlugin`, etc.), inserts initial `VulkanContextResource`, and defines/schedules **application-specific** systems (`setup_scene_ecs`, `background_resize_system`).
- **Key Structs (Defined In `main.rs`)**:
    - `BackgroundQuad` (Derives `Component`, marker struct)
- **Key Methods (Bevy Systems)**:
    - `main() -> ()`
    - `setup_scene_ecs(...) -> ()`: Spawns initial application-specific ECS entities (incl. background, sample text). Runs after `CoreSet::CreateRenderer`.
    - `background_resize_system(...) -> ()`: Reads `WindowResized`, updates background `ShapeData.vertices`. (App-specific update logic).
- **Notes**: Relies on plugins for framework setup, rendering, input, text handling, and cleanup. Defines application structure and specific scene/behavior.

### `src/gui_framework/mod.rs`
- **Purpose**: Declares and re-exports framework modules (`components`, `context`, `events`, `interaction`, `plugins`, `rendering`).
- **Notes**: Provides access to framework internals if needed.

### `src/gui_framework/components/mod.rs`
- **Purpose**: Declares and re-exports Bevy ECS component modules.
- **Modules**: `shape_data.rs`, `visibility.rs`, `interaction.rs`, `text_data.rs`, `text_layout.rs`.
- **Exports**: `ShapeData`, `Visibility`, `Interaction`, `Text`, `FontId`, `TextAlignment`, `TextLayoutOutput`, `PositionedGlyph`.

### `src/gui_framework/components/shape_data.rs`
- **Purpose**: Defines the visual shape data for an entity.
- **Key Structs**: `ShapeData { vertices: Arc<Vec<Vertex>>, vertex_shader_path: String, fragment_shader_path: String }` (Derives `Component`, `Debug`, `Clone`, `Reflect`).
- **Notes**: Used by `rendering_system` (core plugin) and `background_resize_system` (main.rs). Registered by `GuiFrameworkCorePlugin`.

### `src/gui_framework/components/visibility.rs`
- **Purpose**: Defines a custom visibility component to avoid `bevy_render`.
- **Key Structs**: `Visibility(pub bool)` (Derives `Component`, `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Reflect`).
- **Key Methods**: `is_visible(&self) -> bool`.
- **Notes**: Used by `rendering_system` and `text_layout_system` (core plugin). Registered by `GuiFrameworkCorePlugin`.

### `src/gui_framework/components/interaction.rs`
- **Purpose**: Defines interaction properties for an entity.
- **Key Structs**: `Interaction { clickable: bool, draggable: bool }` (Derives `Component`, `Debug`, `Clone`, `Copy`, `Reflect`).
- **Notes**: Used by `interaction_system` (interaction plugin). Registered by `GuiFrameworkInteractionPlugin`.

### `src/gui_framework/components/text_data.rs`
- **Purpose**: Defines the input data for a text entity.
- **Key Structs**:
    - `FontId(pub usize)` (Derives `Debug`, `Clone`, `PartialEq`, `Eq`, `Hash`, `Reflect`) - Placeholder.
    - `TextAlignment` (enum: `Left`, `Center`, `Right`) (Derives `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Reflect`, `Default`).
    - `Text { content: String, size: f32, color: Color, alignment: TextAlignment, bounds: Option<Vec2> }` (Derives `Component`, `Debug`, `Clone`, `Reflect`, `Default`).
- **Notes**: Used by `text_layout_system` (core plugin). Registered by `GuiFrameworkCorePlugin`. Uses `bevy_color::Color`.

### `src/gui_framework/components/text_layout.rs`
- **Purpose**: Defines intermediate and output data structures for text layout results.
- **Key Structs**:
    - `PositionedGlyph { glyph_info: GlyphInfo, layout_glyph: LayoutGlyph, vertices: [Vec2; 4] }` (Derives `Debug`, `Clone`). Contains non-reflectable `LayoutGlyph`.
    - `TextLayoutOutput { glyphs: Vec<PositionedGlyph> }` (Derives `Component`, `Debug`, `Clone`, `Default`). Contains non-reflectable `PositionedGlyph`.
- **Notes**: Written by `text_layout_system`, read by `rendering_system` (core plugin). Cannot be fully reflected due to containing external `cosmic_text::LayoutGlyph`.

### `src/gui_framework/context/mod.rs`
- **Purpose**: Re-exports Vulkan context modules.
- **Modules**: `vulkan_context.rs`, `vulkan_setup.rs`.

### `src/gui_framework/context/vulkan_context.rs`
- **Purpose**: Holds core Vulkan resources and related state.
- **Key Structs**: `VulkanContext { ... current_swap_extent: vk::Extent2D, ... shape_pipeline_layout: Option<vk::PipelineLayout>, text_pipeline_layout: Option<vk::PipelineLayout>, ... allocator: Option<Arc<Allocator>>, ... command_pool: Option<vk::CommandPool>, ... }`.
- **Key Methods**: `new() -> Self`.
- **Notes**: Managed via `VulkanContextResource`. Holds `Arc<Allocator>`.

### `src/gui_framework/context/vulkan_setup.rs`
- **Purpose**: Sets up and tears down core Vulkan resources managed by `VulkanContext`. Includes explicit allocator drop before device destruction.
- **Key Methods**: `setup_vulkan(...) -> ()`, `cleanup_vulkan(...) -> ()`.
- **Notes**: Called by systems in `GuiFrameworkCorePlugin`. Still uses `winit::window::Window`.

### `src/gui_framework/events/mod.rs`
- **Purpose**: Declares and re-exports Bevy Event modules.
- **Modules**: `interaction_events.rs`.

### `src/gui_framework/events/interaction_events.rs`
- **Purpose**: Defines Bevy events related to user interaction.
- **Key Structs**: `EntityClicked { entity: Entity }`, `EntityDragged { entity: Entity, delta: Vec2 }`, `HotkeyActionTriggered { action: String }`.
- **Notes**: Written by systems in `GuiFrameworkInteractionPlugin`, read by other plugin/app systems. Registered by `GuiFrameworkInteractionPlugin`.

### `src/gui_framework/interaction/mod.rs`
- **Purpose**: Declares interaction-related submodules (currently only `hotkeys`).
- **Modules**: `hotkeys.rs`.

### `src/gui_framework/interaction/hotkeys.rs`
- **Purpose**: Defines hotkey configuration loading from TOML.
- **Key Structs**: `HotkeyConfig`, `HotkeyError`.
- **Key Methods**: `load_config(...) -> Result<Self, HotkeyError>`, `get_action(&self, key_combo: &str) -> Option<&String>`.
- **Notes**: Used by `load_hotkeys_system` (interaction plugin). `HotkeyConfig` stored within `HotkeyResource`.

### `src/gui_framework/plugins/mod.rs`
- **Purpose**: Declares and re-exports framework plugin modules.
- **Modules**: `core.rs`, `interaction.rs`, `movement.rs`, `bindings.rs`.

### `src/gui_framework/plugins/core.rs`
- **Purpose**: Plugin for core Vulkan setup, rendering (shapes & text), text foundation (setup, layout, atlas upload), resize handling, and cleanup.
- **Key Structs**: `GuiFrameworkCorePlugin`, `CoreSet` (enum SystemSet: `SetupVulkan`, `CreateRenderer`, `CreateGlyphAtlas`, `CreateFontServer`, `CreateSwashCache`, `HandleResize`, `TextLayout`, `Render`, `Cleanup`).
- **Key Methods (Bevy Systems)**: `setup_vulkan_system() -> ()`, `create_renderer_system() -> ()`, `create_glyph_atlas_system() -> ()`, `create_font_server_system() -> ()`, `create_swash_cache_system() -> ()`, `handle_resize_system(...) -> ()`, `text_layout_system(...) -> ()`, `rendering_system(...) -> ()`, `cleanup_trigger_system(world: &mut World) -> ()`.
- **Notes**: Adds systems to `Startup`, `Update`, `Last` schedules. Configures `CoreSet` ordering. Registers core types. Inserts `RendererResource`, `GlyphAtlasResource`, `FontServerResource`, `SwashCacheResource`. `text_layout_system` performs CPU layout and triggers glyph upload. `rendering_system` prepares shape data, **generates text vertices**, and calls `Renderer::render`. `cleanup_trigger_system` cleans Vulkan resources.

### `src/gui_framework/plugins/interaction.rs`
- **Purpose**: Plugin for input processing (mouse, keyboard), hotkey loading/dispatch, and window close requests.
- **Key Structs**: `GuiFrameworkInteractionPlugin`, `InteractionSet` (enum SystemSet).
- **Key Methods (Bevy Systems)**: `load_hotkeys_system(...) -> ()`, `interaction_system(...) -> ()`, `hotkey_system(...) -> ()`, `handle_close_request(...) -> ()`.
- **Notes**: Adds systems to `Startup`, `Update` schedules. Configures `InteractionSet`. Registers `Interaction`, `HotkeyResource`, `HotkeyConfig`, interaction events. Inserts `HotkeyResource`.

### `src/gui_framework/plugins/movement.rs`
- **Purpose**: Optional plugin providing default entity movement based on `EntityDragged` events.
- **Key Structs**: `GuiFrameworkDefaultMovementPlugin`, `MovementSet` (enum SystemSet).
- **Key Methods (Bevy Systems)**: `movement_system(...) -> ()`.
- **Notes**: Adds system to `Update` schedule, ordered `.after(InteractionSet::InputHandling)`.

### `src/gui_framework/plugins/bindings.rs`
- **Purpose**: Optional plugin providing default handling for specific `HotkeyActionTriggered` events (e.g., "CloseRequested").
- **Key Structs**: `GuiFrameworkDefaultBindingsPlugin`, `BindingsSet` (enum SystemSet).
- **Key Methods (Bevy Systems)**: `app_control_system(...) -> ()`.
- **Notes**: Adds system to `Update` schedule, ordered `.after(InteractionSet::InputHandling)`.

### `src/gui_framework/rendering/mod.rs`
- **Purpose**: Re-exports rendering sub-modules and key types (`Renderer`, `GlyphAtlas`, `FontServer`, etc.).
- **Modules**: `buffer_manager.rs`, `command_buffers.rs`, `pipeline_manager.rs`, `render_engine.rs`, `resize_handler.rs`, `shader_utils.rs`, `swapchain.rs`, `pub glyph_atlas.rs`, `pub font_server.rs`.
- **Exports**: `Renderer`, `GlyphAtlas`, `GlyphAtlasResource`, `GlyphInfo`, `FontServer`, `FontServerResource`.

### `src/gui_framework/rendering/command_buffers.rs`
- **Purpose**: Records Vulkan command buffers for drawing shapes and text. Sets dynamic viewport/scissor. Clears framebuffer.
- **Key Methods**: `record_command_buffers(platform: &mut VulkanContext, prepared_shape_draws: &[PreparedDrawData], text_vertex_buffer: vk::Buffer, glyph_atlas_descriptor_set: vk::DescriptorSet, text_command: Option<&TextRenderCommandData>, extent: vk::Extent2D, text_pipeline: vk::Pipeline) -> ()`. (**Updated signature**)
- **Notes**: Called by `render_engine.rs`. Binds shape resources per entity. Binds text pipeline, dynamic text vertex buffer, and atlas descriptor set once for all text. **Uses incorrect workaround for text descriptor Set 0 binding.**

### `src/gui_framework/rendering/render_engine.rs`
- **Purpose**: Orchestrates custom Vulkan rendering per frame. Manages `BufferManager` (shapes), sync objects, **dynamic text vertex buffer, text pipeline, and glyph atlas descriptor set**. Handles resize via `ResizeHandler`. Calculates projection matrix. Integrates shape and text rendering paths.
- **Key Structs**: `Renderer { buffer_manager: BufferManager, descriptor_pool: vk::DescriptorPool, descriptor_set_layout: vk::DescriptorSetLayout, text_descriptor_set_layout: vk::DescriptorSetLayout, text_vertex_buffer: vk::Buffer, text_vertex_allocation: Option<Allocation>, text_vertex_buffer_capacity: u32, glyph_atlas_descriptor_set: vk::DescriptorSet, text_pipeline: vk::Pipeline }`. (**Updated fields**)
- **Key Methods**: `new(platform: &mut VulkanContext, extent: vk::Extent2D) -> Self`, `resize_renderer(vulkan_context: &mut VulkanContext, width: u32, height: u32) -> ()`, `render(platform: &mut VulkanContext, shape_commands: &[RenderCommandData], text_vertices: &[TextVertex], glyph_atlas_resource: &GlyphAtlasResource) -> ()`, `cleanup(platform: &mut VulkanContext) -> ()`. (**Updated signatures/methods**)
- **Notes**: Managed via `RendererResource`. Called by `rendering_system` (core plugin). `new` initializes shape/text layouts, creates text pipeline and dynamic vertex buffer. `render` updates shape buffers (via `BufferManager`), updates/resizes text vertex buffer, ensures atlas descriptor set exists, calls `record_command_buffers`. `cleanup` destroys shape/text layouts, text pipeline, text buffer, descriptor pool.

### `src/gui_framework/rendering/pipeline_manager.rs`
- **Purpose**: **Initialization helper.** Creates Vulkan `PipelineLayout`s (shape, text), `DescriptorSetLayout`s (shape, text), and a shared `DescriptorPool`.
- **Key Structs**: `PipelineManager { shape_pipeline_layout: vk::PipelineLayout, shape_descriptor_set_layout: vk::DescriptorSetLayout, text_pipeline_layout: vk::PipelineLayout, text_descriptor_set_layout: vk::DescriptorSetLayout, descriptor_pool: vk::DescriptorPool }`.
- **Key Methods**: `new(platform: &mut VulkanContext) -> Self`.
- **Notes**: Provides layouts/pool to `Renderer` during initialization.

### `src/gui_framework/rendering/buffer_manager.rs`
- **Purpose**: Manages global projection UBO and per-entity Vulkan resources (vertex buffers, transform UBOs, descriptor sets) **for shapes only**. Caches shape pipelines/shaders. Updates vertex buffers based on `RenderCommandData.vertices_changed`. Updates descriptor sets immediately per-entity.
- **Key Structs**: `EntityRenderResources`, `PipelineCacheKey`, `ShaderCacheKey`, `BufferManager`.
- **Key Methods**: `new(platform: &mut VulkanContext, _pipeline_layout: vk::PipelineLayout, descriptor_set_layout: vk::DescriptorSetLayout, descriptor_pool: vk::DescriptorPool) -> Self`, `prepare_frame_resources(platform: &mut VulkanContext, render_commands: &[crate::RenderCommandData]) -> Vec<PreparedDrawData>`, `cleanup(platform: &mut VulkanContext) -> ()`.
- **Notes**: Creates/updates resources based on `RenderCommandData`. Uses persistently mapped pointers. Uses `shape_pipeline_layout`. **Lacks resource removal for despawned entities.** Does not handle text vertices/pipelines/descriptors.

### `src/gui_framework/rendering/resize_handler.rs`
- **Purpose**: Handles window resizing logic: cleans up old swapchain resources, creates new swapchain (using physical extent), creates framebuffers, updates projection uniform buffer (using logical size + Y-flip).
- **Key Structs**: `ResizeHandler` (stateless).
- **Key Methods**: `resize(vulkan_context: &mut VulkanContext, logical_extent: vk::Extent2D, uniform_allocation: &mut vk_mem::Allocation) -> ()`.
- **Notes**: Called by `Renderer::resize_renderer`.

### `src/gui_framework/rendering/shader_utils.rs`
- **Purpose**: Loads compiled SPIR-V shader modules from files.
- **Key Methods**: `load_shader(device: &ash::Device, filename: &str) -> vk::ShaderModule`.
- **Notes**: Used by `BufferManager` (shapes) and `Renderer` (text).

### `src/gui_framework/rendering/swapchain.rs`
- **Purpose**: Manages Vulkan swapchain creation/recreation, associated image views, framebuffers, and render pass. Cleans up these resources.
- **Key Methods**: `create_swapchain(platform: &mut VulkanContext, extent: vk::Extent2D) -> vk::SurfaceFormatKHR`, `create_framebuffers(platform: &mut VulkanContext, surface_format: vk::SurfaceFormatKHR) -> ()`, `cleanup_swapchain_resources(platform: &mut VulkanContext) -> ()`.
- **Notes**: Called by `Renderer::new` and `ResizeHandler::resize`. Uses physical device capabilities and stores chosen physical extent in `platform.current_swap_extent`.

### `src/gui_framework/rendering/glyph_atlas.rs`
- **Purpose**: Manages the Vulkan texture atlas for caching rasterized glyphs, including packing, rasterization (via `swash`), and upload to GPU.
- **Key Structs**:
    - `GlyphInfo { pixel_x: u32, pixel_y: u32, pixel_width: u32, pixel_height: u32, uv_min: [f32; 2], uv_max: [f32; 2] }` (Derives `Clone`, `Copy`, `Reflect`).
    - `GlyphAtlas { image: vk::Image, allocation: Option<Allocation>, image_view: vk::ImageView, sampler: vk::Sampler, extent: vk::Extent2D, format: vk::Format, current_y: u32, current_line_x: u32, current_line_max_h: u32, padding: u32, glyph_cache: HashMap<CacheKey, GlyphInfo>, scale_context: ScaleContext }`.
- **Key Methods**: `new(vk_context: &mut VulkanContext, initial_extent: vk::Extent2D) -> Result<Self, String>`, `add_glyph(&mut self, vk_context: &VulkanContext, cache_key: CacheKey, swash_image: &swash::scale::image::Image) -> Result<&GlyphInfo, String>`, `upload_glyph_bitmap(vk_context: &VulkanContext, x: u32, y: u32, width: u32, height: u32, bitmap_data: &[u8]) -> Result<(), String>`, `cleanup(vk_context: &VulkanContext) -> ()`.
- **Notes**: Creates/manages Vulkan `Image`, `ImageView`, `Sampler` via `vk-mem`. `add_glyph` implements packing/upload logic. Cleaned up by `cleanup_trigger_system`. Managed via `GlyphAtlasResource`.

### `src/gui_framework/rendering/font_server.rs`
- **Purpose**: Manages font loading and access using `cosmic-text` and `fontdb`.
- **Key Structs**: `FontServer { font_system: FontSystem, font_database: fontdb::Database }`.
- **Key Methods**: `new() -> Self`.
- **Notes**: Loads system fonts via `fontdb` on initialization. Provides `cosmic_text::FontSystem` for shaping/layout. Managed via `FontServerResource`.

### `build.rs`
- **Purpose**: Compiles GLSL shaders (shapes and text) to SPIR-V, copies assets (`user/hotkeys.toml`).
- **Key Methods**: `main() -> ()`, `compile_and_copy_shaders() -> ()`, `copy_user_files() -> ()`.
- **Notes**: Requires `glslc`. Implicitly handles new text shaders.

## Shaders
- **Location**: `shaders/` (Source GLSL), compiled `.spv` copied to `target/<profile>/shaders/` by `build.rs`.
- **Shape Files**: `background.vert`, `background.frag`, `triangle.vert`, `triangle.frag`, `square.vert`, `square.frag`.
- **Shape Roles**: Support orthographic projection (Set 0, Binding 0), object transform matrix (Set 0, Binding 1). Use vertex position (location 0). Loaded by `BufferManager`. `background.vert` ignores object transform.
- **Text Files**: `text.vert`, `text.frag`.
- **Text Roles**: `text.vert` uses projection UBO (Set 0, Binding 0). `text.frag` uses glyph atlas sampler (Set 1, Binding 0). Input attributes: position (loc 0), UV (loc 1). Loaded by `Renderer` for text pipeline creation.

## Dependencies
- `ash = "0.38"`, `vk-mem = "0.4"`, `ash-window = "0.13"`, `raw-window-handle = "0.6"`, `toml = "0.8"`, `thiserror = "2.0"`, `bevy_app = "0.15"`, `bevy_core = "0.15"`, `bevy_ecs = "0.15"`, `bevy_log = "0.15"`, `bevy_utils = "0.15"`, `bevy_window = "0.15"`, `bevy_winit = "0.15"`, `bevy_reflect = "0.15"`, `bevy_input = "0.15"`, `bevy_time = "0.15"`, `bevy_diagnostic = "0.15"`, `bevy_a11y = "0.15"`, `bevy_math = "0.15"`, `bevy_transform = "0.15"`, `bevy_color = "0.15"`, `winit = "0.30.9"`, `cosmic-text = "0.14"`, `fontdb = "0.23"`, `swash = "0.2"`, `rectangle-pack = "0.4"`.
- **Build Dependencies**: `walkdir = "2"`.