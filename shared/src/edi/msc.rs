use super::bus::{EDIEvent, emit_event};
use super::pad::PADDecoder;
use crate::utils;
use derivative::Derivative;
use log;
use thiserror::Error;
use serde::Serialize;


const FPAD_LEN: usize = 2;

#[derive(Debug, Error)]
pub enum FormatError {
    #[error("AU start values are zero")]
    StartValuesZero,
}

#[derive(Debug, Clone, Serialize)]
pub struct AudioFormat {
    sbr: bool,
    ps: bool,
    codec: String,
    samplerate: u8,
    bitrate: usize,
    au_count: usize,
    channels: u8,
}

impl AudioFormat {
    pub fn from_bytes(sf: &[u8], sf_len: usize) -> Result<Self, FormatError> {
        if sf[3] == 0x00 && sf[4] == 0x00 {
            return Err(FormatError::StartValuesZero);
        }

        let h = sf[2];

        let dac_mode = (h & 0x40) != 0;
        let sbr = (h & 0x20) != 0;
        let ps = (h & 0x08) != 0;
        let channel_mode = (h & 0x10) != 0;

        // let channel_mode_x = h & 0x10;

        // log::debug!("channel mode: {:?} - {}", channel_mode_x, channel_mode);

        let codec = match (sbr, ps) {
            (true, true) => "HE-AACv2",
            (true, false) => "HE-AAC",
            (false, _) => "AAC-LC",
        }
        .to_string();

        let samplerate = if dac_mode { 48 } else { 32 };
        let bitrate = sf_len / 120 * 8;

        let au_count = match (samplerate, sbr) {
            (48, true) => 3,
            (48, false) => 6,
            (_, true) => 2,
            (_, false) => 4,
        };

        let channels = if channel_mode || ps { 2 } else { 1 };

        Ok(Self {
            sbr,
            ps,
            codec,
            samplerate,
            bitrate,
            au_count,
            channels,
        })
    }
}

#[derive(Derivative, Clone, Serialize)]
#[derivative(Debug)]
pub struct AACPResult {
    pub scid: u8,
    pub audio_format: Option<AudioFormat>,
    #[derivative(Debug(format_with = "AACPResult::debug_frames"))]
    pub frames: Vec<Vec<u8>>,
}

impl AACPResult {
    pub fn new(scid: u8, audio_format: Option<AudioFormat>, frames: Vec<Vec<u8>>) -> Self {
        Self { scid, audio_format, frames }
    }
    fn debug_frames(frames: &Vec<Vec<u8>>, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", frames.len())
    }
}

#[derive(Derivative, Clone, Serialize)]
#[derivative(Debug)]
pub struct PADResult {
    pub fpad: Vec<u8>,
    #[derivative(Debug(format_with = "PADResult::debug_xpad"))]
    pub xpad: Vec<u8>,
}

impl PADResult {
    pub fn new(fpad: Vec<u8>, xpad: Vec<u8>) -> Self {
        Self { fpad, xpad }
    }
    fn debug_xpad(xpad: &Vec<u8>, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{} bytes", xpad.len())
    }
}

#[derive(Debug, Error)]
pub enum FeedError {
    #[error("Frame length mismatch: {l1} != {l2}")]
    FrameLengtMismatch { l1: usize, l2: usize },

    #[error("Frame length invalid: {l}")]
    FrameLengtInvalid { l: usize },
}

