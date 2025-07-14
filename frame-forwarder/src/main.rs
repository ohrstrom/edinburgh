mod edi_frame_extractor;

use bytes::Bytes;
use clap::Parser;
use dashmap::DashMap;
use edi_frame_extractor::EDIFrameExtractor;
use futures_util::{SinkExt, StreamExt};
use log;
use std::io;
use std::sync::Arc;
use tokio::io::Interest;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, oneshot, Mutex};
use tokio_tungstenite::accept_hdr_async;
use tokio_tungstenite::tungstenite::handshake::server::{Request, Response};
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;
use tokio_tungstenite::tungstenite::Message as WsMessage;

type SharedReceivers = Arc<
    DashMap<
        String,
        (
            broadcast::Sender<Vec<u8>>,
            tokio::task::JoinHandle<()>,
            Arc<Mutex<Option<oneshot::Receiver<Result<(), String>>>>>,
        ),
    >,
>;

/// EDI Frame Forwarder
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Server listening address
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    /// Server listening port
    #[arg(long, default_value = "9000")]
    port: Option<u16>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    std::env::set_var("RUST_LOG", "debug");
    env_logger::init();

    let args = Args::parse();
    let addr = format!("{}:{}", args.host, args.port.unwrap());

    eprintln!(
        "# Starting server on:\n\
         # ws://{addr}/\n\
         # connect to: ws://{addr}/<edi-host>/<edi-port>",
        addr = addr
    );

    let ws_listener = TcpListener::bind(addr).await?;
    let ws_clients: SharedReceivers = Arc::new(DashMap::new());

    tokio::spawn(edi_extractor_cleanup_task(ws_clients.clone()));

    while let Ok((stream, _)) = ws_listener.accept().await {
        let receivers = ws_clients.clone();
        tokio::spawn(handle_ws_connection(stream, receivers));
    }

    Ok(())
}

async fn handle_ws_connection(stream: TcpStream, ws_clients: SharedReceivers) {
    let mut uri_holder = None;

    let ws_stream = match accept_hdr_async(stream, |req: &Request, resp: Response| {
        uri_holder = Some(req.uri().clone());
        Ok(resp)
    })
    .await
    {
        Ok(ws) => ws,
        Err(e) => {
            log::error!("ws handshake failed: {}", e);
            return;
        }
    };

    let uri = match uri_holder {
        Some(uri) => uri,
        None => {
            log::error!("Failed to extract ws URI.");
            return;
        }
    };

    let parts: Vec<&str> = uri.path().trim_matches('/').split('/').collect();
    if parts.len() != 3 || parts[0] != "ws" {
        log::error!("Invalid ws path: {}", uri);
        return;
    }

    let host = parts[1].to_string();
    let port = parts[2].to_string();
    let key = format!("{}:{}", host, port);

    log::info!("New ws client for: {}", key);

    let (mut ws_stream, mut rx, conn_signal) = {
        let entry = ws_clients.entry(key.clone()).or_insert_with(|| {
            let (tx, _) = broadcast::channel(100);
            let (conn_status_tx, conn_status_rx) = oneshot::channel();

            let task_handle = tokio::spawn(start_edi_extractor(
                host.clone(),
                port.clone(),
                tx.clone(),
                conn_status_tx,
            ));
            (tx, task_handle, Arc::new(Mutex::new(Some(conn_status_rx))))
        });

        (ws_stream, entry.0.subscribe(), entry.2.clone())
    };

    // Check TCP connection status before entering main loop
    if let Some(conn_signal) = conn_signal.lock().await.take() {
        /*
        if let Err(conn_err) = conn_signal.await.unwrap_or_else(|_| Err("Internal channel error".into())) {
            log::error!("TCP connection failed for {}: {}", key, conn_err);
            let close_frame = CloseFrame {
                code: CloseCode::Error,
                reason: conn_err.into(),
            };
            let _ = ws_stream.close(Some(close_frame)).await;
            return;
        }
        */
        if let Err(conn_err) = conn_signal
            .await
            .unwrap_or_else(|_| Err("Internal channel error".into()))
        {
            log::error!("TCP connection failed for {}: {}", key, conn_err);
            let close_frame = CloseFrame {
                code: CloseCode::Error,
                reason: conn_err.into(),
            };
            if let Err(e) = ws_stream.close(Some(close_frame)).await {
                log::warn!("Failed to send WS close frame: {}", e);
                return;
            }

            // Explicitly flush/await the close handshake by reading from ws_stream until it closes.
            while let Some(msg) = ws_stream.next().await {
                match msg {
                    Ok(_) => continue, // ignore any further messages
                    Err(_) => break,   // connection error or closed
                }
            }

            return;
        }
    }

    loop {
        tokio::select! {
            // Handle disconnect or incoming client message
            ws_msg = ws_stream.next() => {
                match ws_msg {
                    Some(Ok(WsMessage::Close(frame))) => {
                        log::info!("Client sent close frame: {:?}", frame);
                        break;
                    }
                    Some(Ok(_)) => {
                        // eventually handle client messages (ping/keepalive) here
                        continue;
                    }
                    Some(Err(e)) => {
                        log::warn!("WebSocket client error: {}", e);
                        break;
                    }
                    None => {
                        log::info!("Client disconnected (stream ended)");
                        break;
                    }
                }
            }

            // broadcast data from the TCP source
            broadcast_msg = rx.recv() => {
                match broadcast_msg {
                    Ok(data) => {
                        if let Err(e) = ws_stream.send(WsMessage::Binary(Bytes::from(data))).await {
                            log::warn!("WebSocket send error: {}", e);
                            break;
                        }
                    }
                    Err(_) => {
                        // Sender dropped or channel closed
                        break;
                    }
                }
            }
        }
    }

    log::info!("Disconnected ws client for: {}", key);
    drop(rx);
}

