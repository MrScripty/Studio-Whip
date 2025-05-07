# Modules Documentation for `rusty_whip`

## Project Overview: `rusty_whip`

### Purpose
`rusty_whip` is an advanced 2D & 3D content generation application with GPU-accelerated AI diffusion/inference, multimedia creation, and story-driven workflows, targeting P2P networking. **It uses the Bevy engine (v0.15) for its core application structure, input handling, ECS, and hierarchy, while strictly avoiding Bevy's rendering stack.**
### Current State (Post BufferManager Despawn Handling Refactor)
- **Bevy Integration**: Application runs using `bevy_app::App`, `bevy_winit`, `bevy_input`, `bevy_transform`, `bevy_log`, `bevy_reflect`, `bevy_color`, `bevy_hierarchy`, and other core non-rendering Bevy plugins.
- **Plugin Architecture**: Core framework logic (rendering, interaction, text foundation, default behaviors) refactored into modular Bevy plugins (`GuiFrameworkCorePlugin`, `GuiFrameworkInteractionPlugin`, etc.) using `SystemSet`s for execution ordering. Application-specific logic remains in `main.rs`.
- **ECS Core**: Application state and logic managed via Bevy ECS components (`ShapeData`, `Visibility`, `Interaction`, `Transform`, `BackgroundQuad`, `Text`, `TextLayoutOutput`, `EditableText`, `Focus`, `CursorState`, `CursorVisual`, `TextBufferCache`, `TextSelection`). `TextRenderData` is a component definition used internally by `TextRenderer`'s cache. User input processed by plugin systems, triggering Bevy events (`EntityClicked`, `EntityDragged`, `HotkeyActionTriggered`, `YrsTextChanged`, `TextFocusChanged`). Dragging correctly updates `Transform` (Y-axis inversion fixed).
- **Reflection**: Core framework components, events, and resources implement `Reflect` where feasible and are registered by the plugins. `TextLayoutOutput`, `PositionedGlyph`, `TextRenderData`, `TextBufferCache`, `TextSelection`, and **`BufferManagerResource`** currently do not support reflection due to containing external or Vulkan types, or by design.
- **Math Migration**: Uses `bevy_math` types (`Vec2`, `Mat4`).
- **Rendering Bridge**: Custom Vulkan context (`VulkanContext`) managed via `VulkanContextResource`. Custom Vulkan renderer (`Renderer`) accessed via `RendererResource`. **`BufferManager` is now also a Bevy resource (`BufferManagerResource`).** All resources defined in `lib.rs`.
- **Rendering Status**: Rendering triggered by `rendering_system`. `Renderer::render` uses `BufferManagerResource` for shape/cursor prep and `TextRenderer` for text prep. `TextRenderer` internally manages per-entity `TextRenderData`. `BufferManager` (via `BufferManagerResource`) creates/updates Vulkan resources for shapes/cursors and **now handles cleanup for despawned entities via `buffer_manager_despawn_cleanup_system`**. `TextRenderingResources` holds shared text pipeline/atlas descriptor. `GlyphAtlas` manages glyph texture. Command buffers are per-image, reset individually. `Renderer::render` locking strategy refined. Depth testing enabled. Visual output functional.
- **Text Handling**:
    - **Foundation**: `Text` component, `FontServer`, `GlyphAtlas`, `SwashCache` resource, and `text_layout_system`.
    - **Rendering**: `TextRenderer::prepare_text_draws` handles per-entity text resource creation/updates.
    - **Interaction**: `interaction_system`, `text_drag_selection_system`, `text_editing_system`.
    - **Visual Cursor**: `manage_cursor_visual_system`, `update_cursor_transform_system`.
- **Yrs Integration**: Basic setup with `YrsDocResource`.
- **Shutdown**: Robust shutdown via `cleanup_trigger_system`, cleaning ECS components, `RendererResource` (and its internals including command pool), **`BufferManagerResource` (and its internal caches/pipelines)**, and shared Vulkan resources.
- **Features Active**: All previously listed features, plus **robust Vulkan resource cleanup for despawned shapes and cursors via `BufferManager` and `buffer_manager_despawn_cleanup_system`**.
- All previously listed tasks and internal refactors, including Phase 3 (BufferManager Despawn Handling), **Complete**.

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
                    text_drag.rs
                    text_editing.rs
                    utils.rs
                plugins/
                    bindings.rs
                    core.rs # <-- Updated
                    interaction.rs
                    mod.rs
                    movement.rs
                rendering/
                    buffer_manager.rs # <-- Updated
                    command_buffers.rs
                    font_server.rs
                    glyph_atlas.rs
                    mod.rs
                    pipeline_manager.rs
                    render_engine.rs # <-- Updated
                    resize_handler.rs
                    shader_utils.rs
                    swapchain.rs
                    text_renderer.rs
                mod.rs
            lib.rs # <-- Updated
            main.rs
        shaders/
            shape.frag
            shape.vert
            text.frag
            text.vert
        user/
            hotkeys.toml
        Documentation/
            modules.md # This file
            architecture.md
            # ... other docs ...
        Cargo.toml
        build.rs


