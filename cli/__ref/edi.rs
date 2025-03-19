use std::cell::RefCell;
use std::fmt::{self, format};
use std::ops::Deref;

use log::{debug, error, info, warn};

use crate::audio::AudioDecoder;
use crate::fic::FICDecoder;
use crate::utils::{calc_crc16_ccitt, calc_crc_fire_code};

#[derive(Debug)]
pub struct FrameDecodeError(pub String);

impl fmt::Display for FrameDecodeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "FrameDecodeError: {}", self.0)
    }
}

#[derive(Debug)]
pub struct UnsupportedTagError(pub String);

impl fmt::Display for UnsupportedTagError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "UnsupportedTagError: {}", self.0)
    }
}

#[derive(Debug)]
pub struct TagDecodeError(pub String);

impl fmt::Display for TagDecodeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "TagDecodeError: {}", self.0)
    }
}

impl std::error::Error for FrameDecodeError {}
impl std::error::Error for UnsupportedTagError {}
impl std::error::Error for TagDecodeError {}

#[derive(Debug)]
struct SyncMagic {
    pattern: Vec<u8>,
    name: String,
}

impl SyncMagic {
    fn new(pattern: Vec<u8>, name: impl Into<String>) -> Self {
        Self {
            pattern,
            name: name.into(),
        }
    }

    fn matches(&self, data: &[u8]) -> bool {
        data.starts_with(&self.pattern)
    }
}

#[derive(Debug)]
pub struct TagBase {
    name: String,
    len: usize,
    header: Vec<u8>,
    value: Vec<u8>,
    // NOTE: not sure if this is a good idea...
    //       as decoded_value currently this is either FIC or MSTn
    decoded_value: Vec<u8>,
}

#[derive(Debug)]
pub enum Tag {
    _PTR(TagBase),
    _DMY(TagBase),
    DETI(TagBase),
    ESTn(TagBase),
    INFO(TagBase),
    NASC(TagBase),
    FRPD(TagBase),
}

impl Deref for Tag {
    type Target = TagBase;

    fn deref(&self) -> &Self::Target {
        match self {
            Tag::_PTR(tag)
            | Tag::_DMY(tag)
            | Tag::DETI(tag)
            | Tag::ESTn(tag)
            | Tag::INFO(tag)
            | Tag::NASC(tag)
            | Tag::FRPD(tag) => tag,
        }
    }
}

impl fmt::Display for Tag {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let kind = match self {
            Tag::_PTR(_) => "*PTR",
            Tag::_DMY(_) => "*DMY",
            Tag::DETI(_) => "DETI",
            Tag::ESTn(_) => "ESTn",
            Tag::INFO(_) => "INFO",
            Tag::NASC(_) => "NASC",
            Tag::FRPD(_) => "FRPD",
        };
        // let v = &self.value[0..4];
        write!(f, "{}: {:>8} bytes", kind, self.len)
    }
}

impl Tag {
    // fn from_bytes(data: &[u8]) -> Self {
    fn from_bytes(data: &[u8]) -> Result<Self, UnsupportedTagError> {
        let name = match std::str::from_utf8(&data[0..4]) {
            Ok(name) => name.to_string(),
            // Err(_) => return Err(UnsupportedTagError("invalid tag name UTF-8".to_string()))
            Err(_) => {
                return Err(UnsupportedTagError(format!(
                    "invalid tag name UTF-8: {:?}",
                    &data[0..4]
                )))
            }
        };

        let len = ((data[4] as usize) << 24)
            | ((data[5] as usize) << 16)
            | ((data[6] as usize) << 8)
            | (data[7] as usize);

        // let value = data[8..].to_vec();
        let header = data[0..8].to_vec();
        let value = data[8..].to_vec();
        let decoded_value = Vec::new();

        let base = TagBase {
            name,
            len,
            header,
            value,
            decoded_value,
        };

        match base.name.as_str() {
            // straight name based
            "*ptr" => Ok(Tag::_PTR(base)),
            "*dmy" => Ok(Tag::_DMY(base)),
            "deti" => Ok(Tag::DETI(base)),
            "info" => Ok(Tag::INFO(base)),
            "nasc" => Ok(Tag::NASC(base)),
            "frpd" => Ok(Tag::FRPD(base)),
            // pattern based
            name if name.starts_with("est") => Ok(Tag::ESTn(base)),
            // unsupported
            _ => Err(UnsupportedTagError("foo <missing>".to_string())),
        }
    }

