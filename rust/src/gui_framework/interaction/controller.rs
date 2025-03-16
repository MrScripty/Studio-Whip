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
}