use crate::dab::bus::{emit_event, DabEvent};
use crate::dab::tables::EBU_LATIN_TO_UNICODE;
use derive_more::Debug;
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

#[derive(Debug, Clone)]
pub struct DlObject {
    pub scid: u8,
    toggle: u8,
    #[debug(skip)]
    chars: Vec<u8>,
    charset: u8,
    #[debug("{} tags", dl_plus_tags.len())]
    dl_plus_tags: Vec<DlPlusTag>,
    pub seg_count: u8,
}

impl DlObject {
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
    pub fn is_dl_plus(&self) -> bool {
        !self.dl_plus_tags.is_empty()
    }
    pub fn get_dl_plus(&self) -> Vec<DlPlusTagDecoded> {
        let label = self.decode_label();
        let label_chars: Vec<char> = label.chars().collect();

        /*
        // not safe: slice index starts at 21 but ends at 20
        self.dl_plus_tags
            .iter()
            .map(|tag| {
                let start = tag.start as usize;
                let end = (start + tag.len as usize).min(label_chars.len());
                let value: String = label_chars[start..end].iter().collect();
                DlPlusTagDecoded {
                    kind: DlPlusContentType::from(tag.kind),
                    value,
                }
            })
            .collect()
        */

        let len = label_chars.len();
        self.dl_plus_tags
            .iter()
            .filter_map(|tag| {
                let kind = DlPlusContentType::from(tag.kind);

                if matches!(kind, DlPlusContentType::Dummy) {
                    log::debug!("Ignoring DL+ tag with kind: {}", kind);
                    return None;
                }

                let start = tag.start as usize;

                if start >= len {
                    log::warn!("DL+ tag start {} >= len {}", start, len);
                    return None;
                }

                let mut end = start.saturating_add(tag.len as usize);
                if end > len {
                    end = len;
                }

                if end <= start {
                    return None;
                }

                let value: String = label_chars[start..end].iter().collect();

                Some(DlPlusTagDecoded { kind, value })
            })
            .collect()
    }
}

impl Serialize for DlObject {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut s = serializer.serialize_struct("DlObject", 5)?;
        s.serialize_field("scid", &self.scid)?;
        s.serialize_field("charset", &self.charset)?;

        // derived fields
        s.serialize_field("label", &self.decode_label())?;
        s.serialize_field("dl_plus", &self.get_dl_plus())?;

        s.end()
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct DlPlusTag {
    pub kind: u8,
    pub start: u8,
    pub len: u8,
}

impl DlPlusTag {
    pub fn new(kind: u8, start: u8, len: u8) -> Self {
        Self { kind, start, len }
    }
}

#[derive(Serialize, Debug)]
pub struct DlPlusTagDecoded {
    pub kind: DlPlusContentType,
    pub value: String,
}

#[derive(Debug, Serialize, Clone, Copy)]
#[repr(u8)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DlPlusContentType {
    Dummy = 0,
    ItemTitle = 1,
    ItemAlbum = 2,
    ItemArtist = 4,
    StationnameLong = 32,
    // TODO: complete options...
    Unknown(u8),
}

impl From<u8> for DlPlusContentType {
    fn from(value: u8) -> Self {
        match value {
            0 => DlPlusContentType::Dummy,
            1 => DlPlusContentType::ItemTitle,
            2 => DlPlusContentType::ItemAlbum,
            4 => DlPlusContentType::ItemArtist,
            32 => DlPlusContentType::StationnameLong,
            _ => DlPlusContentType::Unknown(value),
        }
    }
}

impl fmt::Display for DlPlusContentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DlPlusContentType::Dummy => write!(f, "DUMMY"),
            DlPlusContentType::ItemTitle => write!(f, "ITEM_TITLE"),
            DlPlusContentType::ItemArtist => write!(f, "ITEM_ARTIST"),
            DlPlusContentType::ItemAlbum => write!(f, "ITEM_ALBUM"),
            DlPlusContentType::StationnameLong => write!(f, "STATIONNAME_LONG"),
            DlPlusContentType::Unknown(v) => write!(f, "UNKNOWN_{}", v),
        }
    }
}

#[derive(Debug)]
pub struct DlDecoder {
    scid: u8,
    current: Option<DlObject>,
    last_toggle: Option<u8>,
}

impl DlDecoder {
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
                log::debug!("[{:2}] DL: CMD clear display", self.scid);
                // TODO: implement reset
            }
            (true, 0b0010) => {
                // TODO: abort if t != toggle
                let _t = data[0] & 0x80;

                if data.len() < 3 {
                    log::warn!(
                        "[{:2}] DL+ too short: expected min 3 bytes, got {}",
                        self.scid,
                        data.len()
                    );
                    return None;
                }

                self.parse_dl_plus(&data[2..]);

                // do not continue on DL+ command
                return None;
            }
            (true, _) => {
                log::debug!(
                    "[{:2}] DL: unexpected command: 0x{:02X}",
                    self.scid,
                    data[0]
                );
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

            self.current = Some(DlObject::new(self.scid, toggle, charset.unwrap_or(0)));
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
                "[{:2}] DL: segment too short: expected {} bytes, got {}",
                self.scid,
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
            log::warn!("[{:2}] DL+ empty command", self.scid);
            return;
        }

        // there is a bug when pad is short (6 or 8)

        let cid = (data[0] >> 4) & 0x0F;

        if cid != 0 {
            log::debug!("[{:2}] DL+ unexpected command ID = {}", self.scid, cid);
            return;
        }

        // log::debug!("DL Plus: {:?}", cid);

        let _cb = data[0] & 0x0F;
        let _it_toggle = (data[0] >> 3) & 0x01;
        let _it_running = (data[0] >> 2) & 0x01;
        let num_tags = (data[0] & 0x03) + 1;

        // log::debug!("DL+ CID = {}, CB = {}, tags = {} # {} bytes", cid, cb, num_tags, data.len());

        // if data.len() < 0 + num_tags as usize * 3 {
        if data.len() < 1 + num_tags as usize * 3 {
            log::debug!(
                "[{:2}] DL+ unexpected length, expected at least {}",
                self.scid,
                1 + num_tags * 3
            );
            return;
        }

        for i in 0..num_tags {
            let base = 1 + (i * 3) as usize;
            let content_type = data[base] & 0x7F;
            let start = data[base + 1] & 0x7F;
            let len = (data[base + 2] & 0x7F) + 1;

            let tag = DlPlusTag::new(content_type, start, len);

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
                log::debug!(
                    "[{:2}] DL: {} - {:?}",
                    self.scid,
                    current.decode_label(),
                    current
                );

                // log::debug!("[{:2}] DL{} {}", self.scid, if !current.dl_plus_tags.is_empty() {"+"} else {" "}, current.decode_label());

                // log::debug!("{:?}", current.get_dl_plus());

                // let json = serde_json::to_string_pretty(&current).unwrap();
                // println!("{}", json);

                emit_event(DabEvent::DlObjectReceived(current.clone()));
                self.last_toggle = Some(current.toggle);
            }
        }
    }
}
