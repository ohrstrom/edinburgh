use serde::Serialize;

use super::ensemble::Ensemble;
use super::msc::AacpResult;
use super::pad::dl::DlObject;
use super::pad::mot::MotImage;
use super::DabStats;

#[derive(Debug, Serialize)]
pub enum DabEvent {
    //
    EnsembleUpdated(Ensemble),
    AacpFramesExtracted(AacpResult),
    //
    MotImageReceived(MotImage),
    DlObjectReceived(DlObject),
    //
    DabStatsUpdated(DabStats),
}

#[cfg(target_arch = "wasm32")]
mod platform {
    use super::*;
    use futures::channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
    use once_cell::unsync::OnceCell;
    use std::cell::RefCell;
    use std::rc::Rc;

    thread_local! {
        static EVENT_TX: OnceCell<Rc<RefCell<UnboundedSender<DabEvent>>>> = OnceCell::new();
    }

    pub fn init_event_bus() -> UnboundedReceiver<DabEvent> {
        let (tx, rx) = unbounded::<DabEvent>();
        EVENT_TX.with(|cell| {
            cell.set(Rc::new(RefCell::new(tx)))
                .expect("Already initialized");
        });
        rx
    }

    pub fn emit_event(event: DabEvent) {
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

    static EVENT_TX: OnceCell<Mutex<UnboundedSender<DabEvent>>> = OnceCell::new();

    pub fn init_event_bus() -> UnboundedReceiver<DabEvent> {
        let (tx, rx) = unbounded_channel::<DabEvent>();
        EVENT_TX
            .set(Mutex::new(tx))
            .expect("Event bus already initialized");
        rx
    }

    pub fn emit_event(event: DabEvent) {
        if let Some(tx) = EVENT_TX.get() {
            let _ = tx.lock().unwrap().send(event);
        }
    }
}

// re-export unified interface from the platform module
pub use platform::{emit_event, init_event_bus};
