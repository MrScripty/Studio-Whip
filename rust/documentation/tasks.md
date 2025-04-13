# Tasks for `rusty_whip` GUI Framework Enhancements (March 19, 2025 - Updated)

## Overview
These tasks enhance `gui_framework` to support a future divider system in `gui_app`, adding efficient element creation, grouping, instancing, and input handling. The framework remains generic, with `gui_app` building specific UIs atop it. Recent focus: Implementing event bus, refactoring rendering, logical grouping, batch updates via events, visibility toggling, and a configurable hotkey system. The main event loop now uses `EventLoop::run` within `main.rs`.

## Task 1: Implement Event Bus and Convert Existing Functionality
- **Goal**: Introduce an event bus to decouple components and convert current interactions (dragging, instancing) to use it.
- **Status**: **Complete**
- **Summary**: Implemented `EventBus`, converted dragging/instancing, refactored `Renderer`/`BufferManager`/`PipelineManager`, implemented `SceneEventHandler`.

## Task 2: Redesign Grouping System for Logical Organization
- **Goal**: Redesign groups as named, logical containers decoupled from rendering/interaction, supporting multiple group membership per object. Prepare for batch operations.
- **Affected Modules**: `src/gui_framework/scene/scene.rs`, `src/gui_framework/scene/group.rs`, `src/gui_framework/mod.rs`.
- **Status**: **Complete**
- **Summary**: Created `group.rs` with `GroupManager`/`GroupEditor`, integrated into `Scene`.

## Task 3: Implement Group Batch Update Trigger via Events
- **Goal**: Add functionality to `GroupEditor` to efficiently *trigger* updates for all objects within a group by publishing *individual* `FieldUpdated` events for each object. Supports fields like `visible`, `is_draggable`, `offset`, `depth`.
- **Affected Modules**: `src/gui_framework/scene/group.rs`, `src/gui_framework/scene/scene.rs`, `src/gui_framework/event_bus.rs`, `src/main.rs` (event handling).
- **Status**: **Complete**
- **Summary**: Added `visible` state flag to `RenderObject`. Added `FieldError` enum. Added `FieldUpdated` variant to `BusEvent` (using `Arc<dyn Any + Send + Sync>`). Implemented `GroupEditor::set_field` to publish individual events for group members. Updated `SceneEventHandler` (subscribed in `main.rs`) to handle `FieldUpdated` and modify `RenderObject` state. Tested in `main.rs`.
- **Notes**: Implements batch *triggering*. Changing `depth` requires renderer re-sorting (not implemented). Changing `visible` requires renderer modification (Task 3.1).

## Task 3.1: Implement Visibility Check in Renderer
- **Goal**: Modify the rendering loop to query and respect the `RenderObject.visible` state flag, skipping draw calls for non-visible objects.
- **Affected Modules**: `src/gui_framework/rendering/renderable.rs`, `src/gui_framework/rendering/buffer_manager.rs`, `src/gui_framework/rendering/command_buffers.rs`.
- **Status**: **Complete**
- **Summary**: Added `visible: bool` field to `Renderable`. Updated `BufferManager::new` to copy visibility state from `RenderObject` to `Renderable`. Modified `record_command_buffers` to check `renderable.visible` before issuing draw commands. Tested by setting initial visibility in `main.rs`.

## Task 4: Implement Keyboard Hotkey System
- **Goal**: Add a configurable hotkey system using a TOML file (`user/hotkeys.toml`) to map keys/modifiers to action strings, gracefully handling undefined hotkeys. Use `Escape` key for closing the window via event bus and proxy.
- **Affected Modules**: `src/gui_framework/interaction/controller.rs`, `src/gui_framework/interaction/hotkeys.rs`, `src/gui_framework/mod.rs`, `src/gui_framework/event_bus.rs`, `src/main.rs`, `build.rs`, `Cargo.toml`.
- **Status**: **Complete**
- **Summary**: Added `toml` and `thiserror` dependencies. Created `hotkeys.rs` for config loading/parsing (`HotkeyConfig`, `HotkeyError`) and key formatting (`format_key_event`). Updated `InteractionController` to load config relative to executable (using path from `build.rs`), track modifier state (`current_modifiers`, handles `ModifiersChanged`), and publish `BusEvent::HotkeyPressed(Some(action_string))` on recognized key presses. Updated `build.rs` (`copy_user_files`) to copy `user/hotkeys.toml` to the target directory. Updated `main.rs` to use `EventLoop::run` and `EventLoopProxy<UserEvent>`, added `HotkeyActionHandler` subscriber that listens for `HotkeyPressed(Some("CloseRequested"))` and sends `UserEvent::Exit` via proxy to trigger clean shutdown. Tested `Escape`, `Ctrl+S`, `Alt+P`.
- **Constraints**: Uses `EventBus` and `EventLoopProxy`. Relies on `build.rs` copying config.

