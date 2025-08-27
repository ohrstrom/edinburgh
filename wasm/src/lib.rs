use log::{self, Level};
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;

use serde_wasm_bindgen::to_value;
use wasm_bindgen::JsValue;

use futures::lock::Mutex;
use futures::StreamExt;

use shared::dab::bus::{init_event_bus, DabEvent};
use shared::dab::DabSource;
use shared::utils;

#[derive(Clone)]
#[wasm_bindgen]
pub struct EDI {
    inner: Rc<Mutex<DabSource>>,
    event_target: web_sys::EventTarget,
}

#[wasm_bindgen]
impl EDI {
    #[wasm_bindgen(constructor)]
    #[allow(clippy::new_without_default)]
    pub fn new() -> EDI {
        utils::set_panic_hook();
        let _ = console_log::init_with_level(Level::Info);

        let mut event_rx = init_event_bus();
        log::info!("EDI:init");

        let edi_source = Rc::new(Mutex::new(DabSource::new(None, None, None)));

        let event_target: web_sys::EventTarget =
            web_sys::EventTarget::new().unwrap().unchecked_into();

        let edi = EDI {
            inner: edi_source,
            event_target,
        };

        let edi_clone = edi.clone();

        spawn_local(async move {
            while let Some(event) = event_rx.next().await {
                let js_event = match &event {
                    DabEvent::EnsembleUpdated(ensemble) => {
                        let data = to_value(&ensemble).unwrap();
                        Some(Self::create_event("ensemble_updated", &data))
                    }
                    DabEvent::AacpFramesExtracted(aac) => {
                        let data = to_value(&aac).unwrap();
                        Some(Self::create_event("aac_segment", &data))
                    }
                    DabEvent::MotImageReceived(mot) => {
                        let data = to_value(&mot).unwrap();
                        Some(Self::create_event("mot_image", &data))
                    }
                    DabEvent::DlObjectReceived(dl) => {
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
    pub async fn feed(&self, data: &[u8]) -> Result<(), JsValue> {
        let data = data.to_vec();
        let mut inner = self.inner.lock().await;
        inner.feed(&data).await;
        Ok(())
    }

    #[wasm_bindgen]
    pub async fn reset(&self) -> Result<(), JsValue> {
        let mut inner = self.inner.lock().await;
        inner.reset();
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
}
