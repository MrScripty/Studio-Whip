# Modules Documentation for `rusty_whip`

## Project Overview: `rusty_whip`
### Purpose
`rusty_whip` is an advanced 2D & 3D content generation application with GPU-accelerated AI diffusion/inference, multimedia creation, and story-driven workflows, targeting P2P networking. **It is currently migrating its core logic to the Bevy engine (v0.15), strictly avoiding Bevy's rendering stack.**
### Current State (Post Task 6.3 Step 7 Implementation)
- **Bevy Integration**: Application runs using `bevy_app::App`, `bevy_winit`, `bevy_input`, `bevy_transform`, and other core non-rendering Bevy plugins.
- **ECS Migration**: Core application state managed via Bevy ECS components. User input processed by Bevy systems, triggering Bevy events.
- **Math Migration**: Uses `bevy_math` types (`Vec2`, `Mat4`).
- **Rendering Bridge**: Custom Vulkan context (`VulkanContext`) managed via `VulkanContextResource`. Custom Vulkan renderer (`Renderer`) accessed via `RendererResource`.
- **Rendering Status**: Rendering is triggered by `rendering_system` which collects `RenderCommandData`. `BufferManager` now implements core logic to create/update Vulkan resources (buffers, pipelines, descriptor sets) based on this data. `command_buffers` uses the prepared data to record draw calls. **Visual output is likely incorrect and requires debugging. Optimizations (caching, resource removal) are needed.** `Renderable` struct removed.
- **Features Active**: Bevy app structure, windowing, logging, input handling (click, drag, hotkeys), ECS component/event usage, `bevy_transform`, core Vulkan setup, hotkey loading, basic ECS-driven Vulkan resource management.
- **Modules Removed**: `gui_framework/scene/`, `gui_framework/event_bus.rs`, `gui_framework/interaction/controller.rs`, `gui_framework/rendering/renderable.rs`.
- Task 1-5 **Complete** (Legacy). Task 6.1, 6.2 **Complete**. Task 6.3 Steps 1-6, 8 **Complete**. Step 7 **Implementation Complete (Needs Optimization/Debugging)**.

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
                    buffer_manager.rs   # Core logic implemented, needs opt.
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
            # ... shader files ...
        user/
            hotkeys.toml
        Documentation/
            # ... docs ...
        utilities/
            # ... utils ...
        Cargo.toml
        build.rs
        compile_shaders.ps1
        compile_shaders.sh


## Modules and Their Functions

### `src/lib.rs`
- **Purpose**: Defines shared types (`Vertex`, `RenderCommandData`, `PreparedDrawData`).
- **Key Structs**:
    - `Vertex { position: [f32; 2] }`
    - `RenderCommandData { entity_id: Entity, transform_matrix: Mat4, vertices: Arc<Vec<Vertex>>, vertex_shader_path: String, fragment_shader_path: String, depth: f32 }`
    - `PreparedDrawData { pipeline: vk::Pipeline, vertex_buffer: vk::Buffer, vertex_count: u32, descriptor_set: vk::DescriptorSet }`
- **Notes**: Uses `ash::vk`. `Arc` used in `RenderCommandData`.

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
- **Notes**: Uses Bevy App structure. Manages Vulkan state via `Resource` bridge pattern. Rendering uses actual `Renderer` whose `BufferManager` now implements core resource creation.

### `src/gui_framework/mod.rs`
- **Purpose**: Declares and re-exports framework modules. Exports types needed for Vulkan backend and hotkey config.
- **Notes**: No longer declares/exports `scene`, `event_bus`, or `renderable`. `interaction` module only contains `hotkeys`.

### `src/gui_framework/components/mod.rs`
- **Purpose**: Declares and re-exports Bevy ECS component modules.
- **Modules**: `shape_data.rs`, `visibility.rs`, `interaction.rs`.

