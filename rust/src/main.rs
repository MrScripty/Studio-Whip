use winit::event_loop::{EventLoop, ControlFlow};
use rusty_whip::gui_framework::{VulkanContext, Scene, RenderObject, VulkanContextHandler};
use rusty_whip::Vertex;

fn main() {
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);
    let vulkan_context = VulkanContext::new();
    let mut scene = Scene::new();

    let width = 600.0;
    let height = 300.0;

    scene.add_object(RenderObject {
        vertices: vec![
            Vertex { position: [0.0, 0.0] },
            Vertex { position: [0.0, height] },
            Vertex { position: [width, height] },
            Vertex { position: [width, 0.0] },
        ],
        vertex_shader_filename: "background.vert.spv".to_string(),
        fragment_shader_filename: "background.frag.spv".to_string(),
        depth: 0.0,
        on_window_resize_scale: true,
        on_window_resize_move: false,
        offset: [0.0, 0.0],
    });

    scene.add_object(RenderObject {
        vertices: vec![
            Vertex { position: [275.0, 125.0] },
            Vertex { position: [300.0, 175.0] },
            Vertex { position: [325.0, 125.0] },
        ],
        vertex_shader_filename: "triangle.vert.spv".to_string(),
        fragment_shader_filename: "triangle.frag.spv".to_string(),
        depth: 1.0,
        on_window_resize_scale: false,
        on_window_resize_move: true,
        offset: [100.0, 100.0], // Increased for visibility
    });

    scene.add_object(RenderObject {
        vertices: vec![
            Vertex { position: [100.0, 50.0] },
            Vertex { position: [100.0, 100.0] },
            Vertex { position: [150.0, 100.0] },
            Vertex { position: [150.0, 50.0] },
        ],
        vertex_shader_filename: "square.vert.spv".to_string(),
        fragment_shader_filename: "square.frag.spv".to_string(),
        depth: 2.0,
        on_window_resize_scale: false,
        on_window_resize_move: true,
        offset: [0.0, 0.0],
    });

    let mut handler = VulkanContextHandler::new(vulkan_context, scene);
    event_loop.run_app(&mut handler).unwrap();
}