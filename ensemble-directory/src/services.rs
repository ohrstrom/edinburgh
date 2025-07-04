use anyhow;
use regex::Regex;
use serde::Serialize;
use std::any;
use std::io;
use std::sync::{Arc, Mutex};
use tokio::io::Interest;
use tokio::net::TcpStream;
use tokio::sync::RwLock;
use tokio::time::{self, timeout, Duration};
use tracing as log;

use crate::edi_frame_extractor::EDIFrameExtractor;
use shared::edi::EDISource;
use shared::edi::Ensemble;

#[derive(Serialize)]
pub struct Message {
    pub message: String,
}

#[derive(Clone, Debug)]
pub struct ScanTarget {
    pub host: String,
    pub port_range: (u16, u16),
}

impl std::str::FromStr for ScanTarget {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let re = Regex::new(r"^(?P<host>[^:]+):(?P<start>\d+)(?:-(?P<end>\d+))?$")
            .map_err(|_| "Invalid regex".to_string())?;

        let caps = re
            .captures(s)
            .ok_or_else(|| "Invalid format: must be host:port or host:port-port".to_string())?;

        let host = caps.name("host").unwrap().as_str().to_string();
        let start_port = caps
            .name("start")
            .unwrap()
            .as_str()
            .parse::<u16>()
            .map_err(|_| "Invalid start port".to_string())?;
        let end_port = match caps.name("end") {
            Some(m) => m
                .as_str()
                .parse::<u16>()
                .map_err(|_| "Invalid end port".to_string())?,
            None => start_port,
        };

        Ok(Self {
            host,
            port_range: (start_port, end_port),
        })
    }
}

#[derive(Clone)]
pub struct DirectoryService {
    pub ensembles: Arc<RwLock<Vec<Ensemble>>>,
    pub ctr: Arc<RwLock<u32>>,
    pub scan_targets: Vec<ScanTarget>,
}

impl DirectoryService {
    pub fn new(scan_targets: Vec<ScanTarget>) -> Arc<Self> {
        let svc = Arc::new(Self {
            ensembles: Arc::new(RwLock::new(Vec::new())),
            ctr: Arc::new(RwLock::new(0)),
            scan_targets,
        });

        let svc_clone = Arc::clone(&svc);

        tokio::spawn(async move {
            svc_clone.run_scan().await;
        });

        svc
    }

    pub fn get_root(&self) -> Message {
        Message {
            message: "/".into(),
        }
    }

    pub async fn get_ensembles(&self) -> Vec<Ensemble> {
        self.ensembles.read().await.clone()
    }

    pub async fn get_ctr(&self) -> u32 {
        *self.ctr.read().await
    }

    async fn run_scan(self: Arc<Self>) {
        let mut interval = time::interval(Duration::from_secs(20));
        interval.tick().await; // eat the first tick

        let endpoints: Vec<String> = self
            .scan_targets
            .iter()
            .flat_map(|target| {
                let (start, end) = target.port_range;
                (start..=end).map(move |port| format!("{}:{}", target.host, port))
            })
            .collect();

        println!("targets:   {:?}", self.scan_targets);
        println!("endpoints: {:?}", endpoints);

        loop {
            // dummy - increment counter
            {
                let mut lock = self.ctr.write().await;
                *lock += 1;
            }

            let mut ensembles = Vec::new();

            for endpoint in &endpoints {
                match scan(endpoint.to_string()).await {
                    Ok(ensemble) => {
                        log::info!(
                            "Scanning endpoint complete: {} - 0x{:4x} - {}",
                            endpoint,
                            ensemble.eid.unwrap_or(0),
                            ensemble.label.as_deref().unwrap_or("-")
                        );
                        ensembles.push(ensemble);
                    }
                    Err(err) => {
                        log::error!("Failed to scan ensemble: {}", err);
                        continue;
                    }
                };
            }

            {
                let mut lock = self.ensembles.write().await;
                *lock = ensembles;
            }

            // sleep until next scan
            interval.tick().await;
        }
    }
}

async fn scan(endpoint: String) -> anyhow::Result<Ensemble> {
    log::debug!("Scanning endpoint: {}", endpoint);

    let stream = TcpStream::connect(endpoint).await?;
    let mut filled = 0;
    let mut extractor = EDIFrameExtractor::new();

    let (done_tx, mut done_rx) = tokio::sync::oneshot::channel::<Ensemble>();
    let done_tx = Arc::new(Mutex::new(Some(done_tx)));

    let mut source = EDISource::new(
        None,
        Some(Box::new({
            let done_tx = Arc::clone(&done_tx);
            move |ensemble: &Ensemble| {
                if ensemble.complete {
                    let mut guard = done_tx.lock().unwrap();
                    if let Some(tx) = guard.take() {
                        let _ = tx.send(ensemble.clone());
                    }
                }
            }
        })),
        None,
    );

    loop {
        tokio::select! {
            Ok(ensemble) = &mut done_rx => {
                return Ok(ensemble);
            }
            ready = timeout(Duration::from_secs(5), stream.ready(Interest::READABLE)) => {
                match ready {
                    Ok(Ok(ready)) => {
                    if ready.is_readable() {
                        match stream.try_read(&mut extractor.frame.data[filled..]) {
                            Ok(0) => {
                                log::info!("Connection closed by peer");
                                anyhow::bail!("Connection closed before ensemble complete");
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
                                        extractor.frame.reset();
                                        filled = 0;
                                    }
                                }
                            }
                            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => continue,
                            Err(e) => anyhow::bail!("Read error: {}", e),
                        }
                    }
                     }
                    Ok(Err(e)) => anyhow::bail!("Stream error: {}", e),
                    Err(_) => anyhow::bail!("No data from stream for 5s"),
                }
            }
        }
    }
}
