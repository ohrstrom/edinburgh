use nix::fcntl::{fcntl, FcntlArg, OFlag};
use nix::sys::select::{select, FdSet};
use nix::sys::time::{TimeVal, TimeValLike};
use std::io::{self, Read};
use std::os::unix::io::AsRawFd;
use std::time::{Duration, Instant};

struct EDISource {
    ensemble_frame: Vec<u8>,
    layer: String,
    sync_magic: Vec<(Vec<u8>, String)>,
    initial_frame_size: usize,
    ensemble_bytes_count: usize,
    ensemble_frames_count: usize,
    ensemble_bytes_total: Option<usize>,
    ensemble_progress_next_ms: usize,
}

impl EDISource {
    fn new() -> Self {
        let sync_magic = vec![
            (vec![b'A', b'F'], "AF".to_string()),
            (vec![b'f', b'i', b'o', b'_'], "File IO".to_string()),
        ];

        EDISource {
            ensemble_frame: vec![0; 4096],
            layer: String::new(),
            sync_magic,
            initial_frame_size: 4096,
            ensemble_bytes_count: 0,
            ensemble_frames_count: 0,
            ensemble_bytes_total: None,
            ensemble_progress_next_ms: 0,
        }
    }

    fn add_sync_magic(&mut self, pattern: Vec<u8>, name: &str) {
        self.sync_magic.push((pattern, name.to_string()));
    }

    fn detect_sync_magic(&self) -> &str {
        for offset in 0..self.ensemble_frame.len() {
            for (pattern, name) in &self.sync_magic {
                if self.ensemble_frame[offset..].starts_with(pattern) {
                    return name;
                }
            }
        }
        "other"
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
            // forward to player
            self.ensemble_frame.clear(); // Placeholder for actual processing
        } else {
            // parse TAG packet for TAG items (skipping any TAG packet padding)
            let mut i = 0;
            while i < self.ensemble_frame.len() - 8 {
                let tag_item = &self.ensemble_frame[8 + i..];
                if tag_item.len() < 8 {
                    break; // Avoid out-of-bounds access
                }
                let tag_name_bytes = &tag_item[..4];
                let tag_name = match std::str::from_utf8(tag_name_bytes) {
                    Ok(name) => name.to_string(),
                    Err(_) => format!("{:?}", tag_name_bytes),
                };
                let tag_len = (tag_item[4] as usize) << 24
                    | (tag_item[5] as usize) << 16
                    | (tag_item[6] as usize) << 8
                    | (tag_item[7] as usize);
                let tag_value = &tag_item[8..];

                if tag_value.len() < tag_len {
                    break; // Avoid out-of-bounds access
                }

                let tag_item_len_bytes = 4 + 4 + (tag_len + 7) / 8;

                // AF Packet/PFT Fragment
                if tag_name == "afpf" {
                    // forward to player
                    // Placeholder for actual processing
                    // observer.EnsembleProcessFrame(tag_value);

                    continue;
                }

                // Timestamp - ignored
                if tag_name == "time" {
                    continue;
                }

                eprintln!(
                    "EDISource: ignored unsupported TAG item '{}' ({} bits)",
                    tag_name, tag_len
                );

                i += tag_item_len_bytes;
            }
        }
    }

    fn update_progress(&self) -> bool {
        // Placeholder for actual progress update logic
        true
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let stdin = io::stdin();
    let stdin_fd = stdin.as_raw_fd();
    let mut buffer = [0; 4096];
    let timeout = Duration::from_secs(10);
    let mut last_input_time = Instant::now();
    let mut edi_source = EDISource::new();

    // Set non-blocking mode
    let old_flags = fcntl(stdin_fd, FcntlArg::F_GETFL).unwrap();
    fcntl(
        stdin_fd,
        FcntlArg::F_SETFL(OFlag::from_bits_truncate(old_flags) | OFlag::O_NONBLOCK),
    )
    .unwrap();

    let mut filled = 0;
    let mut sync_skipped = 0;

    loop {
        // Use loop to do some regular work
        // observer.EnsembleDoRegularWork(); // Placeholder for actual work

        let mut fds = FdSet::new();
        fds.insert(stdin_fd);

        let mut select_timeval = TimeVal::nanoseconds(100000);

        let ready_fds = select(None, &mut fds, None, None, Some(&mut select_timeval)).unwrap();
        if ready_fds == 0 {
            if last_input_time.elapsed() >= timeout {
                println!("No input for 10 seconds, exiting...");
                break;
            }
            continue;
        }

        let bytes_read = stdin.lock().read(&mut buffer)?;
        if bytes_read == 0 {
            if last_input_time.elapsed() >= timeout {
                println!("No input for 10 seconds, exiting...");
                break;
            }
            continue;
        }

        last_input_time = Instant::now();
        edi_source
            .ensemble_frame
            .extend_from_slice(&buffer[..bytes_read]);
        filled += bytes_read;

        if filled < edi_source.ensemble_frame.len() {
            continue;
        }

        // Check for frame sync i.e. if any sync magic matches
        let mut offset = 0;
        let mut detected_sync_magic = "other".to_string();
        for i in 0..edi_source.ensemble_frame.len()
            - (edi_source
                .sync_magic
                .iter()
                .map(|(p, _)| p.len())
                .max()
                .unwrap_or(0)
                - 1)
        {
            let sync_magic = edi_source.detect_sync_magic();
            if sync_magic != "other" {
                detected_sync_magic = sync_magic.to_string(); // clone into an owned String
                offset = i;
                break;
            }
        }

        if offset > 0 {
            // Buffer not (yet) synced, discard buffer start
            edi_source.ensemble_frame.drain(0..offset);
            filled -= offset;
            sync_skipped += offset;
        } else {
            // Buffer synced
            if sync_skipped > 0 {
                eprintln!("EDISource: skipping {} bytes for sync", sync_skipped);
                sync_skipped = 0;
            }

            let frame_completed = {
                // Pass the owned value by reference
                edi_source.check_frame_completed(&detected_sync_magic)
            };

            if frame_completed {
                edi_source.ensemble_frames_count += 1;

                // If present, update progress every 500ms
                if let Some(_total) = edi_source.ensemble_bytes_total {
                    if edi_source.ensemble_frames_count * 24 >= edi_source.ensemble_progress_next_ms
                    {
                        if !edi_source.update_progress() {
                            return Ok(());
                        }
                        edi_source.ensemble_progress_next_ms += 500;
                    }
                }

                edi_source.process_completed_frame(&detected_sync_magic);

                edi_source
                    .ensemble_frame
                    .resize(edi_source.initial_frame_size, 0);
                filled = 0;
            }
        }
    }

    Ok(())
}
