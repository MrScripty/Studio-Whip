use std::any::Any;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub enum BusEvent {
    ObjectMoved(usize, [f32; 2], Option<usize>),
    InstanceAdded(usize, usize, [f32; 2]),
    ObjectPicked(usize, Option<usize>),
    ObjectClicked(usize, Option<usize>),
    RedrawRequested,
    HotkeyPressed(Option<String>),
}

pub trait EventHandler: Send + Sync {
    fn handle(&mut self, event: &BusEvent);
    fn as_any(&self) -> &dyn Any;
}

type HandlerVec = Vec<Arc<Mutex<dyn EventHandler>>>;

#[derive(Clone, Default)]
pub struct EventBus {
    subscribers: Arc<Mutex<HandlerVec>>,
}

impl EventBus {
    pub fn new() -> Self {
        Self {
            subscribers: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn subscribe_handler<H: EventHandler + 'static>(&self, handler: H) {
        let handler_arc_mutex: Arc<Mutex<dyn EventHandler>> = Arc::new(Mutex::new(handler));
        let mut subs = self.subscribers.lock().unwrap();
        subs.push(handler_arc_mutex);
    }

    pub fn subscribe_arc<T>(&self, handler_arc: Arc<Mutex<T>>)
    where
        T: EventHandler + 'static,
    {
        let handler_dyn_arc: Arc<Mutex<dyn EventHandler>> = handler_arc;
        let mut subs = self.subscribers.lock().unwrap();
        subs.push(handler_dyn_arc);
    }

    pub fn publish(&self, event: BusEvent) {
        let subs_guard = self.subscribers.lock().unwrap();
        let handlers_to_notify = subs_guard.clone();
        drop(subs_guard);

        for handler_arc_mutex in handlers_to_notify {
            if let Ok(mut handler_guard) = handler_arc_mutex.lock() {
                 handler_guard.handle(&event);
            } else {
                 eprintln!("[EventBus] Warning: Could not lock an event handler mutex.");
            }
        }
    }

    pub fn clear(&self) {
        // Lock the subscriber list
        if let Ok(mut subs) = self.subscribers.lock() {
            // Clear the vector, dropping all contained Arcs
            subs.clear();
        } else {
            eprintln!("[EventBus] Warning: Could not lock subscribers mutex to clear.");
        }
    }
}