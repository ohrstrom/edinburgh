#[derive(Debug)]
pub struct MOTDecoder {
    pub header: Vec<u8>,
    pub body: Vec<u8>,
    pub started: bool,
}

impl MOTDecoder {
    pub fn new() -> Self {
        Self {
            header: Vec::new(),
            body: Vec::new(),
            started: false,
        }
    }

    pub fn feed(&mut self, seg: &[u8]) {
        if let Some((seg_kind, offset)) = Self::parse_seg(seg) {
            // TODO: that's wrong. i think CRC is at the end...
            let payload = &seg[offset..];

            if seg_kind == 3 {
                log::debug!(
                    "MOT Header: offset: {} - data: {} bytes",
                    offset,
                    payload.len()
                );
                self.header = payload.to_vec();
                self.started = true;
                self.body.clear();

                // TODO: get expected header & body size. print them here...

                let header_hi = payload[6];
                let header_lo = payload[7];
                let segment_size = (((header_hi & 0x1F) as u16) << 8) | header_lo as u16;

                log::debug!("MOT Header: body size: {}", segment_size);


            } else if seg_kind == 4 && self.started {
                log::debug!(
                    "MOT Body:   offset: {} - data: {} bytes",
                    offset,
                    payload.len()
                );
                self.body.extend_from_slice(payload);
            }
        } else {
            log::debug!("Segment not valid MOT header/body");
        }

        if self.body.len() > 0 {
            log::debug!("MOT: {}", self.body.len());
        }
    }

    fn parse_seg(seg: &[u8]) -> Option<(u8, usize)> {
        if seg.len() < 2 {
            return None;
        }

        let header = seg[0];

        let extension_flag = header & 0x80 != 0;
        let crc_flag = header & 0x40 != 0;
        let segment_flag = header & 0x20 != 0;
        let user_access = header & 0x10 != 0;
        let seg_kind = header & 0x0F;

        // Only accept MOT Header (3) or Body (4)
        if !crc_flag || !segment_flag || !user_access {
            return None;
        }

        if seg_kind != 3 && seg_kind != 4 {
            return None;
        }

        let offset = 2 + if extension_flag { 2 } else { 0 };

        Some((seg_kind, offset))
    }
}
