mod audio;
mod tui;

use std::io;
use std::sync::{Arc, Once};

use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

use tokio::io::Interest;
use tokio::net::TcpStream;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::sync::RwLock;

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

fn install_panic_hook() {
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    install_panic_hook();
    let args = Args::parse();

    let filter = if args.tui {
        EnvFilter::new("error")
    } else {
        EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            EnvFilter::new({
                if args.verbose {
                    "info,edinburgh=debug"
                } else {
                    "info"
                }
            })
        })
    };

    let show_level = filter
        .max_level_hint()
        .map(|lvl| lvl >= LevelFilter::DEBUG)
        .unwrap_or(false);

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_level(show_level)
        .with_target(show_level && !args.verbose)
        .without_time()
        .init();

    tracing::debug!("{:?}", args);

    let scid = Arc::new(RwLock::new(args.scid));
    let sid = args.sid;

    let use_jack: bool = {
        #[cfg(all(feature = "jack", target_os = "linux"))]
        {
            args.jack
        }
        #[cfg(not(all(feature = "jack", target_os = "linux")))]
        {
            false
        }
    };

    // TUI
    // TUI main -> TUI
    let (tui_tx, tui_rx) = unbounded_channel::<TuiEvent>();

    // TUI -> main
    let (tui_cmd_tx, mut tui_cmd_rx) = unbounded_channel::<TuiCommand>();

    // TUI audio -> TUI
    let (audio_tx, audio_rx) = unbounded_channel::<AudioEvent>();

    let tui_enabled = args.tui;

    // check if this is a good idea?
    if tui_enabled {
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

    #[allow(clippy::type_complexity)]
    let on_ensemble_updated_callback: Option<Box<dyn FnMut(&Ensemble) + Send>> = Some(Box::new({
        let scid = Arc::clone(&scid);
        move |e: &Ensemble| {
            if !e.complete {
                return;
            }

            if !tui_enabled {
                print_ensemble(e);
            }

            // how ugly can it get ;)
            if let Some(sid) = sid {
                let scid_selected = !scid.try_read().map(|g| g.is_none()).unwrap_or(false);

                if !scid_selected {
                    let svc = e.services.iter().find(|s| s.sid == sid);
                    let component = svc.and_then(|s| s.components.first());

                    if let Some(c) = component {
                        let scid = Arc::clone(&scid);
                        let selected_scid = c.subchannel_id;

                        tokio::spawn(async move {
                            *scid.write().await = selected_scid;
                        });

                        tracing::info!("Select SubCh {} for SID 0x{:4X}", c.scid, sid);
                    }
                }
            }
        }
    }));

    let mut source = DabSource::new(args.scid, on_ensemble_updated_callback, None);

    let edi_rx = init_event_bus();

    // let stream = TcpStream::connect(args.addr).await?;

    let stream = match TcpStream::connect(args.addr.clone()).await {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("Unable to connect to {}: {}", args.addr, e);
            return Err(e.into());
        }
    };

    let mut filled = 0;

    let mut extractor = EdiFrameExtractor::new();

    let event_handler = DabEventHandler::new(
        Arc::clone(&scid),
        use_jack,
        edi_rx,
        tui_tx.clone(),
        audio_tx.clone(),
    );

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
                                    // println!("frame completed: {}", extractor.frame);
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
            Some(cmd) = tui_cmd_rx.recv() => {
                match cmd {
                    TuiCommand::ScIDSelected(scid_val) => {
                        let mut scid = scid.write().await;
                        *scid = Some(scid_val);
                    }
                    TuiCommand::Shutdown => {
                        break;
                    }
                }
            }
        }
    }

    Ok(())
}

struct DabEventHandler {
    edi_rx: UnboundedReceiver<DabEvent>,
    scid: Arc<RwLock<Option<u8>>>,
    use_jack: bool,
    audio_decoder: Option<AudioDecoder>,
    // tui
    tui_tx: UnboundedSender<TuiEvent>,
    audio_tx: UnboundedSender<AudioEvent>,
}

// hm - this is kind of verbose. theoretically DabEvents could be consumed directly in TUI
// but this does not work with the current edi_rx implementation
impl DabEventHandler {
    pub fn new(
        scid: Arc<RwLock<Option<u8>>>,
        use_jack: bool,
        edi_rx: UnboundedReceiver<DabEvent>,
        tui_tx: UnboundedSender<TuiEvent>,
        audio_tx: UnboundedSender<AudioEvent>,
    ) -> Self {
        Self {
            edi_rx,
            scid,
            use_jack,
            audio_decoder: None,
            tui_tx,
            audio_tx,
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
                    let scid = *self.scid.read().await;
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
                        tracing::warn!("Could not send TUI update: {:?}", e);
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
                        tracing::warn!("Could not send TUI update: {:?}", e);
                    }
                }
                DabEvent::DabStatsUpdated(s) => {
                    if let Err(e) = self.tui_tx.send(TuiEvent::DabStatsUpdated(s)) {
                        tracing::warn!("Could not send TUI update: {:?}", e);
                    }
                }
            }
        }
    }
}

// the print once logic here seems to be very ugly. think about a better way...
static PRINT_ENSEMBLE_ONCE: Once = Once::new();

fn print_ensemble(ensemble: &Ensemble) {
    if !ensemble.complete {
        return;
    }

    PRINT_ENSEMBLE_ONCE.call_once(|| {
        tracing::info!(
            "Ensemble: {} - EID 0x{:04x}",
            ensemble.label.as_deref().unwrap_or("<no label>"),
            ensemble.eid.unwrap_or(0)
        );

        let mut sorted_subchannels = ensemble.subchannels.iter().collect::<Vec<_>>();
        sorted_subchannels.sort_by_key(|svc| svc.id);

        for sc in sorted_subchannels {
            tracing::info!(
                "SubCh {:4}   start {:4}   CUs {:3}   {}   {:3} kbps ",
                sc.id,
                sc.start.unwrap_or(0),
                sc.size.unwrap_or(0),
                sc.pl.as_deref().unwrap_or(""),
                sc.bitrate.unwrap_or(0),
            );
        }

        let mut sorted_services = ensemble.services.iter().collect::<Vec<_>>();
        sorted_services.sort_by_key(|svc| svc.label.as_deref().unwrap_or("").to_lowercase());

        for service in sorted_services {
            let comp = service.components.first();

            let (codec, bitrate, scid) = if let Some(c) = comp {
                let af = c.audio_format.as_ref();
                (
                    af.map(|a| a.codec.as_str()).unwrap_or("-"),
                    af.map(|a| a.bitrate).unwrap_or(0),
                    c.scid,
                )
            } else {
                ("-", 0, 0)
            };

            tracing::info!(
                "SubCh {:4}   0x{:4X}   {:<16} ({})\t   {:<10}   {:3} kbps",
                scid,
                service.sid,
                service.label.as_deref().unwrap_or("<no label>"),
                service.short_label.as_deref().unwrap_or(""),
                codec,
                bitrate
            );
        }
    });
}
