pub mod bus;
mod ensemble;
mod fic;
mod frame;
pub mod msc;
pub mod pad;
mod tables;

use derivative::Derivative;
use log;
use msc::{AACPExctractor, FeedResult};
use serde::Serialize;

use bus::EDIEvent;
pub use ensemble::Ensemble;
use frame::Frame;
use frame::Tag;

#[derive(Debug, Serialize)]
pub struct AACPFrame {
    pub scid: u8,
    pub data: Vec<u8>,
}

impl AACPFrame {
    pub fn from_bytes(scid: u8, data: Vec<u8>) -> Self {
        AACPFrame { scid, data }
    }
}

impl Drop for AACPFrame {
    fn drop(&mut self) {
        self.data.clear();
    }
}

#[derive(Debug)]
pub struct EDISubchannel {
    scid: u8,
    audio_extractor: AACPExctractor,
}

impl EDISubchannel {
    pub fn new(scid: u8) -> Self {
        EDISubchannel {
            scid,
            audio_extractor: AACPExctractor::new(scid),
        }
    }
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct EDISource {
    ensemble: Ensemble,
    subchannels: Vec<EDISubchannel>,
    scid: u8,
    #[derivative(Debug = "ignore")]
    on_ensemble_update: Option<Box<dyn FnMut(&Ensemble) + Send>>,
    #[derivative(Debug = "ignore")]
    on_aac_segment: Option<Box<dyn FnMut(&AACPFrame) + Send>>,
}

impl EDISource {
    pub fn new(
        scid: Option<u8>,
        on_ensemble_update: Option<Box<dyn FnMut(&Ensemble) + Send>>,
        on_aac_segment: Option<Box<dyn FnMut(&AACPFrame) + Send>>,
    ) -> Self {
        EDISource {
            ensemble: Ensemble::new(),
            subchannels: Vec::new(),
            // scid: scid.unwrap_or(0),
            scid: scid.unwrap_or(0),
            //
            on_ensemble_update: on_ensemble_update,
            on_aac_segment: on_aac_segment,
        }
    }

    pub async fn feed(&mut self, data: &[u8]) {
        match Frame::from_bytes(data) {
            Ok(frame) => {
                for tag in &frame.tags {
                    match tag {
                        Tag::DETI(tag) => {
                            if self.ensemble.feed(tag).await {
                                if let Some(ref mut callback) = self.on_ensemble_update {
                                    let _ = callback(&self.ensemble);
                                }
                            }
                        }

                        // AAC-segments
                        Tag::EST(tag) => {
                            let scid = tag.value[0] >> 2;

                            let slice_data = &tag.value[3..];
                            let slice_len = (tag.len / 8).saturating_sub(3);

                            if scid == 0 {
                                let dbg = &slice_data[..slice_len.min(slice_data.len())];
                                let head = &dbg[..dbg.len().min(8)];
                                let tail = &dbg[dbg.len().saturating_sub(8)..];

                                // NOTE: until here dablin & edinburgh behave IDENTICA!
                                println!(
                                    "SLICE: scid={} len={} head={:02X?} tail={:02X?}",
                                    scid, slice_len, head, tail
                                );
                            }

                            let sc = match self.subchannels.iter_mut().find(|x| x.scid == scid) {
                                Some(sc) => sc,
                                None => {
                                    let mut sc = EDISubchannel::new(scid);
                                    sc.audio_extractor.extract_pad = self.scid == scid;
                                    self.subchannels.push(sc);
                                    self.subchannels.last_mut().unwrap()
                                }
                            };

                            match sc
                                .audio_extractor
                                // .feed(&slice_data, slice_len)
                                .feed(&slice_data[..slice_len], slice_len)
                                .await
                            {
                                Ok(FeedResult::Complete(r)) => {
                                    // audio frames
                                    for frame in r.frames {
                                        let aac_frame = AACPFrame::from_bytes(scid, frame);
                                        if let Some(ref mut callback) = self.on_aac_segment {
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
                        Tag::DMY(_tag) => {}

                        // unknown tags
                        Tag::FSST(_tag) => {}
                        Tag::FPTT(_tag) => {}
                        Tag::FSID(_tag) => {} // unsupported tags
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

    pub fn set_scid(&mut self, scid: u8) {
        self.scid = scid;
    }

    pub fn reset(&mut self) {
        log::info!("EDISource: reset");
        self.ensemble.reset();
        self.subchannels.clear();
    }
}
