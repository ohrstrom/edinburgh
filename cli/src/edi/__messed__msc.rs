use super::bus::EDIEvent;
use crate::utils;
use derivative::Derivative;
use log;
use std::cmp::min;
use std::vec;
use thiserror::Error;

#[cfg(target_arch = "wasm32")]
use futures::channel::mpsc::UnboundedSender;

#[cfg(not(target_arch = "wasm32"))]
use tokio::sync::mpsc::UnboundedSender;

const FPAD_LEN: usize = 2;
const PAD_BUFFER_SIZE: usize = 256;
const LENS: [usize; 8] = [4, 6, 8, 12, 16, 24, 32, 48];

fn format_pad_bits(byte: u8) -> String {
    let t = (byte >> 7) & 0x1;
    let s = (byte >> 5) & 0x3; // bits 6 and 5
    let c = (byte >> 4) & 0x1;

    let segment_type = match s {
        0b00 => "intermediate",
        0b01 => "last",
        0b10 => "first",
        0b11 => "one-and-only",
        _ => "invalid", // unreachable
    };

    let message_type = if c == 1 { "command" } else { "message" };

    format!(
        "T={} S={:02b} ({}) C={} ({})",
        t, s, segment_type, c, message_type
    )
}

fn printable_ascii_or_dot(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|&b| match b {
            0x20..=0x7E => b as char, // printable ASCII
            _ => '.', // replace everything else with `.`
        })
        .collect()
}

fn ascii_printable(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|&b| match b {
            0x20..=0x7E => b as char, // printable ASCII
            _ => '.',                 // everything else as dot
        })
        .collect()
}

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

#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub struct AACPResult {
    pub scid: u8,
    // #[derivative(Debug = "ignore")]
    #[derivative(Debug(format_with = "AACPResult::debug_frames"))]
    pub frames: Vec<Vec<u8>>,
    // pub pad: Vec<XPADResult>,
}

impl AACPResult {
    pub fn new(scid: u8, frames: Vec<Vec<u8>>) -> Self {
        Self { scid, frames }
    }
    fn debug_frames(frames: &Vec<Vec<u8>>, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", frames.len())
    }
}

fn readable_label(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|&b| match b {
            0x20..=0x7E => b as char,           // standard printable ASCII
            0xA0..=0xFF => b as char,           // Latin-1 supplement
            0x09 | 0x0A | 0x0D => ' ',          // tabs and line breaks → space
            _ => '.',                           // control chars and junk
        })
        .collect()
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum XPADIndicator {
    Short,
    Variable,
    Extended,
    Reserved,
}

#[derive(Debug)]
enum FPADType {
    NoXPAD,
    ShortXPAD,
    VariableXPAD,
    Reserved(u8),
}

#[derive(Debug, Clone, Copy)]
pub struct FPAD {
    pub ci_flag: bool,
    pub xpad_indicator: XPADIndicator,
}

