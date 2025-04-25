# How to Use the `rusty_whip` GUI Framework Plugins

This guide explains how to integrate and use the modular GUI framework plugins built on Bevy's core features (ECS, Events, Input, etc.) and a custom Vulkan renderer backend.

## Overview

The framework provides foundational elements for building 2D applications within Bevy, specifically avoiding Bevy's built-in rendering stack (`bevy_render`). It offers:

*   Custom Vulkan rendering driven by Bevy ECS components.
*   Input handling (mouse clicks, drags) mapped to Bevy events.
*   A configurable hotkey system.
*   Default systems for common behaviors (movement, basic action bindings).

## Prerequisites

1.  **Bevy Core Plugins:** Your Bevy `App` needs the core non-rendering plugins. A minimal setup typically includes:
    *   `LogPlugin`
    *   `TimePlugin`
    *   `TransformPlugin`
    *   `InputPlugin`
    *   `WindowPlugin`
    *   `AccessibilityPlugin`
    *   `WinitPlugin`
2.  **Vulkan SDK:** Ensure the Vulkan SDK (1.3+) is installed and configured correctly (including `glslc` in your PATH for shader compilation via `build.rs`).
3.  **Custom Vulkan Backend:** The framework relies on its internal Vulkan setup.

## Adding Framework Plugins

The framework is split into several plugins. Add them to your `App` in `main.rs`:

```rust
use rusty_whip::gui_framework::plugins::{
    core::GuiFrameworkCorePlugin,
    interaction::GuiFrameworkInteractionPlugin,
    movement::GuiFrameworkDefaultMovementPlugin, // Optional
    bindings::GuiFrameworkDefaultBindingsPlugin, // Optional
};
use rusty_whip::VulkanContextResource; // For initial resource insertion
use rusty_whip::gui_framework::VulkanContext; // For initial resource insertion
use std::sync::{Arc, Mutex};

fn main() {
    // 1. Initialize Vulkan Context Resource
    let vulkan_context = Arc::new(Mutex::new(VulkanContext::new()));

    App::new()
        // Add necessary Bevy core plugins...
        .add_plugins((
            // Minimal Bevy Plugins...
            // LogPlugin, TimePlugin, TransformPlugin, InputPlugin,
            // WindowPlugin, AccessibilityPlugin, WinitPlugin,
        ))

        // 2. Insert Vulkan Context Resource BEFORE framework plugins
        .insert_resource(VulkanContextResource(vulkan_context))

        // 3. Add Framework Plugins
        .add_plugins((
            GuiFrameworkCorePlugin,          // Essential for rendering & core setup
            GuiFrameworkInteractionPlugin,   // Essential for input & hotkeys
            GuiFrameworkDefaultMovementPlugin, // Optional: Handles EntityDragged -> Transform
            GuiFrameworkDefaultBindingsPlugin, // Optional: Handles "CloseRequested" hotkey
        ))

        // Add your application's systems, resources, etc.
        // .add_systems(Startup, setup_my_app_scene)
        // .add_systems(Update, my_app_logic)

        .run();
}
```

*   `GuiFrameworkCorePlugin`: **Required.** Sets up Vulkan, the custom renderer, handles resizing, runs the rendering system, and manages cleanup. Registers core types (`ShapeData`, `Visibility`, `Vertex`).
*   `GuiFrameworkInteractionPlugin`: **Required.** Sets up input handling (mouse, keyboard hotkeys), loads `hotkeys.toml`, and sends interaction events. Registers interaction types/events.
*   `GuiFrameworkDefaultMovementPlugin`: **Optional.** Provides a basic `movement_system` that updates `Transform` components based on `EntityDragged` events. Disable this plugin if you want custom drag handling.
*   `GuiFrameworkDefaultBindingsPlugin`: **Optional.** Provides a basic `app_control_system` that sends an `AppExit` event when a `HotkeyActionTriggered` event with the action `"CloseRequested"` is received. Disable this plugin for custom hotkey action handling.

## Creating UI Elements

UI elements are standard Bevy entities with specific components:

