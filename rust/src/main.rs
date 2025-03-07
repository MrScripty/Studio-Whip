use winit::event_loop::{EventLoop, ControlFlow};
use rusty_whip::application::App;

#[allow(deprecated)]
fn main() {
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App::new();
    event_loop.run_app(&mut app).unwrap();
}