    fn len_bytes(&self) -> usize {
        4 + 4 + (self.len + 7) / 8
    }

    pub fn decode(self) -> Result<Self, TagDecodeError> {
        match self {
            Tag::_PTR(tag) => Self::decode_ptr(tag),
            Tag::_DMY(tag) => Self::decode_dmy(tag),
            Tag::INFO(tag) => Self::decode_info(tag),
            Tag::NASC(tag) => Self::decode_nasc(tag),
            Tag::FRPD(tag) => Self::decode_frpd(tag),
            Tag::DETI(tag) => Self::decode_deti(tag),
            Tag::ESTn(tag) => Self::decode_est(tag),
            _ => Err(TagDecodeError(format!(
                "Decoding not implemented for tag: {}",
                self.name
            ))),
        }
    }

    fn decode_ptr(tag: TagBase) -> Result<Self, TagDecodeError> {
        // debug!("Decoding _PTR");
        if tag.len != 64 {
            return Err(TagDecodeError(format!(
                "ignored *ptr TAG with wrong length ({} bits)",
                tag.len
            )));
        }

        let protocol_name = match std::str::from_utf8(&tag.value[..4]) {
            Ok(pt) => pt,
            Err(_) => "",
        };
        let maj = ((tag.value[4] as u16) << 8) | tag.value[5] as u16;
        let min = ((tag.value[6] as u16) << 8) | tag.value[7] as u16;

        // let protocol_name = std::str::from_utf8(tag.value.get(..4).unwrap_or(&[])).unwrap_or("");
        // let maj = u16::from_be_bytes(tag.value.get(4..6).map(|b| [b[0], b[1]]).unwrap_or([0, 0]));
        // let min = u16::from_be_bytes(tag.value.get(6..8).map(|b| [b[0], b[1]]).unwrap_or([0, 0]));

        if protocol_name != "DETI" {
            return Err(TagDecodeError(format!(
                "ignored *ptr TAG with protocol name: {}",
                protocol_name
            )));
        }

        if maj != 0x0000 || min != 0x0000 {
            return Err(TagDecodeError(format!(
                "ignored *ptr TAG unsupported version: 0x{:04X} - 0x{:04X}",
                maj, min
            )));
        }

        Ok(Tag::_PTR(tag))
    }

    fn decode_dmy(tag: TagBase) -> Result<Self, TagDecodeError> {
        Ok(Tag::_DMY(tag))
    }

    fn decode_info(tag: TagBase) -> Result<Self, TagDecodeError> {
        debug!("Decoding INFO");
        // TODO: decode INFO
        Ok(Tag::INFO(tag))
    }

    fn decode_nasc(tag: TagBase) -> Result<Self, TagDecodeError> {
        debug!("Decoding NASC");
        // TODO: decode NASC
        Ok(Tag::NASC(tag))
    }

    fn decode_frpd(tag: TagBase) -> Result<Self, TagDecodeError> {
        debug!("Decoding FRPD");
        // TODO: decode FRPD
        Ok(Tag::FRPD(tag))
    }

