mod audio;
mod dec;
mod edi;
mod fic;
mod utils;

use bytemuck::cast_slice;
use colog;
use log::{debug, error, info};
use nix::fcntl::{fcntl, FcntlArg, OFlag};
use std::env;
use std::io::Read;
use std::os::unix::io::AsRawFd;

use bytes::Bytes;
use std::net::{TcpListener, TcpStream};
use std::sync::{
    mpsc::{self, Receiver, Sender},
    Arc, Mutex,
};
use std::thread;
use tungstenite::accept;
use tungstenite::protocol::Message;

use edi::EDISource;

fn start_websocket_server(au_rx: Receiver<Vec<u8>>, pcm_rx: Receiver<Vec<f32>>) {
    let server = TcpListener::bind("127.0.0.1:9001").expect("Failed to bind WebSocket server");

    let clients = Arc::new(Mutex::new(Vec::new()));
    let clients_accept = Arc::clone(&clients);

    thread::spawn(move || {
        for stream in server.incoming() {
            match stream {
                Ok(stream) => match accept(stream) {
                    Ok(ws_stream) => {
                        info!("New WebSocket client connected");
                        clients_accept.lock().unwrap().push(ws_stream);
                    }
                    Err(e) => {
                        error!("Error during WebSocket handshake: {}", e);
                    }
                },
                Err(e) => {
                    error!("Error accepting connection: {}", e);
                }
            }
        }
    });

    // NOTE: at the moment only one RECV works at a time

    while let Ok(au_data) = au_rx.recv() {

        let mut clients_lock = clients.lock().unwrap();
        let au_bytes = Bytes::from(au_data.to_vec());

        // debug!("AU: {}", au_data.len());

        clients_lock.retain_mut(
            |client| match client.send(Message::Binary(au_bytes.clone())) {
                Ok(_) => true,
                Err(e) => {
                    error!("Error sending message to client: {}", e);
                    false
                }
            },
        );
    }

    while let Ok(pcm_data) = pcm_rx.recv() {
        let pcm_bytes: &[u8] = cast_slice(&pcm_data);
        let pcm_bytes = Bytes::from(pcm_bytes.to_vec());

        let mut clients_lock = clients.lock().unwrap();

        // debug!("PCM: {}", pcm_data.len());

        clients_lock.retain_mut(
            |client| match client.send(Message::Binary(pcm_bytes.clone())) {
                Ok(_) => true,
                Err(e) => {
                    error!("Error sending message to client: {}", e);
                    false
                }
            },
        );
    }
}

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

    // websocket
    let (au_tx, au_rx): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = mpsc::channel();
    let (pcm_tx, pcm_rx): (Sender<Vec<f32>>, Receiver<Vec<f32>>) = mpsc::channel();

    thread::spawn(move || {
        start_websocket_server(au_rx, pcm_rx);
    });

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
                        // edi_source.process_frame();

                        match edi_source.process_frame() {
                            Ok(r) => {
                                // debug!("Frame completed: tags: {} - pcm: {}", r.tags.len(), r.pcm_data.len());

                                if !r.au_frames.is_empty() {
                                    // debug!("au frames:  {}", r.au_frames.len());
                                    for au_frame in r.au_frames {
                                        if let Err(e) = au_tx.send(au_frame) {
                                            error!("Failed to send AU frame over channel: {}", e);
                                        }
                                    }
                                }

                                if !r.pcm.is_empty() {
                                    // debug!("pcm frames: {}", r.pcm.len());

                                    // TODO: send pcm data via websocket
                                    if let Err(e) = pcm_tx.send(r.pcm) {
                                        error!("Failed to send PCM data over channel: {}", e);
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Error processing frame: {}", e);
                            }
                        }

                        // debug!("Frame completed: {}", edi_source.frame.data.len());

                        let leftover = filled.saturating_sub(edi_source.frame.data.len());

                        if leftover > 0 {
                            debug!("preserving {} bytes leftover", leftover);
                            // TODO: i guess this is not correct ;) - do we even need it?
                            let framne_start = edi_source.frame.data.len();
                            edi_source.frame.data.copy_within(framne_start..filled, 0);
                            filled = leftover;
                        } else {
                            edi_source.frame.reset();
                            filled = 0;
                        }
                    }
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
