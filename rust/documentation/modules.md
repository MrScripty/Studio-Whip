# Modules Documentation for `rusty_whip`

## Project Overview: `rusty_whip`
### Purpose
`rusty_whip` is an advanced 2D & 3D content generation application with GPU-accelerated AI diffusion/inference, multimedia creation, and story-driven workflows, targeting P2P networking. **It has migrated its core logic to the Bevy engine (v0.15), strictly avoiding Bevy's rendering stack.**
### Current State (Post Task 6.3 Follow-up & Cleanup)
- **Bevy Integration**: Application runs using `bevy_app::App`, `bevy_winit`, `bevy_input`, `bevy_transform`, and other core non-rendering Bevy plugins.
- **ECS Core**: Application state and logic managed via Bevy ECS components (`ShapeData`, `Visibility`, `Interaction`, `Transform`). User input processed by Bevy systems, triggering Bevy events (`EntityClicked`, `EntityDragged`, `HotkeyActionTriggered`).
- **Math Migration**: Uses `bevy_math` types (`Vec2`, `Mat4`).
- **Rendering Bridge**: Custom Vulkan context (`VulkanContext`) managed via `VulkanContextResource`. Custom Vulkan renderer (`Renderer`) accessed via `RendererResource`.
- **Rendering Status**: Rendering is triggered by `rendering_system` which collects `RenderCommandData` from ECS entities. `BufferManager` creates/updates Vulkan resources (buffers, descriptor sets) based on this data, utilizing **pipeline and shader caching**. `command_buffers` uses the prepared data to record draw calls. Synchronization and resize handling are corrected. **Visual output is functional.** **Optimizations needed: resource removal for despawned entities, vertex buffer updates.**
- **Features Active**: Bevy app structure, windowing, logging, input handling (click, drag, hotkeys), ECS component/event usage, `bevy_transform`, core Vulkan setup, hotkey loading, ECS-driven Vulkan resource management with caching, corrected synchronization and resize handling.
- Task 1-5 **Complete** (Legacy). Task 6.1, 6.2 **Complete**. Task 6.3 **Complete**. Task 6.3 Follow-up (Caching, Debugging, Synchronization, Resize) **Complete**. Legacy `event_bus` and `scene` modules **removed**.

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
                rendering/
                    buffer_manager.rs
                    command_buffers.rs
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
            # ... GLSL shader source files (.vert, .frag) ...
        user/
            hotkeys.toml
        Documentation/
            modules.md # This file
            architecture.md
            # ... other docs ...
        utilities/
            # ... utils ...
        Cargo.toml
        build.rs
        # Removed compile_shaders scripts


## Modules and Their Functions

### `src/lib.rs`
- **Purpose**: Defines shared types (`Vertex`, `RenderCommandData`, `PreparedDrawData`).
- **Key Structs**:
    - `Vertex { position: [f32; 2] }`
    - `RenderCommandData { entity_id: Entity, transform_matrix: Mat4, vertices: Arc<Vec<Vertex>>, vertex_shader_path: String, fragment_shader_path: String, depth: f32 }`
    - `PreparedDrawData { pipeline: vk::Pipeline, vertex_buffer: vk::Buffer, vertex_count: u32, descriptor_set: vk::DescriptorSet }`
- **Notes**: Uses `ash::vk`, `bevy_ecs::Entity`, `bevy_math::Mat4`, `std::sync::Arc`.

### `src/main.rs`
- **Purpose**: Entry point; sets up `bevy_app::App`, core Bevy plugins, resources (`VulkanContextResource`, `RendererResource`, `HotkeyResource`), events, and systems for lifecycle, ECS setup, input, state updates, and triggering the custom Vulkan rendering backend.
- **Key Structs (Defined In `main.rs`)**:
    - `VulkanContextResource(Arc<Mutex<VulkanContext>>)`
    - `RendererResource(Arc<Mutex<Renderer>>)`
    - `HotkeyResource(HotkeyConfig)`