### `src/gui_framework/components/shape_data.rs`
- **Purpose**: Defines the visual shape data for an entity.
- **Key Structs**: `ShapeData { vertices: Arc<Vec<Vertex>>, vertex_shader_path: String, fragment_shader_path: String }`.
- **Notes**: Uses `Arc` for vertices. Used by `rendering_system` to create `RenderCommandData`.

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
- **Purpose**: Holds core Vulkan resources (instance, device, allocator, pipeline layout, etc.) and related state.
- **Key Structs**: `VulkanContext { entry, instance, surface_loader, surface, device, queue, queue_family_index, allocator, swapchain_loader, swapchain, images, image_views, vertex_buffer, vertex_allocation, render_pass, framebuffers, vertex_shader, fragment_shader, pipeline_layout: Option<vk::PipelineLayout>, command_pool, command_buffers, image_available_semaphore, render_finished_semaphore, fence, current_image }`.
- **Key Methods**: `new() -> Self`.
- **Notes**: Managed via `VulkanContextResource`. `pipeline_layout` added. Some fields like `vertex_buffer`, `vertex_shader`, etc. are likely obsolete now.

### `src/gui_framework/context/vulkan_setup.rs`
- **Purpose**: Sets up and tears down core Vulkan resources managed by `VulkanContext`.
- **Key Methods**: `setup_vulkan(vk_context: &mut VulkanContext, window: &winit::window::Window) -> ()`, `cleanup_vulkan(vk_context: &mut VulkanContext) -> ()`.
- **Notes**: Called by Bevy systems in `main.rs`. Still uses `winit::window::Window`.

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
- **Purpose**: Records Vulkan command buffers based on `PreparedDrawData`.
- **Key Methods**: `record_command_buffers(platform: &mut VulkanContext, prepared_draws: &[PreparedDrawData], pipeline_layout: vk::PipelineLayout, extent: vk::Extent2D) -> ()`.
- **Notes**: Called by `render_engine.rs`. Consumes `PreparedDrawData` generated by `BufferManager`. Draw logic is basic (no instancing yet).

### `src/gui_framework/rendering/render_engine.rs`
- **Purpose**: Orchestrates custom Vulkan rendering per frame. Manages `BufferManager`, descriptor pool/layout handles, and sync objects. Calls `BufferManager` to prepare resources and `command_buffers` to record draws. Uses `bevy_math::Mat4`.
- **Key Structs**: `Renderer { buffer_manager: BufferManager, current_extent: vk::Extent2D, descriptor_pool: vk::DescriptorPool, descriptor_set_layout: vk::DescriptorSetLayout }`.
- **Key Methods**:
  - `new(platform: &mut VulkanContext, extent: vk::Extent2D) -> Self`
  - `resize_renderer(&mut self, vulkan_context: &mut VulkanContext, width: u32, height: u32) -> ()`
  - `render(&mut self, platform: &mut VulkanContext, render_commands: &[RenderCommandData]) -> ()`
  - `cleanup(&mut self, platform: &mut VulkanContext) -> ()`
- **Notes**: Managed via `RendererResource`. Called by `rendering_system`. Calls implemented `BufferManager::prepare_frame_resources`. Cleans up descriptor pool/layout.

### `src/gui_framework/rendering/pipeline_manager.rs`
- **Purpose**: **Initialization helper.** Creates Vulkan `PipelineLayout`, `DescriptorSetLayout`, and `DescriptorPool` during application setup.
- **Key Structs**: `PipelineManager { pipeline_layout: vk::PipelineLayout, descriptor_set_layout: vk::DescriptorSetLayout, descriptor_pool: vk::DescriptorPool }`.
- **Key Methods**: `new(platform: &mut VulkanContext) -> Self`.
- **Notes**: Provides layout/pool to `Renderer` during initialization. No longer has a `cleanup` method.

### `src/gui_framework/rendering/buffer_manager.rs`
- **Purpose**: Manages global projection UBO and per-entity Vulkan resources (buffers, pipelines, shaders, descriptor sets) based on ECS `RenderCommandData`. **Core implementation complete but needs optimization (caching, resource removal).**
- **Key Structs**:
    - `EntityRenderResources { vertex_buffer: vk::Buffer, vertex_allocation: vk_mem::Allocation, vertex_count: u32, offset_uniform: vk::Buffer, offset_allocation: vk_mem::Allocation, descriptor_set: vk::DescriptorSet, pipeline: vk::Pipeline, vertex_shader: vk::ShaderModule, fragment_shader: vk::ShaderModule }`
    - `BufferManager { uniform_buffer, uniform_allocation, entity_cache: HashMap<Entity, EntityRenderResources>, descriptor_set_layout, descriptor_pool }`
