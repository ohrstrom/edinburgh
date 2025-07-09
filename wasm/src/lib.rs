/* NOTE: check implimentatios...
use wee_alloc::WeeAlloc;
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;
*/

use log::{self, Level};
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys;

use serde_wasm_bindgen::to_value;
use wasm_bindgen::JsValue;

use futures::StreamExt;

use console_log;
use derivative::Derivative;
use shared::edi::bus::{init_event_bus, EDIEvent};
use shared::edi::EDISource;
use shared::utils;

#[derive(Clone)]
#[wasm_bindgen]
pub struct EDI {
    inner: Rc<RefCell<EDISource>>,
    event_target: web_sys::EventTarget,
}

#[wasm_bindgen]
impl EDI {
    #[wasm_bindgen(constructor)]
    pub fn new() -> EDI {
        utils::set_panic_hook();
        let _ = console_log::init_with_level(Level::Info);

        let mut event_rx = init_event_bus();
        log::info!("EDI:init");

        let edi_source = Rc::new(RefCell::new(EDISource::new(None, None, None)));

        let event_target: web_sys::EventTarget =
            web_sys::EventTarget::new().unwrap().unchecked_into();

        let edi = EDI {
            inner: edi_source,
            event_target,
        };

        // Clone the edi instance for the async task.
        let edi_clone = edi.clone();

        spawn_local(async move {
            while let Some(event) = event_rx.next().await {
                let js_event = match &event {
                    EDIEvent::EnsembleUpdated(ensemble) => {
                        let data = to_value(&ensemble).unwrap();
                        Some(Self::create_event("ensemble_updated", &data))
                    }
                    EDIEvent::AACPFramesExtracted(aac) => {
                        let data = to_value(&aac).unwrap();
                        Some(Self::create_event("aac_segment", &data))
                    }
                    EDIEvent::MOTImageReceived(mot) => {
                        let data = to_value(&mot).unwrap();
                        Some(Self::create_event("mot_image", &data))
                    }
                    EDIEvent::DLObjectReceived(dl) => {
                        let data = to_value(&dl).unwrap();
                        Some(Self::create_event("dl_object", &data))
                    }
                    _ => None,
                };

                if let Some(js_event) = js_event {
                    edi_clone.event_target.dispatch_event(&js_event).unwrap();
                }
            }
        });

        edi
    }

    fn create_event(name: &str, detail: &JsValue) -> web_sys::CustomEvent {
        let init = web_sys::CustomEventInit::new();
        init.set_detail(detail);
        web_sys::CustomEvent::new_with_event_init_dict(name, &init).unwrap()
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

    #[wasm_bindgen(js_name = addEventListener)]
    pub fn add_event_listener(&self, event: &str, cb: &web_sys::js_sys::Function) {
        self.event_target
            .add_event_listener_with_callback(event, cb)
            .unwrap();
    }

    #[wasm_bindgen(js_name = removeEventListener)]
    pub fn remove_event_listener(&self, event: &str, cb: &web_sys::js_sys::Function) {
        self.event_target
            .remove_event_listener_with_callback(event, cb)
            .unwrap();
    }

    /*
    #[wasm_bindgen]
    pub fn on_ensemble_update(&self, callback: web_sys::js_sys::Function) {
        *self.on_ensemble_update_cb.borrow_mut() = Some(callback);
    }

    #[wasm_bindgen]
    pub fn on_aac_segment(&self, callback: web_sys::js_sys::Function) {
        *self.on_aac_segment_cb.borrow_mut() = Some(callback);
    }

    #[wasm_bindgen]
    pub fn on_mot_image_received(&self, callback: web_sys::js_sys::Function) {
        *self.on_mot_image_received_cb.borrow_mut() = Some(callback);
    }

    #[wasm_bindgen]
    pub fn on_dl_object_received(&self, callback: web_sys::js_sys::Function) {
        *self.on_dl_object_received_cb.borrow_mut() = Some(callback);
    }
    */
}
