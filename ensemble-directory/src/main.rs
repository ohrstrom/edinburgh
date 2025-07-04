use std::sync::Arc;
mod edi_frame_extractor;
mod services;

use shared::utils;

use anyhow;
use axum::{extract::State, routing::get, serve, Json, Router};
use clap::Parser;
use serde_json::{json, Value};
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

    /// Scan targets
    /// multiple targets can be specified
    /// format: <host>:<start_port>-<end_port>
    #[arg(long, value_delimiter = ',')]
    scan_targets: Vec<ScanTarget>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().with_env_filter("info").init();

    let args = Args::parse();
    let addr = format!("{}:{}", args.host, args.port.unwrap());

    println!("ARGS: {:?}", args);

    log::info!("Starting service on http://{}/", addr);

    let svc = services::DirectoryService::new(args.scan_targets);

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
            .route(
                "/ctr",
                get(|State(service): State<Arc<DirectoryService>>| async move {
                    Json(service.get_ctr().await)
                }),
            )
            .with_state(svc);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
