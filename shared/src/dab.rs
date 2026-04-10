pub mod bus;
mod ensemble;
mod fic;
mod frame;
pub mod msc;
pub mod pad;
mod tables;
mod utils;

use derive_more::Debug;
pub use ensemble::{Ensemble, Subchannel};
use frame::Frame;
use frame::Tag;
use log;
use msc::{AacpExctractor, FeedResult};
use serde::Serialize;

use bus::{emit_event, DabEvent};

#[derive(Debug, Serialize)]
pub struct AacpFrame {
    pub scid: u8,
    pub data: Vec<u8>,
}

impl AacpFrame {
    pub fn from_bytes(scid: u8, data: Vec<u8>) -> Self {
        AacpFrame { scid, data }
    }
}

impl Drop for AacpFrame {
    fn drop(&mut self) {
        self.data.clear();
    }
}

#[derive(Debug)]
pub struct DabSubchannel {
    scid: u8,
    audio_extractor: AacpExctractor,
}

impl DabSubchannel {
    pub fn new(scid: u8) -> Self {
        DabSubchannel {
            scid,
            audio_extractor: AacpExctractor::new(scid),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DabStats {
    pub rx_rate: usize,
    pub rx_bytes: u64,
    pub rx_frames: u64,
}

impl Default for DabStats {
    fn default() -> Self {
        Self::new()
    }
}

impl DabStats {
    pub fn new() -> Self {
        DabStats {
            rx_rate: 0,
            rx_bytes: 0,
            rx_frames: 0,
        }
    }
    pub fn feed(&mut self, data: &[u8]) {
        let bytes = data.len();

        self.rx_bytes += bytes as u64;
        self.rx_frames += 1;

        emit_event(DabEvent::DabStatsUpdated(self.clone()));
    }
}

pub type EnsembleUpdateCallback = Box<dyn FnMut(&Ensemble) + Send>;

pub type AacpSegmentCallback = Box<dyn FnMut(&AacpFrame) + Send>;

#[derive(Debug)]
pub struct DabSource {
    ensemble: Ensemble,
    subchannels: Vec<DabSubchannel>,
    scid: u8,
    #[debug(skip)]
    on_ensemble_update: Option<EnsembleUpdateCallback>,
    #[debug(skip)]
    on_aac_segment: Option<AacpSegmentCallback>,
    stats: DabStats,
}

impl DabSource {
    pub fn new(
        scid: Option<u8>,
        on_ensemble_update: Option<EnsembleUpdateCallback>,
        on_aac_segment: Option<AacpSegmentCallback>,
    ) -> Self {
        let stats = DabStats::new();
        DabSource {
            ensemble: Ensemble::new(),
            subchannels: Vec::new(),
            scid: scid.unwrap_or(0),
            on_ensemble_update,
            on_aac_segment,
            stats,
        }
    }

    pub async fn feed(&mut self, data: &[u8]) {
        self.stats.feed(data);

        match Frame::from_bytes(data) {
            Ok(frame) => {
                for tag in &frame.tags {
                    match tag {
                        Tag::Deti(tag) => {
                            if self.ensemble.feed(tag).await {
                                if let Some(ref mut callback) = self.on_ensemble_update {
                                    callback(&self.ensemble);
                                }
                            }
                        }

                        // AAC-segments
                        Tag::Est(tag) => {
                            let scid = tag.value[0] >> 2;

                            let slice_data = &tag.value[3..];
                            let slice_len = (tag.len / 8).saturating_sub(3);

                            if scid == 0 {
                                let dbg = &slice_data[..slice_len.min(slice_data.len())];
                                let head = &dbg[..dbg.len().min(8)];
                                let tail = &dbg[dbg.len().saturating_sub(8)..];

                                // until here dablin & edinburgh behave IDENTICA!
                                println!(
                                    "SLICE: scid={} len={} head={:02X?} tail={:02X?}",
                                    scid, slice_len, head, tail
                                );
                            }

                            let sc = match self.subchannels.iter_mut().find(|x| x.scid == scid) {
                                Some(sc) => sc,
                                None => {
                                    let mut sc = DabSubchannel::new(scid);
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
                                    // "inject" audio format into ensemble
                                    self.ensemble.update_audio_format(r.scid, r.audio_format);

                                    // audio frames
                                    for frame in r.frames {
                                        let aac_frame = AacpFrame::from_bytes(scid, frame);
                                        if let Some(ref mut callback) = self.on_aac_segment {
                                            callback(&aac_frame);
                                        }
                                    }
                                }
                                Ok(FeedResult::Buffering) => {
                                    continue;
                                }
                                Err(err) => {
                                    log::warn!("Error feeding frame: {}", err);
                                }
                            }
                        }

                        // ignored tags
                        Tag::Ptr(_tag) => {}
                        Tag::Dmy(_tag) => {}

                        // unknown tags (at least to me...)
                        Tag::Fsst(_tag) => {}
                        Tag::Fptt(_tag) => {}
                        Tag::Fsid(_tag) => {} // unsupported tags
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
            }
        }
    }

    pub fn set_scid(&mut self, scid: u8) {
        self.scid = scid;
    }

    pub fn reset(&mut self) {
        log::info!("DabSource: reset");
        self.ensemble.reset();
        self.subchannels.clear();
    }
}
