use tokio::net::TcpStream;
use tokio_tungstenite::accept_async;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use warp::Filter;
use futures::StreamExt;
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    let ws_route = warp::path("ws").and(warp::ws()).map(|ws: warp::ws::Ws| {
        ws.on_upgrade(handle_connection)
    });

    warp::serve(ws_route).run(([0, 0, 0, 0], 8765)).await;
}

async fn handle_connection(ws_stream: warp::ws::WebSocket) {
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    let mut tcp_stream = TcpStream::connect("your-tcp-server.com:1234").await.unwrap();

    let ws_to_tcp = async {
        while let Some(Ok(msg)) = ws_receiver.next().await {
            if let Ok(text) = msg.to_str() {
                tcp_stream.write_all(text.as_bytes()).await.unwrap();
            }
        }
    };

    let tcp_to_ws = async {
        let mut buf = vec![0; 1024];
        while let Ok(n) = tcp_stream.read(&mut buf).await {
            if n == 0 { break; }
            ws_sender.send(warp::ws::Message::text(String::from_utf8_lossy(&buf[..n]))).await.unwrap();
        }
    };

    tokio::select! {
        _ = ws_to_tcp => {}
        _ = tcp_to_ws => {}
    }
}
