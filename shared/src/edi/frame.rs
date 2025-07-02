use derivative::Derivative;
use log;
use serde::Serialize;
use thiserror::Error;

use super::fic::{FICDecoder, FIG};

#[derive(Debug, Error)]
pub enum FrameDecodeError {
    #[error("Frame too short: {l}")]
    FrameTooShort { l: usize },

    #[error("Unknown frame: {kind}")]
    UnknownKind { kind: String },
}

#[derive(Debug, Serialize)]
pub struct FrameDecodeResult {
    pub tags: Vec<Tag>,
}

impl FrameDecodeResult {
    pub fn new(tags: Vec<Tag>) -> Self {
        Self { tags }
    }
}

#[derive(Debug, Serialize)]
pub struct Frame {
    data: Vec<u8>,
}

impl Frame {
    pub fn from_bytes(data: &[u8]) -> Result<FrameDecodeResult, FrameDecodeError> {
        if data.len() < 12 {
            return Err(FrameDecodeError::FrameTooShort { l: data.len() });
        }

        let kind = std::str::from_utf8(&data[..2]).unwrap();

        if kind != "AF" {
            return Err(FrameDecodeError::UnknownKind {
                kind: kind.to_string(),
            });
        }

        // LEN: combine bytes 2-5 into a length value.
        let len = u32::from_be_bytes([data[2], data[3], data[4], data[5]]) as usize;

        let mut tags: Vec<Tag> = Vec::new();

        let mut i = 0usize;

        while i < len.saturating_sub(8) {
            let start = 10 + i;

            // avoid overflow
            if start + 8 > data.len() {
                break;
            }

            let tag_item = &data[start..];

            let tag_len =
                u32::from_be_bytes([tag_item[4], tag_item[5], tag_item[6], tag_item[7]]) as usize;

            match Self::parse_tag(tag_item) {
                Ok(tag) => {
                    // log::debug!("tag_item: B {:?}", tag_item.len());
                    tags.push(tag);
                }
                Err(e) => {
                    log::error!("Error parsing tag: {:?}", e);
                }
            }

            i += 4 + 4 + (tag_len + 7) / 8;
        }

        let result = FrameDecodeResult::new(tags);

        Ok(result)
    }

    fn parse_tag(data: &[u8]) -> Result<Tag, TagError> {
        let name = std::str::from_utf8(data.get(..4).unwrap_or(&[])).unwrap_or("");
        let kind = if name.starts_with("est") { "est" } else { name };
        // let value = data[8..].to_vec();

        match kind {
            // tags we actually care
            "deti" => match DETITag::from_bytes(data) {
                Ok(tag) => Ok(Tag::DETI(tag)),
                Err(e) => Err(e),
            },
            "est" => match ESTTag::from_bytes(data) {
                Ok(tag) => Ok(Tag::EST(tag)),
                Err(e) => Err(e),
            },
            // tags i guess we don't care
            "*ptr" => Ok(Tag::PTR(PTRTag())),
            "*dmy" => Ok(Tag::DMY(DMYTag())),
            // tags i don't know what they are...
            "Fsst" => Ok(Tag::FSST(FSSTTag {})),
            "Fptt" => Ok(Tag::FPTT(FPTTTag {})),
            "Fsid" => Ok(Tag::FSID(FSIDTag {})),
            _ => Err(TagError::Unsupported {
                name: kind.to_string(),
            }),
        }
    }
}

#[derive(Debug, Error)]
pub enum TagError {
    #[error("Unsupported tag: {name}")]
    Unsupported { name: String },

    #[error("Invalid size: {l}")]
    InvalidSize { l: usize },
}

#[derive(Debug, Serialize)]
pub enum Tag {
    DETI(DETITag),
    EST(ESTTag),
    //
    PTR(PTRTag),
    DMY(DMYTag),
    //
    FSST(FSSTTag),
    FPTT(FPTTTag),
    FSID(FSIDTag),
}

// tags i don't think we have to care about
#[derive(Debug, Serialize)]
pub struct PTRTag();

#[derive(Debug, Serialize)]
pub struct DMYTag();

// tags we care about
#[derive(Debug, Serialize)]
pub struct DETITag {
    // DAB ETI(LI) Management
    pub atstf: Vec<u8>,
    pub figs: Vec<FIG>,
    pub rfudf: Vec<u8>,
}

impl DETITag {
    pub fn from_bytes(data: &[u8]) -> Result<Self, TagError> {
        if data.len() < 8 {
            return Err(TagError::InvalidSize { l: data.len() });
        }

        let len = u32::from_be_bytes([data[4], data[5], data[6], data[7]]) as usize;
        let header = data[0..8].to_vec();
        let value = data[8..].to_vec();

        let has_atstf = (value[0] & 0x80) != 0;
        let has_ficf = (value[0] & 0x40) != 0;
        let has_rfudf = (value[0] & 0x20) != 0;

        let stat = value[2];
        let mid = value[3] >> 6;

        let fic_len = match (has_ficf, mid) {
            (true, 3) => 128, // Mode III
            (true, _) => 96,  // Modes I, II, and IV
            (false, _) => 0,
        };

        let len_atstf = if has_atstf { 8 } else { 0 };
        let len_rfudf = if has_rfudf { 3 } else { 0 };

        let len_calc = 2 + 4 + len_atstf + fic_len + len_rfudf;

        if len_calc * 8 != len {
            return Err(TagError::InvalidSize { l: len });
        }

        // log::debug!(
        //     "TAG_DETI: ATSTF: {} - FIC: {} - RFUD: {}",
        //     has_atstf,
        //     has_ficf,
        //     has_rfudf
        // );

        // NOTE: just dummy values for now
        let atstf = vec![];
        let mut figs = vec![];
        let rfudf = vec![];

        if has_ficf {
            let fic_start = 2 + 4 + if has_atstf { 8 } else { 0 };
            let fic_data = &value[fic_start..fic_start + fic_len];

            match FICDecoder::from_bytes(fic_data) {
                Ok(_figs) => {
                    figs.extend(_figs);
                }
                Err(e) => {
                    log::error!("Error decoding FIC: {:?}", e);
                }
            }
        }

        Ok(Self { atstf, figs, rfudf })
    }
}

#[derive(Derivative, Serialize)]
#[derivative(Debug)]
pub struct ESTTag {
    pub len: usize,
    pub header: Vec<u8>,
    #[derivative(Debug = "ignore")]
    pub value: Vec<u8>,
}

impl ESTTag {
    pub fn from_bytes(data: &[u8]) -> Result<Self, TagError> {
        if data.len() < 8 {
            return Err(TagError::InvalidSize { l: data.len() });
        }

        let len = u32::from_be_bytes([data[4], data[5], data[6], data[7]]) as usize;
        let header = data[0..8].to_vec();
        let value = data[8..].to_vec();

        // TODO: maybe add some checks?

        // println!("ESTTag: len: {}, header: {:?}, value: {:?}", len, header, value);

        // let scid = value[0] >> 2;
        // if scid == 13 {
        //     println!("ESTTag: SCID: {} - header: {:?} - data: {:?}", scid, header, &value[..11]);
        // }
        

        Ok(Self { len, header, value })
    }
}

// some tags seen on sat2edi - don't know what do do with them...
#[derive(Debug, Serialize)]
pub struct FSSTTag {}

#[derive(Debug, Serialize)]
pub struct FPTTTag {}

#[derive(Debug, Serialize)]
pub struct FSIDTag {}