## Modules and Their Functions

### `src/lib.rs`
- **Purpose**: Defines shared types (`Vertex`, `TextVertex`, `RenderCommandData`, `PreparedDrawData`, `PreparedTextDrawData`) and shared framework resources (`VulkanContextResource`, `RendererResource`, **`BufferManagerResource`**, `HotkeyResource`, `GlyphAtlasResource`, `FontServerResource`, `SwashCacheResource`, `GlobalProjectionUboResource`, `TextRenderingResources`, `YrsDocResource`).
- **Key Structs**:
    - `Vertex { position: [f32; 2] }` (Derives `Reflect`)
    - `TextVertex { position: [f32; 2], uv: [f32; 2] }` (Derives `Debug`, `Clone`, `Copy`)
    - `RenderCommandData { entity_id: Entity, transform_matrix: Mat4, vertices: Arc<Vec<Vertex>>, color: Color, depth: f32, vertices_changed: bool }`
    - `PreparedDrawData { pipeline: vk::Pipeline, vertex_buffer: vk::Buffer, vertex_count: u32, descriptor_set: vk::DescriptorSet, color: [f32; 4] }`
    - `PreparedTextDrawData { pipeline: vk::Pipeline, vertex_buffer: vk::Buffer, vertex_count: u32, projection_descriptor_set: vk::DescriptorSet, atlas_descriptor_set: vk::DescriptorSet }` (Derives `Debug`, `Clone`)
    - `YrsDocResource { doc: Arc<yrs::Doc>, text_map: Arc<Mutex<HashMap<Entity, TextRef>>> }` (Derives `Resource`)
    - `GlobalProjectionUboResource { buffer: vk::Buffer, allocation: vk_mem::Allocation, descriptor_set: vk::DescriptorSet }` (Derives `Resource`)
    - `TextRenderingResources { pipeline: vk::Pipeline, atlas_descriptor_set: vk::DescriptorSet }` (Derives `Resource`)
    - `PreparedTextDrawsResource(pub Vec<PreparedTextDrawData>)` (Derives `Resource`, `Default`)
    - `VulkanContextResource(pub Arc<Mutex<VulkanContext>>)` (Derives `Resource`, `Clone`)
    - `RendererResource(pub Arc<Mutex<Renderer>>)` (Derives `Resource`, `Clone`)
    - `BufferManagerResource(pub Arc<Mutex<BufferManager>>)` **(New Resource)** (Derives `Resource`, `Clone`)
    - `HotkeyResource(pub HotkeyConfig)` (Derives `Resource`, `Debug`, `Clone`, `Default`, `Reflect`)
    - `GlyphAtlasResource(pub Arc<Mutex<GlyphAtlas>>)` (Derives `Resource`, `Clone`)
    - `FontServerResource(pub Arc<Mutex<FontServer>>)` (Derives `Resource`, `Clone`)
    - `SwashCacheResource(pub Mutex<SwashCache>)` (Derives `Resource`)
- **Notes**: `BufferManagerResource` added. Uses `ash::vk`, `bevy_ecs::Entity`, `bevy_math::Mat4`, `std::sync::Arc`, `bevy_reflect::Reflect`, `cosmic_text::SwashCache`, `yrs`, `bevy_color::Color`.

### `src/main.rs`
- **Purpose**: Entry point; sets up `bevy_app::App`, core Bevy plugins (including `HierarchyPlugin`), framework plugins (`GuiFrameworkCorePlugin`, etc.), inserts initial `VulkanContextResource`, `YrsDocResource`, and defines/schedules **application-specific** systems (`setup_scene_ecs`, `background_resize_system`).
- **Key Structs (Defined In `main.rs`)**:
    - `BackgroundQuad` (Derives `Component`, marker struct)
- **Key Methods (Bevy Systems)**:
    - `main() -> ()`
    - `setup_scene_ecs(mut commands: Commands, primary_window_q: Query<&Window, With<PrimaryWindow>>, yrs_res: ResMut<YrsDocResource>) -> ()`: Spawns initial application-specific ECS entities (background, shapes with **color**, sample text with `EditableText`), populates `YrsDocResource`. Runs after `CoreSet::CreateTextResources` and `InteractionSet::LoadHotkeys`.
    - `background_resize_system(mut resize_reader: EventReader<bevy_window::WindowResized>, mut background_query: Query<&mut ShapeData, With<BackgroundQuad>>) -> ()`: Reads `WindowResized`, updates background `ShapeData.vertices`. (App-specific update logic).
- **Notes**: Relies on plugins for framework setup, rendering, input, text handling, and cleanup. Defines application structure and specific scene/behavior. Imports `Text`, `TextAlignment`, `Color`, `EditableText`. Adds `HierarchyPlugin`.

### `src/gui_framework/mod.rs`
- **Purpose**: Declares and re-exports framework modules (`components`, `context`, `events`, `interaction`, `plugins`, `rendering`).
- **Notes**: Provides access to framework internals if needed. Exports `VulkanContext`, `setup_vulkan`, `cleanup_vulkan`, `HotkeyConfig`, `HotkeyError`.

