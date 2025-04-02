use winit::event::{Event, WindowEvent, ElementState, MouseButton};
use winit::window::Window;
use crate::{Scene, Renderer};
use crate::gui_framework::event_bus::{EventBus, BusEvent as BusEvent};
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
}

impl InteractionController {
    pub fn new() -> Self {
        Self {
            mouse_state: MouseState {
                is_dragging: false,
                last_position: None,
                dragged_object: None,
            },
            context: CursorContext::Canvas,
        }
    }

    pub fn handle_event(&mut self, event: &Event<()>, scene: Option<&Scene>, _renderer: Option<&mut Renderer>, window: &Window, event_bus: &Arc<EventBus>) {
        if let Event::WindowEvent { event, .. } = event {
            match event {
                WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Left, .. } => {
                    if matches!(self.context, CursorContext::Canvas) {
                        self.mouse_state.is_dragging = true;
                        let pos = self.mouse_state.last_position.unwrap_or([0.0, 0.0]);
                        if let Some(scene_ref) = scene {
                            if let Some(target) = scene_ref.pick_object_at(pos[0], pos[1]) {
                                self.mouse_state.dragged_object = Some(target);
                                //println!("Clicked object: {:?}", target);
                                // Use the new name BusEvent directly
                                event_bus.publish(BusEvent::ObjectPicked(target.0, target.1));
                            }
                        } else {
                             // This case should ideally not happen for MouseInput if called correctly from window_handler
                             println!("[Controller] Warning: Scene reference not provided during MouseInput press.");
                        }
                        //println!("Dragging started at {:?}", pos);
                    }
                }
                WindowEvent::CursorMoved { position, .. } => {
                    let pos = [position.x as f32, position.y as f32];
                    if matches!(self.context, CursorContext::Canvas) && self.mouse_state.is_dragging {
                        if let Some(last_pos) = self.mouse_state.last_position {
                            let delta = [pos[0] - last_pos[0], last_pos[1] - pos[1]];
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
                        //println!("Dragging stopped at {:?}", self.mouse_state.last_position);
                        // Optionally publish an event like DragStopped if needed later
                    }
                }
                _ => (),
            }
        }
    }
}