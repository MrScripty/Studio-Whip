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
        is_draggable: false,
        instances: Vec::new(),
    });

    let triangle_id = scene.add_object(RenderObject {
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
        offset: [0.0, 0.0],
        is_draggable: true,
        instances: Vec::new(),
    });

    let square_id = scene.add_object(RenderObject {
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
        is_draggable: true,
        instances: Vec::new(),
    });

    scene.add_instance(triangle_id, [50.0, 50.0]);
    scene.add_instance(triangle_id, [-50.0, -50.0]);
    scene.add_instance(square_id, [100.0, 0.0]);
    
    let _small_square_id = scene.add_object(RenderObject {
        vertices: vec![
            Vertex { position: [400.0, 200.0] },
            Vertex { position: [400.0, 230.0] },
            Vertex { position: [430.0, 230.0] },
            Vertex { position: [430.0, 200.0] },
        ],
        vertex_shader_filename: "square.vert.spv".to_string(),
        fragment_shader_filename: "square.frag.spv".to_string(),
        depth: 3.0,
        on_window_resize_scale: false,
        on_window_resize_move: true,
        offset: [0.0, 0.0],
        is_draggable: true,
        instances: Vec::new(),
    });
    let _vertical_rect_id = scene.add_object(RenderObject {
        vertices: vec![
            Vertex { position: [450.0, 190.0] },
            Vertex { position: [450.0, 240.0] },
            Vertex { position: [470.0, 240.0] },
            Vertex { position: [470.0, 190.0] },
        ],
        vertex_shader_filename: "square.vert.spv".to_string(),
        fragment_shader_filename: "square.frag.spv".to_string(),
        depth: 4.0,
        on_window_resize_scale: false,
        on_window_resize_move: true,
        offset: [0.0, 0.0],
        is_draggable: true,
        instances: Vec::new(),
    });

    // Use Scene-integrated GroupManager
    scene.groups().add_group("test_group").unwrap();
    
    // Add objects to test_group
    {
        let mut test_group = scene.groups().group("test_group").unwrap();
        test_group.add_object(0); // Background
        test_group.add_object(1); // Triangle
        test_group.add_object(999); // Invalid ID, not checked yet
    }
    
    // Remove an object
    {
        let mut test_group = scene.groups().group("test_group").unwrap();
        test_group.remove_object(0); // Remove background
    }
    
    // Create another group and add overlapping objects
    scene.groups().add_group("another_group").unwrap();
    {
        let mut another_group = scene.groups().group("another_group").unwrap();
        another_group.add_object(1); // Triangle (in both groups)
        another_group.add_object(3); // Small square
    }
    
    // List objects
    {
        let test_group = scene.groups().group("test_group").unwrap();
        let _objects = test_group.list_objects(); // [1]
    }
    
    // Delete test_group
    scene.groups().delete_group("test_group").unwrap(); // Only "another_group" remains with [1, 3]

    let mut handler = VulkanContextHandler::new(vulkan_context, scene);
    event_loop.run_app(&mut handler).unwrap();
}