use derivative::Derivative;
use log;
use std::collections::BTreeMap;
use thiserror::Error;

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

    fn is_valid_mot_type(&self, kind: i8) -> bool {
        // Check if kind is a valid MOT type (using ETSI EN 301 234)
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

const XPADCI_LEN_LOOKUP: [usize; 8] = [4, 6, 8, 12, 16, 24, 32, 48];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XPADCI {
    pub kind: i8,
    pub len: usize,
}

impl XPADCI {
    pub fn new(len: usize, kind: u8) -> Self {
        Self {
            len,
            kind: kind as i8,
        }
    }
    pub fn from_raw(raw: u8) -> Self {
        let len_index = (raw >> 5) as usize;
        let len = XPADCI_LEN_LOOKUP.get(len_index).copied().unwrap_or(0);
        let kind = raw & 0x1F;
        // log::debug!("XPAD RAW: len = {}, type = {}", len, kind);
        Self::new(len, kind)
    }
    pub fn reset() -> Self {
        Self { kind: -1, len: 0 }
    }

    pub fn is_valid(&self) -> bool {
        self.kind != -1
    }
}

#[derive(Debug)]
pub struct DLDataGroup {
    pub size_needed: usize,
    pub data: Vec<u8>,
}

impl DLDataGroup {
    fn new() -> Self {
        Self {
            size_needed: 2 + 2, // default minimum: header + CRC
            data: Vec::new(),
        }
    }
    fn init(&mut self) {
        self.size_needed = 2 + 2; // default minimum: header + CRC
        self.data.clear();
    }

    pub fn feed(&mut self, input: &[u8]) -> Option<Vec<u8>> {
        let remaining = self.size_needed.saturating_sub(self.data.len());
        self.data.extend_from_slice(&input[..input.len().min(remaining)]);

        // once we have the 2-byte header, compute actual size
        if self.data.len() >= 2 && self.size_needed == 4 {
            let is_command = self.data[0] & 0x10 != 0;
            let field_len = if is_command {
                match self.data[0] & 0x0F {
                    0x01 => 0, // Remove label
                    0x02 => (self.data.get(1).cloned().unwrap_or(0) & 0x0F) + 1, // DL+
                    _ => 0,
                }
            } else {
                (self.data[0] & 0x0F) + 1
            };
            self.size_needed = 2 + field_len as usize + 2; // 2 header + data + 2 CRC
        }

        if self.data.len() == self.size_needed {
            let mut complete = Vec::new();
            std::mem::swap(&mut complete, &mut self.data);
            Some(complete)
        } else {
            None
        }
    }

    fn __feed(&mut self, data: &[u8]) -> Option<Vec<u8>> {
        // TODO: implement feed logic
        None
    }
}

#[derive(Debug)]
pub struct MOTDataGroup {
    pub size_needed: usize,
    pub data: Vec<u8>,
}

impl MOTDataGroup {
    fn new() -> Self {
        Self {
            size_needed: 0,
            data: Vec::new(),
        }
    }
    fn init(&mut self, size: usize) {
        self.size_needed = size;
        self.data.clear();
    }

    fn feed(&mut self, data: &[u8]) -> Option<Vec<u8>> {
        let remaining = self.size_needed.saturating_sub(self.data.len());
        self.data.extend_from_slice(&data[..data.len().min(remaining)]);

        if self.data.len() == self.size_needed {
            let mut dg = Vec::new();
            std::mem::swap(&mut self.data, &mut dg);
            Some(dg)
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct PADDecoder {
    scid: u8,
    last_xpad_ci: Option<XPADCI>,
    //
    next_dg_size: usize,
    //
    dl_dg: DLDataGroup,
    mot_dg: MOTDataGroup,
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
            next_dg_size: 0,
            //
            dl_dg: DLDataGroup::new(),
            mot_dg: MOTDataGroup::new(),
            //
            dl_assembler: DLAssembler::new(),
            mot_assembler: MOTAssembler::new(),
        }
    }
    pub fn feed(&mut self, fpad_bytes: &[u8], xpad_bytes: &[u8]) {
        if fpad_bytes.len() < 2 {
            log::warn!("PADDecoder: Missing FPAD bytes");
            return;
        }

        let used_xpad_len = xpad_bytes.len().min(64);
        let mut xpad: Vec<u8> = xpad_bytes[..used_xpad_len].iter().rev().copied().collect();

        let fpad_type = fpad_bytes[0] >> 6;
        let xpad_ind = (fpad_bytes[0] & 0x30) >> 4;
        let ci_flag = fpad_bytes[1] & 0x02 != 0;

        let prev_xpad_ci = self.last_xpad_ci.clone();
        self.last_xpad_ci = None;

        if fpad_type != 0b00 {
            return;
        }

        let (ci_list, ci_header_len) = if ci_flag {
            Self::build_ci_list(&xpad, fpad_bytes)
        } else if (xpad_ind == 0b01 || xpad_ind == 0b10) {
            if let Some(prev_ci) = &prev_xpad_ci {
                if prev_ci.is_valid() {
                    (vec![prev_ci.clone()], 0)
                } else {
                    return;
                }
            } else {
                return;
            }
        } else {
            return;
        };

        // if ci_list.is_empty() {
        //     return;
        // }

        if ci_list.is_empty() {
            if let Some(prev_ci) = prev_xpad_ci {
                self.last_xpad_ci = Some(prev_ci);
            }
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

        let mut offset = ci_header_len;
        let mut ci_kind_continued: Option<i8> = None;

        // log::debug!("NUM CIs: {}", ci_list.len());

        for (i, ci) in ci_list.iter().enumerate() {
            // log::debug!("CI = type {:2}, len {:2}", ci.kind, ci.len);
            self.process_ci(false, ci, &xpad[offset..offset + ci.len]);
            offset += ci.len;

            match ci.kind {
                1 => ci_kind_continued = Some(1),
                2 | 3 => ci_kind_continued = Some(3),
                12 | 13 => ci_kind_continued = Some(13),
                _ => {}
            }
        }

        // Set up last_xpad_ci for continuation next time:
        if let Some(kind) = ci_kind_continued {
            self.last_xpad_ci = Some(XPADCI {
                kind: kind,
                len: announced_len,
            });
            // log::debug!("Updated last_xpad_ci: type={}, len={}", kindcont, announced_len);
        } else {
            log::debug!("No continuation set for last_xpad_ci");
        }
    }

    fn build_ci_list(xpad: &[u8], fpad: &[u8]) -> (Vec<XPADCI>, usize) {
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
                // short format: 1 byte
                if xpad.len() >= 1 {
                    let kind = xpad[0] & 0x1F;
                    if kind != 0 {
                        ci_list.push(XPADCI::new(3, kind));
                        ci_header_len = 1;
                    }
                }
            }
            0b10 => {
                // long format: multiple CIs
                for &raw in xpad.iter().take(4) {
                    let kind = raw & 0x1F;
                    ci_header_len += 1;
                    if kind == 0 {
                        break;
                    }
                    ci_list.push(XPADCI::from_raw(raw));
                }
            }
            _ => {}
        }

        (ci_list, ci_header_len)
    }

    fn process_ci(&mut self, is_continuation: bool, ci: &XPADCI, payload: &[u8]) {
        // log::debug!("CI: kind: {}", ci.kind);
        match ci.kind {
            1 => {
                // DGLI - Data Group Length Indicator
                let dg_size = ((payload[0] & 0x3F) as u16) << 8 | payload[1] as u16;
                // log::debug!("DGLI: len: {}", dg_size);
                self.next_dg_size = dg_size as usize;
            }
            2 | 3 => {
                let is_start = ci.kind == 2 && !is_continuation;

                // log::debug!("CI: kind: {} - {} bytes", ci.kind, ci.len);

                if is_start && self.dl_dg.data.is_empty() {
                    self.dl_dg.init();
                }

                if let Some(dg_data) = self.dl_dg.feed(&payload) {
                    // log::debug!("DL Data Group complete: {} bytes", dg_data.len());
                    log::debug!("DL Data Group complete: {:?}", dg_data);
                    // self.dl_assembler.feed(&dg_data);
                }
            }
            12 | 13 => {
                // log::debug!("CI: kind: {} - {} bytes", ci.kind, ci.len);

                let is_start = ci.kind == 12 && !is_continuation;
                if is_start {
                    // MOT start. initialize DG
                    self.mot_dg.init(self.next_dg_size);
                    self.next_dg_size = 0;
                }

                if let Some(dg_data) = self.mot_dg.feed(&payload) {
                    // log::debug!("MOT Data Group complete: {} bytes", dg_data.len());
                    // self.mot_assembler.feed(&dg_data);
                }


            }
            _ => log::warn!("Unhandled CI type: {}", ci.kind),
        }
    }
}
