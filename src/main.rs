use nix::fcntl::{fcntl, FcntlArg, OFlag};
use nix::sys::select::{select, FdSet};
use nix::sys::time::{TimeVal, TimeValLike};
use std::env;
use std::io::Read;
use std::net::TcpStream;
use std::os::fd::BorrowedFd;
use std::os::unix::io::AsRawFd;
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug)]
struct FIG0Header {
    cn: bool,
    oe: bool,
    pd: bool,
    extension: u8,
}

impl FIG0Header {
    fn new(byte: u8) -> Self {
        Self {
            cn: (byte & 0x80) != 0,  // Bit 7
            oe: (byte & 0x40) != 0,  // Bit 6
            pd: (byte & 0x20) != 0,  // Bit 5
            extension: byte & 0x1F,  // Bits 0-4
        }
    }
}

// Dummy CRC-16 CCITT function – replace with your actual CRC calculation.
pub fn calc_crc16_ccitt(data: &[u8]) -> u16 {
    let initial_invert = true;
    let final_invert = true;
    let gen_polynom: u16 = 0x1021;

    let mut crc: u16 = if initial_invert { 0xFFFF } else { 0x0000 };

    for &byte in data {
        crc ^= (byte as u16) << 8;
        for _ in 0..8 {
            if crc & 0x8000 != 0 {
                crc = (crc << 1) ^ gen_polynom;
            } else {
                crc <<= 1;
            }
        }
    }

    if final_invert {
        crc ^= 0xFFFF;
    }

    crc
}

fn decode_fic(fic_data: &[u8], fic_len: usize) {
    // println!("Processing FIC data {:?}", fic_len);
    // Implement actual FIC processing as needed.

    if (fic_len % 32) != 0 {
        println!("Invalid FIC data length {:?}", fic_len);
        return;
    }

    for chunk in fic_data.chunks_exact(32) {
        proccess_fib(chunk);
    }
}

fn proccess_fib(data: &[u8]) {
    let crc_stored = u16::from_be_bytes([data[30], data[31]]);
    let crc_calced = calc_crc16_ccitt(&data[0..30]);

    // NOTE: for some reasons here CRC calculation is not working.
    if crc_stored != crc_calced {
        return;
    }

    // println!("CRC: {:04X} : {:04X}", crc_stored, crc_calced);

    let mut offset = 0;

    // Iterate over FIGs
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
            0 => process_fig0(&data[offset..offset + len], len),
            1 => process_fig1(&data[offset..offset + len], len),
            _ => eprintln!(
                "FICDecoder: received unsupported FIG {} with {} bytes",
                fig_type, len
            ),
        }

        offset += len;
    }
}

fn process_fig0(data: &[u8], len: usize) {
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
            // println!("FIG0: {}", header.extension);
            process_fig0_0(data, len);
        },
        1 => {
            // println!("FIG0: {}", header.extension);
            process_fig0_1(data, len);
        },
        _ => return,
    }
}

fn process_fig0_0(data: &[u8], len: usize) {
	// FIG 0/0 - Ensemble information
	// EId and alarm flag only

    if len < 4 {
        eprintln!("FICDecoder: FIG 0/0 has invalid length {}", len);
        return;
    }

    // Extract 16-bit Ensemble ID (Big-Endian)
    let eid = u16::from_be_bytes([data[0], data[1]]);

    // Extract alarm flag (bit 5 of data[2])
    let al_flag = (data[2] & 0x20) != 0;

    println!("FICDecoder: FIG 0/0 EId: 0x{:04X} - alarm flag: {}", eid, al_flag);
}

fn process_fig0_1(data: &[u8], len: usize) {
    return;
    println!("FICDecoder: FIG 0/1 with {} bytes", data.len());
}


fn process_fig1(data: &[u8], len: usize) {
    return;
    println!("Processing FIG1 with {} bytes: {:?}", data.len(), data);
}

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

struct EDISource {
    ensemble_frame: Vec<u8>,
    layer: String,
    sync_magic: Vec<SyncMagic>,
    initial_frame_size: usize,
}

impl EDISource {
    fn new() -> Self {
        let sync_magic = vec![
            SyncMagic::new(vec![b'A', b'F'], "AF"),
            SyncMagic::new(vec![b'f', b'i', b'o', b'_'], "File IO"),
        ];
        EDISource {
            ensemble_frame: vec![0; 4096],
            layer: String::new(),
            sync_magic,
            initial_frame_size: 4096,
        }
    }