async fn start_edi_extractor(
    host: String,
    port: String,
    tx: broadcast::Sender<Vec<u8>>,
    conn_status_tx: oneshot::Sender<Result<(), String>>,
) {
    let endpoint = format!("{}:{}", host, port);
    log::info!("Starting TCP receiver for: {}", endpoint);

    match TcpStream::connect(&endpoint).await {
        Ok(stream) => {
            // Notify successful connection
            let _ = conn_status_tx.send(Ok(()));

            let extractor = Arc::new(Mutex::new(EDIFrameExtractor::new()));
            let mut filled = 0;

            loop {
                let ready = match stream.ready(Interest::READABLE).await {
                    Ok(ready) => ready,
                    Err(e) => {
                        log::error!("Error on {}: {}", endpoint, e);
                        break;
                    }
                };

                if ready.is_readable() {
                    let mut extractor = extractor.lock().await;

                    match stream.try_read(&mut extractor.frame.data[filled..]) {
                        Ok(0) => {
                            log::info!("Connection to {} closed by peer", endpoint);
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
                                    let _ = tx.send(extractor.frame.data.clone());
                                    extractor.frame.reset();
                                    filled = 0;
                                }
                            }
                        }
                        Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => continue,
                        Err(e) => {
                            log::error!("Error on {}: {}", endpoint, e);
                            break;
                        }
                    }
                }
            }
        }
        Err(e) => {
            log::error!("Failed to connect (B) to {}: {}", endpoint, e);
            // Notify TCP connection failure explicitly
            let _ = conn_status_tx.send(Err(format!("TCP connection failed: {}", e)));
        }
    }
}

async fn edi_extractor_cleanup_task(ws_clients: SharedReceivers) {
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;

        let keys_to_remove: Vec<String> = ws_clients
            .iter()
            .filter_map(|entry| {
                if entry.value().0.receiver_count() == 0 {
                    Some(entry.key().clone())
                } else {
                    None
                }
            })
            .collect();

        for key in keys_to_remove {
            if let Some((_, (_sender, handle, _err_handle))) = ws_clients.remove(&key) {
                log::info!("Stopping unused TCP receiver for: {}", key);
                handle.abort();
            }
        }
    }
}