    fn decode_deti(mut tag: TagBase) -> Result<Self, TagDecodeError> {
        // DAB ETI(LI) Management (deti)

        let has_atstf = (tag.value[0] & 0x80) != 0;
        let has_ficf = (tag.value[0] & 0x40) != 0;
        let has_rfudf = (tag.value[0] & 0x20) != 0;

        let stat = tag.value[2];
        let mid = tag.value[3] >> 6;

        let fic_len = if has_ficf {
            if mid == 3 {
                128 // Mode III
            } else {
                96 // Mode I, II and IV
            }
        } else {
            0
        };

        let tag_len_bytes_calced =
            2 + 4 + if has_atstf { 8 } else { 0 } + fic_len + if has_rfudf { 3 } else { 0 };

        // debug!("Decoding DETI - tl-calc: {} <> tl: {}", tag_len_bytes_calced * 8, tag.len);

        if tag.len != tag_len_bytes_calced * 8 {
            return Err(TagDecodeError(format!(
                "ignored DETI TAG with wrong length ({} bits)",
                tag.len
            )));
        }

        if has_ficf {
            let fic_start = 2 + 4 + if has_atstf { 8 } else { 0 };
            let fic = &tag.value[fic_start..fic_start + fic_len];
            // debug!("FIC: {} bytes", fic.len());

            // NOTE: not sure if this is a good idea...
            tag.decoded_value = fic.to_vec();
        }

        // debug!("Decoding DETI - ATSDF: {} FIC: {} RFUDF: {} | {} | mid: {} | F:: {}", has_atstf, has_ficf, has_rfudf, stat, mid, fic_len);
        Ok(Tag::DETI(tag))
    }

    fn decode_est(mut tag: TagBase) -> Result<Self, TagDecodeError> {
        // ETI Sub-Channel Stream
        let scn = tag.header[3];

        if scn < 1 || scn >= 64 {
            return Err(TagDecodeError(format!(
                "ignored ESTn TAG not in 1-64: {}",
                scn
            )));
        }

        if tag.len < 3 * 8 {
            return Err(TagDecodeError(format!(
                "ignored ESTn TAG - too short ({} bits)",
                tag.len
            )));
        }

        // SSTC: Sub-channel Stream Characterization
        let scid = tag.value[0] >> 2;
        let sad = ((tag.value[0] as u16 & 0b00000011) << 8) | tag.value[1] as u16;
        let tpl = tag.value[2] >> 2;
        let rfa = tag.value[2] & 0b00000011;

        if sad > 863 {
            return Err(TagDecodeError(format!(
                "ignored ESTn TAG with invalid SAD: {}",
                sad
            )));
        }

        // debug!("Decoding EST - SC-ID: {:>2} | {} {} {}", scid, sad, tpl, rfa);

        // MST: Main Stream Data
        if tag.value.len() < 3 {
            return Err(TagDecodeError(format!(
                "ignored ESTn TAG - MST too short ({} bytes)",
                tag.value.len()
            )));
        }

        // TODO: not sure if we can do this like this ;)
        let mst = &tag.value[3..];
        // let slice_len = (4 + 4 + (&tag.len + 7) / 8).saturating_sub(3);
        // debug!("SLICE_LEN: {}", slice_len);
        // debug!("Decoding EST - SC-ID: {:>2} | MST: {} bytes", scid, mst.len());

        // NOTE: not sure if this is a good idea...
        tag.decoded_value = mst.to_vec();

        Ok(Tag::ESTn(tag))
    }
}

#[derive(Debug)]
pub struct AFFrame {
    // NOTE: it looks like we only have AF frames..
    pub data: Vec<u8>,
    pub initial_size: usize,
    sync_magic: SyncMagic,
}

impl AFFrame {
    pub fn new() -> Self {
        AFFrame {
            data: vec![0; 8],
            initial_size: 8,
            sync_magic: SyncMagic::new(vec![b'A', b'F'], "AF"),
        }
    }

    // scan the frame for a sync magic
    pub fn find_sync_magic(&self) -> Option<usize> {
        // maximum magic length.
        let magic_len = self.sync_magic.pattern.len();

        for offset in 0..=self.data.len().saturating_sub(magic_len) {
            let slice = &self.data[offset..];
            if self.sync_magic.matches(slice) {
                return Some(offset);
            }
        }
        None
    }

