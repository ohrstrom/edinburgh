use serde::Serialize;

use super::ensemble::Ensemble;
use super::msc::AACPResult;
use super::pad::dl::DLObject;
use super::pad::mot::MOTImage;

#[derive(Debug, Serialize)]
pub enum EDIEvent {
    EnsembleUpdated(Ensemble),
    AACPFramesExtracted(AACPResult),
    //
    MOTImageReceived(MOTImage),
    DLObjectReceived(DLObject),
}

#[cfg(target_arch = "wasm32")]
mod platform {
    use super::*;
    use futures::channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
    use once_cell::unsync::OnceCell;
    use std::cell::RefCell;
    use std::rc::Rc;

    thread_local! {
        static EVENT_TX: OnceCell<Rc<RefCell<UnboundedSender<EDIEvent>>>> = OnceCell::new();
    }

    pub fn init_event_bus() -> UnboundedReceiver<EDIEvent> {
        let (tx, rx) = unbounded::<EDIEvent>();
        EVENT_TX.with(|cell| {
            cell.set(Rc::new(RefCell::new(tx)))
                .expect("Already initialized");
        });
        rx
    }

    pub fn emit_event(event: EDIEvent) {
        EVENT_TX.with(|cell| {
            if let Some(tx) = cell.get() {
                let _ = tx.borrow_mut().unbounded_send(event);
            }
        });
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod platform {
    use super::*;
    use once_cell::sync::OnceCell;
    use std::sync::Mutex;
    use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

    static EVENT_TX: OnceCell<Mutex<UnboundedSender<EDIEvent>>> = OnceCell::new();

    pub fn init_event_bus() -> UnboundedReceiver<EDIEvent> {
        let (tx, rx) = unbounded_channel::<EDIEvent>();
        EVENT_TX
            .set(Mutex::new(tx))
            .expect("Event bus already initialized");
        rx
    }

    pub fn emit_event(event: EDIEvent) {
        if let Some(tx) = EVENT_TX.get() {
            let _ = tx.lock().unwrap().send(event);
        } else {
            eprintln!("Event bus not initialized");
        }
    }
}

// re-export unified interface from the platform module
pub use platform::{emit_event, init_event_bus};
