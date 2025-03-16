use winit::event::{Event, WindowEvent, ElementState, MouseButton};
use winit::window::Window;
use crate::Scene;
use crate::Renderer;

pub struct MouseState {
    pub is_dragging: bool,
    pub last_position: Option<[f32; 2]>,
    pub dragged_object: Option<usize>,
}

pub enum CursorContext {
    Canvas, // Draggable region
    Other,  // Non-draggable region
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
            context: CursorContext::Canvas, // Default to Canvas for simplicity
        }
    }

    pub fn handle_event(&mut self, event: &Event<()>, _scene: Option<&Scene>, _renderer: Option<&mut Renderer>, _window: &Window) {
        //println!("Event received: {:?}", event); // Debug log
        if let Event::WindowEvent { event, .. } = event {
            match event {
                WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Left, .. } => {
                    if matches!(self.context, CursorContext::Canvas) {
                        self.mouse_state.is_dragging = true;
                        let pos = self.mouse_state.last_position.unwrap_or([0.0, 0.0]);
                        println!("Dragging started at {:?}", pos);
                    } else {
                        println!("Press ignored (not in Canvas context)");
                    }
                }
                WindowEvent::CursorMoved { position, .. } => {
                    let pos = [position.x as f32, position.y as f32];
                    if matches!(self.context, CursorContext::Canvas) && self.mouse_state.is_dragging {
                        if let Some(last_pos) = self.mouse_state.last_position {
                            let delta = [pos[0] - last_pos[0], pos[1] - last_pos[1]];
                            //println!("Dragging delta: {:?}", delta);
                        } else {
                            //println!("No last position for delta calculation");
                        }
                        self.mouse_state.last_position = Some(pos);
                    } else {
                        self.mouse_state.last_position = Some(pos);
                        //println!("Position updated to {:?} (not dragging or not in Canvas)", pos);
                    }
                }
                WindowEvent::MouseInput { state: ElementState::Released, button: MouseButton::Left, .. } => {
                    if matches!(self.context, CursorContext::Canvas) && self.mouse_state.is_dragging {
                        self.mouse_state.is_dragging = false;
                        //println!("Dragging stopped at {:?}", self.mouse_state.last_position);
                    } else {
                        println!("Release ignored (not dragging or not in Canvas)");
                    }
                }
                _ => (),
            }
        }
    }
}