*   **`Transform`:** (From `bevy_transform`) Defines position (X, Y) and depth (Z). Higher Z values are rendered on top.
*   **`ShapeData`:** (From `rusty_whip::gui_framework::components`) Defines the visual geometry.
    *   `vertices: Arc<Vec<Vertex>>`: The vertex data defining the shape (using `rusty_whip::Vertex`). Use `Arc` for potential sharing. Vertices are defined in local coordinates relative to the entity's `Transform`.
    *   `vertex_shader_path: String`: Path to the compiled vertex shader (`.vert.spv`).
    *   `fragment_shader_path: String`: Path to the compiled fragment shader (`.frag.spv`).
*   **`Visibility`:** (From `rusty_whip::gui_framework::components`) Custom visibility component (`Visibility(bool)`). Controls whether the entity is rendered by the custom Vulkan renderer. Defaults to `true`.
*   **`Interaction`:** (From `rusty_whip::gui_framework::components`) Defines interactivity.
    *   `clickable: bool`: If `true`, the `interaction_system` can send `EntityClicked` events for this entity.
    *   `draggable: bool`: If `true`, the `interaction_system` can send `EntityDragged` events for this entity.

**Example:**

```rust
use bevy_ecs::prelude::*;
use bevy_transform::prelude::Transform;
use rusty_whip::gui_framework::components::{ShapeData, Visibility, Interaction};
use rusty_whip::Vertex;
use std::sync::Arc;

fn setup_my_app_scene(mut commands: Commands) {
    commands.spawn((
        ShapeData {
            vertices: Arc::new(vec![
                Vertex { position: [-25.0, -25.0] },
                Vertex { position: [ 0.0,  25.0] },
                Vertex { position: [ 25.0, -25.0] },
            ]),
            vertex_shader_path: "triangle.vert.spv".to_string(),
            fragment_shader_path: "triangle.frag.spv".to_string(),
        },
        Transform::from_xyz(100.0, 150.0, 1.0), // Position (100, 150), Depth 1
        Visibility(true),
        Interaction { clickable: true, draggable: true },
    ));
}
```

## Reacting to Events

Your application systems can react to events sent by the `GuiFrameworkInteractionPlugin`:

*   **`EntityClicked { entity: Entity }`:** Sent when a clickable entity is clicked.
*   **`EntityDragged { entity: Entity, delta: Vec2 }`:** Sent when a draggable entity is being dragged. `delta` represents the change in cursor position since the last frame.
*   **`HotkeyActionTriggered { action: String }`:** Sent when a key combination matching an entry in `hotkeys.toml` is pressed. The `action` field contains the string defined in the config file.

**Example:**

```rust
use bevy_ecs::prelude::*;
use rusty_whip::gui_framework::events::{EntityClicked, HotkeyActionTriggered};
use bevy_log::info;

fn my_click_handler(mut ev_clicked: EventReader<EntityClicked>) {
    for ev in ev_clicked.read() {
        info!("Entity {:?} was clicked!", ev.entity);
        // Add logic here, e.g., change component state
    }
}

fn my_hotkey_handler(mut ev_hotkey: EventReader<HotkeyActionTriggered>) {
    for ev in ev_hotkey.read() {
        match ev.action.as_str() {
            "MyCustomAction" => info!("My custom action triggered!"),
            "AnotherAction" => info!("Another action!"),
            // Note: "CloseRequested" is handled by GuiFrameworkDefaultBindingsPlugin if enabled
            _ => {}
        }
    }
}

// Add these systems to your App's Update schedule
// .add_systems(Update, (my_click_handler, my_hotkey_handler))
```

## Configuration

*   **Hotkeys:** Configure key bindings in `user/hotkeys.toml` located next to your executable. The format is ` "KeyCombo" = "ActionString" `. Example:
    ```toml
    "Ctrl+S" = "SaveFile"
    "Escape" = "CloseRequested" # Handled by default bindings plugin
    "Space" = "MyCustomAction"
    ```
    The `build.rs` script should copy this file to the target directory.

## Coordinate System

*   The framework uses a **Y-up** world coordinate system, with the origin (0,0) typically at the **bottom-left** of the window.
*   The custom Vulkan renderer handles the necessary projection matrix adjustments (including a Y-flip) to render correctly.
*   Input coordinates from Bevy (usually Y-down from top-left) are adjusted by the `interaction_system` before hit-testing against world coordinates.
*   When applying movement from `EntityDragged` events, remember that a positive `delta.y` from Bevy's input means the cursor moved *down*, so you typically *subtract* `delta.y` when updating a `Transform`'s Y position in the Y-up world space (as done by the default `movement_system`).
