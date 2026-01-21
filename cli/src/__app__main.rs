mod audio;
mod tui;

use std::io;
use std::sync::{Arc, Once};

use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

use tokio::io::Interest;
use tokio::net::TcpStream;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::sync::{RwLock, watch};

use clap::Parser;
use clap_num::maybe_hex;

use shared::dab::bus::{init_event_bus, DabEvent};
use shared::dab::{DabSource, Ensemble};
use shared::edi_frame_extractor::EdiFrameExtractor;

use audio::{AudioDecoder, AudioEvent};
use tui::{TuiCommand, TuiEvent};

/// EDInburgh
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// EDI host:port to connect to
    #[arg(long, short)]
    addr: String,

    /// Subchannel ID to select [optional]
    #[arg(long, short, conflicts_with = "sid")]
    scid: Option<u8>,

    /// Service ID to select, decimal or HEX [optional]
    #[arg(long, short = 'S', value_parser = maybe_hex::<u16>, conflicts_with = "scid")]
    sid: Option<u16>,

    /// Use Jack output. Device name is: cpal_client_out
    #[cfg(all(feature = "jack", target_os = "linux"))]
    #[arg(long, short, default_value_t = false)]
    jack: bool,

    /// Enable TUI
    #[arg(long, short, default_value_t = false)]
    tui: bool,

    /// Verbose logging
    #[arg(long = "verbose", short = 'v')]
    verbose: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_panic_hook();
    let args = Args::parse();
    App::new(args).await?.run().await
}

fn init_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let _ = ratatui::crossterm::terminal::disable_raw_mode();
        let mut stdout = std::io::stdout();
        let _ = ratatui::crossterm::execute!(
            stdout,
            ratatui::crossterm::terminal::LeaveAlternateScreen,
            ratatui::crossterm::event::DisableMouseCapture,
            ratatui::crossterm::cursor::Show
        );
        eprintln!("\n\n=== PANIC ===\n{info}");
    }));
}

fn init_tracing(args: &Args) {
    let base = if args.tui { "error" } else if args.verbose { "info,edinburgh=debug" } else { "info" };
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(base));

    let show_level = filter.max_level_hint().map(|lvl| lvl >= LevelFilter::DEBUG).unwrap_or(false);
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_level(show_level)
        .with_target(show_level && !args.verbose)
        .without_time()
        .init();
}

fn use_jack(args: &Args) -> bool {
    #[cfg(all(feature = "jack", target_os = "linux"))]
    { args.jack }
    #[cfg(not(all(feature = "jack", target_os = "linux")))]
    { false }
}

struct App {
    args: Args,
    scid_tx: watch::Sender<Option<u8>>,
    scid_rx: watch::Receiver<Option<u8>>,
    tui_tx: UnboundedSender<TuiEvent>,
    tui_cmd_rx: UnboundedReceiver<TuiCommand>,
    audio_tx: UnboundedSender<AudioEvent>,
}

impl App {
    async fn new(args: Args) -> anyhow::Result<Self> {
        let (tui_tx, tui_rx) = unbounded_channel();
        let (tui_cmd_tx, tui_cmd_rx) = unbounded_channel();
        let (audio_tx, audio_rx) = unbounded_channel();
        let (scid_tx, scid_rx) = watch::channel(args.scid);

        if args.tui {
            let addr = args.addr.clone();
            tokio::spawn(tui::run_tui(addr, *scid_rx.borrow(), tui_tx.clone(), tui_rx, tui_cmd_tx, audio_rx));
        }

        Ok(Self { args, scid_tx, scid_rx, tui_tx, tui_cmd_rx, audio_tx })
    }

    async fn run(mut self) -> anyhow::Result<()> {
        init_tracing(&self.args);
        let edi_rx = init_event_bus();

        let mut stream = TcpStream::connect(&self.args.addr).await?;
        let mut source = DabSource::new(self.args.scid, None, None);
        let mut extractor = EdiFrameExtractor::new();

        let mut handler = DabEventHandler::new(
            self.scid_rx.clone(),
            use_jack(&self.args),
            edi_rx,
            self.tui_tx.clone(),
            self.audio_tx.clone(),
            self.args.sid,
        );

        tokio::spawn(async move { handler.run().await });

        self.event_loop(stream, source, extractor).await
    }