    pub fn check_completed(&mut self) -> bool {
        let d = &self.data;
        if d.len() == 8 {
            // header only > retrieve payload len and resize frame
            let len = (d[2] as usize) << 24
                | (d[3] as usize) << 16
                | (d[4] as usize) << 8
                | (d[5] as usize);
            self.resize(10 + len + 2);
            false
        } else {
            true
        }
    }

    pub fn resize(&mut self, new_size: usize) {
        self.data.resize(new_size, 0);
    }

    pub fn reset(&mut self) {
        self.resize(self.initial_size);
        // self.resize(size);
    }

    pub fn decode(&self) -> Result<Vec<Tag>, FrameDecodeError> {
        let d = &self.data;

        if d.len() < 12 {
            // warn!("AFFrame: frame too short");
            return Err(FrameDecodeError(format!(
                "Frame too short (len = {})",
                d.len()
            )));
        }

        // SYNC: combine first two bytes into a u16.
        let sync = ((d[0] as u16) << 8) | d[1] as u16;
        match sync {
            0x4146 /* 'AF' */ => { /* supported */ }
            _ => {
                // warn!("AFFrame: packet with SYNC = 0x{:04X}", sync);
                return Err(FrameDecodeError(format!("packet with SYNC = 0x{:04X}", sync)));
            }
        }

        // LEN: combine bytes 2-5 into a length value.
        let len = ((d[2] as usize) << 24)
            | ((d[3] as usize) << 16)
            | ((d[4] as usize) << 8)
            | (d[5] as usize);

        // debug!("AFFrame: packet with LEN = {}", len);

        // CF: Bit 7 (0x80) of byte 8 must be set.
        let cf = (d[8] & 0x80) != 0;
        if !cf {
            return Err(FrameDecodeError(format!(
                "ignored EDI AF packet without CRC"
            )));
        }

        // MAJ: bits 6-4 of byte 8.
        let maj = (d[8] & 0x70) >> 4;
        if maj != 0x01 {
            return Err(FrameDecodeError(format!(
                "ignored EDI AF packet with MAJ = 0x{:02X}",
                maj
            )));
        }

        // MIN: bits 3-0 of byte 8.
        let min = d[8] & 0x0F;
        if min != 0x00 {
            return Err(FrameDecodeError(format!(
                "ignored EDI AF packet with MIN = 0x{:02X}",
                min
            )));
        }

        // debug!("AFFrame: packet with LEN = {} - MAJ:{:02X} - MIN: {:02X}", len, maj, min);

        // PT: byte 9 must be 'T'
        if d[9] != b'T' {
            return Err(FrameDecodeError(format!(
                "EDIPlayer: ignored EDI AF packet with PT = '{}'",
                d[9] as char
            )));
        }

        // Check that frame is long enough for CRC:
        if d.len() < 10 + len + 2 {
            return Err(FrameDecodeError(format!(
                "frame too short for CRC check: {}",
                d.len()
            )));
        }
        // // CRC: stored CRC from the two bytes after the data.
        let crc_stored = ((d[10 + len] as u16) << 8) | d[10 + len + 1] as u16;
        let crc_calced = calc_crc16_ccitt(&d[0..10 + len]);

        // println!("CRC: {:04X} : {:04X}", crc_stored, crc_calced);

        if crc_stored != crc_calced {
            return Err(FrameDecodeError(format!(
                "CRC mismatch {:04X} <> {:04X}",
                crc_stored, crc_calced
            )));
        }

        // debug!("AFFrame: packet with LEN = {} - MAJ:{:02X} - MIN: {:02X}", len, maj, min);

        // all checks done. continue to extract the frame data...

        let mut tags: Vec<Tag> = Vec::new();

        let mut i = 0usize;
        while i < len.saturating_sub(8) {
            let start = 10 + i;

            // avoid overflow
            if start + 8 > d.len() {
                break;
            }

            let tag_item = &d[start..];

            match Tag::from_bytes(tag_item) {
                Ok(tag) => {
                    i += tag.len_bytes();
                    tags.push(tag);
                }
                Err(e) => {
                    warn!("{}", e);
                    let tag_len = ((tag_item[4] as usize) << 24)
                        | ((tag_item[5] as usize) << 16)
                        | ((tag_item[6] as usize) << 8)
                        | (tag_item[7] as usize);
                    i += 4 + 4 + (tag_len + 7) / 8;
                }
            }

            // let tag = Tag::from_bytes(tag_item);
            // i += tag.len_bytes();
            // tags.push(tag);
        }

        Ok(tags)
    }
}

