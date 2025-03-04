use crate::utils::{calc_crc16_ccitt, calc_crc_fire_code, is_aac};
use log::{debug, info, trace, warn};
use std::thread;
use std::time::{Duration, Instant};
use access_unit::{detect_audio, aac};

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
pub struct EDISource {
    pub ensemble_frame: Vec<u8>,
    pub initial_frame_size: usize,
    layer: String,
    sync_magic: Vec<SyncMagic>,
    edi_extractor: EDIExtractor,
}

impl EDISource {
    pub fn new() -> Self {
        let sync_magic = vec![
            SyncMagic::new(vec![b'A', b'F'], "AF"),
            SyncMagic::new(vec![b'f', b'i', b'o', b'_'], "File IO"),
        ];
        EDISource {
            ensemble_frame: vec![0; 4096],
            layer: String::new(),
            sync_magic,
            initial_frame_size: 4096,
            edi_extractor: EDIExtractor::new(false),
        }
    }

    /// Scan the ensemble_frame for a sync magic.
    /// Returns Some((offset, name)) if a match is found.
    pub fn find_sync_magic(&self) -> Option<(usize, &str)> {
        // Determine the maximum magic length.
        let max_magic_len = self.sync_magic.iter().map(|sm| sm.len()).max().unwrap_or(0);

        // Loop over possible offsets where a pattern might appear.
        // Use saturating_sub to avoid underflow.
        for offset in 0..=self.ensemble_frame.len().saturating_sub(max_magic_len) {
            let slice = &self.ensemble_frame[offset..];
            for sm in &self.sync_magic {
                if sm.matches(slice) {
                    return Some((offset, &sm.name));
                }
            }
        }
        None
    }

    pub fn check_frame_completed(&mut self, matched_sync_magic: &str) -> bool {
        if self.ensemble_frame.len() == 8 {
            // 1. header only (to retrieve payload len)
            if matched_sync_magic == "AF" {
                let len = (self.ensemble_frame[2] as usize) << 24
                    | (self.ensemble_frame[3] as usize) << 16
                    | (self.ensemble_frame[4] as usize) << 8
                    | (self.ensemble_frame[5] as usize);
                self.ensemble_frame.resize(10 + len + 2, 0);
            } else {
                let len = (self.ensemble_frame[4] as usize) << 24
                    | (self.ensemble_frame[5] as usize) << 16
                    | (self.ensemble_frame[6] as usize) << 8
                    | (self.ensemble_frame[7] as usize);
                self.ensemble_frame.resize(4 + 4 + len / 8, 0);
            }
            false
        } else {
            // 2. complete packet
            true
        }
    }

    pub fn process_completed_frame(&mut self, matched_sync_magic: &str) {
        // show layer
        if self.layer != matched_sync_magic {
            self.layer = matched_sync_magic.to_string();

            info!("EDISource: detected {} layer", self.layer);
        }

        if matched_sync_magic == "AF" {
            // println!("process: {}", matched_sync_magic);
        } else {
            println!("what to do???: {}", matched_sync_magic);
        }

        self.edi_extractor.process_frame(&self.ensemble_frame);
    }
}

#[derive(Debug)]
pub struct FIG0Header {
    cn: bool,
    oe: bool,
    pd: bool,
    extension: u8,
}

impl FIG0Header {
    fn new(byte: u8) -> Self {
        Self {
            cn: (byte & 0x80) != 0, // Bit 7
            oe: (byte & 0x40) != 0, // Bit 6
            pd: (byte & 0x20) != 0, // Bit 5
            extension: byte & 0x1F, // Bits 0-4
        }
    }
}

#[derive(Debug)]
pub struct FIG1Header {
    charset: u8,
    oe: bool,
    extension: u8,
}

impl FIG1Header {
    fn new(byte: u8) -> Self {
        Self {
            charset: byte >> 4,     // Upper 4 bits
            oe: (byte & 0x08) != 0, // Bit 3 (boolean)
            extension: byte & 0x07, // Lower 3 bits
        }
    }
}

#[derive(Debug)]
struct FICLabel {
    charset: u8,
    label: [u8; 16],
    short_label_mask: u16,
}

