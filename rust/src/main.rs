use winit::event_loop::{EventLoop, ControlFlow};
use winit::dpi::PhysicalSize;
use rusty_whip::{Platform, Scene, RenderObject, Vertex, window_management::PlatformHandler};

fn main() {
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);
    let platform = Platform::new();
    let mut scene = Scene::new();

    // Background RenderObject (NDC quad, scales to window)
    scene.add_object(RenderObject {
        vertices: vec![
            Vertex { position: [-1.0, -1.0] }, // Bottom-left
            Vertex { position: [-1.0, 1.0] },  // Top-left
            Vertex { position: [1.0, 1.0] },   // Top-right
            Vertex { position: [1.0, -1.0] },  // Bottom-right
        ],
        vertex_shader_filename: "background.vert.spv".to_string(),
        fragment_shader_filename: "background.frag.spv".to_string(),
        depth: 0.0,                        // Background at back
        on_window_resize_scale: true,      // Scales to fill window
        on_window_resize_move: false,      // Doesn’t move
    });

    // Triangle RenderObject (moves proportionally)
    scene.add_object(RenderObject {
        vertices: vec![
            Vertex { position: [-0.5, -0.5] },
            Vertex { position: [0.0, 0.5] },
            Vertex { position: [0.5, -0.25] },
        ],
        vertex_shader_filename: "triangle.vert.spv".to_string(),
        fragment_shader_filename: "triangle.frag.spv".to_string(),
        depth: 2.0,                        // Middle layer
        on_window_resize_scale: false,     // Doesn’t scale
        on_window_resize_move: true,       // Moves with resize
    });

    // Square RenderObject (moves proportionally)
    scene.add_object(RenderObject {
        vertices: vec![
            Vertex { position: [-0.25, -0.25] },
            Vertex { position: [-0.25, 0.25] },
            Vertex { position: [0.25, 0.25] },
            Vertex { position: [0.25, -0.25] },
        ],
        vertex_shader_filename: "square.vert.spv".to_string(),
        fragment_shader_filename: "square.frag.spv".to_string(),
        depth: 1.0,                        // Front layer
        on_window_resize_scale: false,     // Doesn’t scale
        on_window_resize_move: true,       // Moves with resize
    });

    let mut handler = PlatformHandler::new(platform, scene);
    event_loop.run_app(&mut handler).unwrap();
}