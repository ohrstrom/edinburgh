use std::sync::Arc;
mod edi_frame_extractor;
mod services;

use axum::{extract::State, routing::get, Json, Router};
use clap::Parser;
use tower_http::cors::{Any, CorsLayer};
use tracing as log;
use tracing_subscriber;

use services::{DirectoryService, ScanTarget};

/// Ensemble directory service
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Server listening address
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    /// Server listening port
    #[arg(long, default_value = "9001")]
    port: Option<u16>,

    /// Scan pattern
    /// format: host:port-port or host:port  
    /// repeat for multiple targets
    #[arg(long = "scan")]
    scan_targets: Vec<ScanTarget>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .with_target(false)
        .without_time()
        .init();

    let args = Args::parse();
    let addr = format!("{}:{}", args.host, args.port.unwrap());

    log::info!("Starting service on http://{}/", addr);

    let svc = services::DirectoryService::new(args.scan_targets);

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app =
        Router::new()
            .route(
                "/",
                get(|State(service): State<Arc<DirectoryService>>| async move {
                    Json(service.get_root())
                }),
            )
            .route(
                "/ensembles",
                get(|State(service): State<Arc<DirectoryService>>| async move {
                    Json(service.get_ensembles().await)
                }),
            )
            .with_state(svc)
            .layer(cors);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