    /// Scan the ensemble_frame for a sync magic.
    /// Returns Some((offset, name)) if a match is found.
    fn find_sync_magic(&self) -> Option<(usize, &str)> {
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

    fn check_frame_completed(&mut self, matched_sync_magic: &str) -> bool {
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

    fn process_completed_frame(&mut self, matched_sync_magic: &str) {
        // show layer
        if self.layer != matched_sync_magic {
            self.layer = matched_sync_magic.to_string();
            eprintln!("EDISource: detected {} layer", self.layer);
        }

        if matched_sync_magic == "AF" {
            // println!("process: {}", matched_sync_magic);
        } else {
            println!("what to do???: {}", matched_sync_magic);
        }
    }
}

struct EDIPlayer {
    next_frame_time: Option<Instant>,
    disable_int_catch_up: bool,
}

impl EDIPlayer {
    fn new(disable_int_catch_up: bool) -> Self {
        Self {
            next_frame_time: None,
            disable_int_catch_up,
        }
    }

    fn process_frame(&mut self, data: &[u8]) {
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

        self.decode_frame(data);
    }

    fn decode_frame(&self, edi_frame: &[u8]) {
        // Check minimum frame length needed for header fields.
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
                        decode_fic(fic_data, fic_len);
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
                let subchid = tag_item[8] >> 2;
                // Here you might lock your audio service and feed data.
                // println!("EDIPlayer: received est tag for subchid {}", subchid);
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // expect the endpoint as the first command-line argument.
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <host:port>", args[0]);
        std::process::exit(1);
    }
    let endpoint = &args[1];
    println!("Connecting to {}", endpoint);

    // connect to the TCP endpoint.
    let mut stream = TcpStream::connect(endpoint)?;
    let stream_fd = stream.as_raw_fd();

    // set the stream to non-blocking mode.
    let old_flags = fcntl(stream_fd, FcntlArg::F_GETFL)?;
    fcntl(
        stream_fd,
        FcntlArg::F_SETFL(OFlag::from_bits_truncate(old_flags) | OFlag::O_NONBLOCK),
    )?;

    let mut buffer = [0; 4096];
    let timeout = Duration::from_secs(5);
    let mut last_input_time = Instant::now();

    // EDI
    let mut filled = 0;
    let mut sync_skipped = 0;
    let mut edi_source = EDISource::new();
    let mut edi_player = EDIPlayer::new(false);

    loop {
        let mut fds = FdSet::new();
        // insert the stream's file descriptor.
        fds.insert(unsafe { BorrowedFd::borrow_raw(stream_fd) });

        // set a short timeout for select.
        let mut select_timeval = TimeVal::nanoseconds(100_000);
        let ready_fds = select(None, &mut fds, None, None, Some(&mut select_timeval))?;

        if ready_fds == 0 {
            if last_input_time.elapsed() >= timeout {
                println!("No input for 5 seconds, exiting...");
                break;
            }
            continue;
        }

        // read data from the TCP stream.
        let bytes_read = stream.read(&mut edi_source.ensemble_frame[filled..])?;
        filled += bytes_read;

        if bytes_read == 0 {
            if last_input_time.elapsed() >= timeout {
                println!("No input for 5 seconds, exiting...");
                break;
            }
            continue;
        }

        last_input_time = Instant::now();

        if filled < edi_source.ensemble_frame.len() {
            continue;
        }

        // now we have a complete frame
        // println!("EDI frame: {} : {}", edi_source.ensemble_frame.len(), filled);

        // println!("magic: {:?}", edi_source.find_sync_magic());

        // check for frame sync i.e. if any sync magic matches
        if let Some((offset, name)) = edi_source.find_sync_magic() {
            let name = name.to_owned();
            if offset > 0 {
                edi_source.ensemble_frame.copy_within(offset.., 0);
                filled = filled.saturating_sub(offset);
                sync_skipped += offset;
                // println!("sync magic '{}' found at offset {} : {}", name, offset, filled);
                // println!(
                //     "sync magic '{}' found at offset {}. Discarding {} bytes for sync",
                //     name, offset, sync_skipped
                // );
                continue;
            } else {
                // println!("sync magic '{}' at offset 0", name);
                // Buffer is synced.
                if sync_skipped > 0 {
                    sync_skipped = 0;
                }

                if edi_source.check_frame_completed(&name) {
                    edi_source.process_completed_frame(&name);

                    // player
                    edi_player.process_frame(&edi_source.ensemble_frame);

                    edi_source
                        .ensemble_frame
                        .resize(edi_source.initial_frame_size, 0);
                    filled = 0;
                } else {
                    println!("frame not completed: {}", name);
                }
            }
        } else {
            println!("no sync magic found in frame!");
        }

        // reset for the next frame:
        // filled = 0;
        // edi_source.ensemble_frame.fill(0);
    }

    Ok(())
}
