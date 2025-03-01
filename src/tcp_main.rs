use nix::fcntl::{fcntl, FcntlArg, OFlag};
use nix::sys::select::{select, FdSet};
use nix::sys::time::{TimeVal, TimeValLike};
use std::env;
use std::io::{Read};
use std::net::TcpStream;
use std::os::fd::BorrowedFd;
use std::os::unix::io::AsRawFd;
use std::time::{Duration, Instant};


struct EDISource {
    ensemble_frame: Vec<u8>,
}

impl EDISource {
    fn new() -> Self {
        EDISource {
            ensemble_frame: vec![0; 4096],
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
        let bytes_read = stream.read(&mut buffer)?;
        if bytes_read == 0 {
            if last_input_time.elapsed() >= timeout {
                println!("No input for 5 seconds, exiting...");
                break;
            }
            continue;
        }

        last_input_time = Instant::now();

        // println!("Read {} bytes", bytes_read);
        // println!("LIT  {:?}", last_input_time.elapsed());
        // (optionally, you can process buffer[0..bytes_read] here)


        // handle EDI source
        edi_source
            .ensemble_frame
            .extend_from_slice(&buffer[..bytes_read]);

        filled += bytes_read;

        println!("FIL frame: {} : {}", filled, edi_source.ensemble_frame.len());

        if filled < edi_source.ensemble_frame.len() {
            continue;
        }


        println!("EDI frame: {} bytes", edi_source.ensemble_frame.len());

    }

    Ok(())
}