    async fn event_loop(&mut self, mut stream: TcpStream, mut source: DabSource, mut extractor: EdiFrameExtractor) -> anyhow::Result<()> {

        use tokio::io::AsyncReadExt;
        let mut filled = 0;

        loop {
            tokio::select! {

                // EDI TCP stream
                ready = stream.ready(Interest::READABLE) => {
                    let ready = ready?;
                    if ready.is_readable() {
                        match stream.try_read(&mut extractor.frame.data[filled..]) {
                            Ok(0) => {
                                tracing::info!("Connection closed by peer");
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
                                        tracing::trace!("frame completed: {}", extractor.frame);
                                        extractor.frame.reset();
                                        filled = 0;
                                    }
                                }
                            }
                            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => continue,
                            Err(e) => {
                                return Err(e.into());
                            },
                        }
                    }
                }

                // TUI command handler
                Some(cmd) = self.tui_cmd_rx.recv() => match cmd {
                    TuiCommand::ScIDSelected(v) => { let _ = self.scid_tx.send(Some(v)); }
                    TuiCommand::Shutdown => break,
                }
            }
        }

        Ok(())
    }
}

pub struct DabEventHandler {
    scid_rx: watch::Receiver<Option<u8>>,
    scid_tx: Option<watch::Sender<Option<u8>>>,
    use_jack: bool,
    audio_decoder: Option<AudioDecoder>,
    edi_rx: UnboundedReceiver<DabEvent>,
    tui_tx: UnboundedSender<TuiEvent>,
    audio_tx: UnboundedSender<AudioEvent>,
    printed_ensemble: bool,
    target_sid: Option<u16>,
}

impl DabEventHandler {
    pub fn new(
        scid_rx: watch::Receiver<Option<u8>>,
        use_jack: bool,
        edi_rx: UnboundedReceiver<DabEvent>,
        tui_tx: UnboundedSender<TuiEvent>,
        audio_tx: UnboundedSender<AudioEvent>,
        target_sid: Option<u16>,
    ) -> Self {
        Self {
            scid_rx,
            scid_tx: None,
            use_jack,
            audio_decoder: None,
            edi_rx,
            tui_tx,
            audio_tx,
            printed_ensemble: false,
            target_sid,
        }
    }

    pub async fn run(mut self) {
        while let Some(event) = self.edi_rx.recv().await {
            match event {
                DabEvent::EnsembleUpdated(ensemble) => {
                    if ensemble.complete {
                        tracing::debug!("[0x{:4X}] Ensemble updated", ensemble.eid.unwrap_or(0));
                        if let Err(e) = self.tui_tx.send(TuiEvent::EnsembleUpdated(ensemble)) {
                            tracing::warn!("Could not send TUI update: {:?}", e);
                        }
                    }
                }
                DabEvent::AacpFramesExtracted(r) => {
                    let scid = *self.scid_rx.borrow();
                    if r.scid == scid.unwrap_or(0) {
                        if r.audio_format.is_none() {
                            tracing::warn!("No audio format for SCID: {}", r.scid);
                            continue;
                        }

                        let audio_format = r.audio_format.as_ref().unwrap();

                        // create aduio decoder if needed
                        if self.audio_decoder.is_none() {
                            let audio_decoder = AudioDecoder::new(
                                r.scid,
                                self.use_jack,
                                audio_format.clone(),
                                self.audio_tx.clone(),
                            );
                            self.audio_decoder = Some(audio_decoder);
                        }

                        // feed audio decoder
                        if let Some(ref mut audio_decoder) = self.audio_decoder {
                            audio_decoder.feed(&r);
                        }
                    }
                }
                DabEvent::MotImageReceived(m) => {
                    tracing::debug!(
                        "[{:2}] MOT {:9} - {} bytes",
                        m.scid,
                        m.mimetype.to_uppercase(),
                        m.data.len(),
                    );
                    if let Err(e) = self.tui_tx.send(TuiEvent::MotImageReceived(m)) {
                        tracing::warn!("MOT: could not send TUI update: {:?}", e);
                    }
                }
                DabEvent::DlObjectReceived(d) => {
                    tracing::debug!(
                        "[{:2}] DL{} {}",
                        d.scid,
                        if d.is_dl_plus() { "+" } else { " " },
                        d.decode_label()
                    );
                    if let Err(e) = self.tui_tx.send(TuiEvent::DlObjectReceived(d)) {
                        tracing::warn!("DL: could not send TUI update: {:?}", e);
                    }
                }
                DabEvent::DabStatsUpdated(s) => {
                    if let Err(e) = self.tui_tx.send(TuiEvent::DabStatsUpdated(s)) {
                        tracing::warn!("DAB: could not send TUI update: {:?}", e);
                    }
                }
            }
        }
    }
}