- **Key Methods (Bevy Systems)**:
    - `main() -> ()`
    - `setup_vulkan_system(...) -> Result<(), String>`: Initializes Vulkan context.
    - `create_renderer_system(...) -> Result<(), String>`: Creates `RendererResource`, stores `pipeline_layout` in `VulkanContext`.
    - `setup_scene_ecs(...) -> ()`: Loads `HotkeyResource`, spawns initial ECS entities.
    - `interaction_system(...) -> ()`: Processes mouse clicks/drags, writes interaction events.
    - `hotkey_system(...) -> ()`: Processes keyboard input, writes hotkey events.
    - `movement_system(...) -> ()`: Updates `Transform` components based on drag events.
    - `app_control_system(...) -> ()`: Handles application exit via hotkey events.
    - `handle_resize_system(...) -> ()`: Calls `Renderer::resize_renderer`.
    - `rendering_system(...) -> ()`: Queries ECS (`GlobalTransform`, `ShapeData`, `Visibility`), collects `RenderCommandData`, calls `Renderer::render`.
    - `handle_close_request(...) -> ()`: Sends `AppExit` on window close request.
    - `cleanup_system(...) -> ()`: Cleans up `RendererResource` and `VulkanContextResource` on `AppExit`.
- **Notes**: Uses Bevy App structure. Manages Vulkan state via `Resource` bridge pattern. Rendering uses actual `Renderer` whose `BufferManager` now implements caching.

### `src/gui_framework/mod.rs`
- **Purpose**: Declares and re-exports framework modules. Exports types needed for Vulkan backend and hotkey config.
- **Notes**: `interaction` module only contains `hotkeys`.

### `src/gui_framework/components/mod.rs`
- **Purpose**: Declares and re-exports Bevy ECS component modules.
- **Modules**: `shape_data.rs`, `visibility.rs`, `interaction.rs`.

### `src/gui_framework/components/shape_data.rs`
- **Purpose**: Defines the visual shape data for an entity.
- **Key Structs**: `ShapeData { vertices: Arc<Vec<Vertex>>, vertex_shader_path: String, fragment_shader_path: String }`.
- **Notes**: Uses `Arc` for vertices. Used by `rendering_system` to create `RenderCommandData`. Shader paths are relative to `shaders/` dir and expect `.spv` extension (compilation handled by `build.rs`).

### `src/gui_framework/components/visibility.rs`
- **Purpose**: Defines a custom visibility component to avoid `bevy_render`.
- **Key Structs**: `Visibility(pub bool)`.
- **Notes**: Used by `rendering_system` to filter visible entities.

### `src/gui_framework/components/interaction.rs`
- **Purpose**: Defines interaction properties for an entity.
- **Key Structs**: `Interaction { clickable: bool, draggable: bool }`.
- **Notes**: Used by `interaction_system` to determine how entities react to input.

### `src/gui_framework/context/mod.rs`
- **Purpose**: Re-exports Vulkan context modules.
- **Modules**: `vulkan_context.rs`, `vulkan_setup.rs`.

### `src/gui_framework/context/vulkan_context.rs`
- **Purpose**: Holds core Vulkan resources (instance, physical device, logical device, allocator, pipeline layout, swapchain info, etc.) and related state.
- **Key Structs**: `VulkanContext { entry, instance, surface_loader, surface, physical_device: Option<vk::PhysicalDevice>, device, queue, queue_family_index, allocator, swapchain_loader, swapchain, current_swap_extent: vk::Extent2D, images, image_views, render_pass, framebuffers, pipeline_layout: Option<vk::PipelineLayout>, command_pool, command_buffers, image_available_semaphore, render_finished_semaphore, fence, current_image }`.
- **Key Methods**: `new() -> Self`.
- **Notes**: Managed via `VulkanContextResource`. Holds actual swap extent.

### `src/gui_framework/context/vulkan_setup.rs`
- **Purpose**: Sets up and tears down core Vulkan resources managed by `VulkanContext`.
- **Key Methods**: `setup_vulkan(vk_context: &mut VulkanContext, window: &winit::window::Window) -> ()`, `cleanup_vulkan(vk_context: &mut VulkanContext) -> ()`.
- **Notes**: Called by Bevy systems in `main.rs`. Stores chosen `physical_device` in `VulkanContext`. Still uses `winit::window::Window`. Enables validation layers in debug builds.

### `src/gui_framework/events/mod.rs`
- **Purpose**: Declares and re-exports Bevy Event modules.
- **Modules**: `interaction_events.rs`.

### `src/gui_framework/events/interaction_events.rs`
- **Purpose**: Defines Bevy events related to user interaction.
- **Key Structs**: `EntityClicked { entity: Entity }`, `EntityDragged { entity: Entity, delta: Vec2 }`, `HotkeyActionTriggered { action: String }`.
- **Notes**: Written by input systems (`interaction_system`, `hotkey_system`), read by logic systems (`movement_system`, `app_control_system`).

### `src/gui_framework/interaction/mod.rs`
- **Purpose**: Declares interaction-related submodules (currently only `hotkeys`).
- **Modules**: `hotkeys.rs`.