#[derive(Debug)]
pub enum FeedResult {
    Complete(AACPResult),
    Buffering,
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct AACPExctractor {
    scid: u8,
    f_len: usize,
    f_count: usize,
    f_sync: usize,
    sf_len: usize,
    sf_raw: Vec<u8>,
    sf_buff: Vec<u8>,
    au_count: usize,
    au_start: Vec<usize>,
    audio_format: Option<AudioFormat>,
    au_frames: Vec<Vec<u8>>,
    pad_decoder: PADDecoder,
    //
    pub extract_pad: bool,
}

impl AACPExctractor {
    pub fn new(scid: u8) -> Self {
        Self {
            scid,
            f_len: 0,
            f_count: 0,
            f_sync: 0,
            sf_len: 0,
            sf_raw: Vec::new(),
            sf_buff: Vec::new(),
            au_count: 0,
            au_start: vec![0; 7],
            audio_format: None,
            au_frames: Vec::new(),
            pad_decoder: PADDecoder::new(scid),
            //
            extract_pad: false,
        }
    }
    pub async fn feed(
        &mut self,
        data: &[u8],
        f_len: usize,
    ) -> Result<FeedResult, FeedError> {
        self.au_frames.clear();

        if self.f_len != 0 {
            if self.f_len != f_len {
                return Err(FeedError::FrameLengtMismatch {
                    l1: f_len,
                    l2: self.f_len,
                });
            }
        } else {
            if f_len < 10 {
                return Err(FeedError::FrameLengtInvalid { l: f_len });
            }

            if (5 * f_len) % 120 != 0 {
                return Err(FeedError::FrameLengtInvalid { l: f_len });
            }

            self.f_len = f_len;
            self.sf_len = 5 * f_len;

            self.sf_raw.clear();
            self.sf_buff.clear();

            self.sf_raw.resize(self.sf_len, 0);
            self.sf_buff.resize(self.sf_len, 0);
        }

        // NOTE: problem start ?
        if self.f_count == 5 {
            self.sf_raw.copy_within(self.f_len.., 0);
        } else {
            self.f_count += 1;
        }

        let start = (self.f_count - 1) * self.f_len;
        let end = start + self.f_len;
        self.sf_raw[start..end].copy_from_slice(&data[..self.f_len]);

        if self.f_count < 5 {
            return Ok(FeedResult::Buffering);
        }

        self.sf_buff.copy_from_slice(&self.sf_raw[0..self.sf_len]);
        // NOTE: problem end ?

        /*
        let start = self.f_count * self.f_len;
        let end = start + self.f_len;
        self.sf_raw[start..end].copy_from_slice(&data[..self.f_len]);
        self.f_count += 1;

        if self.f_count < 5 {
            return Ok(FeedResult::Buffering);
        }

        // Now we have 5 frames collected
        self.sf_buff.copy_from_slice(&self.sf_raw[0..self.sf_len]);
        self.f_count = 0;
        */


        if !self.re_sync() {
            if self.f_sync == 0 {
                log::debug!("AD: SF sync START {} frames", self.f_sync);
            }
            self.f_sync += 1;

            return Ok(FeedResult::Buffering);
        }

        if self.f_sync > 0 {
            log::debug!("SF {} sync OK after {} frames", self.scid, self.f_sync);
            self.f_sync = 0;
        }

        if self.audio_format.is_none() && self.sf_buff.len() >= 11 {
            match AudioFormat::from_bytes(&self.sf_buff, self.sf_len) {
                Ok(af) => {
                    log::info!("SCID: {} {:?}", self.scid, af);
                    self.audio_format = Some(af);
                }
                Err(err) => {
                    // NOTE: silenced log for the moment
                    log::warn!("Format error: {} {:?}", self.scid, err);
                }
            }
        }

        for i in 0..self.au_count {
            // NOTE: check if this is correct
            let au_data = &self.sf_buff[self.au_start[i]..self.au_start[i + 1]];
            let au_len = self.au_start[i + 1] - self.au_start[i];

            let au_crc_stored = ((au_data[au_len - 2] as u16) << 8) | au_data[au_len - 1] as u16;
            let au_crc_calced = utils::calc_crc16_ccitt(&au_data[0..au_len - 2]);

            if au_crc_stored != au_crc_calced {
                log::warn!("AD: AU CRC mismatch!");
                continue;
            }

            // copy AU frames to buffer. do not forget to remove last two bytes (CRC)
            self.au_frames.push(au_data[..au_len - 2].to_vec());

            // check for PAD data. locked to SCID 6 (edi-ch.digris.net:8855 0x4DA4 open broadcast)
            /**/
            // if self.scid == 10 {
            //     let pad = Self::extract_pad(&au_data[..au_len - 2]);
            //     if let Some(pad) = pad {
            //         self.pad_decoder.feed(&pad.fpad, &pad.xpad);
            //     }
            // }


            // if self.extract_pad {
                let pad = Self::extract_pad(&au_data[..au_len - 2]);
                if let Some(pad) = pad {
                    self.pad_decoder.feed(&pad.fpad, &pad.xpad);
                }
            // }
        }

        self.f_count = 0;

        let result: AACPResult = AACPResult::new(self.scid, self.audio_format.clone(), self.au_frames.clone());

        emit_event(EDIEvent::AACPFramesExtracted(result.clone()));

        self.au_frames.clear();

        Ok(FeedResult::Complete(result))
    }

