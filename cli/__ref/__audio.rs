use crate::utils;
use derivative::Derivative;
use log;
use std::cmp::min;
use std::vec;
use thiserror::Error;

const FPAD_LEN: usize = 2;
const PAD_BUFFER_SIZE: usize = 256;
const LENS: [usize; 8] = [4, 6, 8, 12, 16, 24, 32, 48];

#[derive(Debug, Error)]
pub enum FormatError {
    #[error("AU start values are zero")]
    StartValuesZero,
}

#[derive(Debug)]
pub struct AudioFormat {
    is_sbr: bool,
    is_ps: bool,
    codec: String,
    samplerate: u8,
    bitrate: usize,
    au_count: usize,
}

impl AudioFormat {
    pub fn from_bytes(sf: &[u8], sf_len: usize) -> Result<Self, FormatError> {
        if sf[3] == 0x00 && sf[4] == 0x00 {
            return Err(FormatError::StartValuesZero);
        }

        let h = sf[2];

        let dac_rate = (h & 0x40) != 0;
        let is_sbr = (h & 0x20) != 0;
        let is_ps = (h & 0x08) != 0;

        let codec = match (is_sbr, is_ps) {
            (true, true) => "HE-AAC v2",
            (true, false) => "HE-AAC",
            (false, _) => "AAC-LC",
        }
        .to_string();

        let samplerate = if dac_rate { 48 } else { 32 };
        let bitrate = sf_len / 120 * 8;

        let au_count = match (samplerate, is_sbr) {
            (48, true) => 3,
            (48, false) => 6,
            (_, true) => 2,
            (_, false) => 4,
        };

        Ok(Self {
            is_sbr,
            is_ps,
            codec,
            samplerate,
            bitrate,
            au_count,
        })
    }
}

// #[derive(Debug, Clone)]
// pub struct XPADResult {
//     pub data: Vec<u8>,
//     pub full_data: Vec<u8>,
// }

// impl XPADResult {
//     pub fn new(data: Vec<u8>, full_data: Vec<u8>) -> Self {
//         Self { data, full_data }
//     }
// }

#[derive(Debug)]
pub struct AACPResult {
    pub frames: Vec<Vec<u8>>,
    // pub pad: Vec<XPADResult>,
}

