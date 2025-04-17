# Modules Documentation for `rusty_whip`

## Project Overview: `rusty_whip`
### Purpose
`rusty_whip` is an advanced 2D & 3D content generation application with GPU-accelerated AI diffusion/inference, multimedia creation, and story-driven workflows, targeting P2P networking. **It is currently migrating its core logic to the Bevy engine (v0.15), strictly avoiding Bevy's rendering stack.**
### Current State (Post Task 6.3 Partial Completion)
- **Bevy Integration**: Application runs using `bevy_app::App`, `bevy_winit`, `bevy_input`, `bevy_transform`, and other core non-rendering Bevy plugins.
- **ECS Migration**: Core application state (transforms, shapes, visibility, interaction flags) managed via Bevy ECS components. User input processed by Bevy systems, triggering Bevy events. Old `Scene`, `EventBus`, `InteractionController`, `ClickRouter` removed.
- **Math Migration**: Uses `bevy_math` types (`Vec2`, `Mat4`).
- **Rendering Bridge**: Custom Vulkan context (`VulkanContext`) managed via `VulkanContextResource`. Custom Vulkan renderer (`Renderer`) accessed via `RendererResource` (currently holding a placeholder).
- **Rendering Status**: Rendering is triggered by a Bevy system (`rendering_system`) but uses **placeholder logic**. The custom Vulkan backend (`Renderer`, `BufferManager`, etc.) is **not yet consuming ECS data** and requires significant refactoring (Task 6.3 Step 7). Visual output does not reflect the ECS state.
- **Features Active**: Bevy app structure, windowing, logging, input handling (click, drag, hotkeys via `ButtonInput`), ECS component/event usage, `bevy_transform`, core Vulkan setup, hotkey loading.
- **Features Inactive/Placeholder**: Actual scene rendering based on ECS data, Vulkan buffer management based on ECS data.
- **Modules Removed**: `gui_framework/scene/`, `gui_framework/event_bus.rs`, `gui_framework/interaction/controller.rs`.
- Task 1-5 **Complete** (Legacy). Task 6.1, 6.2 **Complete**. Task 6.3 Steps 1-6, 8 **Complete**. Step 7 **Pending**.

## Module Structure
Studio_Whip/
    LICENSE
    README.md
    .gitignore
    Rust/
        src/
            gui_framework/
                components/             # NEW: Bevy ECS Components
                    interaction.rs
                    mod.rs
                    shape_data.rs
                    visibility.rs
                context/
                    vulkan_context.rs
                    vulkan_setup.rs
                    mod.rs
                events/                 # NEW: Bevy Events
                    interaction_events.rs
                    mod.rs
                interaction/            # Modified: Only hotkeys remain
                    hotkeys.rs
                    mod.rs
                rendering/              # Modified: Adapting to ECS
                    buffer_manager.rs   # Needs major rework
                    command_buffers.rs  # Needs rework
                    mod.rs
                    pipeline_manager.rs # Adapted
                    render_engine.rs    # Needs rework
                    renderable.rs       # Likely to be removed/replaced
                    resize_handler.rs   # Needs rework
                    shader_utils.rs
                    swapchain.rs
                mod.rs
            lib.rs
            main.rs                     # Heavily modified for Bevy App/ECS/Systems
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
- **Purpose**: Defines the shared `Vertex` type. No longer broadly re-exports `gui_framework`.
- **Key Structs**: `Vertex { position: [f32; 2] }`.

### `src/main.rs`
- **Purpose**: Entry point; sets up `bevy_app::App`, core Bevy plugins (Input, Transform, Window, Log, etc., *excluding rendering*), Bevy resources (`VulkanContextResource`, `RendererResource` [placeholder], `HotkeyResource`), custom Bevy events, and Bevy systems for lifecycle management, ECS setup, input processing, state updates, and triggering the custom Vulkan rendering backend (via placeholder).
- **Key Structs (Defined In `main.rs`)**:
    - `VulkanContextResource(Arc<Mutex<VulkanContext>>)`
    - `RendererResource(Arc<Mutex<PlaceholderRenderer>>)` // Placeholder type used
    - `HotkeyResource(HotkeyConfig)`
    - `PlaceholderRenderer` // Temporary struct definition for bridging
    - `RenderCommandData` // Placeholder struct for ECS data -> Renderer