### `src/gui_framework/components/mod.rs`
- **Purpose**: Declares and re-exports Bevy ECS component modules.
- **Modules**: `shape_data.rs`, `visibility.rs`, `interaction.rs`, `text_data.rs`, `text_layout.rs`.
- **Exports**: `ShapeData`, `Visibility`, `Interaction`, `Text`, `FontId`, `TextAlignment`, `EditableText`, `Focus`, `CursorState`, `CursorVisual`, `TextSelection`, `TextLayoutOutput`, `PositionedGlyph`, `TextRenderData`, `TextBufferCache`.

### `src/gui_framework/components/shape_data.rs`
- **Purpose**: Defines the visual shape data for an entity (used for shapes and cursor).
- **Key Structs**: `ShapeData { vertices: Arc<Vec<Vertex>>, color: Color }` (Derives `Component`, `Debug`, `Clone`, `Reflect`).
- **Key Methods**: `impl Default for ShapeData { default() -> Self }` (sets magenta color).
- **Notes**: Used by `rendering_system` (core plugin) and `background_resize_system` (main.rs). Registered by `GuiFrameworkCorePlugin`. Depth handled by `Transform`. Default color is Magenta.

### `src/gui_framework/components/visibility.rs`
- **Purpose**: Defines a custom visibility component to avoid `bevy_render`.
- **Key Structs**: `Visibility(pub bool)` (Derives `Component`, `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Reflect`).
- **Key Methods**: `is_visible(&self) -> bool`, `impl Default for Visibility { default() -> Self }` (visible by default).
- **Notes**: Used by `rendering_system`, `text_layout_system`, `manage_cursor_visual_system` (core plugin). Registered by `GuiFrameworkCorePlugin`.

### `src/gui_framework/components/interaction.rs`
- **Purpose**: Defines interaction properties for an entity.
- **Key Structs**: `Interaction { clickable: bool, draggable: bool }` (Derives `Component`, `Debug`, `Clone`, `Copy`, `Reflect`).
- **Key Methods**: `impl Default for Interaction { default() -> Self }` (not interactive by default).
- **Notes**: Used by `interaction_system` (interaction plugin). Registered by `GuiFrameworkInteractionPlugin`.

### `src/gui_framework/components/text_data.rs`
- **Purpose**: Defines the input data and state for a text entity, including editing state.
- **Key Structs**:
    - `FontId(pub usize)` (Derives `Debug`, `Clone`, `PartialEq`, `Eq`, `Hash`, `Reflect`) - Placeholder.
    - `TextAlignment` (enum: `Left`, `Center`, `Right`) (Derives `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Reflect`, `Default`).
    - `Text { size: f32, color: Color, alignment: TextAlignment, bounds: Option<Vec2> }` (Derives `Component`, `Debug`, `Clone`, `Reflect`, `Default`). **Note: `content` removed, fetched from `YrsDocResource`.**
    - `EditableText` (Derives `Component`, `Debug`, `Clone`, `Copy`, `Default`, `Reflect`) - Marker for editable text.
    - `Focus` (Derives `Component`, `Debug`, `Clone`, `Copy`, `Default`, `Reflect`) - Marker for focused text.
    - `CursorState { position: usize, line: usize }` (Derives `Component`, `Debug`, `Clone`, `Copy`, `Default`, `Reflect`) - Stores logical cursor byte offset and line index.
    - `CursorVisual` (Derives `Component`, `Debug`, `Clone`, `Copy`, `Default`, `Reflect`) - Marker for the visual cursor entity.
    - `TextSelection { start: usize, end: usize }` (Derives `Component`, `Debug`, `Clone`, `Copy`, `Default`, `Reflect`) - Stores selection byte offsets.
- **Notes**: `Text` used by `text_layout_system` (core plugin). `EditableText`, `Focus`, `CursorState`, `TextSelection` used by systems in `GuiFrameworkInteractionPlugin` and `GuiFrameworkCorePlugin`. Registered by respective plugins. Uses `bevy_color::Color`.

### `src/gui_framework/components/text_layout.rs`
- **Purpose**: Defines intermediate and output data structures for text layout results, per-entity text rendering resources, and cached layout buffers.
- **Key Structs**:
    - `PositionedGlyph { glyph_info: GlyphInfo, layout_glyph: LayoutGlyph, vertices: [Vec2; 4] }` (Derives `Debug`, `Clone`). Contains non-reflectable `LayoutGlyph`.
    - `TextLayoutOutput { glyphs: Vec<PositionedGlyph> }` (Derives `Component`, `Debug`, `Clone`, `Default`). Contains non-reflectable `PositionedGlyph`.
    - `TextRenderData { vertex_count: u32, vertex_buffer: vk::Buffer, vertex_alloc: vk_mem::Allocation, transform_ubo: vk::Buffer, transform_alloc: vk_mem::Allocation, descriptor_set_0: vk::DescriptorSet }` (Derives `Component`). Holds per-entity Vulkan resources for text. Not Reflectable. **Used internally by `TextRenderer`'s cache.**
    - `TextBufferCache { buffer: Option<cosmic_text::Buffer> }` (Derives `Component`). Caches the `cosmic_text::Buffer` used for layout. Not Reflectable.