### `src/gui_framework/interaction/hotkeys.rs`
- **Purpose**: Defines hotkey configuration loading from TOML and error handling.
- **Key Structs**: `HotkeyConfig { mappings: HashMap<String, String> }`, `HotkeyError { ... }`.
- **Key Methods**: `HotkeyConfig::load_config(...) -> Result<Self, HotkeyError>`, `HotkeyConfig::get_action(...) -> Option<&String>`.
- **Notes**: Loaded into `HotkeyResource` by `setup_scene_ecs` in `main.rs`. Used by `hotkey_system`.

### `src/gui_framework/rendering/mod.rs`
- **Purpose**: Re-exports rendering sub-modules and key types (`Renderer`).
- **Modules**: `buffer_manager.rs`, `command_buffers.rs`, `pipeline_manager.rs`, `render_engine.rs`, `resize_handler.rs`, `shader_utils.rs`, `swapchain.rs`.

### `src/gui_framework/rendering/command_buffers.rs`
- **Purpose**: Records Vulkan command buffers based on `PreparedDrawData` for the current swapchain image. Resets command pool before recording.
- **Key Methods**: `record_command_buffers(platform: &mut VulkanContext, prepared_draws: &[PreparedDrawData], pipeline_layout: vk::PipelineLayout, extent: vk::Extent2D) -> ()`.
- **Notes**: Called by `render_engine.rs`. Consumes `PreparedDrawData` generated by `BufferManager`. Uses extent provided by `Renderer` (which should be `platform.current_swap_extent`). Allocates command buffers if needed (e.g., first frame).

### `src/gui_framework/rendering/render_engine.rs`
- **Purpose**: Orchestrates custom Vulkan rendering per frame. Manages `BufferManager`, descriptor pool/layout handles, command pool, and sync objects. Handles frame synchronization using fences. Calls `BufferManager` to prepare resources and `command_buffers` to record draws. Triggers swapchain recreation on resize. Uses `bevy_math::Mat4`.
- **Key Structs**: `Renderer { buffer_manager: BufferManager, descriptor_pool: vk::DescriptorPool, descriptor_set_layout: vk::DescriptorSetLayout }`.
- **Key Methods**:
  - `new(platform: &mut VulkanContext, extent: vk::Extent2D) -> Self`
  - `resize_renderer(&mut self, vulkan_context: &mut VulkanContext, width: u32, height: u32) -> ()`
  - `render(&mut self, platform: &mut VulkanContext, render_commands: &[RenderCommandData]) -> ()` (Waits for fence, prepares resources, acquires image, records commands, submits, presents).
  - `cleanup(&mut self, platform: &mut VulkanContext) -> ()`
- **Notes**: Managed via `RendererResource`. Called by `rendering_system`. Uses `platform.current_swap_extent` for rendering dimensions. Includes checks for `None` resources during shutdown.

### `src/gui_framework/rendering/pipeline_manager.rs`
- **Purpose**: **Initialization helper.** Creates Vulkan `PipelineLayout`, `DescriptorSetLayout`, and `DescriptorPool` during application setup.
- **Key Structs**: `PipelineManager { pipeline_layout: vk::PipelineLayout, descriptor_set_layout: vk::DescriptorSetLayout, descriptor_pool: vk::DescriptorPool }`.
- **Key Methods**: `new(platform: &mut VulkanContext) -> Self`.
- **Notes**: Provides layout/pool to `Renderer` during initialization. No longer has a `cleanup` method.

### `src/gui_framework/rendering/buffer_manager.rs`
- **Purpose**: Manages global projection UBO and per-entity Vulkan resources (vertex buffers, transform UBOs, descriptor sets). Caches pipelines and shader modules.
- **Key Structs**:
    - `EntityRenderResources { vertex_buffer: vk::Buffer, vertex_allocation: vk_mem::Allocation, vertex_count: u32, offset_uniform: vk::Buffer, offset_allocation: vk_mem::Allocation, descriptor_set: vk::DescriptorSet }`
    - `PipelineCacheKey { vertex_shader_path: String, fragment_shader_path: String }`
    - `ShaderCacheKey = String`
    - `BufferManager { uniform_buffer, uniform_allocation, entity_cache: HashMap<Entity, EntityRenderResources>, pipeline_cache: HashMap<PipelineCacheKey, vk::Pipeline>, shader_cache: HashMap<ShaderCacheKey, vk::ShaderModule>, descriptor_set_layout, descriptor_pool }`
