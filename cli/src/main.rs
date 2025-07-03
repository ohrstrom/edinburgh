mod edi_frame_extractor;

use shared::utils;

use colog;
use log;
use std::io;
use std::io::Write;
use std::sync::{Arc, Mutex};

use clap::Parser;
use derivative::Derivative;
use edi_frame_extractor::EDIFrameExtractor;
use faad2::{version, Decoder};
use futures::channel::mpsc::unbounded;
use rodio::{buffer::SamplesBuffer, OutputStream, Sink};
use tokio::io::Interest;
use tokio::net::TcpStream;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

use shared::edi::bus::{init_event_bus, EDIEvent};
use shared::edi::{AACPFrame, EDISource, Ensemble};

pub fn pp_ensemble(ensemble: &Ensemble) {
    // Top-level ensemble line
    println!(
        "0x{:04x}: \"{}\" | \"{}\"",
        ensemble.eid.unwrap_or(0),
        ensemble.label.clone().unwrap_or_default(),
        ensemble.short_label.clone().unwrap_or_default()
    );

    for service in &ensemble.services {
        println!(
            " - 0x{:04x}: \"{}\" | \"{}\"",
            service.sid,
            service.label.clone().unwrap_or_default(),
            service.short_label.clone().unwrap_or_default()
        );

        for component in &service.components {
            let language = match &component.language {
                Some(lang) => format!("{:?}", lang),
                None => "NONE".to_string(),
            };

            let apps = if component.user_apps.is_empty() {
                "NONE".to_string()
            } else {
                component
                    .user_apps
                    .iter()
                    .map(|ua| format!("{:?}", ua))
                    .collect::<Vec<_>>()
                    .join(", ")
            };

            println!("   - {:03}: {} - {}", component.scid, language, apps);
        }
    }
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct AudioDecoder {
    asc: Vec<u8>,
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
        // let asc = vec![0x13, 0x14, 0x56, 0xE5, 0x98]; // HE-AAC - extracted from dablin at runtime

        println!("FAAD2 version: {:?}", version());

        // example program HE-AAC-v2
        // cargo run -- --addr edi-uk.digris.net:8851 --scid 13
        // 13 0C 56 E5 9D 48 80 // HE-AAC-v2 - extracted from dablin at runtime
        let asc = vec![0x13, 0x0C, 0x56, 0xE5, 0x9D, 0x48, 0x80]; // HE-AAC v2 - 24kHz

        //               14    0C    56    E5    AD    48    80
        // let asc = vec![0x14, 0x0C, 0x56, 0xE5, 0xAD, 0x48, 0x80]; // HE-AAC v2 - 16kHz

        // let asc = vec![0x14, 0x0C, 0x56, 0xE5, 0x9D, 0x48, 0x80]; // HE-AAC v2 - 16kHz
        // let asc = vec![0x12, 0x0C, 0x56, 0xE5, 0x9D, 0x48, 0x80]; // HE-AAC v2 - 32kHz

        let decoder = Decoder::new(&asc).unwrap();

        let (stream, handle) = OutputStream::try_default().expect("Error creating output stream");
        let sink = Sink::try_new(&handle).expect("Error creating sink");

        Self {
            asc,
            decoder,
            _stream: stream, // NOTE: we need to keep the stream alive
            sink,
        }
    }
    fn feed(&mut self, au_data: &[u8]) {
        // log::debug!("AU: {} bytes - {:?} ...", au_data.len(), &au_data[0..8]);

        match self.decoder.decode(&au_data) {
            Ok(r) => {
                self.sink.append(SamplesBuffer::new(
                    r.channels as u16,
                    r.sample_rate as u32,
                    r.samples,
                ));
            }
            // Err(e) => {
            //     log::error!("DEC: {}", e);
            //     return;
            // }
            Err(e) => {
                log::error!("DEC: {} — resetting decoder", e);

                match Decoder::new(&self.asc) {
                    Ok(new_decoder) => {
                        self.decoder = new_decoder;
                        log::warn!("Decoder reset done — will try next AU");
                    }
                    Err(_) => {
                        log::error!("Decoder reset failed — stuck until next good AU!");
                    }
                }

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
    pub fn new(scid: Option<u8>, receiver: UnboundedReceiver<EDIEvent>) -> Self {
        let audio_decoder = AudioDecoder::new();
        Self {
            scid,
            receiver,
            audio_decoder,
        }
    }

    pub async fn run(mut self) {
        let mut ensemble_complete = false;
        while let Some(event) = self.receiver.recv().await {
            match event {
                EDIEvent::EnsembleUpdated(ensemble) => {
                    log::debug!(
                        "Ensemble updated: 0x{:4x} - {}",
                        ensemble.eid.unwrap_or(0),
                        ensemble.complete
                    );
                    if ensemble.complete {
                        println!("{:?}", ensemble);

                        pp_ensemble(&ensemble);
                    }
                }
                EDIEvent::AACPFramesExtracted(r) => {
                    if r.scid == self.scid.unwrap_or(0) {
                        for frame in r.frames {
                            self.audio_decoder.feed(&frame);
                        }
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
    log::debug!("{:#?}", args);

    // cli args
    let endpoint = args.addr;

    log::debug!("endpoint: {}", endpoint);

    let stream = TcpStream::connect(endpoint).await?;
    let mut buffer = vec![0; 4096];

    let mut filled = 0;
    let mut sync_skipped = 0;

    let mut extractor = EDIFrameExtractor::new();

    let event_rx = init_event_bus();

    /*
    let audio_decoder = Arc::new(Mutex::new(AudioDecoder::new()));

    let aac_callback: Box<dyn FnMut(&AACPFrame) + Send> = Box::new({
        let audio_decoder = Arc::clone(&audio_decoder);
        move |frame: &AACPFrame| {
            if let Ok(mut decoder) = audio_decoder.lock() {
                if frame.scid == args.scid.unwrap_or(0) {
                    decoder.feed(&frame.data);
                }
            }
        }
    });

    let mut source = EDISource::new(event_tx, Some(aac_callback));
    */

    let mut source = EDISource::new(args.scid, None);

    let event_handler = EDIHandler::new(args.scid, event_rx);

    tokio::spawn(async move {
        event_handler.run().await;
    });

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
