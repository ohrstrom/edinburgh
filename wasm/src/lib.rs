/* NOTE: check implimentatios...
use wee_alloc::WeeAlloc;
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;
*/

mod audio;

use log::{self, Level};
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::js_sys;

use serde_wasm_bindgen::to_value;
use wasm_bindgen::JsValue;

use futures::channel::mpsc::unbounded;
use futures::StreamExt;

use console_log;

use shared::utils;
use shared::edi::bus::EDIEvent;
use shared::edi::EDISource;

#[derive(Clone)]
#[wasm_bindgen]
pub struct EDI {
    inner: Rc<RefCell<EDISource>>,
    cb: Rc<RefCell<Option<js_sys::Function>>>,
    on_ensemble_update_cb: Rc<RefCell<Option<js_sys::Function>>>,
    on_aac_segment_cb: Rc<RefCell<Option<js_sys::Function>>>,
}

#[wasm_bindgen]
impl EDI {
    #[wasm_bindgen(constructor)]
    pub fn new() -> EDI {
        utils::set_panic_hook();
        let _ = console_log::init_with_level(Level::Debug);

        let (event_tx, mut event_rx) = unbounded::<EDIEvent>();
        log::info!("EDI:init");

        let edi_source = Rc::new(RefCell::new(EDISource::new(None, event_tx, None)));

        let cb = Rc::new(RefCell::new(None));
        let on_ensemble_update_cb = Rc::new(RefCell::new(None));
        let on_aac_segment_cb = Rc::new(RefCell::new(None));

        let edi = EDI {
            inner: edi_source,
            cb: Rc::clone(&cb),
            on_ensemble_update_cb: Rc::clone(&on_ensemble_update_cb),
            on_aac_segment_cb: Rc::clone(&on_aac_segment_cb),
        };

        // Clone the edi instance for the async task.
        let edi_clone = edi.clone();

        spawn_local(async move {
            while let Some(event) = event_rx.next().await {
                match &event {
                    EDIEvent::EnsembleUpdated(ensemble) => {
                        if let Some(cb) = edi_clone.on_ensemble_update_cb.borrow().as_ref() {
                            let this = JsValue::NULL;
                            let event_data = to_value(&ensemble).unwrap();
                            cb.call1(&this, &event_data).unwrap();
                        }
                    }
                    EDIEvent::AACPFramesExtracted(r) => {
                        if let Some(cb) = edi_clone.on_aac_segment_cb.borrow().as_ref() {
                            let this = JsValue::NULL;
                            let event_data = to_value(&r).unwrap();
                            cb.call1(&this, &event_data).unwrap();
                            // for frame in &r.frames {
                            //     let event_data = to_value(&frame).unwrap();
                            //     cb.call1(&this, &event_data).unwrap();
                            // }
                        }
                    }
                }

                if let Some(callback) = edi_clone.cb.borrow().as_ref() {
                    let this = JsValue::NULL;
                    let event_data = to_value(&event).unwrap();
                    callback.call1(&this, &event_data).unwrap();
                }
            }
        });

        edi
    }

    #[wasm_bindgen]
    pub async fn feed(&mut self, data: &[u8]) -> Result<(), JsValue> {
        let data = data.to_vec(); // copy data to avoid contention
        self.inner.borrow_mut().feed(&data).await;
        Ok(())
    }

    #[wasm_bindgen]
    pub async fn reset(&mut self) -> Result<(), JsValue> {
        self.inner.borrow_mut().reset();
        Ok(())
    }

    #[wasm_bindgen]
    pub fn on_edi_event(&self, callback: js_sys::Function) {
        *self.cb.borrow_mut() = Some(callback);
    }

    #[wasm_bindgen]
    pub fn on_ensemble_update(&self, callback: js_sys::Function) {
        *self.on_ensemble_update_cb.borrow_mut() = Some(callback);
    }

    #[wasm_bindgen]
    pub fn on_aac_segment(&self, callback: js_sys::Function) {
        *self.on_aac_segment_cb.borrow_mut() = Some(callback);
    }
}