- **Key Methods**:
  - `new(platform: &mut VulkanContext, _pipeline_layout: vk::PipelineLayout, descriptor_set_layout: vk::DescriptorSetLayout, descriptor_pool: vk::DescriptorPool) -> Self`
  - `prepare_frame_resources(&mut self, platform: &mut VulkanContext, render_commands: &[RenderCommandData]) -> Vec<PreparedDrawData>` (Uses caches, updates UBOs/descriptor sets).
  - `cleanup(&mut self, platform: &mut VulkanContext) -> ()` (Cleans caches and entity resources).
- **Notes**: Creates/updates resources based on `RenderCommandData`. Uses persistently mapped pointers correctly. **Lacks resource removal for despawned entities and vertex buffer updates.** Pipeline state defines only vertex input binding/location 0.

### `src/gui_framework/rendering/resize_handler.rs`
- **Purpose**: Handles window resizing logic: cleans up old swapchain resources, creates new swapchain/framebuffers, updates projection uniform buffer using the actual swap extent.
- **Key Structs**: `ResizeHandler` (stateless).
- **Key Methods**: `resize(vulkan_context: &mut VulkanContext, new_extent: vk::Extent2D, uniform_allocation: &mut vk_mem::Allocation) -> ()`.
- **Notes**: Called by `Renderer::resize_renderer`. Uses `cleanup_swapchain_resources`, `create_swapchain`, `create_framebuffers`. Uses `vulkan_context.current_swap_extent` for UBO update.

### `src/gui_framework/rendering/shader_utils.rs`
- **Purpose**: Loads compiled SPIR-V shader modules from files.
- **Key Methods**: `load_shader(device: &ash::Device, filename: &str) -> vk::ShaderModule`.
- **Notes**: Used by `BufferManager`. Expects `.spv` files in `shaders/` relative path (within target dir).

### `src/gui_framework/rendering/swapchain.rs`
- **Purpose**: Manages Vulkan swapchain creation/recreation, associated image views, framebuffers, and render pass. Cleans up these resources.
- **Key Methods**:
  - `create_swapchain(platform: &mut VulkanContext, extent: vk::Extent2D) -> vk::SurfaceFormatKHR` (Stores chosen extent in `platform.current_swap_extent`).
  - `create_framebuffers(platform: &mut VulkanContext, surface_format: vk::SurfaceFormatKHR) -> ()` (Uses `platform.current_swap_extent`).
  - `cleanup_swapchain_resources(platform: &mut VulkanContext) -> ()`.
- **Notes**: Called by `Renderer::new` and `ResizeHandler::resize`. Uses `platform.physical_device`.

### `build.rs`
- **Purpose**: Compiles GLSL shaders (`.vert`, `.frag`) to SPIR-V (`.spv`) using `glslc`, copies compiled shaders and user configuration files (e.g., `user/hotkeys.toml`) to the appropriate target build directory. Tracks shader source files for changes.
- **Key Methods**: `main() -> ()`, `compile_and_copy_shaders() -> ()`, `copy_user_files() -> ()`.
- **Notes**: Ensures shaders and runtime config are available to the executable. Uses `walkdir`. Requires `glslc` (from Vulkan SDK or PATH).

## Shaders
- **Location**: `shaders/` (Source GLSL), compiled `.spv` copied to `target/<profile>/shaders/` by `build.rs`.
- **Files**: `background.vert`, `background.frag`, `triangle.vert`, `triangle.frag`, `square.vert`, `square.frag`, `test_shader.vert`.
- **Roles**: Support orthographic projection (UBO binding 0), object transform matrix (UBO binding 1). Use vertex position (location 0) as input. Loaded by `BufferManager`.

## Dependencies
- `ash = "0.38"`, `vk-mem = "0.4"`, `ash-window = "0.13"`, `raw-window-handle = "0.6"`, `toml = "0.8"`, `thiserror = "2.0"`, **`bevy_app = "0.15"`**, **`bevy_core = "0.15"`**, **`bevy_ecs = "0.15"`**, **`bevy_log = "0.15"`**, **`bevy_utils = "0.15"`**, **`bevy_window = "0.15"`**, **`bevy_winit = "0.15"`**, **`bevy_reflect = "0.15"`**, **`bevy_input = "0.15"`**, **`bevy_time = "0.15"`**, **`bevy_diagnostic = "0.15"`**, **`bevy_a11y = "0.15"`**, **`bevy_math = "0.15"`**, **`bevy_transform = "0.15"`**. `winit = "0.30.9"` (Used internally by `bevy_winit` and `vulkan_setup`).
- **Build Dependencies**: `walkdir = "2"`.