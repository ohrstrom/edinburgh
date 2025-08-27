use std::sync::Arc;
mod services;

use axum::{extract::State, routing::get, Json, Router};
use clap::Parser;
use tower_http::cors::{Any, CorsLayer};
use tracing as log;

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
    /// format: host:port-port or host:port,
    /// repeat for multiple targets
    #[arg(long = "scan", required = true)]
    scan_targets: Vec<ScanTarget>,

    /// Scan interval, in seconds
    #[arg(long = "scan-interval", default_value = "60")]
    scan_interval: u64,

    /// Scan timeout, in seconds
    #[arg(long = "scan-timeout", default_value = "5")]
    scan_timeout: u64,

    /// Scan parallelism: number of concurrent scans
    #[arg(long = "scan-parallel", default_value = "8")]
    scan_num_parallel: usize,

    /// Verbose logging
    #[arg(long = "verbose", short = 'v')]
    verbose: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let log_level = if args.verbose { "debug" } else { "info" };

    tracing_subscriber::fmt()
        .with_env_filter(log_level)
        .with_target(false)
        .without_time()
        .init();

    let addr = format!("{}:{}", args.host, args.port.unwrap());

    // validate that timeout is less than interval
    if args.scan_timeout >= args.scan_interval {
        log::error!(
            "scan timeout ({}) must be less than scan interval ({})",
            args.scan_timeout,
            args.scan_interval
        );
        std::process::exit(1);
    }

    log::info!("Starting API on http://{}/", addr);

    let svc = services::DirectoryService::new(
        args.scan_targets,
        args.scan_interval,
        args.scan_timeout,
        args.scan_num_parallel,
    );

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
