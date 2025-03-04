mod edi;
mod utils;

use colog;
use log::{debug, info, error};
use nix::fcntl::{fcntl, FcntlArg, OFlag};
use std::env;
use std::io::Read;
use std::net::TcpStream;
use std::os::unix::io::AsRawFd;

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


    // EDI frame
    let mut filled = 0;
    let mut sync_skipped = 0;
    let mut edi_source = EDISource::new();

    loop {

        match stream.read(&mut edi_source.frame.data[filled..]) {
            Ok(0) => {
                // Connection closed
                info!("Connection closed by peer");
                break;
            }
            Ok(n) => {
                // Successfully read `n` bytes
                // debug!("Received {} bytes: {:?}", n, &buffer[..n]);
                filled += n;

                // debug!("Received {} bytes - filled: {}", n, filled);

                if filled < edi_source.frame.data.len() {
                    // continue reading until the buffer is full
                    continue;
                }

                // Process the received data
                if let Some(offset) = edi_source.frame.find_sync_magic() {

                    if offset > 0 {
                        edi_source.frame.data.copy_within(offset.., 0);
                        filled -= offset;
                        sync_skipped += offset;

                        continue;

                    } else {
                        sync_skipped = 0;

                    }

                    // check frame completeness
                    if edi_source.frame.check_completed() {

                        edi_source.process_frame();

                        // debug!("Frame completed: {}", edi_source.frame.data.len());

                        edi_source.frame.reset();
                        filled = 0;

                        // // preserve leftover bytes
                        // let leftover = filled - edi_source.frame.data.len();
                        // if leftover > 0 {
                        //     let frame_start = edi_source.frame.data.len();
                        //     edi_source.frame.data.copy_within(frame_start.., 0);
                        // }
                        // filled = leftover;
    
                        
                    }

                    // debug!("Frame completed: {}", edi_source.frame);



                    // reset frame & counter
                    // edi_source.frame.reset();
                    // filled = 0;

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