- **Key Methods (Bevy Systems)**:
    - `main() -> ()`
    - `setup_vulkan_system(...) -> Result<(), String>`: Initializes Vulkan context.
    - `create_renderer_system(...) -> Result<(), String>`: Creates placeholder `RendererResource`.
    - `setup_scene_ecs(...) -> ()`: Loads `HotkeyResource`, spawns initial ECS entities.
    - `interaction_system(...) -> ()`: Processes mouse clicks/drags, writes `EntityClicked`/`EntityDragged` events.
    - `hotkey_system(...) -> ()`: Processes keyboard input, writes `HotkeyActionTriggered` events.
    - `movement_system(...) -> ()`: Updates `Transform` components based on `EntityDragged` events.
    - `app_control_system(...) -> ()`: Handles application exit based on `HotkeyActionTriggered` events.
    - `handle_resize_system(...) -> ()`: Calls placeholder `Renderer::resize_renderer`.
    - `rendering_system(...) -> ()`: Queries ECS, calls placeholder `Renderer::render`. **(Needs rework)**.
    - `handle_close_request(...) -> ()`: Sends `AppExit` on window close request.
    - `cleanup_system(...) -> ()`: Cleans up `RendererResource` (placeholder) and `VulkanContextResource` on `AppExit`.
- **Notes**: Uses Bevy App structure. Manages Vulkan state via `Resource` bridge pattern. Rendering logic is placeholder. Input handled by Bevy systems.

### `src/gui_framework/mod.rs`
- **Purpose**: Declares and re-exports framework modules. Exports types needed for Vulkan backend and hotkey config.
- **Notes**: No longer declares/exports `scene` or `event_bus`. `interaction` module only contains `hotkeys`.

### `src/gui_framework/components/mod.rs`
- **Purpose**: Declares and re-exports Bevy ECS component modules.
- **Modules**: `shape_data.rs`, `visibility.rs`, `interaction.rs`.

### `src/gui_framework/components/shape_data.rs`
- **Purpose**: Defines the visual shape data for an entity.
- **Key Structs**: `ShapeData { vertices: Vec<Vertex>, vertex_shader_path: String, fragment_shader_path: String }`.
- **Notes**: Used by `rendering_system` (eventually) to determine what to draw.

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

### `src/gui_framework/context/vulkan_context.rs`
- **Purpose**: Holds core Vulkan resources (instance, device, allocator, etc.) and related state.
- **Key Structs**: `VulkanContext { ... }`.
- **Key Methods**: `new() -> Self`.
- **Notes**: Managed via `VulkanContextResource` in `main.rs`.

### `src/gui_framework/context/vulkan_setup.rs`
- **Purpose**: Sets up and tears down core Vulkan resources managed by `VulkanContext`.
- **Key Methods**: `setup_vulkan(...) -> ()`, `cleanup_vulkan(...) -> ()`.
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

### `src/gui_framework/interaction/hotkeys.rs`
- **Purpose**: Defines hotkey configuration loading from TOML and error handling.
- **Key Structs**: `HotkeyConfig { mappings: HashMap<String, String> }`, `HotkeyError { ... }`.
- **Key Methods**: `HotkeyConfig::load_config(...) -> Result<Self, HotkeyError>`, `HotkeyConfig::get_action(...) -> Option<&String>`.
- **Notes**: Loaded into `HotkeyResource` by `setup_scene_ecs` in `main.rs`. Used by `hotkey_system`. `format_key_event` function (using winit types) is no longer used by active systems.

### `src/gui_framework/rendering/mod.rs`
- **Purpose**: Re-exports rendering sub-modules and key types (`Renderer`, `Renderable`).

### `src/gui_framework/rendering/command_buffers.rs`
- **Purpose**: Records Vulkan command buffers. **Currently uses placeholder `Renderable` data; needs rework to use ECS data.**
- **Key Methods**: `record_command_buffers(platform: &mut VulkanContext, renderables: &[Renderable], ...) -> ()`.
- **Notes**: Checks `renderable.visible`. Called by `render_engine.rs` and `resize_handler.rs`.

### `src/gui_framework/rendering/renderable.rs`
- **Purpose**: **Legacy struct.** Defines properties of a Vulkan-renderable object. **Likely to be replaced by direct use of ECS components or `RenderCommandData`.**
- **Key Structs**: `Renderable { ... }`.

### `src/gui_framework/rendering/render_engine.rs`
- **Purpose**: Orchestrates custom Vulkan rendering. **Currently uses placeholder logic and does not consume ECS data. Needs significant rework.** Manages `PipelineManager`, `BufferManager`. Uses `bevy_math::Mat4`.
- **Key Structs**: `Renderer { pipeline_manager: PipelineManager, buffer_manager: BufferManager }`.
- **Key Methods**:
  - `new(platform: &mut VulkanContext, extent: vk::Extent2D) -> Self` (Signature changed)
  - `resize_renderer(&mut self, vulkan_context: &mut VulkanContext, width: u32, height: u32) -> ()` (Signature changed)
  - `render(&mut self, platform: &mut VulkanContext) -> ()` (Signature changed, needs ECS data param)
  - `cleanup(&mut self, platform: &mut VulkanContext) -> ()` (Signature changed to `&mut self`)
