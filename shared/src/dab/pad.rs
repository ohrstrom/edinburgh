pub mod dl;
pub mod mot;

use derive_more::Debug;
use log;

use dl::DlDecoder;
use mot::MotDecoder;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum XPadIndicator {
    Short,
    Variable,
    Extended,
    Reserved,
}

#[derive(Debug, Clone, Copy)]
pub struct FPad {
    pub ci_flag: bool,
    pub xpad_indicator: XPadIndicator,
}

impl From<u8> for FPad {
    fn from(byte: u8) -> Self {
        let ci_flag = byte & 0b1000_0000 != 0;
        let indicator_bits = (byte >> 5) & 0b11;
        let xpad_indicator = match indicator_bits {
            0b00 => XPadIndicator::Short,
            0b01 => XPadIndicator::Variable,
            0b10 => XPadIndicator::Extended,
            _ => XPadIndicator::Reserved,
        };
        FPad {
            ci_flag,
            xpad_indicator,
        }
    }
}

const XPADCI_LEN_LOOKUP: [usize; 8] = [4, 6, 8, 12, 16, 24, 32, 48];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XPadCI {
    pub kind: i8,
    pub len: usize,
}

impl XPadCI {
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
pub struct MscDataGroup {
    pub is_valid: bool,
    pub extension_flag: bool,
    pub segment_flag: bool,
    pub user_access_flag: bool,
    pub seg_type: u8,
    pub continuity_index: u8,
    pub repetition_index: u8,
    pub extension_field: Option<u16>,
    pub last_flag: bool,
    pub segment_num: Option<u16>,
    pub transport_id_flag: bool,
    pub length_indicator: u8,
    pub transport_id: Option<u16>,
    pub end_user_addr_field: Vec<u8>,
    #[debug("{} bytes", data_field.len())]
    pub data_field: Vec<u8>,
}

impl MscDataGroup {
    pub fn from_bytes(data: &[u8]) -> Self {
        let mut dg = MscDataGroup {
            is_valid: false,
            extension_flag: false,
            segment_flag: false,
            user_access_flag: false,
            seg_type: 0,
            continuity_index: 0,
            repetition_index: 0,
            extension_field: None,
            last_flag: false,
            segment_num: None,
            transport_id_flag: false,
            length_indicator: 0,
            transport_id: None,
            end_user_addr_field: Vec::new(),
            data_field: Vec::new(),
        };

        if data.len() < 2 {
            return dg; // invalid: too short
        }

        let mut idx = 0;

        let header = data[idx];
        idx += 1;

        let crc_flag = header & 0x40 != 0;
        dg.extension_flag = header & 0x80 != 0;
        dg.segment_flag = header & 0x20 != 0;
        dg.user_access_flag = header & 0x10 != 0;
        dg.seg_type = header & 0x0F;

        let second_byte = data[idx];
        idx += 1;

        dg.continuity_index = (second_byte >> 4) & 0x0F;
        dg.repetition_index = second_byte & 0x0F;

        if dg.extension_flag {
            if data.len() < idx + 2 {
                return dg;
            }
            dg.extension_field = Some(((data[idx] as u16) << 8) | data[idx + 1] as u16);
            idx += 2;
        }

        if dg.segment_flag {
            if data.len() < idx + 2 {
                return dg;
            }
            let high = data[idx] as u16 & 0x7F;
            let low = data[idx + 1] as u16;
            dg.last_flag = data[idx] & 0x80 != 0;
            dg.segment_num = Some((high << 8) | low);
            idx += 2;
        }

        if dg.user_access_flag {
            let byte = data[idx];
            idx += 1;

            dg.transport_id_flag = byte & 0x10 != 0;
            dg.length_indicator = byte & 0x0F;

            if dg.transport_id_flag {
                if data.len() < idx + 2 {
                    return dg;
                }
                dg.transport_id = Some(((data[idx] as u16) << 8) | data[idx + 1] as u16);
                idx += 2;
            }

            let transport_id_len = if dg.transport_id_flag { 2 } else { 0 };
            let address_len = (dg.length_indicator as usize).saturating_sub(transport_id_len);

            if address_len > 0 && data.len() >= idx + address_len {
                dg.end_user_addr_field = data[idx..idx + address_len].to_vec();
                idx += address_len;
            }
        }

        // at this point, idx is at the start of the data field
        let crc_len = if crc_flag { 2 } else { 0 };
        if data.len() >= idx + crc_len {
            let data_field_len = data.len() - idx - crc_len;

            // should we remove first 2 bytes of data first?
            //       they contain segmentation metadata.

            dg.data_field = data[idx..idx + data_field_len].to_vec();
        } else {
            log::warn!("MscDataGroup: Not enough data for data field");
        }

        dg.is_valid = true; // this should be checked ;)
        dg
    }
}

#[derive(Debug)]
pub struct DlDataGroup {
    pub size_needed: usize,
    pub data: Vec<u8>,
}

impl DlDataGroup {
    fn new() -> Self {
        Self {
            size_needed: 2 + 2, // default minimum: header + CRC
            data: Vec::new(),
        }
    }
    pub fn feed(&mut self, payload: &[u8]) -> Option<Vec<u8>> {
        self.data.extend_from_slice(payload);

        let field_len = (self.data[0] & 0x0F) + 1;
        self.size_needed = 2 + field_len as usize + 2;

        if self.data.len() >= self.size_needed {
            let mut complete = Vec::new();
            std::mem::swap(&mut complete, &mut self.data);
            Some(complete)
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct MotDataGroup {
    pub size_needed: usize,
    pub data: Vec<u8>,
}

impl MotDataGroup {
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

    fn feed(&mut self, data: &[u8]) -> Option<MscDataGroup> {
        let remaining = self.size_needed.saturating_sub(self.data.len());
        self.data
            .extend_from_slice(&data[..data.len().min(remaining)]);

        if self.data.len() == self.size_needed {
            let dg = MscDataGroup::from_bytes(&self.data);
            self.data.clear();
            Some(dg)
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct PadDecoder {
    #[allow(dead_code)]
    scid: u8,
    last_xpad_ci: Option<XPadCI>,
    next_dg_size: usize,
    dl_dg: DlDataGroup,
    mot_dg: MotDataGroup,
    dl_decoder: DlDecoder,
    mot_decoder: MotDecoder,
}

impl PadDecoder {
    pub fn new(scid: u8) -> Self {
        Self {
            scid,
            last_xpad_ci: None,
            next_dg_size: 0,
            dl_dg: DlDataGroup::new(),
            mot_dg: MotDataGroup::new(),
            dl_decoder: DlDecoder::new(scid),
            mot_decoder: MotDecoder::new(scid),
        }
    }
    pub fn feed(&mut self, fpad_bytes: &[u8], xpad_bytes: &[u8]) {
        if fpad_bytes.len() < 2 {
            log::warn!("PadDecoder: Missing FPAD bytes");
            return;
        }

        let used_xpad_len = xpad_bytes.len().min(64);
        let xpad: Vec<u8> = xpad_bytes[..used_xpad_len].iter().rev().copied().collect();

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
        } else if xpad_ind == 0b01 || xpad_ind == 0b10 {
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
                "PadDecoder: Announced X-PAD length mismatch ({} vs {}) — discarding",
                announced_len,
                xpad.len()
            );
            return;
        }

        let mut offset = ci_header_len;
        let mut ci_kind_continued: Option<i8> = None;

        for ci in ci_list.iter() {
            self.process_ci(false, ci, &xpad[offset..offset + ci.len]);
            offset += ci.len;

            match ci.kind {
                1 => ci_kind_continued = Some(1),
                2 | 3 => ci_kind_continued = Some(3),
                12 | 13 => ci_kind_continued = Some(13),
                _ => {}
            }
        }

        // set up last_xpad_ci for continuation next time:
        if let Some(kind) = ci_kind_continued {
            self.last_xpad_ci = Some(XPadCI {
                kind,
                len: announced_len,
            });
        } else {
            log::debug!("No continuation set for last_xpad_ci");
        }
    }

    fn build_ci_list(xpad: &[u8], fpad: &[u8]) -> (Vec<XPadCI>, usize) {
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
                if !xpad.is_empty() {
                    let kind = xpad[0] & 0x1F;
                    if kind != 0 {
                        ci_list.push(XPadCI::new(3, kind));
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
                    ci_list.push(XPadCI::from_raw(raw));
                }
            }
            _ => {}
        }

        (ci_list, ci_header_len)
    }

    fn process_ci(&mut self, is_continuation: bool, ci: &XPadCI, payload: &[u8]) {
        match ci.kind {
            1 => {
                // DGLI - Data Group Length Indicator
                let dg_size = ((payload[0] & 0x3F) as u16) << 8 | payload[1] as u16;
                self.next_dg_size = dg_size as usize;
            }
            2 | 3 => {
                // log::debug!("CI: kind: {} - {} bytes - data: {:?}", ci.kind, ci.len, payload);

                /*
                let is_start = ci.kind == 2 && !is_continuation;

                if is_start && self.dl_dg.data.is_empty() {
                    log::debug!("DG: init");
                    self.dl_dg.init();
                }
                */

                let _is_start = ci.kind == 2;

                if let Some(data) = self.dl_dg.feed(payload) {
                    self.dl_decoder.feed(&data);
                }
            }
            12 | 13 => {

                let is_start = ci.kind == 12 && !is_continuation;
                if is_start {
                    // MOT start. initialize DG
                    self.mot_dg.init(self.next_dg_size);
                    self.next_dg_size = 0;
                }

                if let Some(dg) = self.mot_dg.feed(payload) {
                    self.mot_decoder.feed(&dg);
                }
            }
            _ => log::warn!("Unhandled CI type: {}", ci.kind),
        }
    }
}
