use super::MSCDataGroup;
use crate::edi::bus::{EDIEvent, emit_event};
use derivative::Derivative;
use md5::{compute, Digest};
use base64;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct MOTImage {
    pub scid: u8,
    pub mimetype: String,
    #[serde(serialize_with = "MOTImage::serialize_md5")]
    pub md5: [u8; 16],
    pub len: usize,
    #[serde(serialize_with = "MOTImage::serialize_data")]
    pub data: Vec<u8>,
}

impl MOTImage {
    pub fn new(scid: u8, kind: u16, data: Vec<u8>) -> Self {
        let mimetype = match kind {
            1 => "image/jpeg",
            3 => "image/png",
            _ => {
                log::warn!("MOT unknown image type: {}", kind);
                "application/octet-stream"
            }
        }
        .to_string();

        let hash = compute(&data).into();

        Self {
            scid,
            mimetype,
            md5: hash,
            len: data.len(),
            data,
        }
    }
    fn serialize_md5<S>(md5: &[u8; 16], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let hex = md5.iter().map(|b| format!("{:02x}", b)).collect::<String>();
        serializer.serialize_str(&hex)
    }
    fn serialize_data<S>(data: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let encoded = base64::encode(data);
        serializer.serialize_str(&encoded)
    }
}

#[derive(Debug)]
pub struct MOTObject {
    scid: u8,
    // raw values
    pub transport_id: u16,
    pub header: Vec<u8>,
    pub body: Vec<u8>,
    pub header_complete: bool,
    pub body_complete: bool,

    // available after parsing
    // primary MOT header
    pub body_size: Option<usize>,
    pub content_type: Option<u8>,
    pub content_subtype: Option<u16>,
    // extension headers
    pub content_name: Option<String>,
}

impl MOTObject {
    pub fn new(scid: u8, transport_id: u16) -> Self {
        Self {
            scid,
            transport_id,
            header: Vec::new(),
            body: Vec::new(),
            header_complete: false,
            body_complete: false,
            //
            body_size: None,
            content_type: None,
            content_subtype: None,
            content_name: None,
        }
    }

    pub fn is_complete(&self) -> bool {
        self.header_complete && self.body_complete
    }

    pub fn parse_header(&mut self) {
        // log::debug!("MOT parse header: {} bytes", self.header.len());

        if self.header.len() < 7 {
            log::warn!("MOT header too short, skipping");
            return;
        }

        let data = &self.header;

        // Parse header size (12 bits across bytes 3–5) (does not work)
        let header_size = (((data[3] & 0x0F) as usize) << 9)
            | ((data[4] as usize) << 1)
            | ((data[5] as usize) >> 7);

        // Parse header size (12 bits: bits 28–39)
        // let header_size = (((data[3] as usize) & 0x0F) << 8)
        //     | (data[4] as usize);

        if header_size > data.len() {
            log::warn!(
                "MOT header incomplete (expected {}, got {})",
                header_size,
                data.len()
            );
            return;
        }

        // Parse body size (28 bits across bytes 0–3)
        let body_size = ((data[0] as usize) << 20)
            | ((data[1] as usize) << 12)
            | ((data[2] as usize) << 4)
            | ((data[3] as usize) >> 4);

        // Parse content type (6 bits) and subtype (10 bits)
        let content_type = (data[5] >> 1) & 0x3F;
        let content_subtype = (((data[5] & 0x01) as u16) << 8) | data[6] as u16;

        // Update fields
        self.body_size = Some(body_size);
        self.content_type = Some(content_type);
        self.content_subtype = Some(content_subtype);

        // parse header extensions
        let mut n = 7;

        while n < header_size {
            let pli = (data[n] >> 6) & 0x03;
            let param_id = data[n] & 0x3F;
            n += 1;

            let mut data_field_len = 0;

            match pli {
                0 => {} // no data field
                1 => data_field_len = 1,
                2 => data_field_len = 4,
                3 => {
                    if n >= header_size {
                        log::warn!("MOT header corrupted");
                        break;
                    }
                    let mut len = (data[n] & 0x7F) as usize;
                    if data[n] & 0x80 != 0 {
                        n += 1;
                        if n >= header_size {
                            log::warn!("MOT header invalid");
                            break;
                        }
                        len = (len << 8) | data[n] as usize;
                    }
                    n += 1;
                    data_field_len = len;
                }
                _ => {}
            }

            log::debug!(
                "MOT header: param_id = {:#04x} (PLI = {}) - data_field_len = {} bytes",
                param_id,
                pli,
                data_field_len,
            );

            if n + data_field_len > header_size {
                log::warn!(
                    "MOT header incomplete (expected {}, got {})",
                    header_size,
                    data_field_len
                );
                break;
            }

            let field_data = &data[n..n + data_field_len];

            // ContentName (ParamID = 0x0C)
            if param_id == 0x0C && field_data.len() > 1 {
                let charset_id = field_data[0] >> 4; // reserved: field_data[0] & 0x0F
                let name_bytes = &field_data[1..];
                let name = String::from_utf8_lossy(name_bytes).to_string();
                self.content_name = Some(name.clone());

                log::debug!(
                    "MOT ContentName: {:?} (charset_id = {})",
                    self.content_name,
                    charset_id
                );
            }

            if param_id == 0x23 {
                // MOT parameter CAInfo > scrambled
                log::warn!("MOT CAInfo: scrambled (PLI = {}) > ignored", pli);
                break;
            }

            if param_id == 0x11 {
                // MOT parameter CompressionType
                log::warn!("MOT compressed: scrambled (PLI = {}) > ignored", pli);
                break;
            }

            // Other parameters can be handled here later...
            n += data_field_len;
        }

        log::debug!(
            "MOT header: body_size={}, content_type={}, content_subtype={} - name: {:?}",
            body_size,
            content_type,
            content_subtype,
            self.content_name,
        );

        match content_type {
            2 => {}
            _ => {
                log::warn!("MOT unknown content type: {}", content_type);
            }
        }
    }
}

