use crate::edi::bus::{emit_event, EDIEvent};
use derivative::Derivative;
use serde::{ser::SerializeStruct, Serialize, Serializer};
use std::fmt;

fn decode_chars(chars: &[u8], charset: u8) -> String {
    match charset {
        0xF => String::from_utf8_lossy(chars).to_string(),
        0x4 => chars.iter().map(|&b| b as char).collect(),
        0x0 => chars
            .iter()
            .map(|&b| char::from_u32(EBU_LATIN_TO_UNICODE[b as usize] as u32).unwrap_or('?'))
            .collect(),
        _ => "[unsupported charset]".into(),
    }
}

#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub struct DLObject {
    pub scid: u8,
    toggle: u8,
    #[derivative(Debug = "ignore")]
    chars: Vec<u8>,
    charset: u8,
    dl_plus_tags: Vec<DLPlusTag>,
    pub seg_count: u8,
}

impl DLObject {
    pub fn new(scid: u8, toggle: u8, charset: u8) -> Self {
        Self {
            scid,
            toggle,
            charset,
            chars: Vec::new(),
            dl_plus_tags: Vec::new(),
            seg_count: 0,
        }
    }
    pub fn decode_label(&self) -> String {
        decode_chars(&self.chars, self.charset)
    }
    pub fn get_dl_plus(&self) -> Vec<DLPlusTagDecoded> {
        let label = self.decode_label();
        let label_chars: Vec<char> = label.chars().collect();

        self.dl_plus_tags
            .iter()
            .map(|tag| {
                let start = tag.start as usize;
                let end = (start + tag.len as usize).min(label_chars.len());
                let value: String = label_chars[start..end].iter().collect();
                DLPlusTagDecoded {
                    kind: DLPlusContentType::from(tag.kind),
                    value,
                }
            })
            .collect()
    }
}

impl Serialize for DLObject {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut s = serializer.serialize_struct("DLObject", 5)?;
        s.serialize_field("scid", &self.scid)?;
        s.serialize_field("charset", &self.charset)?;

        // derived fields
        s.serialize_field("label", &self.decode_label())?;
        s.serialize_field("dl_plus", &self.get_dl_plus())?;

        s.end()
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct DLPlusTag {
    pub kind: u8,
    pub start: u8,
    pub len: u8,
}

impl DLPlusTag {
    pub fn new(kind: u8, start: u8, len: u8) -> Self {
        Self { kind, start, len }
    }
}

#[derive(Serialize, Debug)]
pub struct DLPlusTagDecoded {
    pub kind: DLPlusContentType,
    pub value: String,
}

#[derive(Debug, Serialize, Clone, Copy)]
#[repr(u8)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DLPlusContentType {
    ItemTitle = 1,
    ItemArtist = 4,
    ItemAlbum = 2,
    // TODO: complete options...
    Unknown(u8),
}

impl From<u8> for DLPlusContentType {
    fn from(value: u8) -> Self {
        match value {
            1 => DLPlusContentType::ItemTitle,
            2 => DLPlusContentType::ItemAlbum,
            4 => DLPlusContentType::ItemArtist,
            _ => DLPlusContentType::Unknown(value),
        }
    }
}

impl fmt::Display for DLPlusContentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DLPlusContentType::ItemTitle => write!(f, "ITEM_TITLE"),
            DLPlusContentType::ItemArtist => write!(f, "ITEM_ARTIST"),
            DLPlusContentType::ItemAlbum => write!(f, "ITEM_ALBUM"),
            DLPlusContentType::Unknown(v) => write!(f, "UNKNOWN_{}", v),
        }
    }
}

#[derive(Debug)]
pub struct DLDecoder {
    scid: u8,
    current: Option<DLObject>,
    last_toggle: Option<u8>,
}

impl DLDecoder {
    pub fn new(scid: u8) -> Self {
        Self {
            scid,
            current: None,
            last_toggle: None,
        }
    }