impl FICLabel {
    fn new() -> Self {
        Self {
            charset: 0,
            label: [0; 16],
            short_label_mask: 0,
        }
    }

    fn str_label(&self) -> String {
        let label_str = String::from_utf8_lossy(&self.label);

        label_str.trim().to_string()
    }

    fn str_short_label(&self) -> String {
        let mut short_label = String::new();

        for (i, &byte) in self.label.iter().enumerate() {
            if self.short_label_mask & (1 << (15 - i)) != 0 {
                short_label.push(byte as char);
            }
        }

        short_label.trim().to_string()
    }
}

#[derive(Debug)]
struct FICDecoder {
    // TODO: just dummy data for now
    eid: Option<String>,
}

impl FICDecoder {
    fn new() -> Self {
        Self { eid: None }
    }
    /******************************************************************
    FIC processing main entry point
    *******************************************************************/
    fn process(&self, fic_data: &[u8], fic_len: usize) {
        if (fic_len % 32) != 0 {
            eprintln!("FICDecoder: invalid FIC data length {:?}", fic_len);
            return;
        }

        for chunk in fic_data.chunks_exact(32) {
            self.proccess_fib(chunk);
        }
    }

    /******************************************************************
    FIB processing
    *******************************************************************/
    fn proccess_fib(&self, data: &[u8]) {
        let crc_stored = u16::from_be_bytes([data[30], data[31]]);
        let crc_calced = calc_crc16_ccitt(&data[0..30]);

        if crc_stored != crc_calced {
            eprintln!(
                "FICDecoder: CRC mismatch {:04X} <> {:04X}",
                crc_stored, crc_calced
            );
            return;
        }

        // iterate over FIGs in FIB
        let mut offset = 0;

        while offset < 30 && data[offset] != 0xFF {
            let fig_type = data[offset] >> 5;
            let len = (data[offset] & 0x1F) as usize;
            offset += 1; // Move past the type/length byte

            if offset + len > 30 {
                eprintln!("FICDecoder: FIG {} has invalid length {}", fig_type, len);
                break;
            }

            match fig_type {
                // 0 => process_fig0(&data[offset..offset + len]),
                0 => self.process_fig_0(&data[offset..offset + len], len),
                1 => self.process_fig_1(&data[offset..offset + len], len),
                _ => eprintln!(
                    "FICDecoder: received unsupported FIG {} with {} bytes",
                    fig_type, len
                ),
            }

            offset += len;
        }
    }
    /******************************************************************
    FIG 0 processing
    *******************************************************************/
    fn process_fig_0(&self, data: &[u8], len: usize) {
        if data.is_empty() {
            eprintln!("FICDecoder: received empty FIG 0");
            return;
        }

        // Read FIG 0 header (first byte)
        let header = FIG0Header::new(data[0]);

        // Skip the first byte (move `data` forward)
        let data = &data[1..];

        // println!("FIG0: {:?}", header);

        // Ignore next config/other ensembles/data services
        if header.cn || header.oe || header.pd {
            return;
        }

        match header.extension {
            0 => {
                self.process_fig_0_0(data, len);
            }
            1 => {
                self.process_fig_0_1(data, len);
            }
            _ => return,
        }
    }

    fn process_fig_0_0(&self, data: &[u8], len: usize) {
        // FIG 0/0 - Ensemble information
        // EID and alarm flag only

        if len < 4 {
            eprintln!("FICDecoder: FIG 0/0 has invalid length {}", len);
            return;
        }

        // Extract 16-bit Ensemble ID (Big-Endian)
        let eid = u16::from_be_bytes([data[0], data[1]]);

        // Extract alarm flag (bit 5 of data[2])
        let al_flag = (data[2] & 0x20) != 0;

        // debug!(
        //     "FICDecoder: FIG 0/0 EID: 0x{:04X} - alarm flag: {}",
        //     eid, al_flag
        // );
    }