- **Notes**: `TextLayoutOutput` written by `text_layout_system`, read by `TextRenderer::prepare_text_draws` (via `TextLayoutInfo`). `TextRenderData` created/updated/cleaned by `TextRenderer`, read by `command_buffers`. `TextBufferCache` written by `text_layout_system`, read by `interaction_system`, `update_cursor_transform_system`, `text_editing_system`, `text_drag_selection_system`, cleaned by `cleanup_trigger_system`.

### `src/gui_framework/context/mod.rs`
- **Purpose**: Re-exports Vulkan context modules.
- **Modules**: `vulkan_context.rs`, `vulkan_setup.rs`.

### `src/gui_framework/context/vulkan_context.rs`
- **Purpose**: Holds core Vulkan resources and related state.
- **Key Structs**: `VulkanContext { entry: Option<Entry>, instance: Option<Instance>, surface_loader: Option<surface::Instance>, surface: Option<vk::SurfaceKHR>, device: Option<ash::Device>, physical_device: Option<vk::PhysicalDevice>, queue: Option<vk::Queue>, queue_family_index: Option<u32>, allocator: Option<Arc<Allocator>>, swapchain_loader: Option<swapchain::Device>, swapchain: Option<vk::SwapchainKHR>, current_swap_extent: vk::Extent2D, images: Vec<vk::Image>, image_views: Vec<vk::ImageView>, depth_image: Option<vk::Image>, depth_image_allocation: Option<vk_mem::Allocation>, depth_image_view: Option<vk::ImageView>, depth_format: Option<vk::Format>, render_pass: Option<vk::RenderPass>, framebuffers: Vec<vk::Framebuffer>, shape_pipeline_layout: Option<vk::PipelineLayout>, text_pipeline_layout: Option<vk::PipelineLayout>, command_pool: Option<vk::CommandPool>, command_buffers: Vec<vk::CommandBuffer>, image_available_semaphore: Option<vk::Semaphore>, render_finished_semaphore: Option<vk::Semaphore>, fence: Option<vk::Fence>, current_image: usize, debug_utils_loader: Option<debug_utils::Instance>, debug_messenger: Option<vk::DebugUtilsMessengerEXT> }`.
- **Key Methods**: `new() -> Self`.
- **Notes**: Managed via `VulkanContextResource`. Holds `Arc<Allocator>`. `depth_image_allocation` field added. `command_pool` created by `Renderer::new`. `command_buffers` allocated/freed by `swapchain.rs` functions.

### `src/gui_framework/context/vulkan_setup.rs`
- **Purpose**: Sets up and tears down core Vulkan resources managed by `VulkanContext` (instance, device, surface, allocator, debug messenger).
- **Key Methods**: `setup_vulkan(app: &mut VulkanContext, window: &winit::window::Window) -> ()`, `cleanup_vulkan(app: &mut VulkanContext) -> ()`.
- **Notes**: Called by systems in `GuiFrameworkCorePlugin`. Still uses `winit::window::Window`.

### `src/gui_framework/events/mod.rs`
- **Purpose**: Declares and re-exports Bevy Event modules.
- **Modules**: `interaction_events.rs`.
- **Exports**: `EntityClicked`, `EntityDragged`, `HotkeyActionTriggered`, `YrsTextChanged`, `TextFocusChanged`.

### `src/gui_framework/events/interaction_events.rs`
- **Purpose**: Defines Bevy events related to user interaction and text state changes.
- **Key Structs**:
    - `EntityClicked { entity: Entity }` (Derives `Event`, `Reflect`)
    - `EntityDragged { entity: Entity, delta: Vec2 }` (Derives `Event`, `Reflect`)
    - `HotkeyActionTriggered { action: String }` (Derives `Event`, `Reflect`)
    - `YrsTextChanged { entity: Entity }` (Derives `Event`, `Reflect`)
    - `TextFocusChanged { entity: Option<Entity> }` (Derives `Event`, `Reflect`)
- **Notes**: Written by systems in `GuiFrameworkInteractionPlugin`, read by other plugin/app systems. Registered by `GuiFrameworkInteractionPlugin`.

### `src/gui_framework/interaction/mod.rs`
- **Purpose**: Declares interaction-related submodules.
- **Modules**: `hotkeys.rs`, `utils.rs`, `text_drag.rs`, `text_editing.rs`.

### `src/gui_framework/interaction/hotkeys.rs`
- **Purpose**: Defines hotkey configuration loading from TOML and key event formatting.
- **Key Structs**: `HotkeyConfig { mappings: HashMap<String, String> }` (Derives `Debug`, `Clone`, `Default`, `Reflect`), `HotkeyError` (enum).
- **Key Methods**:
    - `HotkeyConfig::load_config(path: &Path) -> Result<Self, HotkeyError>`
    - `HotkeyConfig::get_action(&self, key_combo: &str) -> Option<&String>`
    - `format_key_event(modifiers: ModifiersState, key: PhysicalKey) -> Option<String>`.
