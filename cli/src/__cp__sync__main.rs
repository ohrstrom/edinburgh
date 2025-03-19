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


    let mut stream = TcpStream::connect(endpoint)?;
    let stream_fd = stream.as_raw_fd();
    let stream_fd_old_flags = fcntl(stream_fd, FcntlArg::F_GETFL)?;

    fcntl(
        stream_fd,
        FcntlArg::F_SETFL(OFlag::from_bits_truncate(stream_fd_old_flags) | OFlag::O_NONBLOCK),
    )?;

    let mut filled = 0;
    let mut sync_skipped = 0;
    let mut edi_frame_extractor = EDIFrameExtractor::new();


    loop {
        match stream.read(&mut edi_frame_extractor.frame.data[filled..]) {
            Ok(0) => {
                info!("Connection closed by peer");
                break;
            }
            Ok(n) => {
                filled += n;

                if filled < edi_frame_extractor.frame.data.len() {
                    continue;
                }

                if let Some(offset) = edi_frame_extractor.frame.find_sync_magic() {
                    if offset > 0 {
                        edi_frame_extractor.frame.data.copy_within(offset.., 0);
                        filled -= offset;
                        sync_skipped += offset;

                        continue;
                    } else {
                        sync_skipped = 0;
                    }

                    if edi_frame_extractor.frame.check_completed() {
                        edi_frame_extractor.frame.reset();
                        filled = 0;
                    }
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(std::time::Duration::from_millis(100));
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