    fn process_fig_0_1(&self, data: &[u8], len: usize) {
        // FIG 0/1 - Basic sub-channel organization

        // debug!(
        //     "FICDecoder: FIG 0/1",
        // );

        let mut offset = 0;

        while offset < data.len() {
            let subchid = data[offset] >> 2;
            let start_address = ((data[offset] & 0x03) as usize) << 8 | data[offset + 1] as usize;
            offset += 2;

            // if (data[offset] & 0x80) != 0 {
            //     // long form
            // } else {
            //     // short form
            // }

            // debug!(
            //     "FICDecoder: FIG 0/1 - {}",
            //     subchid,
            // );

        }
    }

    /******************************************************************
    FIG 1 processing
    *******************************************************************/
    fn process_fig_1(&self, data: &[u8], len: usize) {
        if data.is_empty() {
            eprintln!("FICDecoder: received empty FIG 1");
            return;
        }
        // debug!("Processing FIG 1 with {} bytes", data.len());

        // read FIG 1 header (first byte)
        let header = FIG1Header::new(data[0]);

        // debug!("FIG1Header: {:?}", header);

        // skip the first byte (move `data` forward)
        let data = &data[1..];

        // ignore other ensembles
        if header.oe {
            return;
        }

        // determine `len_id` based on `header.extension`
        let len_id = match header.extension {
            0 | 1 => 2, // Ensemble or Programme Service
            4 => {
                if data[0] & 0x80 != 0 {
                    return; // Ignore if P/D = 1
                }
                3 // Service component
            }
            _ => {
                eprintln!(
                    "FICDecoder: unsupported FIG 1 extension {}",
                    header.extension
                );
                return; // Unsupported FIG1 extension
            }
        };

        // calculate expected length
        let len_calced = len_id + 16 + 2;
        if data.len() != len_calced {
            eprintln!(
                "FICDecoder: received FIG 1/{} having {} field bytes (expected: {})",
                header.extension,
                data.len(),
                len_calced
            );
            return;
        }

        // Parse actual label data
        let mut label = FICLabel::new();
        label.charset = header.charset;

        // Copy label (16 bytes)
        label.label.copy_from_slice(&data[len_id..len_id + 16]);

        // Extract `short_label_mask`
        label.short_label_mask = u16::from_be_bytes([data[len_id + 16], data[len_id + 17]]);

        // debug!("FICDecoder: label: \"{:<8}\" - \"{:<16}\"", label.str_short_label(), label.str_label());

        // handle by extension
        match header.extension {
            0 => {
                // Ensemble
                let eid = u16::from_be_bytes([data[0], data[1]]);
                self.process_fig_1_0(eid, label);
            }
            1 => {
                // Programme Service
                let sid = u16::from_be_bytes([data[0], data[1]]);
                self.process_fig_1_1(sid, label);
            }
            4 => {
                // Service Component
                let scids = data[0] & 0x0F;
                let sid = u16::from_be_bytes([data[1], data[2]]);
                self.process_fig_1_4(sid, scids, label);
            }
            _ => {
                eprintln!(
                    "FICDecoder: unsupported FIG 1 extension {}",
                    header.extension
                );
            }
        }
    }



    fn process_fig_1_0(&self, eid: u16, label: FICLabel) {
        // debug!(
        //     "FICDecoder: FIG 1/0 EID: 0x{:04X} - label: {}",
        //     eid,
        //     label.str_label()
        // );
    }

    fn process_fig_1_1(&self, sid: u16, label: FICLabel) {
        // debug!(
        //     "FICDecoder: FIG 1/1 SID: 0x{:04X} - label: {}",
        //     sid,
        //     label.str_label()
        // );
    }

    fn process_fig_1_4(&self, sid: u16, scids: u8, label: FICLabel) {
        // debug!(
        //     "FICDecoder: FIG 1/4 SID: 0x{:04X} SCIDs: {} - label: {}",
        //     sid,
        //     scids,
        //     label.str_label()
        // );
    }
}

#[derive(Debug)]
pub struct EDIExtractor {
    next_frame_time: Option<Instant>,
    disable_int_catch_up: bool,
    fic_decoder: FICDecoder,
    audio_decoder: AudioDecoder,
}

