mod edi;

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

fn start_ws_server(frame_rx: Receiver<Vec<u8>>) {
    let server = TcpListener::bind("127.0.0.1:9000").expect("Failed to bind WebSocket server");

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

    while let Ok(frame_data) = frame_rx.recv() {
        let mut clients_lock = clients.lock().unwrap();
        let frame_bytes = Bytes::from(frame_data.to_vec());

        // debug!("FRAME: {}", frame_data.len());

        clients_lock.retain_mut(
            |client| match client.send(Message::Binary(frame_bytes.clone())) {
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

    let (frame_tx, frame_rx): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = mpsc::channel();

    thread::spawn(move || {
        start_ws_server(frame_rx);
    });

    // EDI frame
    let mut filled = 0;
    let mut sync_skipped = 0;
    let mut edi_source = EDISource::new();

    loop {
        match stream.read(&mut edi_source.frame.data[filled..]) {
            Ok(0) => {
                info!("Connection closed by peer");
                break;
            }
            Ok(n) => {
                filled += n;

                if filled < edi_source.frame.data.len() {
                    continue;
                }

                if let Some(offset) = edi_source.frame.find_sync_magic() {
                    if offset > 0 {
                        edi_source.frame.data.copy_within(offset.., 0);
                        filled -= offset;
                        sync_skipped += offset;

                        continue;
                    } else {
                        sync_skipped = 0;
                    }

                    if edi_source.frame.check_completed() {
                        // debug!("EDI frame complete: {} bytes", edi_source.frame.data.len());

                        frame_tx.send(edi_source.frame.data.clone()).unwrap();

                        edi_source.frame.reset();
                        filled = 0;
                    }
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(std::time::Duration::from_millis(100));
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
