mod edi;
mod edi_frame_extractor;
mod utils;

use colog;
use log;
use std::io;
use std::sync::{Arc, Mutex};

use derivative::Derivative;
use edi_frame_extractor::EDIFrameExtractor;
use faad2::Decoder;
use futures::channel::mpsc::unbounded;
use clap::Parser;
use rodio::{buffer::SamplesBuffer, OutputStream, Sink};
use tokio::io::Interest;
use tokio::net::TcpStream;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

use edi::bus::EDIEvent;
use edi::{AACPFrame, EDISource};

#[derive(Derivative)]
#[derivative(Debug)]
pub struct AudioDecoder {
    #[derivative(Debug = "ignore")]
    decoder: Decoder,
    #[derivative(Debug = "ignore")]
    _stream: OutputStream,
    #[derivative(Debug = "ignore")]
    sink: Sink,
}

impl AudioDecoder {
    pub fn new() -> Self {
        // ASC: audio specific config
        // see: http://wiki.multimedia.cx/index.php?title=MPEG-4_Audio
        let asc = vec![0x13, 0x14, 0x56, 0xE5, 0x98]; // extracted from dablin at runtime

        let decoder = Decoder::new(&asc).unwrap();

        let (stream, handle) = OutputStream::try_default().expect("Error creating output stream");
        let sink = Sink::try_new(&handle).expect("Error creating sink");

        Self {
            decoder,
            _stream: stream, // NOTE:L we need to keep the stream alive
            sink,
        }
    }
    fn feed(&mut self, au_data: &[u8]) {
        match self.decoder.decode(&au_data) {
            Ok(r) => {
                self.sink.append(SamplesBuffer::new(
                    r.channels as u16,
                    r.sample_rate as u32,
                    r.samples,
                ));
            }
            Err(e) => {
                log::error!("DEC: {}", e);
                return;
            }
        }
    }
}

unsafe impl Send for AudioDecoder {}

struct EDIHandler {
    receiver: UnboundedReceiver<EDIEvent>,
    audio_decoder: AudioDecoder,
    scid: Option<u8>,
}

impl EDIHandler {

    pub fn new(
        scid: Option<u8>,
        receiver: UnboundedReceiver<EDIEvent>,
    ) -> Self {
        let audio_decoder = AudioDecoder::new();
        Self {
            scid,
            receiver,
            audio_decoder,
        }
    }

    pub async fn run(mut self) {
        while let Some(event) = self.receiver.recv().await {
            match event {
                EDIEvent::EnsembleUpdated(ensemble) => {
                    // log::debug!("Ensemble updated: {:?}", ensemble);
                    log::debug!("Ensemble updated: 0x{:4x}", ensemble.eid.unwrap_or(0));
                },
                EDIEvent::AACPFramesExtracted(r) => {
                    if r.scid == self.scid.unwrap_or(0) {
                        // log::debug!("AACPFramesExtracted: {:?}", r);
                        for frame in r.frames {
                            self.audio_decoder.feed(&frame);
                        }
                        // self.audio_decoder.feed(&frame.data);
                    }
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

    // log setup
    std::env::set_var("RUST_LOG", "debug");
    // colog::init();
    env_logger::init();

    let args = Args::parse();
    log::debug!("{:#?}", args);

    // cli args
    let endpoint = args.addr;

    log::debug!("endpoint: {}", endpoint);

    let stream = TcpStream::connect(endpoint).await?;
    let mut buffer = vec![0; 4096];

    let mut filled = 0;
    let mut sync_skipped = 0;

    let mut extractor = EDIFrameExtractor::new();

    let (event_tx, mut event_rx) = unbounded_channel::<EDIEvent>();

    let audio_decoder = Arc::new(Mutex::new(AudioDecoder::new()));

    let aac_callback: Box<dyn FnMut(&AACPFrame) + Send> = Box::new({
        let audio_decoder = Arc::clone(&audio_decoder); // Clone Arc for closure capture
        move |frame: &AACPFrame| {
            if let Ok(mut decoder) = audio_decoder.lock() {
                // decoder.feed(&frame.data);
                // if frame.scid == 6 {
                //     decoder.feed(&frame.data);
                // }
            }
        }
    });

    let mut source = EDISource::new(event_tx, Some(aac_callback));

    // let mut source = EDISource::new(event_tx, Some(callback));

    let event_handler = EDIHandler::new(args.scid, event_rx);

    tokio::spawn(async move {
        event_handler.run().await;
    });

    // extractor.frame.data.resize(0, 0);

    loop {
        let ready = stream.ready(Interest::READABLE).await?;
        if ready.is_readable() {
            // match stream.try_read(&mut buffer) {
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
                            log::debug!("offset: {}", offset);
                            extractor.frame.data.copy_within(offset.., 0);
                            filled -= offset;
                            sync_skipped += offset;

                            continue;
                        } else {
                            sync_skipped = 0;
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
