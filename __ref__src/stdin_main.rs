use nix::fcntl::{fcntl, FcntlArg, OFlag};
use nix::sys::select::{select, FdSet};
use nix::sys::time::{TimeVal, TimeValLike};
use std::io::{self, Read};
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
    let stdin = io::stdin();
    let stdin_fd = stdin.as_raw_fd();

    let mut buffer = [0; 4096];

    let timeout = Duration::from_secs(5);
    let mut last_input_time = Instant::now();

    // Set non-blocking mode
    let old_flags = fcntl(stdin_fd, FcntlArg::F_GETFL).unwrap();
    fcntl(
        stdin_fd,
        FcntlArg::F_SETFL(OFlag::from_bits_truncate(old_flags) | OFlag::O_NONBLOCK),
    )
    .unwrap();

    // EDI
    let mut filled = 0;
    let mut edi_source = EDISource::new();

    loop {
        let mut fds = FdSet::new();
        fds.insert(unsafe { BorrowedFd::borrow_raw(stdin_fd) });

        let mut select_timeval = TimeVal::nanoseconds(100000);

        let ready_fds = select(None, &mut fds, None, None, Some(&mut select_timeval)).unwrap();
        if ready_fds == 0 {
            if last_input_time.elapsed() >= timeout {
                println!("No input for 5 seconds, exiting...");
                break;
            }
            continue;
        }

        let bytes_read = stdin.lock().read(&mut buffer)?;
        if bytes_read == 0 {
            if last_input_time.elapsed() >= timeout {
                println!("No input for 5 seconds, exiting...");
                break;
            }
            continue;
        }

        last_input_time = Instant::now();


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