- **Notes**: Managed via `RendererResource` (placeholder). Called by Bevy systems. Cleanup uses `&mut self`.

### `src/gui_framework/rendering/pipeline_manager.rs`
- **Purpose**: Manages Vulkan `PipelineLayout`, `DescriptorSetLayout`, `DescriptorPool`, and the global projection `DescriptorSet`.
- **Key Structs**: `PipelineManager { ... }`.
- **Key Methods**: `new(&mut VulkanContext) -> Self` (Signature changed), `cleanup(self, device: &ash::Device) -> ()`.
- **Notes**: Provides resources to `BufferManager`. Adapted to remove `Scene` dependency.

### `src/gui_framework/rendering/buffer_manager.rs`
- **Purpose**: **Legacy component requiring major rework.** Previously managed Vulkan buffers/allocations/pipelines/shaders/descriptors per object. **Currently only creates global uniform buffer. Needs reimplementation based on ECS data.** Uses `bevy_math::Mat4`.
- **Key Structs**: `BufferManager { uniform_buffer, uniform_allocation, renderables: Vec<Renderable> }`.
- **Key Methods**:
  - `new(&mut VulkanContext, vk::PipelineLayout, vk::DescriptorSetLayout, vk::DescriptorPool) -> Self` (Signature changed, functionality reduced)
  - `update_offset(...) -> ()` (Placeholder)
  - `update_instance_offset(...) -> ()` (Placeholder)
  - `update_instance_buffer(...) -> ()` (Placeholder)
  - `cleanup(mut self, device: &ash::Device, allocator: &vk_mem::Allocator, descriptor_pool: vk::DescriptorPool) -> ()`.
- **Notes**: Takes layout/pool from `PipelineManager`. `renderables` vec is unused by `new`.

### `src/gui_framework/rendering/resize_handler.rs`
- **Purpose**: Handles window resizing logic for Vulkan swapchain and projection. **Vertex resizing logic removed; needs ECS-based replacement.** Uses `bevy_math::Mat4`.
- **Key Structs**: `ResizeHandler` (stateless).
- **Key Methods**: `resize(&mut VulkanContext, &mut Vec<Renderable>, vk::PipelineLayout, vk::DescriptorSet, u32, u32, &mut vk_mem::Allocation) -> ()` (Signature changed).
- **Notes**: Called by `Renderer::resize_renderer`.

### `src/gui_framework/rendering/shader_utils.rs`
- **Purpose**: Loads compiled SPIR-V shader modules from files.
- **Key Methods**: `load_shader(device: &ash::Device, filename: &str) -> vk::ShaderModule`.

### `src/gui_framework/rendering/swapchain.rs`
- **Purpose**: Manages Vulkan swapchain creation/recreation and associated image views and framebuffers.
- **Key Methods**: `create_swapchain(...) -> vk::SurfaceFormatKHR`, `create_framebuffers(...) -> ()`.
- **Notes**: Called by `Renderer::new` and `ResizeHandler::resize`.

### `build.rs`
- **Purpose**: Compiles GLSL shaders to SPIR-V using `glslc` and copies user configuration files (e.g., `user/hotkeys.toml`) to the appropriate target build directory.
- **Key Methods**: `main() -> ()`, `compile_shaders() -> ()`, `copy_user_files() -> ()`.
- **Notes**: Ensures shaders and runtime config are available to the executable.

## Shaders
- **Location**: `shaders/` (Source), copied to target build dir by `build.rs`.
- **Files**: `background.vert.spv`, `background.frag.spv`, `triangle.vert.spv`, `triangle.frag.spv`, `square.vert.spv`, `square.frag.spv`.
- **Roles**: Support orthographic projection (UBO binding 0), object offset (UBO binding 1), and instance offset (vertex attribute binding 1). **Integration with ECS components pending.**

## Dependencies
- `ash = "0.38"`, `vk-mem = "0.4"`, `ash-window = "0.13"`, `raw-window-handle = "0.6"`, `toml = "0.8"`, `thiserror = "2.0"`, **`bevy_app = "0.15"`**, **`bevy_core = "0.15"`**, **`bevy_ecs = "0.15"`**, **`bevy_log = "0.15"`**, **`bevy_utils = "0.15"`**, **`bevy_window = "0.15"`**, **`bevy_winit = "0.15"`**, **`bevy_reflect = "0.15"`**, **`bevy_input = "0.15"`**, **`bevy_time = "0.15"`**, **`bevy_diagnostic = "0.15"`**, **`bevy_a11y = "0.15"`**, **`bevy_math = "0.15"`**, **`bevy_transform = "0.15"`**. (Removed `glam`). `winit = "0.30.9"` (Used internally by `bevy_winit` and `vulkan_setup`).