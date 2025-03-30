use derivative::Derivative;
use serde::Serialize;
use super::MSCDataGroup;

#[derive(Debug)]
pub struct MOTObject {
    // raw values
    pub transport_id: u16,
    pub header: Vec<u8>,
    pub body: Vec<u8>,
    pub header_complete: bool,
    pub body_complete: bool,

    // available after parsing
    pub body_size: Option<usize>,
    pub content_type: Option<u8>,
    pub content_subtype: Option<u16>,
}

impl MOTObject {
    pub fn new(transport_id: u16) -> Self {
        Self {
            transport_id,
            header: Vec::new(),
            body: Vec::new(),
            header_complete: false,
            body_complete: false,
            //
            body_size: None,
            content_type: None,
            content_subtype: None,
        }
    }

    pub fn is_complete(&self) -> bool {
        self.header_complete && self.body_complete
    }

    pub fn parse_header(&mut self) {
        log::debug!("MOT parse header: {} bytes", self.header.len());

        if self.header.len() < 7 {
            log::warn!("MOT header too short, skipping");
            return;
        }

        let data = &self.header;

        // Parse body size (28 bits across bytes 0–3)
        let body_size = ((data[0] as usize) << 20)
            | ((data[1] as usize) << 12)
            | ((data[2] as usize) << 4)
            | ((data[3] as usize) >> 4);

        // Parse header size (12 bits across bytes 3–5) (does not work)
        // let header_size = (((data[3] & 0x0F) as usize) << 9)
        //     | ((data[4] as usize) << 1)
        //     | ((data[5] as usize) >> 7);

        // Parse header size (12 bits: bits 28–39)
        let header_size = (((data[3] as usize) & 0x0F) << 8)
            | (data[4] as usize);


        if header_size > data.len() {
            log::warn!("MOT header incomplete (expected {}, got {})", header_size, data.len());
            return;
        }

        // Parse content type (6 bits) and subtype (10 bits)
        let content_type = (data[5] >> 1) & 0x3F;
        let content_subtype = (((data[5] & 0x01) as u16) << 8) | data[6] as u16;

        // Update fields
        self.body_size = Some(body_size);
        self.content_type = Some(content_type);
        self.content_subtype = Some(content_subtype);

        log::debug!(
            "MOT header: body_size={}, content_type={}, content_subtype={}",
            body_size,
            content_type,
            content_subtype
        );
    }
}

#[derive(Debug)]
pub struct MOTDecoder {
    pub current: Option<MOTObject>,
}

impl MOTDecoder {
    pub fn new() -> Self {
        Self {
            current: None,
        }
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

                let mut obj = MOTObject::new(transport_id);
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

                    log::debug!("MOT: body: {} bytes", data.len());
                    obj.body.extend_from_slice(data);
                    obj.body_complete = dg.last_flag;

                    if obj.is_complete() {
                        log::info!(
                            "MOT complete! Header = {} bytes, Body = {} bytes",
                            obj.header.len(),
                            obj.body.len()
                        );

                        // obj.parse_header();

                        // You can decode `obj` now
                        // For now, just drop it after logging
                        self.current = None;
                    }
                } else {
                    log::warn!("MOT: body segment received without active header");
                }
            }

            _ => {
                log::debug!("MOT: skipping unsupported seg_type {}", seg_type);
            }
        }

    }
}
