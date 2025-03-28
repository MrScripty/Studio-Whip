use winit::event::{Event, WindowEvent, ElementState, MouseButton};
use winit::window::Window;
use crate::{Scene, Renderer};

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

    pub fn handle_event(&mut self, event: &Event<()>, scene: Option<&mut Scene>, renderer: Option<&mut Renderer>, window: &Window) {
        if let Event::WindowEvent { event, .. } = event {
            match event {
                WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Left, .. } => {
                    if matches!(self.context, CursorContext::Canvas) {
                        self.mouse_state.is_dragging = true;
                        let pos = self.mouse_state.last_position.unwrap_or([0.0, 0.0]);
                        if let Some(scene) = scene {
                            if let Some(target) = scene.pick_object_at(pos[0], pos[1]) {
                                self.mouse_state.dragged_object = Some(target);
                                println!("Clicked object: {:?}", target);
                            }
                        }
                        println!("Dragging started at {:?}", pos);
                    }
                }
                WindowEvent::CursorMoved { position, .. } => {
                    let pos = [position.x as f32, position.y as f32];
                    if matches!(self.context, CursorContext::Canvas) && self.mouse_state.is_dragging {
                        if let Some(last_pos) = self.mouse_state.last_position {
                            let delta = [pos[0] - last_pos[0], last_pos[1] - pos[1]]; // Invert Y-delta
                            println!("Dragging delta: {:?}", delta);
                            if let Some(scene) = scene {
                                if let Some((index, instance_id)) = self.mouse_state.dragged_object {
                                    scene.translate_object(index, delta[0], delta[1], instance_id);
                                    window.request_redraw();
                                }
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
                        println!("Dragging stopped at {:?}", self.mouse_state.last_position);
                    }
                }
                _ => (),
            }
        }
    }
}