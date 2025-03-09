use winit::event_loop::{EventLoop, ControlFlow};
use rusty_whip::{Platform, Scene, RenderObject, Vertex, window_management::PlatformHandler};

fn main() {
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);
    let platform = Platform::new();
    let mut scene = Scene::new();

    // Triangle RenderObject
    scene.add_object(RenderObject {
        vertices: vec![
            Vertex { position: [-0.5, -0.5] },
            Vertex { position: [0.0, 0.5] },
            Vertex { position: [0.5, -0.25] },
        ],
        vertex_shader_filename: "triangle.vert.spv".to_string(),
        fragment_shader_filename: "triangle.frag.spv".to_string(),
    });

    // Square RenderObject
    scene.add_object(RenderObject {
        vertices: vec![
            Vertex { position: [-0.25, -0.25] },
            Vertex { position: [-0.25, 0.25] },
            Vertex { position: [0.25, 0.25] },
            Vertex { position: [0.25, -0.25] },
        ],
        vertex_shader_filename: "square.vert.spv".to_string(),
        fragment_shader_filename: "square.frag.spv".to_string(),
    });

    let mut handler = PlatformHandler::new(platform, scene);
    event_loop.run_app(&mut handler).unwrap();
}