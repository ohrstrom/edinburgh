mod audio;
mod edi_frame_extractor;

use shared::utils;

use log;
use std::io;
use std::sync::Arc;

use clap::Parser;
use edi_frame_extractor::EDIFrameExtractor;
use tokio::io::Interest;
use tokio::net::TcpStream;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::RwLock;
use tokio::io::{BufReader, AsyncBufReadExt};

use shared::edi::bus::{init_event_bus, EDIEvent};
use shared::edi::EDISource;

use audio::AudioDecoder;

struct EDIHandler {
    receiver: UnboundedReceiver<EDIEvent>,
    scid: Arc<RwLock<Option<u8>>>,
    audio_decoder: Option<AudioDecoder>,
}

impl EDIHandler {
    pub fn new(scid: Arc<RwLock<Option<u8>>>, receiver: UnboundedReceiver<EDIEvent>) -> Self {
        Self {
            receiver,
            scid,
            audio_decoder: None,
        }
    }

    pub async fn run(mut self) {
        while let Some(event) = self.receiver.recv().await {
            match event {
                EDIEvent::EnsembleUpdated(ensemble) => {
                    log::debug!(
                        "Ensemble updated: 0x{:4x} - {}",
                        ensemble.eid.unwrap_or(0),
                        ensemble.complete
                    );
                    if ensemble.complete {
                        // println!("{:?}", ensemble);
                        for service in &ensemble.services {
                            println!("{:?}", service);
                        }
                    }
                }
                EDIEvent::AACPFramesExtracted(r) => {
                    let scid = *self.scid.read().await;
                    if r.scid == scid.unwrap_or(0) {
                        // println!("R: {:?}", r);

                        if r.audio_format.is_none() {
                            log::warn!("Audio format is None for SCID: {}", r.scid);
                            continue;
                        }

                        let audio_format = r.audio_format.as_ref().unwrap();

                        // create aduio decoder if needed
                        if self.audio_decoder.is_none() {
                            let audio_decoder = AudioDecoder::new(audio_format.clone());
                            self.audio_decoder = Some(audio_decoder);
                        }

                        // feed audio decoder with frames
                        if let Some(ref mut audio_decoder) = self.audio_decoder {
                            audio_decoder.feed(&r);
                        }

                        // for frame in r.frames {
                        //     self.audio_decoder.feed(&frame);
                        // }
                    }
                }
                EDIEvent::MOTImageReceived(m) => {
                    // log::debug!("MOT image received: SCID = {}", m.scid);
                }
                EDIEvent::DLObjectReceived(d) => {
                    // log::debug!("DL obj received: SCID = {}", d.scid);
                }
            }
        }
    }
}

/// EDInburgh
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// EDI host:port to connect to.
    #[arg(short, long)]
    addr: String,

    /// Subchannel ID to extract audio from. [optional]
    #[arg(short, long)]
    scid: Option<u8>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    std::env::set_var("RUST_LOG", "debug");

    // env_logger::init();
    env_logger::builder()
        .format_timestamp(None)
        // .format(|buf, record| {
        //     writeln!(buf, "{}: {}", record.level(), record.args())
        // })
        .init();

    let args = Args::parse();

    let scid = Arc::new(RwLock::new(args.scid));

    // cli args
    let endpoint = args.addr;

    log::debug!("endpoint: {}", endpoint);

    let stream = TcpStream::connect(endpoint).await?;

    let mut filled = 0;

    let mut extractor = EDIFrameExtractor::new();

    let event_rx = init_event_bus();

    let mut source = EDISource::new(args.scid, None, None);

    let event_handler = EDIHandler::new(Arc::clone(&scid), event_rx);

    tokio::spawn(async move {
        event_handler.run().await;
    });

    // let scid_input = Arc::clone(&scid);
    // listen for keyboard input 1 - 9
    tokio::spawn(async move {
        // read key 1-9 from stdin
    });

    loop {
        let ready = stream.ready(Interest::READABLE).await?;
        if ready.is_readable() {
            match stream.try_read(&mut extractor.frame.data[filled..]) {
                Ok(0) => {
                    log::info!("Connection closed by peer");
                    break;
                }
                Ok(n) => {
                    filled += n;

                    if filled < extractor.frame.data.len() {
                        continue;
                    }

                    // Process the received data
                    if let Some(offset) = extractor.frame.find_sync_magic() {
                        if offset > 0 {
                            extractor.frame.data.copy_within(offset.., 0);
                            filled -= offset;
                            continue;
                        }

                        if extractor.frame.check_completed() {
                            // log::debug!("frame: {}", extractor.frame);

                            source.feed(&extractor.frame.data).await;

                            extractor.frame.reset();
                            filled = 0;
                        }
                    }
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    continue;
                }
                Err(e) => {
                    return Err(e.into());
                }
            }
        }
    }
    Ok(())
}