impl EDIExtractor {
    pub fn new(disable_int_catch_up: bool) -> Self {
        Self {
            next_frame_time: None,
            disable_int_catch_up,
            fic_decoder: FICDecoder::new(),
            audio_decoder: AudioDecoder::new(1),
        }
    }

    /******************************************************************
    called from EDISource for each completed frame
    handles schedule and sends frame to decoder
    *******************************************************************/
    fn process_frame(&mut self, edi_frame: &[u8]) {
        let now = Instant::now();
        let init = self.next_frame_time.is_none();

        if init
            || (self.disable_int_catch_up
                && now > self.next_frame_time.unwrap() + Duration::from_millis(24))
        {
            if !init {
                eprintln!("EDIPlayer:resync {:?}", self.next_frame_time);
            }
            self.next_frame_time = Some(now);
        } else {
            let target = self.next_frame_time.unwrap();
            if target > now {
                thread::sleep(target - now);
            }
        }

        // Schedule next frame 24 ms later.
        self.next_frame_time =
            Some(self.next_frame_time.unwrap_or(now) + Duration::from_millis(24));

        self.decode_frame(edi_frame);
    }

    /******************************************************************
    reas frame and determine it's type
    sends data to further processing depending on type
    *******************************************************************/
    fn decode_frame(&mut self, edi_frame: &[u8]) {
        if edi_frame.len() < 12 {
            eprintln!("EDIPlayer: frame too short");
            return;
        }

        // SYNC: combine first two bytes into a u16.
        let sync = ((edi_frame[0] as u16) << 8) | edi_frame[1] as u16;
        match sync {
            0x4146 /* 'AF' */ => { /* supported */ }
            0x5046 /* 'PF' */ => {
                eprintln!("EDIPlayer: ignored unsupported EDI PF packet");
                return;
            }
            _ => {
                eprintln!("EDIPlayer: ignored EDI packet with SYNC = 0x{:04X}", sync);
                return;
            }
        }

        // LEN: combine bytes 2-5 into a length value.
        let len = ((edi_frame[2] as usize) << 24)
            | ((edi_frame[3] as usize) << 16)
            | ((edi_frame[4] as usize) << 8)
            | (edi_frame[5] as usize);

        // CF: Bit 7 (0x80) of byte 8 must be set.
        let cf = (edi_frame[8] & 0x80) != 0;
        if !cf {
            // eprintln!("EDIPlayer: ignored EDI AF packet without CRC");
            return;
        }

        // MAJ: bits 6-4 of byte 8.
        let maj = (edi_frame[8] & 0x70) >> 4;
        if maj != 0x01 {
            // eprintln!("EDIPlayer: ignored EDI AF packet with MAJ = 0x{:02X}", maj);
            return;
        }

        // MIN: bits 3-0 of byte 8.
        let min = edi_frame[8] & 0x0F;
        if min != 0x00 {
            eprintln!("EDIPlayer: ignored EDI AF packet with MIN = 0x{:02X}", min);
            return;
        }

        // PT: byte 9 must be 'T'
        if edi_frame[9] != b'T' {
            eprintln!(
                "EDIPlayer: ignored EDI AF packet with PT = '{}'",
                edi_frame[9] as char
            );
            return;
        }

        // Check that frame is long enough for CRC:
        if edi_frame.len() < 10 + len + 2 {
            eprintln!("EDIPlayer: frame too short for CRC check");
            return;
        }
        // CRC: stored CRC from the two bytes after the data.
        let crc_stored = ((edi_frame[10 + len] as u16) << 8) | edi_frame[10 + len + 1] as u16;
        let crc_calced = calc_crc16_ccitt(&edi_frame[0..10 + len]);

        // println!("CRC: {:04X} : {:04X}", crc_stored, crc_calced);

        if crc_stored != crc_calced {
            eprintln!(
                "EDIPlayer: CRC mismatch {:04X} <> {:04X}",
                crc_stored, crc_calced
            );
            return;
        }

        // Parse TAG packet for TAG items (skip any TAG packet padding)
        let mut i = 0usize;
        while i < len.saturating_sub(8) {
            // Calculate the start index of the tag item.
            let start = 10 + i;
            if start + 8 > edi_frame.len() {
                break;
            }
            let tag_item = &edi_frame[start..];

            // Extract tag name as a &str of 4 bytes.
            let tag_name = match std::str::from_utf8(&tag_item[0..4]) {
                Ok(name) => name,
                Err(_) => {
                    eprintln!("EDIPlayer: invalid tag name UTF-8");
                    break;
                }
            };

            // Get the tag length (in bits) from bytes 4-7.
            let tag_len = ((tag_item[4] as usize) << 24)
                | ((tag_item[5] as usize) << 16)
                | ((tag_item[6] as usize) << 8)
                | (tag_item[7] as usize);

            // Calculate the total length in bytes of the tag item.
            let tag_item_len_bytes = 4 + 4 + (tag_len + 7) / 8;

            let tag_value = &tag_item[8..];

            // Process specific TAG items:
            if tag_name == "*ptr" {
                if tag_len != 64 {
                    eprintln!(
                        "EDIPlayer: ignored *ptr TAG item with wrong length ({} bits)",
                        tag_len
                    );
                    i += tag_item_len_bytes;
                    continue;
                }
                let protocol_type = match std::str::from_utf8(&tag_item[8..12]) {
                    Ok(pt) => pt,
                    Err(_) => "",
                };
                if protocol_type != "DETI" {
                    eprintln!(
                        "EDIPlayer: unsupported protocol type '{}' in *ptr TAG item",
                        protocol_type
                    );
                }
                let major = ((tag_item[12] as u16) << 8) | tag_item[13] as u16;
                let minor = ((tag_item[14] as u16) << 8) | tag_item[15] as u16;
                if major != 0x0000 || minor != 0x0000 {
                    eprintln!(
                        "EDIPlayer: unsupported major/minor revision 0x{:04X}/0x{:04X} in *ptr TAG item",
                        major, minor
                    );
                }
                i += tag_item_len_bytes;
                continue;
            }
            if tag_name == "*dmy" {
                i += tag_item_len_bytes;
                continue;
            }
            // DAB ETI(LI) Management
            if tag_name == "deti" {
                // Remark: For simplicity a minimal check is presented.
                let atstf = (tag_value[0] & 0x80) != 0;
                let ficf = (tag_value[0] & 0x40) != 0;
                let rfudf = (tag_value[0] & 0x20) != 0;

                // STAT
                if tag_value[2] != 0xFF {
                    eprintln!(
                        "EDIPlayer: EDI AF packet with STAT = 0x{:02X}",
                        tag_value[2]
                    );
                    i += tag_item_len_bytes;
                    continue;
                }
                let mid = tag_value[3] >> 6;
                let fic_len = if ficf {
                    if mid == 3 {
                        128
                    } else {
                        96
                    }
                } else {
                    0
                };
                let tag_len_bytes_calced =
                    2 + 4 + if atstf { 8 } else { 0 } + fic_len + if rfudf { 3 } else { 0 };
                if tag_len != tag_len_bytes_calced * 8 {
                    eprintln!(
                        "EDIPlayer: ignored deti TAG item with wrong length ({} bits)",
                        tag_len
                    );
                    i += tag_item_len_bytes;
                    continue;
                }
                if fic_len != 0 {
                    let fic_start = 2 + 4 + if atstf { 8 } else { 0 };

                    if let Some(fic_data) = tag_value.get(fic_start..fic_start + fic_len) {
                        self.fic_decoder.process(fic_data, fic_len);
                    } else {
                        eprintln!("FICDecoder: FIC slice out of bounds!");
                    }
                }
                i += tag_item_len_bytes;
                continue;
            }
            // ETI Sub-Channel Stream
            if tag_name.starts_with("est") && tag_item[3] >= 1 && tag_item[3] <= 64 {
                if tag_len < 3 * 8 {
                    eprintln!(
                        "EDIPlayer: ignored est<n> TAG item with too short length ({} bits)",
                        tag_len
                    );
                    i += tag_item_len_bytes;
                    continue;
                }

                let subchid = tag_value[0] >> 2;
                // Here you might lock your audio service and feed data.
                // println!("EDIPlayer: received est tag for subchid {}", subchid);

                if tag_value.len() >= 3 {
                    let slice_data = &tag_value[3..];
                    let slice_len = (tag_len / 8).saturating_sub(3);
            
                    // self.audio_decoder.process(subchid, &slice_data, slice_len);

                    if let decoder = &mut self.audio_decoder {
                        decoder.process(subchid, &slice_data, slice_len);
                    }



                } else {
                    eprintln!("EDIPlayer: est<n> TAG item without data");
                }



                // self.audio_decoder.process(subchid, &tag_item);


                i += tag_item_len_bytes;
                continue;
            }
            // Information
            if tag_name == "info" {
                let info_len = tag_len / 8;
                let text = match std::str::from_utf8(&tag_item[8..8 + info_len]) {
                    Ok(t) => t,
                    Err(_) => "(invalid UTF-8)",
                };
                eprintln!("EDIPlayer: info TAG item '{}'", text);
                i += tag_item_len_bytes;
                continue;
            }
            // Network Adapted Signalling Channel - ignored
            if tag_name == "nasc" {
                println!("nasc: {:?}", tag_item);
                i += tag_item_len_bytes;
                continue;
            }
            // Frame Padding User Data - ignored
            if tag_name == "frpd" {
                println!("nasc: {:?}", tag_item);
                i += tag_item_len_bytes;
                continue;
            }
            eprintln!(
                "EDIPlayer: ignored unsupported TAG item '{}' ({} bits)",
                tag_name, tag_len
            );
            i += tag_item_len_bytes;
        }
    }
}



