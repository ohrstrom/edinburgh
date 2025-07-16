mod audio;
mod edi_frame_extractor;
mod tui;

use log;
use std::io;
use std::sync::Arc;

use clap::Parser;
use edi_frame_extractor::EDIFrameExtractor;
use tokio::io::Interest;
use tokio::net::TcpStream;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::sync::RwLock;

use shared::edi::bus::{init_event_bus, EDIEvent};
use shared::edi::EDISource;

use audio::{AudioDecoder, AudioEvent};
use tui::{TUICommand, TUIEvent};

// use tui::sls;

/// EDInburgh
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// EDI host:port to connect to
    #[arg(short, long)]
    addr: String,

    /// Subchannel ID to extract audio from [optional]
    #[arg(short, long)]
    scid: Option<u8>,

    /// Enable TUI
    #[arg(long, default_value_t = false)]
    tui: bool,

    /// Log level (ignored in TUI mode)
    #[arg(long, default_value = "info", value_parser = ["debug", "info", "warn", "error"])]
    log_level: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // set log level to error in TUI mode, else use args
    if args.tui {
        std::env::set_var("RUST_LOG", "error");
    } else {
        std::env::set_var("RUST_LOG", args.log_level.clone());
    }

    env_logger::builder().format_timestamp(None).init();

    log::debug!("{:?}", args);

    let scid = Arc::new(RwLock::new(args.scid));

    // TUI
    // TUI main -> TUI
    let (tui_tx, tui_rx) = unbounded_channel::<TUIEvent>();

    // TUI -> main
    let (tui_cmd_tx, mut tui_cmd_rx) = unbounded_channel::<TUICommand>();

    // TUI audio -> TUI
    let (audio_tx, audio_rx) = unbounded_channel::<AudioEvent>();

    // NOTE: check if this is a good idea?
    if args.tui {
        tokio::spawn({
            let addr = args.addr.clone();
            let tui_tx = tui_tx.clone();
            let scid = *scid.read().await;
            async move {
                if let Err(e) = tui::run_tui(addr, scid, tui_tx, tui_rx, tui_cmd_tx, audio_rx).await
                {
                    eprintln!("TUI error: {:?}", e);
                }
            }
        });
    }

    let edi_rx = init_event_bus();

    let stream = TcpStream::connect(args.addr).await?;

    let mut filled = 0;

    let mut extractor = EDIFrameExtractor::new();

    let mut source = EDISource::new(args.scid, None, None);

    let event_handler =
        EDIHandler::new(Arc::clone(&scid), edi_rx, tui_tx.clone(), audio_tx.clone());

    tokio::spawn(async move {
        event_handler.run().await;
    });

    loop {
        tokio::select! {

            // EDI TCP stream
            ready = stream.ready(Interest::READABLE) => {
                let ready = ready?;
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
                            if let Some(offset) = extractor.frame.find_sync_magic() {
                                if offset > 0 {
                                    extractor.frame.data.copy_within(offset.., 0);
                                    filled -= offset;
                                    continue;
                                }

                                if extractor.frame.check_completed() {
                                    source.feed(&extractor.frame.data).await;
                                    // println!("frame completed: {}", extractor.frame);
                                    extractor.frame.reset();
                                    filled = 0;
                                }
                            }
                        }
                        Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => continue,
                        Err(e) => return Err(e.into()),
                    }
                }
            }

            // TUI command handler
            Some(cmd) = tui_cmd_rx.recv() => {
                match cmd {
                    TUICommand::ScIDSelected(scid_val) => {
                        let mut scid = scid.write().await;
                        *scid = Some(scid_val);
                    }
                    TUICommand::Shutdown => {
                        break;
                    }
                }
            }
        }
    }

    Ok(())
}

struct EDIHandler {
    edi_rx: UnboundedReceiver<EDIEvent>,
    scid: Arc<RwLock<Option<u8>>>,
    audio_decoder: Option<AudioDecoder>,
    // tui
    tui_tx: UnboundedSender<TUIEvent>,
    audio_tx: UnboundedSender<AudioEvent>,
}

// hm - this is kind of verbose. theoretically EDIEvents could be consumed directly in TUI
// but this does not work with the current edi_rx implementation
impl EDIHandler {
    pub fn new(
        scid: Arc<RwLock<Option<u8>>>,
        edi_rx: UnboundedReceiver<EDIEvent>,
        tui_tx: UnboundedSender<TUIEvent>,
        audio_tx: UnboundedSender<AudioEvent>,
    ) -> Self {
        Self {
            edi_rx,
            scid,
            audio_decoder: None,
            tui_tx,
            audio_tx,
        }
    }

    pub async fn run(mut self) {
        while let Some(event) = self.edi_rx.recv().await {
            match event {
                EDIEvent::EnsembleUpdated(ensemble) => {
                    if ensemble.complete {
                        log::debug!(
                            "Ensemble updated: 0x{:4x} - complete: {}",
                            ensemble.eid.unwrap_or(0),
                            ensemble.complete
                        );
                        if let Err(e) = self.tui_tx.send(TUIEvent::EnsembleUpdated(ensemble)) {
                            log::warn!("Could not send TUI update: {:?}", e);
                        }
                    }
                }
                EDIEvent::AACPFramesExtracted(r) => {
                    let scid = *self.scid.read().await;
                    if r.scid == scid.unwrap_or(0) {
                        if r.audio_format.is_none() {
                            log::warn!("Audio format is None for SCID: {}", r.scid);
                            continue;
                        }

                        let audio_format = r.audio_format.as_ref().unwrap();

                        // create aduio decoder if needed
                        if self.audio_decoder.is_none() {
                            let audio_decoder = AudioDecoder::new(
                                r.scid,
                                audio_format.clone(),
                                self.audio_tx.clone(),
                            );
                            self.audio_decoder = Some(audio_decoder);
                        }

                        // feed audio decoder with frames
                        if let Some(ref mut audio_decoder) = self.audio_decoder {
                            audio_decoder.feed(&r);
                        }
                    }
                }
                EDIEvent::MOTImageReceived(m) => {
                    if let Err(e) = self.tui_tx.send(TUIEvent::MOTImageReceived(m)) {
                        log::warn!("Could not send TUI update: {:?}", e);
                    }
                }
                EDIEvent::DLObjectReceived(d) => {
                    if let Err(e) = self.tui_tx.send(TUIEvent::DLObjectReceived(d)) {
                        log::warn!("Could not send TUI update: {:?}", e);
                    }
                }
                EDIEvent::EDISStatsUpdated(s) => {
                    if let Err(e) = self.tui_tx.send(TUIEvent::EDISStatsUpdated(s)) {
                        log::warn!("Could not send TUI update: {:?}", e);
                    }
                }
            }
        }
    }
}