- **Key Methods**:
  - `new(platform: &mut VulkanContext, pipeline_layout: vk::PipelineLayout, descriptor_set_layout: vk::DescriptorSetLayout, descriptor_pool: vk::DescriptorPool) -> Self`
  - `prepare_frame_resources(&mut self, platform: &mut VulkanContext, render_commands: &[RenderCommandData]) -> Vec<PreparedDrawData>` (Implemented)
  - `cleanup(&mut self, platform: &mut VulkanContext) -> ()`
- **Notes**: Creates/updates resources based on `RenderCommandData`. Uses persistently mapped pointers correctly. Lacks pipeline/shader caching and resource removal for despawned entities. Needs debugging for visual output.

### `src/gui_framework/rendering/resize_handler.rs`
- **Purpose**: Handles window resizing logic for Vulkan swapchain and projection uniform buffer update.
- **Key Structs**: `ResizeHandler` (stateless).
- **Key Methods**: `resize(vulkan_context: &mut VulkanContext, new_extent: vk::Extent2D, uniform_allocation: &mut vk_mem::Allocation) -> ()`.
- **Notes**: Called by `Renderer::resize_renderer`. Vertex resizing logic removed. Includes error handling for buffer mapping.

### `src/gui_framework/rendering/shader_utils.rs`
- **Purpose**: Loads compiled SPIR-V shader modules from files.
- **Key Methods**: `load_shader(device: &ash::Device, filename: &str) -> vk::ShaderModule`.
- **Notes**: Used by `BufferManager`. Caching is not implemented.

### `src/gui_framework/rendering/swapchain.rs`
- **Purpose**: Manages Vulkan swapchain creation/recreation and associated image views and framebuffers.
- **Key Methods**: `create_swapchain(platform: &mut VulkanContext, extent: vk::Extent2D) -> vk::SurfaceFormatKHR`, `create_framebuffers(platform: &mut VulkanContext, extent: vk::Extent2D, surface_format: vk::SurfaceFormatKHR) -> ()`.
- **Notes**: Called by `Renderer::new` and `ResizeHandler::resize`.

### `build.rs`
- **Purpose**: Compiles GLSL shaders to SPIR-V using `glslc` and copies user configuration files (e.g., `user/hotkeys.toml`) to the appropriate target build directory.
- **Key Methods**: `main() -> ()`, `compile_shaders() -> ()`, `copy_user_files() -> ()`.
- **Notes**: Ensures shaders and runtime config are available to the executable.

## Shaders
- **Location**: `shaders/` (Source), copied to target build dir by `build.rs`.
- **Files**: `background.vert.spv`, `background.frag.spv`, `triangle.vert.spv`, `triangle.frag.spv`, `square.vert.spv`, `square.frag.spv`.
- **Roles**: Support orthographic projection (UBO binding 0), object offset (UBO binding 1). Instancing support (vertex attribute binding 1) is defined in shaders but not yet utilized by `BufferManager`/`command_buffers`. Loaded by `BufferManager`.

## Dependencies
- `ash = "0.38"`, `vk-mem = "0.4"`, `ash-window = "0.13"`, `raw-window-handle = "0.6"`, `toml = "0.8"`, `thiserror = "2.0"`, **`bevy_app = "0.15"`**, **`bevy_core = "0.15"`**, **`bevy_ecs = "0.15"`**, **`bevy_log = "0.15"`**, **`bevy_utils = "0.15"`**, **`bevy_window = "0.15"`**, **`bevy_winit = "0.15"`**, **`bevy_reflect = "0.15"`**, **`bevy_input = "0.15"`**, **`bevy_time = "0.15"`**, **`bevy_diagnostic = "0.15"`**, **`bevy_a11y = "0.15"`**, **`bevy_math = "0.15"`**, **`bevy_transform = "0.15"`**. (Removed `glam`). `winit = "0.30.9"` (Used internally by `bevy_winit` and `vulkan_setup`).