impl From<u8> for FPAD {
    fn from(byte: u8) -> Self {
        let ci_flag = byte & 0b1000_0000 != 0;
        let indicator_bits = (byte >> 5) & 0b11;
        let xpad_indicator = match indicator_bits {
            0b00 => XPADIndicator::Short,
            0b01 => XPADIndicator::Variable,
            0b10 => XPADIndicator::Extended,
            _ => XPADIndicator::Reserved,
        };
        FPAD {
            ci_flag,
            xpad_indicator,
        }
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

    fn get_label(&mut self) -> Option<&[u8]> {
        if self.complete {
            Some(&self.segments)
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
    last_xpad_ci: Option<Vec<XPAD_CI>>,
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
        if xpad_bytes.is_empty() || fpad_bytes.is_empty() {
            // log::warn!("PADDecoder: Missing XPAD or FPAD bytes");
            return;
        }

        let fpad = FPAD::from(fpad_bytes[0]);
        let used_xpad_len = xpad_bytes.len().min(64); // adjust 64 based on actual size
        let mut xpad_reversed: Vec<u8> = xpad_bytes[..used_xpad_len].iter().rev().copied().collect();

        // log::debug!(
        //     "FPAD: ci_flag={}, indicator={:?}",
        //     fpad.ci_flag,
        //     fpad.xpad_indicator
        // );
        // log::debug!("FPAD raw: {:02X?}", fpad_bytes);
        // log::debug!("XPAD: {:02X?}", xpad_bytes);

        let must_reparse = fpad.ci_flag
            || self.last_xpad_ci.is_none()
            || self.last_xpad_ci.as_ref().map_or(true, |list| {
                let total_len: usize = list.iter().map(|ci| ci.ci_len).sum();
                total_len > xpad_reversed.len()
                    || (xpad_reversed.get(0).map(|b| b & 0x1F) != list.get(0).map(|ci| ci.ci_type))
            });

        let ci_list = match fpad.xpad_indicator {
            XPADIndicator::Short | XPADIndicator::Variable => {
                let list = Self::build_ci_list(
                    &xpad_reversed,
                    fpad_bytes,
                    self.last_xpad_ci.as_ref(),
                    must_reparse,
                );

                // ⛑ only cache it if it's not empty and we're reparsing
                if must_reparse && !list.is_empty() {
                    self.last_xpad_ci = Some(list.clone());
                    // log::debug!("Updated last_xpad_ci: {:?}", self.last_xpad_ci);
                }

                list
            }
            _ => vec![],
        };

        // log::debug!("CI List: {:?}", ci_list);

        let mut offset_so_far = 0;
        for (i, ci) in ci_list.iter().enumerate() {
            let start = offset_so_far;
            let end = start + ci.ci_len;

            log::debug!("CI: type: {:2} len: {:2}", ci.ci_type, ci.ci_len);

            if end > xpad_bytes.len() {
                log::debug!(
                    "Skipping CI[{}] type={} — would overrun buffer ({} > {})",
                    i,
                    ci.ci_type,
                    end,
                    xpad_bytes.len()
                );
                break;
            }

            let segment = &xpad_bytes[start..end];

            match ci.ci_type {
                2 | 3 => {
                    let start_flag = segment[0] & 0x80 != 0;
                    let label_data = &segment[1..];

                    // log::debug!(
                    //     "DL CI[{}]: start={}, data={:?}",
                    //     i,
                    //     start_flag,
                    //     String::from_utf8_lossy(label_data)
                    // );

                    self.dl_assembler.feed(start_flag, label_data);

                    // if let Some(label) = self.dl_assembler.get_label() {
                    //     log::info!("DL Label complete: {}", label);
                    // }

                    // if let Some(label) = self.dl_assembler.get_label() {
                    //     let printable = printable_ascii_or_dot(&self.dl_assembler.segments);
                    //     let hex: Vec<String> = self.dl_assembler.segments.iter().map(|b| format!("{:02X}", b)).collect();
                    //     log::info!("DL Label complete: \"{}\" [hex: [{}]]", printable, hex.join(", "));
                    // }

                    // if let Some(label) = self.dl_assembler.get_label() {
                    //     log::info!(
                    //         "DL Label complete: \"{}\" [hex: {:02X?}]",
                    //         label,
                    //         self.dl_assembler.segments
                    //     );
                    // }
                    if self.dl_assembler.complete {
                        // let ascii = ascii_printable(&self.dl_assembler.segments);
                        // log::info!("DL Label: \"{}\"", ascii);
                        self.dl_assembler.complete = false;
                    }
                }
                12 | 13 => {
                    // log::debug!("MOT CI[{}]: {:02X?}", i, segment);
                }
                _ => {
                    // log::debug!(
                    //     "Unknown CI[{}] type={} segment={:02X?}",
                    //     i,
                    //     ci.ci_type,
                    //     segment
                    // );
                }
            }

            offset_so_far += ci.ci_len;
        }
    }

    fn build_ci_list(
        xpad: &[u8],
        fpad_bytes: &[u8],
        last_ci: Option<&Vec<XPAD_CI>>,
        use_new_structure: bool,
    ) -> Vec<XPAD_CI> {
        let mut ci_list = Vec::new();

        if fpad_bytes.is_empty() {
            log::warn!("build_ci_list: FPAD header missing");
            return ci_list;
        }

        let fpad = FPAD::from(fpad_bytes[0]);
        // log::debug!(
        //     "build_ci_list: ci_flag={} xpad_indicator={:?}",
        //     fpad.ci_flag,
        //     fpad.xpad_indicator
        // );

        match fpad.xpad_indicator {
            XPADIndicator::Short => {
                if use_new_structure {
                    if let Some(&ci_byte) = xpad.first() {
                        let ci_type = ci_byte & 0x1F;
                        if ci_type != 0x00 {
                            // log::debug!("Short XPAD: CI type={:#X}, fixed length=3", ci_type);
                            ci_list.push(XPAD_CI { ci_type, ci_len: 3 });
                        } else {
                            // log::debug!("Short XPAD: First CI byte is 0x00, ignoring");
                        }
                    } else {
                        log::debug!("Short XPAD: XPAD is empty");
                    }
                } else if let Some(prev) = last_ci {
                    // log::debug!("Short XPAD: Reusing previous CI list");
                    ci_list = prev.clone();
                }
            }

            XPADIndicator::Variable => {
                if use_new_structure {
                    let mut offset = 0;
                    while offset < xpad.len() && ci_list.len() < 4 {
                        let ci_byte = xpad[offset];
                        let ci_type = ci_byte & 0x1F;

                        if ci_type == 0x00 {
                            // log::debug!(
                            //     "Variable XPAD: terminator encountered at offset {}",
                            //     offset
                            // );
                            break;
                        }

                        let ci_len = match ci_byte >> 5 {
                            0b000 => 4,
                            0b001 => 6,
                            0b010 => 8,
                            0b011 => 12,
                            0b100 => 16,
                            0b101 => 24,
                            0b110 => 32,
                            0b111 => 48,
                            _ => {
                                log::warn!("Unexpected CI length encoding, defaulting to 4");
                                4
                            }
                        };

                        // log::debug!(
                        //     "Variable XPAD: offset={} ci_byte=0x{:02X} => type={} len={}",
                        //     offset, ci_byte, ci_type, ci_len
                        // );
                        // log::debug!(
                        //     "Variable XPAD: offset={} ci_byte=0x{:02X} (xpad[offset..]= {:?}) => type={} len={}",
                        //     offset,
                        //     ci_byte,
                        //     &xpad[offset..min(offset + 8, xpad.len())],
                        //     ci_type,
                        //     ci_len
                        // );

                        // Check that the full segment fits in the buffer
                        if offset + ci_len > xpad.len() {
                            // log::warn!(
                            //     "Variable XPAD: CI segment overruns XPAD buffer (offset={}, len={}, xpad_len={})",
                            //     offset,
                            //     ci_len,
                            //     xpad.len()
                            // );
                            break;
                        }

                        // 🪵 NEW DEBUGGING LINE:
                        let segment = &xpad[offset..offset + ci_len];
                        // log::debug!(
                        //     "CI[{}] type={} len={} segment={:02X?}",
                        //     ci_list.len(),
                        //     ci_type,
                        //     ci_len,
                        //     segment
                        // );

                        ci_list.push(XPAD_CI { ci_type, ci_len });

                        offset += ci_len;
                    }
                } else if let Some(prev) = last_ci {
                    // log::debug!("Variable XPAD: Reusing previous CI list");
                    ci_list = prev.clone();
                }
            }

            XPADIndicator::Extended => {
                log::warn!("XPAD extended format not implemented");
            }

            XPADIndicator::Reserved => {
                log::warn!("XPAD indicator is reserved — ignoring XPAD");
            }
        }

        // log::debug!("build_ci_list: Final CI List = {:?}", ci_list);
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
    pub async fn feed(
        &mut self,
        data: &[u8],
        f_len: usize,
        event_tx: &UnboundedSender<EDIEvent>,
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
                    log::info!("SCID: {} {:?}", self.scid, af);
                    self.audio_format = Some(af);
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
                if self.scid == 2 {
                    self.pad_decoder.feed(&pad_data.0, &pad_data.1);
                }
            }
        }

        self.f_count = 0;

        let result: AACPResult = AACPResult::new(self.scid, self.au_frames.clone());

        #[cfg(not(target_arch = "wasm32"))]
        let _ = event_tx.send(EDIEvent::AACPFramesExtracted(result.clone()));

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
                return false;
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