#[derive(Debug)]
struct AudioDecoder {
    subchid: Option<u8>,
    //
    frame_len: usize,
    //
    frame_count: usize,
    sync_frames: usize,
    //
    sf_raw: Vec<u8>,
    //
    sf: Vec<u8>,
    sf_len: usize,
    sf_format_set: bool,
    sf_format_raw: u8,
    //
    num_aus: usize,
    au_start: Vec<usize>,
}

impl AudioDecoder {
    fn new(subchid: u8) -> Self {
        Self {
            subchid: Some(subchid),
            //
            frame_len: 0,
            //
            frame_count: 0,
            sync_frames: 0,
            sf_raw: Vec::new(),
            sf: Vec::new(),
            sf_len: 0,
            sf_format_set: false,
            sf_format_raw: 0,
            num_aus: 0,
            // au_start: Vec::new(),
            au_start: vec![0; 7],
        }
    }
    fn process(&mut self, subchid: u8, slice: &[u8], len: usize) {

        if (self.subchid != Some(subchid)) {
            // debug!("AudioDecoder: discard subcid {}", subchid);
            return;
        }

        // let len = slice.len();
        
        if self.frame_len != 0 {
            if self.frame_len != len {
                eprintln!("SuperframeFilter: different frame len {} (should be: {}) - frame ignored!", len, self.frame_len);
                return;
            }
        } else {
            if len < 10 {
                eprintln!("SuperframeFilter: frame len {} too short - frame ignored!", len);
                return;
            }
            if (5 * len) % 120 != 0 {
                eprintln!("SuperframeFilter: resulting Superframe len of len {} not divisible by 120 - frame ignored!", len);
                return;
            }
            self.frame_len = len;
            self.sf_len = 5 * self.frame_len;
            self.sf_raw.resize(self.sf_len, 0);
            self.sf.resize(self.sf_len, 0);
        }

        if self.frame_count == 5 {
            self.sf_raw.copy_within(self.frame_len.., 0);
        } else {
            self.frame_count += 1;
        }

        self.sf_raw.splice(((self.frame_count - 1) * self.frame_len)..((self.frame_count) * self.frame_len), slice.iter().copied());

        if self.frame_count < 5 {
            return;
        }

        // debug!(
        //     "AudioDecoder: {} decode {} bytes - len: {} - sf_len: {:?} - frame_cnt: {}",
        //     subchid,
        //     slice.len() / 8 - 3,
        //     len,
        //     self.sf_len,
        //     self.frame_count
        // );



        // NOTE: is this correct?
        // self.sf.copy_from_slice(&self.sf_raw);
        self.sf.copy_from_slice(&self.sf_raw[0..self.sf_len]);

        let (total_corr_count, uncorr_errors) = self.decode_superframe();

        if !self.check_sync() {
            if self.sync_frames == 0 {
                eprintln!("SuperframeFilter: Superframe sync started: {} frames", self.sync_frames);
            }
            self.sync_frames += 1;
            return;
        }

        if self.sync_frames > 0 {
            eprintln!("SuperframeFilter: Superframe sync succeeded after {} frame(s)", self.sync_frames);
            self.sync_frames = 0;
        }

        self.process_format();


        // debug!(
        //     "AudioDecoder: {} decode done - total_corr_count: {} - uncorr_errors: {}",
        //     subchid,
        //     total_corr_count,
        //     uncorr_errors
        // );


        self.frame_count = 0;
        
    }

