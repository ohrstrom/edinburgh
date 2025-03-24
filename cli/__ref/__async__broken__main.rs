mod audio;
mod edi;
mod edi_frame_extractor;
mod utils;

use tokio::io::{AsyncReadExt};
use tokio::net::TcpStream;
use tokio::time::{sleep, Duration};
use futures::channel::mpsc::unbounded;
use log::{debug, error, info};
use colog;
use edi_frame_extractor::EDIFrameExtractor;
use edi::bus::EDIEvent;
use edi::EDISource;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // log setup
    std::env::set_var("RUST_LOG", "debug");
    colog::init();

    // cli args
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <host:port>", args[0]);
        std::process::exit(1);
    }
    let endpoint = &args[1];

    info!("Connecting:  {endpoint}");

    // Async TCP connection
    let mut stream = TcpStream::connect(endpoint).await?;
    stream.set_nodelay(true)?; // Disable Nagle's algorithm for low-latency

    // EDI
    let (event_tx, mut event_rx) = unbounded::<EDIEvent>();

    let mut filled = 0;
    let mut sync_skipped = 0;
    let mut edi_frame_extractor = EDIFrameExtractor::new();
    let mut edi_source = EDISource::new(event_tx);

    let mut buffer = [0u8; 4096]; // Temporary buffer for reading

    loop {
        match stream.read(&mut buffer).await {
            Ok(0) => {
                info!("Connection closed by peer");
                break;
            }
            Ok(n) => {

                let data_len = edi_frame_extractor.frame.data.len();
                if filled + n > data_len {
                    error!(
                        "Buffer overflow: frame.data size {} is smaller than required {}",
                        data_len, filled + n
                    );
                    continue; // Prevent out-of-bounds panic
                }

                edi_frame_extractor.frame.data[filled..filled + n].copy_from_slice(&buffer[..n]);
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
                        edi_source.feed(&edi_frame_extractor.frame.data).await;

                        let leftover = filled.saturating_sub(edi_frame_extractor.frame.data.len());
                        if leftover > 0 {
                            debug!("preserving {} bytes leftover", leftover);
                            // edi_frame_extractor.frame.data.copy_within(
                            //     edi_frame_extractor.frame.data.len()..filled,
                            //     0,
                            // );
                            filled = leftover;
                        } else {
                            edi_frame_extractor.frame.reset();
                            filled = 0;
                        }
                    }
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                sleep(Duration::from_millis(100)).await; // Async sleep to avoid busy waiting
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