    pub fn feed(&mut self, data: &[u8]) -> Option<Vec<u8>> {
        if data.len() < 2 {
            return None;
        }

        let flags = data[0];
        let num_chars = (flags & 0x0F) + 1;
        let is_first = flags & 0x40 != 0;
        let is_last = flags & 0x20 != 0;
        let toggle = (flags & 0x80) >> 7;

        match (data[0] & 0x10 != 0, data[0] & 0x0F) {
            (true, 0b0001) => {
                log::debug!("Clear display command");
                // TODO: implement reset
            }
            (true, 0b0010) => {
                // TODO: abort if t != toggle
                let _t = data[0] & 0x80;

                if data.len() < 3 {
                    log::warn!("DL+: too short: expected min 3 bytes, got {}", data.len());
                    return None;
                }

                self.parse_dl_plus(&data[2..]);

                // do not continue on DL+ command
                return None;
            }
            (true, _) => {
                log::debug!("Unexpected DL command: 0x{:02X}", data[0]);
            }
            _ => {
                // not a DL+ or display-clear command
            }
        }

        let nibble = (data[1] >> 4) & 0x0F;
        let (_seg_no, charset) = if is_first {
            (0, Some(nibble)) // charset = full 4 bits
        } else {
            (nibble & 0x07, None) // charset not in data
        };

        // log::debug!("DL: seg = {}", seg_no);

        if is_first {
            self.flush();

            self.current = Some(DLObject::new(self.scid, toggle, charset.unwrap_or(0)));
        }

        let start = 2;
        let end = start + num_chars as usize;
        if data.len() >= end {
            // self.current.chars.extend_from_slice(&data[start..end]);
            if let Some(current) = self.current.as_mut() {
                current.chars.extend_from_slice(&data[start..end]);
            }
        } else {
            log::warn!(
                "DL: segment too short: expected {} bytes, got {}",
                end,
                data.len()
            );
            return None;
        }

        // log::debug!("DL current chars: {:?}", self.current.chars.len());

        if is_last {
            // log::debug!("DL: {}", self.current.decode_label());
            // self.reset();
        }

        None
    }

    pub fn parse_dl_plus(&mut self, data: &[u8]) {
        if data.is_empty() {
            log::warn!("DL+: empty command");
            return;
        }

        // NOTE: there is a bug when pad is short (6 or 8)

        let cid = (data[0] >> 4) & 0x0F;

        if cid != 0 {
            log::debug!("DL+: unsupported command ID = {}", cid);
            return;
        }

        // log::debug!("DL Plus: {:?}", cid);

        let _cb = data[0] & 0x0F;
        let _it_toggle = (data[0] >> 3) & 0x01;
        let _it_running = (data[0] >> 2) & 0x01;
        let num_tags = (data[0] & 0x03) + 1;

        // log::debug!("DL+: CID = {}, CB = {}, tags = {} # {} bytes", cid, cb, num_tags, data.len());

        // if data.len() < 0 + num_tags as usize * 3 {
        if data.len() < 1 + num_tags as usize * 3 {
            log::debug!(
                "DL+: unexpected length, expected at least {}",
                1 + num_tags * 3
            );
            return;
        }

        for i in 0..num_tags {
            let base = 1 + (i * 3) as usize;
            let content_type = data[base] & 0x7F;
            let start = data[base + 1] & 0x7F;
            let len = (data[base + 2] & 0x7F) + 1;

            let tag = DLPlusTag::new(content_type, start, len);

            // log::debug!(
            //     "DL+ tag: {:?}", tag
            // );

            if let Some(current) = self.current.as_mut() {
                current.dl_plus_tags.push(tag);
            }
        }

        // log::debug!("DL+ it_toggle={}, it_running={}", it_toggle, it_running);
    }

    pub fn flush(&mut self) {
        if let Some(current) = self.current.take() {
            if !current.chars.is_empty() && self.last_toggle != Some(current.toggle) {
                // log::debug!("DL: {} - {:?}", current.decode_label(), current);

                // log::debug!("{:?}", current.get_dl_plus());

                // let json = serde_json::to_string_pretty(&current).unwrap();
                // println!("{}", json);

                emit_event(EDIEvent::DLObjectReceived(current.clone()));
                self.last_toggle = Some(current.toggle);
            }
        }
    }
}