## Task 5: Add Button Functionality
- **Goal**: Extend `gui_framework` to support button behavior on any `RenderObject` via events, using a state flag.
- **Affected Modules**: `src/gui_framework/scene/scene.rs`, `src/gui_framework/interaction/controller.rs`, `src/gui_framework/event_bus.rs`, `src/main.rs` (testing/handling).
- **Status**: Not started
- **Steps**:
    1. Update `RenderObject`: Add `is_button: bool` state flag and `action_id: Option<String>`. Update instantiations in `main.rs`.
    2. Update `InteractionController::handle_event`: On receiving `ObjectPicked`, check if the corresponding `RenderObject.is_button` state flag is true. If yes, publish `BusEvent::ButtonClicked(action_id: Option<String>)`.
    3. Test: Add a button object in `main.rs`, subscribe a handler in `main.rs` to `ButtonClicked`, verify log output on click.
- **Constraints**: Event-driven; relies on state flag; rendering unchanged.

## Task 6: Text Handling - Layout and Rendering Foundation
- **Goal**: Integrate `cosmic-text` for layout/shaping and implement a custom Vulkan bitmap glyph atlas renderer. Display static sample text (English/Chinese placeholder).
- **Affected Modules**: New `src/gui_framework/rendering/text_renderer/mod.rs`, `src/gui_framework/rendering/text_renderer/glyph_atlas.rs`, `src/gui_framework/rendering/buffer_manager.rs` (integration), `src/gui_framework/rendering/render_engine.rs` (integration), `src/gui_framework/scene/scene.rs` (add text objects), `src/main.rs` (testing, initialization), New shaders (`glyph.vert`, `glyph.frag`).
- **Status**: Not started
- **Steps**:
    1. Add dependencies: `cosmic-text`, `fontdb`, `swash`, potentially `rectangle-pack`.
    2. Create `glyph_atlas.rs` module (Vulkan `vk::Image` atlas, rasterization via `swash`, packing, GPU upload, UV tracking).
    3. Create `text_renderer.rs` module (`cosmic_text::FontSystem`, `SwashCache`, API for text buffers, layout triggering, glyph atlas interaction, vertex generation).
    4. Integrate text renderer into `main.rs` initialization and the rendering loop (`render_engine.rs` / `buffer_manager.rs`): resource management, dynamic vertex buffers, pipeline/shaders, atlas binding, drawing.
    5. Modify `RenderObject` or create `TextObject` in `scene.rs`.
    6. Test in `main.rs`: Create text objects, verify rendering.
- **Constraints**: Focus on bitmap rendering. Requires significant Vulkan integration. Defer SDF rendering.

## Task 7: Text Handling - Editing & Interaction
- **Goal**: Integrate `yrs` (`YText`) for collaborative data storage. Implement basic mouse/keyboard editing for text objects.
- **Affected Modules**: `src/gui_framework/interaction/controller.rs`, `src/gui_framework/scene/scene.rs` (manage `YText`), `src/gui_framework/rendering/text_renderer/mod.rs` (update from `YText`, render cursor), `src/main.rs` (testing).
- **Status**: Not started
- **Steps**:
    1. Add dependency: `yrs`.
    2. Replace/Augment text storage in `scene.rs` with `yrs::Text`.
    3. Modify `InteractionController` to track focus, handle keyboard input (generate `YText` ops), handle mouse clicks (calculate position, set cursor).
    4. Modify `TextRenderer` / main loop to observe `YText` changes and trigger re-layout/re-rendering.
    5. Modify `TextRenderer` to draw cursor.
    6. Test in `main.rs`: Create editable text object, edit, move cursor, verify updates.