#[derive(Debug)]
pub struct MOTDecoder {
    scid: u8,
    pub current: Option<MOTObject>,
}

impl MOTDecoder {
    pub fn new(scid: u8) -> Self {
        Self { scid, current: None }
    }
    pub fn feed(&mut self, dg: &MSCDataGroup) {
        if !dg.is_valid || !dg.segment_flag {
            return;
        }

        if dg.data_field.len() < 3 {
            log::warn!("MOT data too short: {} bytes", dg.data_field.len());
            return;
        }

        // log::debug!("MOT DG: {:#?}", dg);

        let seg_type = dg.seg_type;
        let transport_id = dg.transport_id.unwrap_or(0);
        let data = &dg.data_field[2..];

        // log::debug!("MOT DG: type = {} - id = {} - data = {} bytes", seg_type, transport_id, data.len());

        match seg_type {
            3 => {
                // Start new MOT object on header
                log::debug!("MOT: header: {} bytes", data.len());

                let mut obj = MOTObject::new(self.scid, transport_id);
                obj.header.extend_from_slice(data);
                obj.header_complete = dg.last_flag;

                if obj.header_complete {
                    obj.parse_header();
                }

                self.current = Some(obj);
            }

            4 => {
                if let Some(ref mut obj) = self.current {
                    if obj.transport_id != transport_id {
                        log::warn!(
                            "MOT: transport_id mismatch (got {}, expected {})",
                            transport_id,
                            obj.transport_id
                        );
                        return;
                    }

                    // log::debug!("MOT: body: {} bytes", data.len());

                    obj.body.extend_from_slice(data);
                    obj.body_complete = dg.last_flag;

                    if obj.is_complete() {
                        log::info!(
                            "MOT complete! Header = {} bytes, Body = {} bytes",
                            obj.header.len(),
                            obj.body.len()
                        );

                        // obj.parse_header();

                        match obj.content_type {
                            Some(2) => {
                                let mot_image = MOTImage::new(self.scid, obj.content_subtype.unwrap_or(0), obj.body.clone());
                                // log::debug!("MOT image: {:?}", mot_image);
                                emit_event(EDIEvent::MOTImageReceived(mot_image));
                            }
                            _ => {
                                log::warn!("MOT unknown content type: {}", obj.content_type.unwrap_or(0));
                            }
                        }


                        self.current = None;
                    }
                } else {
                    // if we start extracting in the middle of a transmission
                    // log::debug!("MOT: body segment received without active header");
                }
            }

            _ => {
                log::debug!("MOT: skipping unsupported seg_type {}", seg_type);
            }
        }
    }
}