- **Notes**: Used by `load_hotkeys_system` (interaction plugin). `HotkeyConfig` stored within `HotkeyResource`.

### `src/gui_framework/interaction/utils.rs`
- **Purpose**: Provides utility functions related to user interaction, particularly text.
- **Key Methods**: `get_cursor_at_position(buffer: &cosmic_text::Buffer, local_pos_ydown: Vec2) -> Option<cosmic_text::Cursor>`.
- **Notes**: Used by `interaction_system` and `text_drag_selection_system`.

### `src/gui_framework/interaction/text_drag.rs`
- **Purpose**: Handles text selection via mouse dragging.
- **Key Methods (Bevy Systems)**:
    - `text_drag_selection_system(mut cursor_moved_events: EventReader<CursorMoved>, windows: Query<&Window, With<PrimaryWindow>>, mouse_context: Res<MouseContext>, mut text_queries: ParamSet<(...)>) -> ()`.
- **Notes**: Part of `GuiFrameworkInteractionPlugin`. Reads `TextBufferCache`.

### `src/gui_framework/interaction/text_editing.rs`
- **Purpose**: Handles keyboard input for text editing operations.
- **Key Methods (Bevy Systems)**:
    - `text_editing_system(mut commands: Commands, keyboard_input: Res<ButtonInput<KeyCode>>, mut char_events: EventReader<KeyboardInput>, mut focused_query: Query<(Entity, &mut CursorState, &mut TextSelection, &TextBufferCache), (With<Focus>, With<EditableText>)>, yrs_doc_res: Res<YrsDocResource>, mut yrs_text_changed_writer: EventWriter<YrsTextChanged>, mut font_system_res: ResMut<FontServerResource>) -> ()`.
- **Notes**: Part of `GuiFrameworkInteractionPlugin`. Reads `TextBufferCache`.

### `src/gui_framework/plugins/mod.rs`
- **Purpose**: Declares and re-exports framework plugin modules.
- **Modules**: `core.rs`, `interaction.rs`, `movement.rs`, `bindings.rs`.

### `src/gui_framework/plugins/core.rs`
- **Purpose**: Plugin for core Vulkan setup, rendering, text foundation, cursor management, resize, and cleanup. **Manages `BufferManagerResource` lifecycle and despawn cleanup.**
- **Key Structs**: `GuiFrameworkCorePlugin`, `CoreSet` (enum SystemSet: ..., `PreRenderCleanup`, ...).
- **Key Methods (Bevy Systems)**:
    - `setup_vulkan_system(...) -> ()`
    - `create_renderer_system(mut commands: Commands, ...)`: Now passes `Commands` to `Renderer::new` for `BufferManagerResource` insertion.
    - `create_glyph_atlas_system(...) -> ()`
    - `create_font_server_system(...) -> ()`
    - `create_swash_cache_system(...) -> ()`
    - `create_global_ubo_system(...) -> ()`
    - `create_text_rendering_resources_system(...) -> ()`
    - `handle_resize_system(...) -> ()`
    - `text_layout_system(...) -> ()`
    - `manage_cursor_visual_system(...) -> ()`
    - `update_cursor_transform_system(...) -> ()`
    - `apply_deferred(...) -> ()`
    - `buffer_manager_despawn_cleanup_system(mut removed_shapes: RemovedComponents<ShapeData>, mut removed_cursors: RemovedComponents<CursorVisual>, buffer_manager_res_opt: Option<Res<BufferManagerResource>>, vk_context_res_opt: Option<Res<VulkanContextResource>>) -> ()`: **New system** in `CoreSet::PreRenderCleanup` to handle `ShapeData`/`CursorVisual` despawns by calling `BufferManager::remove_entity_resources`.
    - `rendering_system(..., buffer_manager_res: Res<BufferManagerResource>, ...)`: Passes `BufferManagerResource` to `Renderer::render`.
    - `cleanup_trigger_system(world: &mut World) -> ()`: Now also takes and cleans up `BufferManagerResource` by calling its `cleanup` method.
- **Notes**: Adds systems to `Startup`, `Update`, `Last` schedules. `CoreSet` updated with `PreRenderCleanup`. `BufferManagerResource` is not registered for reflection. New despawn cleanup system added.

### `src/gui_framework/plugins/interaction.rs`
- **Purpose**: Plugin for input processing (mouse, keyboard), hotkey loading/dispatch, window close requests, ECS-based text focus, cursor state, selection state management, **text drag selection, and keyboard-based text editing**.
- **Key Structs**:
    - `GuiFrameworkInteractionPlugin`, `InteractionSet` (enum SystemSet: `LoadHotkeys`, `InputHandling`, `WindowClose`).
    - `MouseContextType` (enum: `Idle`, `DraggingShape`, `TextInteraction`) - Local enum.
    - `MouseContext { context: MouseContextType }` (Derives `Resource`, `Default`) - Local resource.
