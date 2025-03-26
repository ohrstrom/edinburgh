use derivative::Derivative;
use log;
use std::collections::BTreeMap;
use thiserror::Error;

const XPAD_CI_LEN_LOOKUP: [usize; 8] = [4, 6, 8, 12, 16, 24, 32, 48];

fn parse_mot_header_size(segment: &[u8]) -> Option<usize> {
    let mut i = 1;

    while i + 1 < segment.len() {
        let tag = segment[i];
        let len = segment[i + 1] as usize;
        i += 2;

        if i + len > segment.len() {
            break;
        }

        log::trace!("MOT header tag: 0x{:02X}, len = {}", tag, len);

        if tag == 0x0D && len == 3 {
            let size = ((segment[i] as usize) << 16)
                | ((segment[i + 1] as usize) << 8)
                | (segment[i + 2] as usize);
            return Some(size);
        }

        i += len;
    }

    None
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum XPADIndicator {
    Short,
    Variable,
    Extended,
    Reserved,
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
struct DLSegment {
    toggle: bool,
    first: bool,
    last: bool,
    dl_plus_link: bool,
    seg_num: u8,
    chars: Vec<u8>,
}

impl DLSegment {
    fn from_bytes(prefix: &[u8; 2], data: &[u8]) -> Self {
        Self {
            toggle: prefix[0] & 0x80 != 0,
            first: prefix[0] & 0x40 != 0,
            last: prefix[0] & 0x20 != 0,
            dl_plus_link: prefix[1] & 0x80 != 0,
            seg_num: if prefix[0] & 0x40 != 0 {
                0
            } else {
                (prefix[1] & 0x70) >> 4
            },
            chars: data.to_vec(),
        }
    }
}

#[derive(Debug)]
struct DLAssembler {
    segments: BTreeMap<u8, DLSegment>,
    current_toggle: Option<bool>,
    complete_toggle: Option<bool>,
}

impl DLAssembler {
    fn new() -> Self {
        Self {
            segments: BTreeMap::new(),
            current_toggle: None,
            complete_toggle: None,
        }
    }

    fn feed(&mut self, start: bool, payload: &[u8]) {
        if start {
            self.segments.clear();
            self.current_toggle = None;
            self.complete_toggle = None;
        }

        let (prefix, data) = payload.split_at(2);
        let prefix: &[u8; 2] = prefix.try_into().unwrap();

        let seg = DLSegment::from_bytes(prefix, data);

        // TODO: needs implementation. not sure if we have "correct" data until here...
    }

    fn is_complete(&self) -> bool {
        // TODO: needs implementation. not sure if we have "correct" data until here...
        false
    }
}

#[derive(Debug)]
pub struct MOTObject {
    pub data: Vec<u8>,
}

#[derive(Debug)]
pub struct MOTAssembler {
    data: Vec<u8>,
    expected_len: usize,
    header_parsed: bool,
    complete: bool,
    in_progress: bool,
}

impl MOTAssembler {
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            expected_len: 0,
            header_parsed: false,
            complete: false,
            in_progress: false,
        }
    }

    pub fn feed(&mut self, start: bool, segment: &[u8]) {
        if start {
            // Reset state on fresh MOT start
            // self.data.clear();
            self.expected_len = 0;
            self.header_parsed = false;
            self.complete = false;
            self.in_progress = true;

            if let Some(size) = parse_mot_header_size(segment) {
                self.expected_len = size;
                self.header_parsed = true;
            } else {
                // log::warn!("MOT: Could not parse header size");
            }

            // Make sure we have enough bytes for header (at least 6)
            // if segment.len() >= 6 {
            //     self.expected_len = ((segment[4] as usize) << 8) | segment[5] as usize;
            //     self.header_parsed = true;
            // } else {
            //     // Not enough for header yet; wait for more data
            //     // self.expected_len = 0;
            // }
        }

        if !self.in_progress || self.complete {
            return;
        }

        self.data.extend_from_slice(segment);

        log::debug!(
            "MOT: data.len = {}, expected_len = {}",
            self.data.len(),
            self.expected_len
        );

        // Fallback: if we didn't parse the header earlier (not enough bytes)
        if !self.header_parsed && self.data.len() >= 6 {
            self.expected_len = ((self.data[4] as usize) << 8) | self.data[5] as usize;
            self.header_parsed = true;
        }

        if self.header_parsed && self.data.len() >= self.expected_len {
            self.complete = true;
            self.in_progress = false;
            // self.data.clear();
        }
    }

    fn is_valid_mot_type(&self, type_: i8) -> bool {
        // Check if type_ is a valid MOT type (using ETSI EN 301 234)
        // TODO: just dummy implementation here...
        true
    }

    pub fn is_complete(&self) -> bool {
        self.complete
    }

    pub fn take(&mut self) -> Option<MOTObject> {
        if !self.complete || self.expected_len == 0 {
            return None;
        }

        let mut mot_data = Vec::with_capacity(self.expected_len);
        std::mem::swap(&mut self.data, &mut mot_data);
        mot_data.truncate(self.expected_len);

        self.expected_len = 0;
        self.header_parsed = false;
        self.complete = false;
        self.in_progress = false;

        Some(MOTObject { data: mot_data })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XPAD_CI {
    pub type_: i8,
    pub len: usize,
}

impl XPAD_CI {
    pub fn new(len: usize, type_: u8) -> Self {
        Self {
            len,
            type_: type_ as i8,
        }
    }
    pub fn from_raw(raw: u8) -> Self {
        let len_index = (raw >> 5) as usize;
        let len = XPAD_CI_LEN_LOOKUP.get(len_index).copied().unwrap_or(0);
        let type_ = raw & 0x1F;
        // log::debug!("XPAD RAW: len = {}, type = {}", len, type_);
        Self::new(len, type_)
    }
    pub fn reset() -> Self {
        Self { type_: -1, len: 0 }
    }

    pub fn is_valid(&self) -> bool {
        self.type_ != -1
    }
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
        if fpad_bytes.len() < 2 {
            log::warn!("PADDecoder: Missing FPAD bytes");
            return;
        }

        let used_xpad_len = xpad_bytes.len().min(64);
        let mut xpad: Vec<u8> = xpad_bytes[..used_xpad_len].iter().rev().copied().collect();

        let fpad_type = fpad_bytes[0] >> 6;
        let xpad_ind = (fpad_bytes[0] & 0x30) >> 4;
        let ci_flag = fpad_bytes[1] & 0x02 != 0;

        // log::debug!("FPAD: {} - {} - {}", fpad_type, xpad_ind, ci_flag);

        let prev_xpad_ci = self.last_xpad_ci.clone();
        self.last_xpad_ci = None;

        if fpad_type != 0b00 {
            return;
        }

        let (ci_list, ci_header_len) = if ci_flag {
            Self::build_ci_list(&xpad, fpad_bytes)
        } else if (xpad_ind == 0b01 || xpad_ind == 0b10) {
            // Continuation: Don't build new CI list, process data directly.
            if let Some(prev_ci) = &prev_xpad_ci {
                if prev_ci.is_valid() {
                    // log::debug!("Continuing previous XPAD CI: {:?}", prev_ci);
                    self.process_continuation_data(prev_ci, &xpad);
                } else {
                    log::debug!("Invalid prev_xpad_ci for continuation");
                }
            } else {
                // log::debug!("No prev_xpad_ci stored for continuation");
            }
            return; // Important: exit after processing continuation
        } else {
            return; // invalid combination, exit early
        };

        if ci_list.is_empty() {
            return;
        }

        let payload_len: usize = ci_list.iter().map(|ci| ci.len).sum();
        let announced_len = ci_header_len + payload_len;

        if announced_len != xpad.len() {
            log::warn!(
                "PADDecoder: Announced X-PAD length mismatch ({} vs {}) — discarding",
                announced_len,
                xpad.len()
            );
            return;
        }

        let mut xpad_offset = ci_header_len;
        let mut xpad_ci_type_continued: Option<i8> = None;

        // log::debug!("NUM CIs: {}", ci_list.len());

        for (i, ci) in ci_list.iter().enumerate() {
            // log::debug!("CI[{}] = type {:2}, len {:2}", i, ci.type_, ci.len);

            self.process_ci_payload(ci, &xpad[xpad_offset..xpad_offset + ci.len]);
            xpad_offset += ci.len;

            match ci.type_ {
                1 => xpad_ci_type_continued = Some(1),
                2 | 3 => xpad_ci_type_continued = Some(3),
                other_type => {
                    if self.mot_assembler.is_valid_mot_type(other_type) {
                        xpad_ci_type_continued = Some(other_type);
                    }
                }
            }
        }

        // Set up last_xpad_ci correctly for continuation next time:
        if let Some(type_cont) = xpad_ci_type_continued {
            self.last_xpad_ci = Some(XPAD_CI {
                type_: type_cont,
                len: announced_len,
            });
            // log::debug!("Updated last_xpad_ci: type={}, len={}", type_cont, announced_len);
        } else {
            log::debug!("No continuation set for last_xpad_ci");
        }

        // TODO: Feed payload data (after header) to DL/MOT assemblers
    }

    fn build_ci_list(xpad: &[u8], fpad: &[u8]) -> (Vec<XPAD_CI>, usize) {
        let mut ci_list = Vec::new();
        let mut ci_header_len = 0;

        if fpad.len() < 2 {
            return (ci_list, ci_header_len);
        }

        let fpad_type = fpad[0] >> 6;
        let xpad_ind = (fpad[0] & 0x30) >> 4;
        let ci_flag = fpad[1] & 0x02 != 0;

        if fpad_type != 0b00 || !ci_flag {
            return (ci_list, ci_header_len);
        }

        match xpad_ind {
            0b01 => {
                if xpad.len() >= 1 {
                    let type_ = xpad[0] & 0x1F;
                    if type_ != 0 {
                        ci_list.push(XPAD_CI::new(3, type_));
                        ci_header_len = 1;
                    }
                }
            }
            0b10 => {
                for &raw in xpad.iter().take(4) {
                    let type_ = raw & 0x1F;
                    ci_header_len += 1;
                    if type_ == 0 {
                        break;
                    }
                    ci_list.push(XPAD_CI::from_raw(raw));
                }
            }
            _ => {}
        }

        (ci_list, ci_header_len)
    }

    fn process_continuation_data(&mut self, prev_ci: &XPAD_CI, payload: &[u8]) {
        match prev_ci.type_ {
            2 | 3 => {
                self.dl_assembler.feed(false, payload);
            }
            other_type if self.mot_assembler.is_valid_mot_type(other_type) => {
                self.mot_assembler.feed(false, payload);

                if self.mot_assembler.is_complete() {
                    let obj = self.mot_assembler.take().unwrap();
                    println!("Received MOT object with {} bytes", obj.data.len());
                }
            }
            _ => log::warn!("Unhandled continuation CI type: {}", prev_ci.type_),
        }
    }

    fn process_ci_payload(&mut self, ci: &XPAD_CI, payload: &[u8]) {
        match ci.type_ {
            1 => {
                // DGLI (placeholder handling)
                // log::debug!("Received DGLI payload: {:02X?}", payload);
            }
            2 | 3 => {
                let is_start = ci.type_ == 2;
                self.dl_assembler.feed(is_start, payload);
            }
            other_type if self.mot_assembler.is_valid_mot_type(other_type) => {
                // let is_start = other_type % 2 == 0; // adjust condition per ETSI spec if needed
                let is_start = other_type == 12;
                self.mot_assembler.feed(is_start, payload);

                if self.mot_assembler.is_complete() {
                    let obj = self.mot_assembler.take().unwrap();
                    println!("Received MOT object with {} bytes", obj.data.len());
                }
            }
            _ => log::warn!("Unhandled CI type: {}", ci.type_),
        }
    }
}
