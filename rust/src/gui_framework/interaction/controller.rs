use winit::event::{Event, WindowEvent, ElementState, MouseButton, KeyEvent};
use winit::keyboard::ModifiersState;
use winit::window::Window;
use crate::{Scene, Renderer};
use crate::gui_framework::event_bus::{EventBus, BusEvent};
use crate::gui_framework::interaction::hotkeys::{HotkeyConfig, HotkeyError, format_key_event};
use std::env;
use std::path::PathBuf;
use std::sync::Arc;

pub struct MouseState {
    pub is_dragging: bool,
    pub last_position: Option<[f32; 2]>,
    pub dragged_object: Option<(usize, Option<usize>)>,
}

pub enum CursorContext {
    Canvas,
    Other,
}

pub struct InteractionController {
    pub mouse_state: MouseState,
    pub context: CursorContext,
    hotkey_config: HotkeyConfig,
    current_modifiers: ModifiersState,
}

impl InteractionController {
    pub fn new() -> Self {
        let mut hotkey_path: Option<PathBuf> = None;
        let mut config_load_error: Option<String> = None;

        // --- Determine Config Path Relative to Executable ---
        match env::current_exe() { // Get path to the executable itself
            Ok(mut exe_path) => {
                // Go up one level from the executable file to its directory (e.g., target/debug/)
                if exe_path.pop() {
                    let path = exe_path.join("user").join("hotkeys.toml"); // Look in user/hotkeys.toml next to exe
                    println!("[Controller] Looking for hotkeys file at: {:?}", path);
                    hotkey_path = Some(path);
                } else {
                        config_load_error = Some("Could not get executable directory.".to_string());
                }
            }
            Err(e) => {
                config_load_error = Some(format!("Failed to get current executable path: {}", e));
            }
        }
        // --- End Determine Config Path ---


        // --- Load Config (remains the same logic, uses the new path) ---
        let config = match hotkey_path {
            Some(path) => {
                // Check if the file actually exists before trying to load
                if path.exists() {
                    HotkeyConfig::load_config(&path).unwrap_or_else(|e| {
                        // Handle read/parse errors
                        match e {
                            HotkeyError::ReadError(io_err) => {
                                eprintln!("[Controller] Error: Failed to read hotkey file '{:?}': {}", path, io_err);
                            }
                            HotkeyError::ParseError(toml_err) => {
                                eprintln!("[Controller] Error: Failed to parse hotkey file '{:?}': {}", path, toml_err);
                            }
                            HotkeyError::FileNotFound(_) => {
                                // Should not happen due to path.exists() check, but handle defensively
                                eprintln!("[Controller] Error: Hotkey file disappeared between check and load: {:?}", path);
                            }
                        }
                        HotkeyConfig::default()
                    })
                } else {
                    // File doesn't exist at the expected location (build script might have failed)
                    println!("[Controller] Info: Hotkey file '{:?}' not found next to executable. Using default empty configuration.", path);
                    HotkeyConfig::default()
                }
            }
            None => {
                eprintln!("[Controller] Error: {}", config_load_error.unwrap_or("Hotkey path could not be determined.".to_string()));
                HotkeyConfig::default()
            }
        };

        Self {
            mouse_state: MouseState {
                is_dragging: false,
                last_position: None,
                dragged_object: None,
            },
            context: CursorContext::Canvas,
            hotkey_config: config,
            current_modifiers: ModifiersState::default(),
        }
    }

