use std::collections::HashMap;
use std::any::{Any, TypeId};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub enum BusEvent {
    ObjectMoved(usize, [f32; 2], Option<usize>), // object_id, delta, instance_id
    InstanceAdded(usize, usize, [f32; 2]),      // object_id, instance_id, offset
    ObjectPicked(usize, Option<usize>),         // object_id, instance_id
    RedrawRequested,
    // Add other events as needed
}

pub trait EventHandler: Send + Sync {
    fn handle(&mut self, event: &BusEvent);
    fn as_any(&self) -> &dyn Any; // Needed for downcasting if specific handler access is required later
}

// Using Arc<Mutex<>> for handlers to allow shared mutable access across threads if needed later,
// although currently single-threaded. Simpler Box<dyn EventHandler> could be used for strictly single-threaded.
type HandlerVec = Vec<Arc<Mutex<dyn EventHandler>>>;

#[derive(Clone, Default)]
pub struct EventBus {
    // Using TypeId of the *event type* could be an alternative, but TypeId of the *handler*
    // allows multiple handlers of the same type to subscribe independently if needed.
    // For now, let's keep it simple: all handlers receive all events.
    // A more complex system might filter by event type.
    subscribers: Arc<Mutex<Vec<Arc<Mutex<dyn EventHandler>>>>>,
}

impl EventBus {
    pub fn new() -> Self {
        Self {
            subscribers: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn subscribe<H: EventHandler + 'static>(&self, handler: H) {
        let mut subs = self.subscribers.lock().unwrap();
        subs.push(Arc::new(Mutex::new(handler)));
    }

    // Consider making publish take `&self` if Event is Clone, simplifies call sites.
    pub fn publish(&self, event: BusEvent) {
        let subs = self.subscribers.lock().unwrap();
        // Clone the list of handlers to avoid holding the lock while calling handle()
        let handlers_to_notify = subs.clone();
        drop(subs); // Release the lock

        for handler_arc in handlers_to_notify {
            // Lock each handler individually
            let mut handler = handler_arc.lock().unwrap();
            handler.handle(&event);
        }
    }
}

// Example usage (will be removed/replaced in actual integration):
/*
struct MyHandler { id: u32 }
impl EventHandler for MyHandler {
    fn handle(&mut self, event: &Event) {
        println!("Handler {} received event: {:?}", self.id, event);
    }
    fn as_any(&self) -> &dyn Any { self }
}

fn test_event_bus() {
    let bus = EventBus::new();
    let handler1 = MyHandler { id: 1 };
    let handler2 = MyHandler { id: 2 };

    bus.subscribe(handler1);
    bus.subscribe(handler2);

    bus.publish(Event::RedrawRequested);
    bus.publish(Event::ObjectMoved(0, [10.0, 5.0], None));
}
*/