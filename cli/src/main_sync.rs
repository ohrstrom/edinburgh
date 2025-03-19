mod audio;
mod edi;
mod edi_frame_extractor;
mod utils;

use colog;
use log::{debug, error, info};
use nix::fcntl::{fcntl, FcntlArg, OFlag};
use std::env;
use std::io::Read;
use std::os::unix::io::AsRawFd;

use std::net::TcpStream;

use futures::channel::mpsc::unbounded;

use edi_frame_extractor::EDIFrameExtractor;

use edi::bus::EDIEvent;
use edi::EDISource;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // log setup
    std::env::set_var("RUST_LOG", "debug");
    colog::init();

    // cli args
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <host:port>", args[0]);
        std::process::exit(1);
    }
    let endpoint = &args[1];

    info!("Connecting:  {endpoint}");

    // tcp connection
    let mut stream = TcpStream::connect(endpoint)?;
    let stream_fd = stream.as_raw_fd();
    let stream_fd_old_flags = fcntl(stream_fd, FcntlArg::F_GETFL)?;

    // set the stream to non-blocking mode.
    fcntl(
        stream_fd,
        FcntlArg::F_SETFL(OFlag::from_bits_truncate(stream_fd_old_flags) | OFlag::O_NONBLOCK),
    )?;

    // EDI
    let (event_tx, mut event_rx) = unbounded::<EDIEvent>();

    let mut filled = 0;
    let mut sync_skipped = 0;
    let mut edi_frame_extractor = EDIFrameExtractor::new();

    let mut edi_source = EDISource::new(event_tx);

    loop {
        match stream.read(&mut edi_frame_extractor.frame.data[filled..]) {
            Ok(0) => {
                // Connection closed
                info!("Connection closed by peer");
                break;
            }
            Ok(n) => {
                filled += n;

                if filled < edi_frame_extractor.frame.data.len() {
                    continue;
                }

                // Process the received data
                if let Some(offset) = edi_frame_extractor.frame.find_sync_magic() {
                    if offset > 0 {
                        log::debug!("offset: {}", offset);
                        edi_frame_extractor.frame.data.copy_within(offset.., 0);
                        filled -= offset;
                        sync_skipped += offset;

                        continue;
                    } else {
                        // log::debug!("sync after: {}", sync_skipped);
                        sync_skipped = 0;
                    }

                    // check frame completeness
                    if edi_frame_extractor.frame.check_completed() {
                        log::debug!("frame: {}", edi_frame_extractor.frame);
                        // log::debug!("frame: {}", edi_frame_extractor.frame);
                        // edi_frame_extractor.process_frame();

                        // edi_source.feed(&edi_frame_extractor.frame.data).await;

                        // log::debug!("Frame completed: {}", edi_frame_extractor.frame.data.len());

                        /*
                        match edi_frame_extractor.process_frame() {
                            Ok(r) => {
                                // debug!("Frame completed: tags: {} - pcm: {}", r.tags.len(), r.pcm_data.len());

                                if !r.au_frames.is_empty() {
                                    // debug!("au frames:  {}", r.au_frames.len());
                                    for au_frame in r.au_frames {
                                        if let Err(e) = au_tx.send(au_frame) {
                                            error!("Failed to send AU frame over channel: {}", e);
                                        }
                                    }
                                }

                                if !r.pcm.is_empty() {
                                    // debug!("pcm frames: {}", r.pcm.len());

                                    // TODO: send pcm data via websocket
                                    if let Err(e) = pcm_tx.send(r.pcm) {
                                        error!("Failed to send PCM data over channel: {}", e);
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Error processing frame: {}", e);
                            }
                        }
                        */

                        // debug!("Frame completed: {}", edi_frame_extractor.frame.data.len());

                        let leftover = filled.saturating_sub(edi_frame_extractor.frame.data.len());

                        if leftover > 0 {
                            debug!("preserving {} bytes leftover", leftover);
                            // TODO: i guess this is not correct ;) - do we even need it?
                            let framne_start = edi_frame_extractor.frame.data.len();
                            edi_frame_extractor
                                .frame
                                .data
                                .copy_within(framne_start..filled, 0);
                            filled = leftover;
                        } else {
                            edi_frame_extractor.frame.reset();
                            filled = 0;
                        }
                    }
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // Non-blocking mode: No data available, continue looping
                std::thread::sleep(std::time::Duration::from_millis(100)); // Avoid busy-waiting
                continue;
            }
            Err(e) => {
                error!("Error reading from stream: {}", e);
                break;
            }
        }
    }

    Ok(())
}
