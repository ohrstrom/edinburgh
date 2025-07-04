use regex::Regex;
use std::any;
use std::io;
use std::sync::Arc;
use tokio::sync::RwLock;
// use std::sync::RwLock;
use anyhow;
use serde::Serialize;
use std::net::Shutdown;
use tokio::io::Interest;
use tokio::net::TcpStream;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::oneshot;
use tokio::sync::Mutex;
use tokio::time::{self, Duration};
use tracing as log;

use crate::edi_frame_extractor::EDIFrameExtractor;
use shared::edi::bus::{init_event_bus, EDIEvent};
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

/*
impl ScanTarget {
    pub fn new(target: String) -> Self {
        let parts: Vec<&str> = target.split(':').collect();
        let host = parts[0].to_string();
        Self { host, port_range }
    }
}
*/

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
    pub event_rx: Arc<tokio::sync::Mutex<UnboundedReceiver<EDIEvent>>>,
    pub scan_targets: Vec<ScanTarget>,
}

impl DirectoryService {
    pub fn new(scan_targets: Vec<ScanTarget>) -> Arc<Self> {
        let event_rx = init_event_bus();

        let svc = Arc::new(Self {
            ensembles: Arc::new(RwLock::new(Vec::new())),
            ctr: Arc::new(RwLock::new(0)),
            event_rx: Arc::new(tokio::sync::Mutex::new(event_rx)),
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

        // let endpoints = vec![
        //     "edi-uk.digris.net:8851",
        //     "edi-uk.digris.net:8852",
        //     "edi-uk.digris.net:8853",
        //     ...
        // ];

        loop {
            // dummy - increment counter
            {
                let mut lock = self.ctr.write().await;
                *lock += 1;
            }

            // dummy - read ensembls
            /*
            let ensembles = load_ensembles().unwrap_or_else(|_| {
                log::error!("Failed to load ensembles, using empty list");
                vec![]
            });
            {
                let mut lock = self.ensembles.write().await;
                *lock = ensembles;
            }
            */

            let mut ensembles = Vec::new();

            // scan ensembles
            // for endpoint in &endpoints {
            //     log::info!("Scanning endpoint: {}", endpoint);
            // }

            let mut interval = time::interval(Duration::from_secs(1));
            interval.tick().await;

            // scan ensembles
            for endpoint in &endpoints {
                log::info!("Scanning endpoint: {}", endpoint);
                match scan(endpoint.to_string(), self.event_rx.clone()).await {
                    Ok(ensemble) => {
                        log::debug!(
                            "Endpoint scan complete: {} - {}",
                            endpoint,
                            ensemble.eid.unwrap_or(0)
                        );
                        ensembles.push(ensemble);
                        interval.tick().await;
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

            // match scan(self.event_rx.clone()).await {
            //     Ok(ensemble) => {
            //         println!("E: {}", ensemble.eid.unwrap_or(0));
            //         {
            //             let mut lock = self.ensembles.write().await;
            //             *lock = vec![ensemble];
            //         }
            //     },
            //     Err(err) => {
            //         log::error!("Failed to scan ensemble: {}", err);
            //         continue;
            //     }
            // };

            // dummy sleep...
            interval.tick().await;
        }
    }
}

/*
fn load_ensembles() -> anyhow::Result<Vec<Ensemble>> {
    let contents = std::fs::read_to_string("directory.yaml")?;
    let ensembles: Vec<Ensemble> = serde_yaml::from_str(&contents)?;
    Ok(ensembles)
}
*/

struct EDIHandler {
    receiver: Arc<tokio::sync::Mutex<UnboundedReceiver<EDIEvent>>>,
    done_tx: oneshot::Sender<Ensemble>,
}

impl EDIHandler {
    pub fn new(
        receiver: Arc<tokio::sync::Mutex<UnboundedReceiver<EDIEvent>>>,
        done_tx: oneshot::Sender<Ensemble>,
    ) -> Self {
        Self { receiver, done_tx }
    }

    pub async fn run(mut self) {
        let mut rx = self.receiver.lock().await;
        while let Some(event) = rx.recv().await {
            match event {
                EDIEvent::EnsembleUpdated(ensemble) => {
                    if ensemble.complete {
                        log::info!("Ensemble complete: {}", ensemble.eid.unwrap_or(0));
                        let _ = self.done_tx.send(ensemble);
                        break;
                    }
                }
                _ => {}
            }
        }
    }
}

async fn scan(
    endpoint: String,
    event_rx: Arc<Mutex<UnboundedReceiver<EDIEvent>>>,
) -> anyhow::Result<Ensemble> {
    // let endpoint = "edi-uk.digris.net:8853";
    let stream = TcpStream::connect(endpoint).await?;

    let mut filled = 0;

    let mut extractor = EDIFrameExtractor::new();

    let mut source = EDISource::new(None, None, None);

    let (done_tx, mut done_rx) = oneshot::channel::<Ensemble>();

    let event_handler = EDIHandler::new(event_rx, done_tx);

    tokio::spawn(async move {
        event_handler.run().await;
    });

    loop {
        tokio::select! {
            Ok(ensemble) = &mut done_rx => {

                drop(stream);
                drop(source);

                return Ok(ensemble);
            }

            ready = stream.ready(Interest::READABLE) => {
                let ready = ready?;
                if ready.is_readable() {
                    match stream.try_read(&mut extractor.frame.data[filled..]) {
                        Ok(0) => {
                            log::info!("Connection closed by peer");
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
                                    extractor.frame.reset();
                                    filled = 0;
                                }
                            }
                        }
                        Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => continue,
                        Err(e) => return Err(e.into()),
                    }
                }
            }
        }
    }

    anyhow::bail!("Failed to complete ensemble scan");

    /*
    loop {
        let ready = stream.ready(Interest::READABLE).await?;
        if ready.is_readable() {
            match stream.try_read(&mut extractor.frame.data[filled..]) {
                Ok(0) => {
                    log::info!("Connection closed by peer");
                    break;
                }
                Ok(n) => {
                    filled += n;

                    if filled < extractor.frame.data.len() {
                        continue;
                    }

                    // Process the received data
                    if let Some(offset) = extractor.frame.find_sync_magic() {
                        if offset > 0 {
                            log::debug!("offset: {}", offset);
                            extractor.frame.data.copy_within(offset.., 0);
                            filled -= offset;

                            continue;
                        }

                        if extractor.frame.check_completed() {
                            // log::debug!("frame: {}", extractor.frame);

                            source.feed(&extractor.frame.data).await;

                            extractor.frame.reset();
                            filled = 0;
                        }
                    }
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    continue;
                }
                Err(e) => {
                    return Err(e.into());
                }
            }
        }
    }
    Ok(())
    */
}
