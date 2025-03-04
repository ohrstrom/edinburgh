use std::fmt;


use log::{debug, info, warn, error};

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

    fn len(&self) -> usize {
        self.pattern.len()
    }

    fn matches(&self, data: &[u8]) -> bool {
        data.starts_with(&self.pattern)
    }
}


#[derive(Debug)]
pub struct FrameDecodeError(pub String);

impl fmt::Display for FrameDecodeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "FrameDecodeError: {}", self.0)
    }
}

impl std::error::Error for FrameDecodeError {}

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

    pub fn decode(&self) -> Result<(), FrameDecodeError> {


        let d = &self.data;

        if d.len() < 12 {
            // warn!("AFFrame: frame too short");
            return Err(FrameDecodeError(format!("Frame too short (len = {})", d.len())));
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
            return Err(FrameDecodeError(format!("ignored EDI AF packet without CRC")));
        }

        // MAJ: bits 6-4 of byte 8.
        let maj = (d[8] & 0x70) >> 4;
        if maj != 0x01 {
            return Err(FrameDecodeError(format!("ignored EDI AF packet with MAJ = 0x{:02X}", maj)));
        }

        // MIN: bits 3-0 of byte 8.
        let min = d[8] & 0x0F;
        if min != 0x00 {
            return Err(FrameDecodeError(format!("ignored EDI AF packet with MIN = 0x{:02X}", min)));
        }

        debug!("AFFrame: packet with LEN = {} - MAJ:{:02X} - MIN: {:02X}", len, maj, min);

        // PT: byte 9 must be 'T'
        // if d[9] != b'T' {
        //     eprintln!(
        //         "EDIPlayer: ignored EDI AF packet with PT = '{}'",
        //         d[9] as char
        //     );
        //     return;
        // }

        // // Check that frame is long enough for CRC:
        // if d.len() < 10 + len + 2 {
        //     eprintln!("EDIPlayer: frame too short for CRC check");
        //     return;
        // }
        // // CRC: stored CRC from the two bytes after the data.
        // let crc_stored = ((d[10 + len] as u16) << 8) | d[10 + len + 1] as u16;
        // let crc_calced = calc_crc16_ccitt(&d[0..10 + len]);

        // // println!("CRC: {:04X} : {:04X}", crc_stored, crc_calced);

        // if crc_stored != crc_calced {
        //     eprintln!(
        //         "EDIPlayer: CRC mismatch {:04X} <> {:04X}",
        //         crc_stored, crc_calced
        //     );
        //     return;
        // }

        Ok(())
    }



}

impl fmt::Display for AFFrame {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "AF ({})", self.data.len())
    }
}

#[derive(Debug)]
pub struct EDISource {
    pub frame: AFFrame,
}

impl EDISource {
    pub fn new() -> Self {
        EDISource {
            frame: AFFrame::new(),
        }
    }
    pub fn process_frame(&self) {
        match self.frame.decode() {
            Ok(_) => {},
            Err(e) => warn!("{}", e),
        }
    }
}