- **Key Methods (Bevy Systems)**:
    - `load_hotkeys_system(mut commands: Commands) -> ()`.
    - `interaction_system(...) -> ()`.
    - `hotkey_system(...) -> ()`.
    - `text_drag_selection_system(...) -> ()`.
    - `text_editing_system(...) -> ()`.
    - `handle_close_request(...) -> ()`.
- **Notes**: Adds systems to `Startup`, `Update` schedules. Configures `InteractionSet` ordering. Registers interaction components, events, and resources.

### `src/gui_framework/plugins/movement.rs`
- **Purpose**: Optional plugin providing default entity movement based on `EntityDragged` events.
- **Key Structs**: `GuiFrameworkDefaultMovementPlugin`, `MovementSet` (enum SystemSet: `ApplyMovement`).
- **Key Methods (Bevy Systems)**: `movement_system(mut drag_evr: EventReader<EntityDragged>, mut query: Query<&mut Transform>) -> ()`.
- **Notes**: Adds system to `Update` schedule, ordered `.after(InteractionSet::InputHandling)`.

### `src/gui_framework/plugins/bindings.rs`
- **Purpose**: Optional plugin providing default handling for specific `HotkeyActionTriggered` events (e.g., "CloseRequested").
- **Key Structs**: `GuiFrameworkDefaultBindingsPlugin`, `BindingsSet` (enum SystemSet: `HandleActions`).
- **Key Methods (Bevy Systems)**: `app_control_system(mut hotkey_evr: EventReader<HotkeyActionTriggered>, mut app_exit_evw: EventWriter<AppExit>) -> ()`.
- **Notes**: Adds system to `Update` schedule, ordered `.after(InteractionSet::InputHandling)`.

### `src/gui_framework/rendering/mod.rs`
- **Purpose**: Re-exports rendering sub-modules and key types (`Renderer`, `GlyphAtlas`, `FontServer`, `TextRenderer`, **`BufferManager`**).
- **Modules**: `buffer_manager.rs`, `command_buffers.rs`, `pipeline_manager.rs`, `render_engine.rs`, `resize_handler.rs`, `shader_utils.rs`, `swapchain.rs`, `glyph_atlas.rs`, `font_server.rs`, `text_renderer.rs`.
- **Exports**: `Renderer`, `GlyphAtlas`, `GlyphAtlasResource`, `GlyphInfo`, `FontServer`, `FontServerResource`, `TextRenderer`, **`BufferManager`**.
- **Notes**: `BufferManager` export is relevant as its type is used in `BufferManagerResource`.

### `src/gui_framework/rendering/command_buffers.rs`
- **Purpose**: Records Vulkan draw commands into a provided command buffer. **Does not manage command buffer lifecycle or command pool state.** Sets dynamic viewport/scissor. Clears color and depth buffers. **Uses push constants for shape/cursor color.**
- **Key Methods**: `record_command_buffers(platform: &VulkanContext, prepared_shape_draws: &[PreparedDrawData], prepared_text_draws: &[PreparedTextDrawData], extent: vk::Extent2D) -> ()`.
- **Notes**: Called by `Renderer::render` with a pre-reset command buffer.

### `src/gui_framework/rendering/render_engine.rs`
- **Purpose**: Orchestrates custom Vulkan rendering per frame. Manages `TextRenderer`. **Accesses `BufferManagerResource` for shape/cursor rendering.** Handles sync objects, resize, and command buffer reset.
- **Key Structs**: `Renderer { /* buffer_manager field removed */ text_renderer: TextRenderer, descriptor_pool: vk::DescriptorPool, descriptor_set_layout: vk::DescriptorSetLayout, text_descriptor_set_layout: vk::DescriptorSetLayout }`.
- **Key Methods**:
    - `new(commands: &mut Commands, platform: &mut VulkanContext, extent: vk::Extent2D) -> Self`: Initializes command pool in `VulkanContext`. Instantiates `BufferManager` and inserts it as `BufferManagerResource` via `commands`. Calls `swapchain.rs` functions for swapchain/framebuffer/command buffer creation.
    - `resize_renderer(vk_context_res: &VulkanContextResource, width: u32, height: u32) -> ()`.
    - `render(vk_context_res: &VulkanContextResource, buffer_manager_res: &BufferManagerResource, shape_commands: &[RenderCommandData], text_layout_infos: &[TextLayoutInfo], global_ubo_res: &GlobalProjectionUboResource, text_global_res: &TextRenderingResources) -> ()`: Takes `BufferManagerResource` to prepare shape draws. Waits for fence, acquires image, locks context, prepares draws, resets active command buffer, calls `record_command_buffers`, submits, unlocks, presents. Refined locking strategy.
    - `cleanup(platform: &mut VulkanContext) -> ()`: Cleans internal `TextRenderer`, its own descriptor layouts and pool, sync objects, and the command pool from `VulkanContext`. **Does not clean `BufferManager` directly (handled by `BufferManagerResource` cleanup).**