- **Constraints**: Basic editing first. Defer complex selections, advanced cursor movement. Focus on local editing; P2P sync later.

## Task 8: Implement Radial Pie Context Menu
- **Goal**: Implement a popup pie-style context menu triggered by a hotkey (e.g., Ctrl+Space), with dynamic options.
- **Affected Modules**: New `src/gui_framework/ui/pie_menu.rs`, `src/gui_framework/interaction/controller.rs`, `src/gui_framework/scene/scene.rs` (or UI manager), `src/gui_framework/rendering/text_renderer/mod.rs` (labels), New shaders (`pie_menu.vert`, `pie_menu.frag`), `src/gui_framework/event_bus.rs`, `src/gui_framework/mod.rs`, `src/main.rs` (handling).
- **Status**: Not started
- **Steps**:
    1. Create `pie_menu.rs` (`PieMenu`, `PieOption`, rendering logic using shapes/text).
    2. Update `Scene` or UI manager: Add `active_menu: Option<PieMenu>`.
    3. Update `InteractionController`: Handle `BusEvent::HotkeyPressed("show_pie_menu")`. Publish `BusEvent::ShowPieMenu(position, context)`.
    4. Update `Scene`/UI manager: Subscribe to `ShowPieMenu`, create `PieMenu`, set `active_menu`.
    5. Update `InteractionController`: Handle clicks when `active_menu` is Some. Determine selection, publish `BusEvent::MenuOptionSelected(action_id)`. Clear `active_menu`.
    6. Update `Renderer`/`BufferManager`/`TextRenderer`: Draw `active_menu`.
    7. Test: Trigger menu, verify display, select option, verify event.
- **Constraints**: Event-driven; uses Task 6 text rendering. Requires hotkey system (Task 4).

## Task 9: Implement Optional Divider System in Framework
- **Goal**: Implement an optional divider system *within* the `gui_framework` for managing resizable layout regions. Provide an API for end-users to enable and configure it.
- **Affected Modules**: New `src/gui_framework/ui/dividers.rs`, `src/gui_framework/scene/scene.rs` (or UI manager), `src/gui_framework/interaction/controller.rs`, `src/gui_framework/mod.rs`, `src/main.rs` (testing/handling).
- **Status**: Not started
- **Steps**:
    1.  Define `Divider`, `DividerSystem` in `dividers.rs`.
    2.  Integrate `DividerSystem` into `Scene` or UI manager. Provide API (`enable_dividers`, `add_divider`).
    3.  Use `Scene::add_object` for visual representation (draggable rectangles).
    4.  Handle `ObjectMoved` for dividers. Constrain movement, calculate region changes, publish `RegionResized` event.
    5.  Expose configuration and `RegionResized` handling via public API.
    6.  Test in `main.rs`: Enable system, add divider, drag, log `RegionResized`.
- **Constraints**: Builds on framework primitives. Needs clear API separation. Layout logic calculates dimensions; rendering content within regions is application's responsibility.

## Task 10: Enhance Prompt Tool - Code Signature Stripping
- **Goal**: Add a new option to `utilities/llm_prompt_tool.sh` that processes Rust files, stripping out function bodies while retaining signatures, `impl` blocks, structs, comments, and other surrounding code.
- **Affected Modules**: `utilities/llm_prompt_tool.sh`.
- **Status**: Not started
- **Steps**:
    1.  Add new menu option (e.g., "5) Get Code Signatures").
    2.  Implement `.rs` file discovery.
    3.  Implement function body stripping logic (identify `fn`, replace `{...}` with placeholder, preserve surrounding code).
    4.  Integrate into file processing loop.
    5.  Comment limitations (macros, formatting).
    6.  Test on project codebase.
- **Constraints**: Bash implementation (`sed`, `awk`, etc.). Aim for common cases.

## Deferred Features (For Future Consideration)
- Text: Markdown rendering, syntax highlighting (`tree-sitter`), code folding.
- Text: Advanced editing (complex selections, IME support).
- Text: SDF-based rendering (`glyphon` or custom).
- Rendering: Depth-based re-sorting of renderables.
- Collaboration: Full P2P synchronization of `yrs` CRDT operations.
- UI: Context switching, undo/redo system.
- General: Performance optimizations (e.g., instance buffer resizing).