impl fmt::Display for AFFrame {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "AF ({})", self.data.len())
    }
}

pub struct EDIFrameResult {
    pub tags: Vec<Tag>,
    pub au_frames: Vec<Vec<u8>>,
    pub pcm: Vec<f32>,
}

impl EDIFrameResult {
    fn new(tags: Vec<Tag>, au_frames: Vec<Vec<u8>>, pcm: Vec<f32>) -> Self {
        EDIFrameResult {
            tags,
            au_frames,
            pcm,
        }
    }
}

#[derive(Debug)]
pub struct EDISource {
    pub frame: AFFrame,
    audio_decoder: RefCell<AudioDecoder>,
}

impl EDISource {
    pub fn new() -> Self {
        EDISource {
            frame: AFFrame::new(),
            audio_decoder: RefCell::new(AudioDecoder::new(1)),
        }
    }
    pub fn process_frame(&mut self) -> Result<EDIFrameResult, FrameDecodeError> {
        let mut audio_decoder = self.audio_decoder.borrow_mut();

        // NOTE: just testing, hold soee aduio data...
        let mut au_frames: Vec<Vec<u8>> = Vec::new();
        let mut pcm_data: Vec<f32> = Vec::new();

        match self.frame.decode() {
            Ok(tags) => {
                let mut decoded_tags: Vec<Tag> = Vec::new();

                // decode tags
                for tag in tags {
                    match tag.decode() {
                        // Ok(decoded_tag) => debug!("X Decoded Tag: {:}", decoded_tag),
                        Ok(decoded_tag) => {
                            // debug!("DECODED: {} - {}", decoded_tag, decoded_tag.decoded_value.len());
                            if decoded_tag.decoded_value.len() > 0 {
                                decoded_tags.push(decoded_tag);
                            }
                        }
                        Err(e) => warn!("X {}", e),
                    }
                }
                // handle decoded tags
                for tag in decoded_tags.iter() {
                    // debug!("{} - {}", tag, tag.decoded_value.len());

                    match tag {
                        Tag::DETI(tag) => {
                            match FICDecoder::from_bytes(&tag.decoded_value) {
                                Ok(fic) => {
                                    // debug!("FIC: {:?}", fic);
                                }
                                Err(e) => {
                                    warn!("{}", e);
                                }
                            }
                        }
                        Tag::ESTn(tag) => {
                            let scid = tag.value[0] >> 2;
                            let slice_data = &tag.value[3..];
                            let slice_len = (tag.len / 8).saturating_sub(3);

                            debug!("AF: slen: {} | {} <> {}", slice_len, slice_data.len(), tag.decoded_value.len());

                            if scid == 6 {
                                match audio_decoder.feed(&slice_data, slice_len) {
                                    Ok(r) => {
                                        // debug!("DR - au frames: {:?}", r.au_frames.len());
                                        au_frames.extend(r.au_frames);
                                        pcm_data.extend(r.pcm);
                                    }
                                    Err(e) => {
                                        // warn!("{}", e);
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
                // let result = EDIFrameResult::new(decoded_tags, Vec::new());
                Ok(EDIFrameResult::new(decoded_tags, au_frames, pcm_data))
            }
            Err(e) => Err(FrameDecodeError(format!("{}", e))),
        }
    }
}