static EBU_LATIN_TO_UNICODE: [u16; 256] = [
    0x0000, 0x0118, 0x012E, 0x0172, 0x0102, 0x0116, 0x010E, 0x0218, 0x021A, 0x010A, 0x000A, 0x000B,
    0x0120, 0x0139, 0x017B, 0x0143, 0x0105, 0x0119, 0x012F, 0x0173, 0x0103, 0x0117, 0x010F, 0x0219,
    0x021B, 0x010B, 0x0147, 0x011A, 0x0121, 0x013A, 0x017C, 0x001F, 0x0020, 0x0021, 0x0022, 0x0023,
    0x0142, 0x0025, 0x0026, 0x0027, 0x0028, 0x0029, 0x002A, 0x002B, 0x002C, 0x002D, 0x002E, 0x002F,
    0x0030, 0x0031, 0x0032, 0x0033, 0x0034, 0x0035, 0x0036, 0x0037, 0x0038, 0x0039, 0x003A, 0x003B,
    0x003C, 0x003D, 0x003E, 0x003F, 0x0040, 0x0041, 0x0042, 0x0043, 0x0044, 0x0045, 0x0046, 0x0047,
    0x0048, 0x0049, 0x004A, 0x004B, 0x004C, 0x004D, 0x004E, 0x004F, 0x0050, 0x0051, 0x0052, 0x0053,
    0x0054, 0x0055, 0x0056, 0x0057, 0x0058, 0x0059, 0x005A, 0x005B, 0x016E, 0x005D, 0x0141, 0x005F,
    0x0104, 0x0061, 0x0062, 0x0063, 0x0064, 0x0065, 0x0066, 0x0067, 0x0068, 0x0069, 0x006A, 0x006B,
    0x006C, 0x006D, 0x006E, 0x006F, 0x0070, 0x0071, 0x0072, 0x0073, 0x0074, 0x0075, 0x0076, 0x0077,
    0x0078, 0x0079, 0x007A, 0x00AB, 0x016F, 0x00BB, 0x013D, 0x0126, 0x00E1, 0x00E0, 0x00E9, 0x00E8,
    0x00ED, 0x00EC, 0x00F3, 0x00F2, 0x00FA, 0x00F9, 0x00D1, 0x00C7, 0x015E, 0x00DF, 0x00A1, 0x0178,
    0x00E2, 0x00E4, 0x00EA, 0x00EB, 0x00EE, 0x00EF, 0x00F4, 0x00F6, 0x00FB, 0x00FC, 0x00F1, 0x00E7,
    0x015F, 0x011F, 0x0131, 0x00FF, 0x0136, 0x0145, 0x00A9, 0x0122, 0x011E, 0x011B, 0x0148, 0x0151,
    0x0150, 0x20AC, 0x00A3, 0x0024, 0x0100, 0x0112, 0x012A, 0x016A, 0x0137, 0x0146, 0x013B, 0x0123,
    0x013C, 0x0130, 0x0144, 0x0171, 0x0170, 0x00BF, 0x013E, 0x00B0, 0x0101, 0x0113, 0x012B, 0x016B,
    0x00C1, 0x00C0, 0x00C9, 0x00C8, 0x00CD, 0x00CC, 0x00D3, 0x00D2, 0x00DA, 0x00D9, 0x0158, 0x010C,
    0x0160, 0x017D, 0x00D0, 0x013F, 0x00C2, 0x00C4, 0x00CA, 0x00CB, 0x00CE, 0x00CF, 0x00D4, 0x00D6,
    0x00DB, 0x00DC, 0x0159, 0x010D, 0x0161, 0x017E, 0x0111, 0x0140, 0x00C3, 0x00C5, 0x00C6, 0x0152,
    0x0177, 0x00DD, 0x00D5, 0x00D8, 0x00DE, 0x014A, 0x0154, 0x0106, 0x015A, 0x0179, 0x0164, 0x00F0,
    0x00E3, 0x00E5, 0x00E6, 0x0153, 0x0175, 0x00FD, 0x00F5, 0x00F8, 0x00FE, 0x014B, 0x0155, 0x0107,
    0x015B, 0x017A, 0x0165, 0x0127,
];