    fn re_sync(&mut self) -> bool {
        let crc_stored = u16::from_be_bytes([self.sf_buff[0], self.sf_buff[1]]);
        let crc_calculated = utils::calc_crc_fire_code(&self.sf_buff[2..11]);

        if crc_stored != crc_calculated {
            return false;
        }

        // abort processing if no audio format is set
        if self.audio_format.is_none() {
            log::debug!("AD: no audio format yet");
            return true;
        }

        // NOTE: is this how it should be done??
        let sf_format = self.audio_format.as_ref().unwrap();

        // set / update values for current sub-frame
        self.au_count = sf_format.au_count;

        // NOTE: check if this is correct
        self.au_start[0] = match (sf_format.samplerate, sf_format.sbr) {
            (48, true) => 6,
            (48, false) => 11,
            (_, true) => 5,
            (_, false) => 8,
        };

        self.au_start[self.au_count] = self.sf_len / 120 * 110;

        self.au_start[1] = ((self.sf_buff[3] as usize) << 4) | ((self.sf_buff[4] >> 4) as usize);

        if self.au_count >= 3 {
            self.au_start[2] =
                (((self.sf_buff[4] & 0x0F) as usize) << 8) | (self.sf_buff[5] as usize);
        }

        if self.au_count >= 4 {
            self.au_start[3] =
                ((self.sf_buff[6] as usize) << 4) | ((self.sf_buff[7] >> 4) as usize);
        }

        if self.au_count == 6 {
            self.au_start[4] =
                (((self.sf_buff[7] & 0x0F) as usize) << 8) | (self.sf_buff[8] as usize);
            self.au_start[5] =
                ((self.sf_buff[9] as usize) << 4) | ((self.sf_buff[10] >> 4) as usize);
        }

        // ✅ ADD THIS LOG
        log::info!(
            "SF sync OK: samplerate={} sbr={} ps={} channels={} au_count={} dac_rate={}",
            sf_format.samplerate,
            sf_format.sbr,
            sf_format.ps,
            sf_format.channels,
            self.au_count,
            if sf_format.samplerate == 48 { "yes" } else { "no" }
        );

        // log::info!(
        //     "AU start table: {:?}",
        //     &self.au_start[..=self.au_count]
        // );

        // log::info!(
        //     "Raw SF header: sf[2]=0x{:02X} sf[3]=0x{:02X} sf[4]=0x{:02X}",
        //     self.sf_buff[2], self.sf_buff[3], self.sf_buff[4]
        // );

        for i in 0..self.au_count {
            if self.au_start[i] >= self.au_start[i + 1] {
                log::warn!("AD: AU start values are invalid!");
                return false;
            }
        }

        return true;
    }

    fn extract_pad(au_data: &[u8]) -> Option<PADResult> {
        if au_data.len() < 3 {
            return None;
        }

        if (au_data[0] >> 5) != 4 {
            // Only process if AU Stream ID indicates DAB+ (0b100)
            return None;
        }

        let mut pad_start = 2;
        let mut pad_len = au_data[1] as usize;

        if pad_len == 255 {
            // actual length is 255 + next byte
            if au_data.len() < 4 {
                return None;
            }
            pad_len += au_data[2] as usize;
            pad_start += 1;
        }

        if pad_len < 2 || au_data.len() < pad_start + pad_len {
            return None;
        }

        let xpad_data = &au_data[pad_start..pad_start + pad_len - FPAD_LEN];
        let fpad_data = &au_data[pad_start + pad_len - FPAD_LEN..pad_start + pad_len];

        let pad = PADResult::new(fpad_data.to_vec(), xpad_data.to_vec());

        // log::debug!("PAD: {:?}", pad);

        // log::debug!(
        //     "PAD: {} - {}",
        //     pad.fpad[0..2]
        //         .iter()
        //         .map(|byte| format!("{:08b}", byte))
        //         .collect::<Vec<_>>()
        //         .join(" "),
        //     pad.xpad.len()
        // );

        return Some(pad);
    }
}