- **Notes**: Managed via `RendererResource`. `BufferManager` is no longer a direct field.

### `src/gui_framework/rendering/text_renderer.rs`
- **Purpose**: Manages per-entity Vulkan resources for text rendering and prepares text draw data.
- **Key Structs**: `TextRenderer { text_render_resources: HashMap<Entity, TextRenderData>, descriptor_pool: vk::DescriptorPool, per_entity_layout_set0: vk::DescriptorSetLayout }`.
- **Key Methods**:
    - `new(descriptor_pool: vk::DescriptorPool, per_entity_layout_set0: vk::DescriptorSetLayout) -> Self`.
    - `prepare_text_draws(&mut self, device: &ash::Device, allocator: &Arc<vk_mem::Allocator>, text_layout_infos: &[TextLayoutInfo], global_ubo_res: &GlobalProjectionUboResource, text_global_res: &TextRenderingResources) -> Vec<PreparedTextDrawData>`.
    - `cleanup(&mut self, device: &ash::Device, allocator: &Arc<vk_mem::Allocator>) -> ()`.
- **Notes**: Instantiated and managed by `Renderer`. Consumes `TextLayoutInfo`.

### `src/gui_framework/rendering/pipeline_manager.rs`
- **Purpose**: **Initialization helper.** Creates Vulkan `PipelineLayout`s (shape, text), `DescriptorSetLayout`s (per-entity, atlas), and a shared `DescriptorPool`. **Shape layout includes push constant range for color.**
- **Key Structs**: `PipelineManager { per_entity_layout: vk::DescriptorSetLayout, atlas_layout: vk::DescriptorSetLayout, shape_pipeline_layout: vk::PipelineLayout, text_pipeline_layout: vk::PipelineLayout, descriptor_pool: vk::DescriptorPool }`.
- **Key Methods**: `new(platform: &mut VulkanContext) -> Self`.
- **Notes**: Provides layouts/pool to `Renderer` and `VulkanContext` during initialization.

### `src/gui_framework/rendering/buffer_manager.rs`
- **Purpose**: Manages per-entity Vulkan resources (vertex buffers, transform UBOs, descriptor sets) **for shapes and cursor visuals**. **Caches the single shape pipeline.** Updates vertex buffers based on `RenderCommandData.vertices_changed`. Updates transform UBOs and descriptor sets every frame. **Includes logic to remove resources for despawned entities.**
- **Key Structs**: `EntityRenderResources { vertex_buffer: vk::Buffer, vertex_allocation: vk_mem::Allocation, vertex_count: u32, offset_uniform: vk::Buffer, offset_allocation: vk_mem::Allocation, descriptor_set: vk::DescriptorSet }`, `PipelineCacheKey { id: u32 }`, `BufferManager { entity_resources: HashMap<Entity, EntityRenderResources>, pipeline_cache: HashMap<PipelineCacheKey, vk::Pipeline>, per_entity_layout: vk::DescriptorSetLayout, descriptor_pool: vk::DescriptorPool }`.
- **Key Methods**:
    - `new(_platform: &mut VulkanContext, per_entity_layout: vk::DescriptorSetLayout, descriptor_pool: vk::DescriptorPool) -> Self`.
    - `prepare_frame_resources(platform: &mut VulkanContext, render_commands: &[RenderCommandData], global_ubo_res: &GlobalProjectionUboResource) -> Vec<PreparedDrawData>`.
    - `remove_entity_resources(&mut self, entity: Entity, device: &ash::Device, allocator: &Arc<vk_mem::Allocator>) -> ()`: **New method** to clean up Vulkan resources (buffers, descriptor set) for a single despawned entity.
    - `cleanup(platform: &mut VulkanContext) -> ()`: Cleans up all internally cached resources (pipelines, remaining entity resources by freeing their descriptor sets). Does not destroy the descriptor pool itself (owned by `Renderer`).
- **Notes**: Managed via `BufferManagerResource`. `remove_entity_resources` is called by `buffer_manager_despawn_cleanup_system`.

### `src/gui_framework/rendering/resize_handler.rs`
- **Purpose**: Handles window resizing logic: waits for device idle, cleans up old swapchain resources (including depth buffer, **command buffers via `swapchain.rs`**), creates new swapchain, creates framebuffers (including depth buffer, **and allocates new command buffers via `swapchain.rs`**).
- **Key Structs**: `ResizeHandler` (stateless).
- **Key Methods**: `resize(vulkan_context: &mut VulkanContext, logical_extent: vk::Extent2D) -> ()`.
- **Notes**: Called by `Renderer::resize_renderer`.

### `src/gui_framework/rendering/shader_utils.rs`
- **Purpose**: Loads compiled SPIR-V shader modules from files.
- **Key Methods**: `load_shader(device: &ash::Device, filename: &str) -> vk::ShaderModule`.
- **Notes**: Used by `BufferManager` (shapes) and `create_text_rendering_resources_system` (text).