    // Handles winit events and publishes corresponding BusEvents
    pub fn handle_event(
        &mut self,
        event: &Event<()>,
        scene: Option<&Scene>, // Provided only when needed (e.g., mouse press for picking)
        _renderer: Option<&mut Renderer>, // Keep in signature if might be needed later
        window: &Window,
        event_bus: &Arc<EventBus>
    ) {
        if let Event::WindowEvent { event, .. } = event {
            match event {
                WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Left, .. } => {
                    if matches!(self.context, CursorContext::Canvas) {
                        self.mouse_state.is_dragging = true;
                        let pos = self.mouse_state.last_position.unwrap_or([0.0, 0.0]);
                        if let Some(scene_ref) = scene { // Scene ref is expected here
                            if let Some(target) = scene_ref.pick_object_at(pos[0], pos[1]) {
                                self.mouse_state.dragged_object = Some(target);
                                event_bus.publish(BusEvent::ObjectPicked(target.0, target.1));
                            }
                        } else {
                             // This case indicates an issue in window_handler not providing scene on press
                             println!("[Controller] Warning: Scene reference not provided during MouseInput press.");
                        }
                    }
                }
                WindowEvent::CursorMoved { position, .. } => {
                    let pos = [position.x as f32, position.y as f32];
                    if matches!(self.context, CursorContext::Canvas) && self.mouse_state.is_dragging {
                        if let Some(last_pos) = self.mouse_state.last_position {
                            let delta = [pos[0] - last_pos[0], last_pos[1] - pos[1]]; // Y-axis handled elsewhere or adjust here
                            if let Some((index, instance_id)) = self.mouse_state.dragged_object {
                                event_bus.publish(BusEvent::ObjectMoved(index, delta, instance_id));
                                window.request_redraw();
                                }
                        }
                        self.mouse_state.last_position = Some(pos);
                    } else {
                        self.mouse_state.last_position = Some(pos);
                    }
                }
                WindowEvent::MouseInput { state: ElementState::Released, button: MouseButton::Left, .. } => {
                    if matches!(self.context, CursorContext::Canvas) && self.mouse_state.is_dragging {
                        self.mouse_state.is_dragging = false;
                        self.mouse_state.dragged_object = None;
                    }
                }

                WindowEvent::ModifiersChanged(modifiers) => {
                    self.current_modifiers = modifiers.state(); // <-- CORRECT WAY
                    println!("[Controller] Modifiers changed: {:?}", self.current_modifiers);
                }

                WindowEvent::KeyboardInput {
                    event: KeyEvent { physical_key, state: ElementState::Pressed, .. },
                    ..
                } => {
                    // --- Use tracked modifiers and config ---
                    let modifiers = self.current_modifiers; // Use the tracked state
                    if let Some(key_combo_str) = format_key_event(modifiers, *physical_key) {
                        println!("[Controller] Key combo detected: {}", key_combo_str); // Debug log
                        if let Some(action) = self.hotkey_config.get_action(&key_combo_str) {
                            println!("[Controller] Action found: '{}'. Publishing event.", action); // Debug log
                            event_bus.publish(BusEvent::HotkeyPressed(Some(action.clone())));
                        } else {
                            println!("[Controller] No action defined for {}", key_combo_str); // Debug log
                            // Decide if you want to publish None for unmapped keys:
                            // event_bus.publish(BusEvent::HotkeyPressed(None));
                        }
                    }

                    // Placeholder for configurable hotkey check using self.hotkey_config
                    // Requires modifier state tracking via ModifiersChanged
                    /*
                    let modifiers = self.current_modifiers;
                    if let Some(key_combo_str) = format_key_event(modifiers, *physical_key) {
                        if let Some(action) = self.hotkey_config.get_action(&key_combo_str) {
                            event_bus.publish(BusEvent::HotkeyPressed(Some(action.clone())));
                        } else {
                            // event_bus.publish(BusEvent::HotkeyPressed(None)); // Optional: Publish for unmapped keys
                        }
                    }
                    */
                }
                // Placeholder for modifier tracking
                // WindowEvent::ModifiersChanged(new_state) => {
                //     self.current_modifiers = *new_state;
                // }
                _ => (), // Ignore other window events in this controller
            }
        }
        // Ignore non-window events like DeviceEvent, UserEvent, etc.
    }
}