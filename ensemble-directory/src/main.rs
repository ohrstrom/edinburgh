mod services;

use axum::{extract::State, routing::get, Json, Router};
use clap::Parser;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tower_http::cors::{Any, CorsLayer};

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

    /// Scan only once and print the result. Not starting a server
    #[arg(long = "once")]
    scan_once: bool,

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

    let run_server = !args.scan_once;

    let addr = format!("{}:{}", args.host, args.port.unwrap());

    // validate that timeout is less than interval
    if args.scan_timeout >= args.scan_interval {
        tracing::error!(
            "scan timeout ({}) must be less than scan interval ({})",
            args.scan_timeout,
            args.scan_interval
        );
        std::process::exit(1);
    }

    let svc = services::DirectoryService::new(
        args.scan_targets,
        args.scan_interval,
        args.scan_timeout,
        args.scan_num_parallel,
    );

    // println!("{:?}", svc.ensembles);

    if args.scan_once {
        while svc.get_num_runs().await == 0 {
            sleep(Duration::from_millis(25)).await;
        }

        let mut dir_ensembles = svc.get_ensembles().await;

        // dir_ensembles.sort_by_key(|e| (e.host.clone(), e.port));
        dir_ensembles.sort_by_key(|e| {
            (
                e.host.clone(),
                e.ensemble.label.as_ref().unwrap_or(&"".into()).clone(),
            )
        });

        for e in dir_ensembles {
            let mux = format!(
                "0x{:4X}  {:16}",
                e.ensemble.eid.unwrap_or(0),
                e.ensemble.label.unwrap_or_default()
            );
            let host = format!("{}:{}", e.host, e.port);

            let mut services = e.ensemble.services;
            services.sort_by_key(|svc| svc.label.as_ref().unwrap_or(&"".into()).clone());

            for svc in services {
                println!(
                    "SVC  0x{:4X}  {:16}  {:8} | {} | {}",
                    svc.sid,
                    svc.label.unwrap_or_default(),
                    svc.short_label.unwrap_or_default(),
                    mux,
                    host
                );
            }
        }
    }

    if run_server {
        tracing::info!("Starting server on http://{}/", addr);

        let cors = CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any);

        let app = Router::new()
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
    }

    Ok(())
}