### `src/gui_framework/rendering/swapchain.rs`
- **Purpose**: Manages Vulkan swapchain creation/recreation, associated image views, **depth buffer resources**, framebuffers (with depth attachment), render pass, **and allocates/frees command buffers per swapchain image**. Cleans up these resources.
- **Key Methods**:
    - `create_swapchain(platform: &mut VulkanContext, extent: vk::Extent2D) -> vk::SurfaceFormatKHR`.
    - `create_framebuffers(platform: &mut VulkanContext, surface_format: vk::SurfaceFormatKHR) -> ()`: **Allocates command buffers into `platform.command_buffers`.**
    - `cleanup_swapchain_resources(platform: &mut VulkanContext) -> ()`: **Frees command buffers from `platform.command_buffers`.**
    - `find_supported_format(...) -> Option<vk::Format>` (helper).
- **Notes**: Called by `Renderer::new` and `ResizeHandler::resize`. Directly modifies `VulkanContext` for swapchain and command buffer resources.

### `src/gui_framework/rendering/glyph_atlas.rs`
- **Purpose**: Manages the Vulkan texture atlas for caching rasterized glyphs using `rectangle-pack` for packing and `swash` for input glyph data, handling uploads to the GPU.
- **Key Structs**:
    - `GlyphInfo { pixel_x: u32, pixel_y: u32, pixel_width: u32, pixel_height: u32, uv_min: [f32; 2], uv_max: [f32; 2] }`.
    - `GlyphAtlas { image: vk::Image, allocation: Option<Allocation>, image_view: vk::ImageView, sampler: vk::Sampler, extent: vk::Extent2D, format: vk::Format, target_bins: BTreeMap<u32, TargetBin>, _padding: u32, glyph_cache: HashMap<CacheKey, GlyphInfo>, _scale_context: ScaleContext }`.
- **Key Methods**:
    - `new(vk_context: &mut VulkanContext, initial_extent: vk::Extent2D) -> Result<Self, String>`.
    - `add_glyph(&mut self, vk_context: &VulkanContext, cache_key: CacheKey, swash_image: &swash::scale::image::Image) -> Result<&GlyphInfo, String>`.
    - `upload_glyph_bitmap(vk_context: &VulkanContext, x: u32, y: u32, width: u32, height: u32, bitmap_data: &[u8]) -> Result<(), String>`.
    - `cleanup(vk_context: &VulkanContext) -> ()`.
- **Notes**: Creates/manages Vulkan `Image`, `ImageView`, `Sampler`. Managed via `GlyphAtlasResource`.

### `src/gui_framework/rendering/font_server.rs`
- **Purpose**: Manages font loading and access using `cosmic-text` and `fontdb`.
- **Key Structs**: `FontServer { font_system: FontSystem, font_database: fontdb::Database }`.
- **Key Methods**: `new() -> Self`.
- **Notes**: Loads system fonts. Managed via `FontServerResource`.

### `build.rs`
- **Purpose**: Compiles GLSL shaders (shapes and text) to SPIR-V, copies assets (`user/hotkeys.toml`).
- **Key Methods**: `main() -> ()`, `compile_and_copy_shaders() -> ()`, `copy_user_files() -> ()`.
- **Notes**: Requires `glslc`.

## Shaders
- **Location**: `shaders/` (Source GLSL), compiled `.spv` copied to `target/<profile>/shaders/` by `build.rs`.
- **Shape Files**: `shape.vert`, `shape.frag`.
- **Shape Roles**: Single pipeline for all shapes **and cursor visuals**. Support orthographic projection (Set 0, Binding 0), object transform matrix (Set 0, Binding 1). Use vertex position (location 0). **Color passed via push constant in `shape.frag`.** Loaded by `BufferManager`.
- **Text Files**: `text.vert`, `text.frag`.
- **Text Roles**: `text.vert` uses projection UBO (Set 0, Binding 0) and transform UBO (Set 0, Binding 1). `text.frag` uses glyph atlas sampler (Set 1, Binding 0). Input attributes: position (loc 0), UV (loc 1). Loaded by `create_text_rendering_resources_system`.

## Dependencies
- `ash = "0.38"`, `vk-mem = "0.4"`, `ash-window = "0.13"`, `raw-window-handle = "0.6"`, `toml = "0.8"`, `thiserror = "2.0"`, `bevy_app = "0.15"`, `bevy_core = "0.15"`, `bevy_ecs = "0.15"`, `bevy_log = "0.15"`, `bevy_utils = "0.15"`, `bevy_window = "0.15"`, `bevy_winit = "0.15"`, `bevy_reflect = "0.15"`, `bevy_input = "0.15"`, `bevy_time = "0.15"`, `bevy_diagnostic = "0.15"`, `bevy_a11y = "0.15"`, `bevy_math = "0.15"`, `bevy_transform = "0.15"`, `bevy_color = "0.15"`, `bevy_hierarchy = "0.15"`, `winit = "0.30.9"`, `cosmic-text = "0.14"`, `fontdb = "0.23"`, `swash = "0.2"`, `rectangle-pack = "0.4"`, `yrs = "0.23"`.
- **Build Dependencies**: `walkdir = "2"`.