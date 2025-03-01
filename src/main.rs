mod edi;
mod utils;

use colog;
use log::{debug, info, trace, warn};
use nix::fcntl::{fcntl, FcntlArg, OFlag};
use nix::sys::select::{select, FdSet};
use nix::sys::time::{TimeVal, TimeValLike};
use std::env;
use std::io::Read;
use std::net::TcpStream;
use std::os::fd::BorrowedFd;
use std::os::unix::io::AsRawFd;
use std::time::{Duration, Instant};

use edi::EDISource;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    std::env::set_var("RUST_LOG", "debug");

    colog::init();

    // expect the endpoint as the first command-line argument.
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <host:port>", args[0]);
        std::process::exit(1);
    }
    let endpoint = &args[1];

    info!("Connecting:  {endpoint}");

    // connect to the TCP endpoint.
    let mut stream = TcpStream::connect(endpoint)?;
    let stream_fd = stream.as_raw_fd();

    // set the stream to non-blocking mode.
    let old_flags = fcntl(stream_fd, FcntlArg::F_GETFL)?;
    fcntl(
        stream_fd,
        FcntlArg::F_SETFL(OFlag::from_bits_truncate(old_flags) | OFlag::O_NONBLOCK),
    )?;

    let timeout = Duration::from_secs(5);
    let mut last_input_time = Instant::now();

    // EDI
    let mut filled = 0;
    let mut sync_skipped = 0;
    let mut edi_source = EDISource::new();

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

        // check for frame sync i.e. if any sync magic matches
        if let Some((offset, name)) = edi_source.find_sync_magic() {
            let name = name.to_owned();
            if offset > 0 {
                edi_source.ensemble_frame.copy_within(offset.., 0);
                filled = filled.saturating_sub(offset);
                sync_skipped += offset;
                // println!(
                //     "sync magic '{}' found at offset {}. Discarding {} bytes for sync",
                //     name, offset, sync_skipped
                // );
                continue;
            } else {
                // buffer is synced.
                if sync_skipped > 0 {
                    sync_skipped = 0;
                }

                if edi_source.check_frame_completed(&name) {
                    edi_source.process_completed_frame(&name);

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
    }

    Ok(())
}