impl AACPResult {
    pub fn new(frames: Vec<Vec<u8>>) -> Self {
        Self { frames }
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

#[derive(Debug)]
enum XPADIndicator {
    None,
    Short,
    Variable,
    Reserved(u8),
}

#[derive(Debug)]
enum FPADType {
    NoXPAD,
    ShortXPAD,
    VariableXPAD,
    Reserved(u8),
}

#[derive(Debug)]
struct FPAD {
    fpad_type: u8, // usually 0b00
    xpad_indicator: XPADIndicator,
    ci_flag: bool,
}

impl FPAD {
    fn parse(fpad_bytes: &[u8]) -> Option<Self> {
        if fpad_bytes.len() != 2 {
            log::debug!("PAD: FPAD length mismatch");
            return None;
        }

        let fpad_type = fpad_bytes[0] >> 6;
        let xpad_indicator = match (fpad_bytes[0] & 0x30) >> 4 {
            0b00 => XPADIndicator::None,
            0b01 => XPADIndicator::Short,
            0b10 => XPADIndicator::Variable,
            other => XPADIndicator::Reserved(other),
        };

        let ci_flag = (fpad_bytes[1] & 0x02) != 0;

        Some(FPAD {
            fpad_type,
            xpad_indicator,
            ci_flag,
        })
    }
}

#[derive(Debug)]
struct DLAssembler {
    segments: Vec<u8>,
    complete: bool,
}

impl DLAssembler {
    fn new() -> Self {
        Self {
            segments: Vec::new(),
            complete: false,
        }
    }

    fn feed(&mut self, start: bool, data: &[u8]) {
        if start {
            self.segments.clear();
        }
        self.segments.extend_from_slice(data);

        // Check ETSI spec conditions for label completion here
        self.complete = true; // placeholder condition
    }

    fn get_label(&self) -> Option<String> {
        if self.complete {
            Some(String::from_utf8_lossy(&self.segments).into_owned())
        } else {
            None
        }
    }
}

#[derive(Debug)]
struct MOTAssembler {
    data: Vec<u8>,
    expected_len: usize,
    complete: bool,
}

impl MOTAssembler {
    fn new() -> Self {
        Self {
            data: Vec::new(),
            expected_len: 0,
            complete: false,
        }
    }

    fn feed(&mut self, start: bool, segment: &[u8]) {
        if start {
            self.data.clear();
            // Parse header here to set expected_len (using ETSI EN 301 234)
        }
        self.data.extend_from_slice(segment);

        // Check if complete (expected_len reached)
        if self.data.len() >= self.expected_len && self.expected_len > 0 {
            self.complete = true;
        }
    }

    fn get_mot_object(&self) -> Option<&[u8]> {
        if self.complete {
            Some(&self.data)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
struct XPAD_CI {
    ci_type: u8,
    ci_len: usize,
}

#[derive(Debug)]
pub struct PADDecoder {
    scid: u8,
    last_xpad_ci: Option<XPAD_CI>,
    //
    dl_assembler: DLAssembler,
    mot_assembler: MOTAssembler,
}

impl PADDecoder {
    pub fn new(scid: u8) -> Self {
        Self {
            scid,
            last_xpad_ci: None,
            //
            dl_assembler: DLAssembler::new(),
            mot_assembler: MOTAssembler::new(),
        }
    }
    pub fn feed(&mut self, xpad_bytes: &[u8], fpad_bytes: &[u8]) {
        let fpad = match FPAD::parse(fpad_bytes) {
            Some(fpad) => fpad,
            None => {
                log::warn!("PAD: FPAD parse error");
                return;
            }
        };

        let used_xpad_len = xpad_bytes.len().min(64); // adjust 64 based on actual size
        let mut xpad: Vec<u8> = xpad_bytes[..used_xpad_len].iter().rev().copied().collect();

        // log::debug!("Decoded FPAD: {:?}", fpad);

        let ci_list = Self::build_ci_list(&fpad, &xpad, self.last_xpad_ci.as_ref());

        if ci_list.is_empty() {
            // log::debug!("No CI found. FPAD: {:?}", fpad);
            return;
        }

        // log::debug!("CI List: {:?}", ci_list);

        let mut offset = ci_list.len();
        for ci in ci_list.iter() {
            let data_end = offset + ci.ci_len;
            if data_end > xpad.len() {
                log::warn!("CI segment overruns available data");
                break;
            }
            let data_segment = &xpad[offset..data_end];

            match ci.ci_type {
                1 => {
                    // TODO: implement
                }
                2 | 3 => {
                    let is_start = ci.ci_type == 2;
                    self.dl_assembler.feed(is_start, data_segment);
                    if let Some(label) = self.dl_assembler.get_label() {
                        log::info!("Dynamic Label: {}", label);
                    }
                }
                12 | 13 => {
                    let is_start = ci.ci_type == 12;
                    self.mot_assembler.feed(is_start, data_segment);
                    if let Some(mot_data) = self.mot_assembler.get_mot_object() {
                        log::info!("Received MOT object of size {}", mot_data.len());
                        // handle MOT object (e.g., display slide)
                    }
                }
                _ => {
                    log::debug!("Unhandled CI type {}", ci.ci_type);
                }
            }

            offset += ci.ci_len;
        }

        self.last_xpad_ci = ci_list.last().cloned();
    }
    fn build_ci_list(fpad: &FPAD, xpad: &[u8], last_ci: Option<&XPAD_CI>) -> Vec<XPAD_CI> {
        let mut ci_list = Vec::new();

        if fpad.ci_flag {
            match fpad.xpad_indicator {
                XPADIndicator::Short => {
                    if !xpad.is_empty() {
                        let ci_type = xpad[0] & 0x1F;
                        if ci_type != 0x00 {
                            ci_list.push(XPAD_CI { ci_type, ci_len: 3 });
                        }
                    }
                }
                XPADIndicator::Variable => {
                    let mut offset = 0;
                    while offset < xpad.len() && ci_list.len() < 4 {
                        let ci_byte = xpad[offset];
                        let ci_type = ci_byte & 0x1F;

                        if ci_type == 0x00 {
                            break;
                        }

                        // Correctly determine ci_len based on the ETSI spec (section 7.4.2)
                        let ci_len = match ci_byte >> 5 {
                            0b000 => 4,
                            0b001 => 6,
                            0b010 => 8,
                            0b011 => 12,
                            0b100 => 16,
                            0b101 => 24,
                            0b110 => 32,
                            0b111 => 48,
                            _ => 4, // default fallback
                        };

                        ci_list.push(XPAD_CI { ci_type, ci_len });
                        offset += 1;
                    }
                }
                _ => {}
            }
        } else if matches!(
            fpad.xpad_indicator,
            XPADIndicator::Short | XPADIndicator::Variable
        ) {
            if let Some(prev_ci) = last_ci {
                ci_list.push(prev_ci.clone());
            }
        }

        ci_list
    }
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
    //
    pad_decoder: PADDecoder,
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
            //
            pad_decoder: PADDecoder::new(scid),
        }
    }
    pub fn feed(&mut self, data: &[u8], f_len: usize) -> Result<FeedResult, FeedError> {
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

        // copy buffer
        self.sf_buff.copy_from_slice(&self.sf_raw[0..self.sf_len]);

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
                    self.audio_format = Some(af);
                    log::info!("Audio format: {} {:?}", self.scid, self.audio_format);
                }
                Err(err) => {
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

            // check for PAD data
            let pad_data = Self::extract_pad(&au_data[..au_len - 2]);

            if let Some(pad_data) = pad_data {
                // NOTE: disabled at the moment
                // self.pad_decoder.feed(&pad_data.0, &pad_data.1);
            }
        }

        self.f_count = 0;

        let result: AACPResult = AACPResult::new(self.au_frames.clone());

        self.au_frames.clear();

        Ok(FeedResult::Complete(result))
    }

    fn re_sync(&mut self) -> bool {
        let crc_stored = u16::from_be_bytes([self.sf_buff[0], self.sf_buff[1]]);
        let crc_calculated = utils::calc_crc_fire_code(&self.sf_buff[2..11]);

        if crc_stored != crc_calculated {
            return false;
        }

        // abort processiung if no audio format is set
        if self.audio_format.is_none() {
            // log::debug!("AD: no audio format yet");
            return true;
        }

        // NOTE: is this how it shoud be done??
        let sf_format = self.audio_format.as_ref().unwrap();

        // set / update values for current subframe
        self.au_count = sf_format.au_count;

        // NOTE: check if this is correct
        self.au_start[0] = match (sf_format.samplerate, sf_format.is_sbr) {
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

        for i in 0..self.au_count {
            if self.au_start[i] >= self.au_start[i + 1] {
                log::warn!("AD: AU start values are invalid!");
            }
        }

        return true;
    }

    fn extract_pad(au_data: &[u8]) -> Option<(Vec<u8>, Vec<u8>)> {
        if au_data.len() < 3 {
            return None;
        }

        if (au_data[0] >> 5) != 4 {
            // NOTE: why do we skip this?
            return None;
        }

        // log::debug!("PAD: {:?}", au_data.len());

        let mut pad_start = 2;
        let mut pad_len = au_data[1] as usize;

        if pad_len == 255 {
            // NOTE: (why) do we need this?
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

        // log::debug!("PAD: X: {:?} <> FP: {:?}", xpad_data.len(), fpad_data.len());

        return Some((xpad_data.to_vec(), fpad_data.to_vec()));
    }
}