    fn check_sync(&self) -> bool {
        let crc_stored = u16::from_be_bytes([self.sf[0], self.sf[1]]);
        let crc_calculated = calc_crc_fire_code(&self.sf[2..11]);

        // debug!("crc: {:04X} : {:04X}", crc_stored, crc_calculated);
        
        crc_stored == crc_calculated
    }

    fn process_format(&mut self) {
        if self.sf.len() < 11 {
            debug!("AudioDecoder: Superframe too short for format processing.");
            return;
        }
    
        // Prevent invalid AU start values
        if self.sf[3] == 0x00 && self.sf[4] == 0x00 {
            debug!("AudioDecoder: AU start values are zero! Aborting format processing.");
            return;
        }
    
        // Derive format from superframe
        let sf2 = self.sf[2];
    
        let dac_rate = (sf2 & 0x40) != 0;
        let sbr_flag = (sf2 & 0x20) != 0;
        let aac_channel_mode = (sf2 & 0x10) != 0;
        let ps_flag = (sf2 & 0x08) != 0;
        let mpeg_surround_config = sf2 & 0x07; // Already a 3-bit value
    
        debug!(
            "AudioDecoder: dac-rate: {} - sbr-flag: {} - cm: {} - ps: {} - sc: {}",
            dac_rate, sbr_flag, aac_channel_mode, ps_flag, mpeg_surround_config
        );
    
        // Determine codec type
        let codec = if sbr_flag {
            if ps_flag { "HE-AAC v2" } else { "HE-AAC" }
        } else {
            "AAC-LC"
        };
    
        let samplerate_khz = if dac_rate { 48 } else { 32 };
    
        let core_mode = if aac_channel_mode || ps_flag {
            "Stereo"
        } else {
            "Mono"
        };
    
        let mode = if mpeg_surround_config != 0 {
            format!("Surround ({})", core_mode)
        } else {
            core_mode.to_string()
        };
    
        let bitrate_kbps = self.sf_len / 120 * 8;
    
        debug!(
            "AudioDecoder: format: {}, {} kHz {} @ {} kBit/s",
            codec, samplerate_khz, mode, bitrate_kbps
        );
    
        // Determine number of AUs
        self.num_aus = if dac_rate {
            if sbr_flag { 3 } else { 6 }
        } else {
            if sbr_flag { 2 } else { 4 }
        };
    
        // Ensure au_start is large enough
        self.au_start = vec![0; self.num_aus + 1];
    
        // Calculate AU starts
        self.au_start[0] = if dac_rate {
            if sbr_flag { 6 } else { 11 }
        } else {
            if sbr_flag { 5 } else { 8 }
        };
    
        if self.num_aus >= self.au_start.len() {
            debug!(
                "AudioDecoder: num_aus {} exceeds au_start bounds {}",
                self.num_aus, self.au_start.len()
            );
            return;
        }
    
        self.au_start[self.num_aus] = self.sf_len / 120 * 110;
    
        self.au_start[1] = ((self.sf[3] as usize) << 4) | ((self.sf[4] >> 4) as usize);
        if self.num_aus >= 3 {
            self.au_start[2] = (((self.sf[4] & 0x0F) as usize) << 8) | (self.sf[5] as usize);
        }
        if self.num_aus >= 4 {
            self.au_start[3] = ((self.sf[6] as usize) << 4) | ((self.sf[7] >> 4) as usize);
        }
        if self.num_aus == 6 {
            self.au_start[4] = (((self.sf[7] & 0x0F) as usize) << 8) | (self.sf[8] as usize);
            self.au_start[5] = ((self.sf[9] as usize) << 4) | ((self.sf[10] >> 4) as usize);
        }
    
        debug!("AudioDecoder: AU start offsets: {:?}", self.au_start);

        // Simple plausibility check for correct order of start offsets
        for i in 0..self.num_aus {
            if self.au_start[i] >= self.au_start[i + 1] {
                debug!("AudioDecoder: AU mismatch {} >= {}", self.au_start[i], self.au_start[i + 1]);
                return;
                // return false;
            }
        }

        // debug!(
        //     "Extracted AU Start Values: {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X}",
        //     self.sf[3], self.sf[4], self.sf[5], self.sf[6], self.sf[7], self.sf[8], self.sf[9], self.sf[10]
        // );


        // iterate over AUs
        for i in 0..self.num_aus {
            // debug!("AudioDecoder: AU #{} - len sf: {} / {}", i, self.sf_len, self.sf.len());


            // let au_data = &self.sf[self.au_start[i]..];
            // let au_len = self.au_start[i + 1] - self.au_start[i];

            let au_data = &self.sf[self.au_start[i]..self.au_start[i + 1]];
            let au_len = self.au_start[i + 1] - self.au_start[i];

            debug!("AudioDecoder: AU #{} - len sf: {} / {} (au_len: {})", 
            i, self.sf_len, self.sf.len(), au_len);


            // // TODO: does never match...
            let au_crc_stored = ((au_data[au_len - 2] as u16) << 8) | au_data[au_len - 1] as u16;
            let au_crc_calced = calc_crc16_ccitt(&au_data[0..au_len - 2]);

            debug!("AudioDecoder: CRC {:04X} <> {:04X}", au_crc_stored, au_crc_calced);

            // au_len -= 2;

            // // send data to decoder

            // let audio_type = detect_audio(&au_data[0..au_len]);
            // // let audio_type = detect_audio(&self.sf);

            // // debug!("TYPE: {:?}", audio_type);

            // let d_is_aac = is_aac(&au_data[0..au_len]);

            // debug!("AudioDecoder: {}", d_is_aac);



            // // c++: aac_dec->DecodeFrame(au_data, au_len);
            // // c++: CheckForPAD(au_data, au_len);

        }

    }
    
    
    fn decode_superframe(&mut self) -> (i32, bool) {
        let sf = &mut self.sf;
        let sf_len = sf.len();
        let subch_index = sf_len / 120;
        let mut total_corr_count = 0;
        let mut uncorr_errors = false;
    
        for i in 0..subch_index {
            let mut rs_packet = [0u8; 120];
    
            for (pos, rs_byte) in rs_packet.iter_mut().enumerate() {
                *rs_byte = sf[pos * subch_index + i];
            }
    
            let mut corr_pos = [0i32; 32]; 
            // let corr_count = self.decode_rs_char(&mut rs_packet, &mut corr_pos);
            let corr_count = 0;

            if corr_count == -1 {
                uncorr_errors = true;
            } else {
                total_corr_count += corr_count;
            }
    
            for j in 0..corr_count as usize {
                let pos = corr_pos[j] - 135;
                if pos < 0 {
                    continue;
                }
                let pos = pos as usize;
                sf[pos * subch_index + i] = rs_packet[pos];
            }
        }

        // self.sf = sf.to_vec();
    
        (total_corr_count, uncorr_errors)
    }
    
    
    
    fn decode_rs_char(&self, rs_packet: &mut [u8], corr_pos: &mut [i32]) -> i32 {
        // Placeholder for RS decoding logic
        // Replace with actual decoding call
        0
    }
}