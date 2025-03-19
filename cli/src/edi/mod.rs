pub mod bus;
mod ensemble;
mod fic;
mod frame;
mod msc;

use msc::{AACExctractor, FeedResult};
// use futures::channel::mpsc::UnboundedSender;
use derivative::Derivative;
use log;
use serde::Serialize;
use std::cell::RefCell;
use std::rc::Rc;

use bus::EDIEvent;
use ensemble::Ensemble;
use frame::Frame;
use frame::Tag;

#[cfg(target_arch = "wasm32")]
use futures::channel::mpsc::UnboundedSender;

#[cfg(not(target_arch = "wasm32"))]
use tokio::sync::mpsc::UnboundedSender;

#[derive(Debug, Serialize)]
pub struct AACFrame {
    pub scid: u8,
    pub data: Vec<u8>,
}

impl AACFrame {
    pub fn from_bytes(scid: u8, data: Vec<u8>) -> Self {
        AACFrame { scid, data }
    }
}

impl Drop for AACFrame {
    fn drop(&mut self) {
        self.data.clear();
    }
}

#[derive(Debug)]
pub struct EDISubchannel {
    scid: u8,
    audio_extractor: AACExctractor,
}

impl EDISubchannel {
    pub fn new(scid: u8) -> Self {
        EDISubchannel {
            scid,
            audio_extractor: AACExctractor::new(scid),
        }
    }
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct EDISource {
    event_tx: UnboundedSender<EDIEvent>,
    ensemble: Ensemble,
    subchannels: Vec<EDISubchannel>,
    scid: u8,
    // on_edi_frame: Option<Box<dyn FnMut(&Frame) + Send>>,
    // on_ensemble_update: Option<Box<dyn FnMut(&Ensemble) + Send>>,
    #[derivative(Debug = "ignore")]
    on_aac_segment: Option<Box<dyn FnMut(&AACFrame) + Send>>,
    // #[derivative(Debug = "ignore")]
    // on_aac_segment: Option<fn(&[u8])>,
}

impl EDISource {
    pub fn new(
        event_tx: UnboundedSender<EDIEvent>,
        on_aac_segment: Option<Box<dyn FnMut(&AACFrame) + Send>>,
        // on_aac_segment: Option<fn(&[u8])>,
    ) -> Self {
        EDISource {
            event_tx,
            ensemble: Ensemble::new(),
            subchannels: Vec::new(),
            scid: 0,
            // on_edi_frame: None,
            // on_ensemble_update: None,
            on_aac_segment: on_aac_segment,
        }
    }

    pub async fn feed(&mut self, data: &[u8]) {
        match Frame::from_bytes(data) {
            Ok(frame) => {
                // if let Some(ref callback) = self.on_edi_frame {
                //     let frame_js = to_value(&frame).unwrap_or(JsValue::NULL);
                //     let _ = callback.call1(&JsValue::NULL, &frame_js).ok();
                // }

                for tag in &frame.tags {
                    match tag {
                        Tag::DETI(tag) => {
                            if self.ensemble.feed(tag, &self.event_tx).await {
                                // if let Some(ref callback) = self.on_ensemble_update {
                                //     let ensemble_js =
                                //         to_value(&self.ensemble).unwrap_or(JsValue::NULL);
                                //     let _ = callback.call1(&JsValue::NULL, &ensemble_js).ok();
                                // }
                                // if let Some(ref callback) = self.on_ensemble_update {
                                //     let _ = callback.call1(&self.ensemble).ok();
                                // }
                            }
                        }

                        // AAC-segments
                        Tag::EST(tag) => {
                            let scid = tag.value[0] >> 2;

                            let slice_data = &tag.value[3..];
                            let slice_len = (tag.len / 8).saturating_sub(3);

                            let sc = match self.subchannels.iter_mut().find(|x| x.scid == scid) {
                                Some(sc) => sc,
                                None => {
                                    let sc = EDISubchannel::new(scid);
                                    self.subchannels.push(sc);
                                    self.subchannels.last_mut().unwrap()
                                }
                            };

                            match sc.audio_extractor.feed(&slice_data, slice_len) {
                                Ok(FeedResult::Complete(r)) => {
                                    // audio frames
                                    for frame in r.frames {
                                        let aac_frame = AACFrame::from_bytes(scid, frame);
                                        // log::debug!("AAC frame: {:?}", aac_frame);
                                        if let Some(ref mut callback) = self.on_aac_segment {
                                            // let _ = callback.call1(&aac_frame).ok();
                                            let _ = callback(&aac_frame);
                                        }
                                    }
                                }
                                Ok(FeedResult::Buffering) => {
                                    continue;
                                }
                                Err(_err) => {
                                    // log::warn!("Error feeding frame: {}", err);
                                }
                            }
                        }

                        // ignored tags
                        Tag::PTR(_tag) => {}
                        Tag::DMY(_tag) => {} // unknown / unsupported tags
                                             /*
                                             tag => {
                                                 log::warn!("Unsupported tag: {:?}", tag);
                                             }
                                             */
                    }
                }
            }
            Err(err) => {
                log::warn!("Error decoding frame: {:?}", err);
                return;
            }
        };
    }

    // config
    pub fn set_scid(&mut self, scid: u8) {
        self.scid = scid;
    }

    // callbacks
    /*
    pub fn set_on_edi_frame(&mut self, callback: Function) {
        self.on_edi_frame = Some(callback);
    }

    pub fn set_on_ensemble_update(&mut self, callback: Function) {
        self.on_ensemble_update = Some(callback);
    }

    pub fn set_on_aac_segment(&mut self, callback: Function) {
        self.on_aac_segment = Some(callback);
    }
    */

    // Callbacks
    // pub fn set_on_edi_frame<F>(&mut self, callback: F)
    // where
    //     F: FnMut(&EDIFrame) + Send + 'static,
    // {
    //     self.on_edi_frame = Some(Box::new(callback));
    // }

    // pub fn set_on_ensemble_update<F>(&mut self, callback: F)
    // where
    //     F: FnMut(&Ensemble) + Send + 'static,
    // {
    //     self.on_ensemble_update = Some(Box::new(callback));
    // }

    // pub fn set_on_aac_segment<F>(&mut self, callback: F)
    // where
    //     F: FnMut(&AACSegment) + Send + 'static,
    // {
    //     self.on_aac_segment = Some(Box::new(callback));
    // }
}
