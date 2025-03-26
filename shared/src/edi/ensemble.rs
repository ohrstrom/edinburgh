// use futures::channel::mpsc::UnboundedSender;
use log;
use serde::Serialize;

use super::bus::EDIEvent;
use super::fic::FIG;
use super::frame::DETITag;

#[cfg(target_arch = "wasm32")]
use futures::channel::mpsc::UnboundedSender;

#[cfg(not(target_arch = "wasm32"))]
use tokio::sync::mpsc::UnboundedSender;

#[derive(Debug, Clone, Serialize)]
pub struct Service {
    pub sid: Option<u16>,
    pub scid: Option<u8>,
    pub tmid: Option<u8>,
    pub label: Option<String>,
    pub short_label: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Subchannel {
    pub id: Option<u8>,
    pub start: Option<usize>,
    pub sid: Option<u16>,
    pub pl: Option<String>,
    pub bitrate: Option<usize>,
    pub label: Option<String>,
    pub short_label: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Ensemble {
    pub eid: Option<u16>,
    pub al_flag: Option<bool>,
    pub label: Option<String>,
    pub short_label: Option<String>,
    pub services: Vec<Service>,
}
impl Ensemble {
    pub fn new() -> Self {
        Ensemble {
            eid: None,
            al_flag: None,
            label: None,
            short_label: None,
            services: Vec::new(),
        }
    }
    pub async fn feed(&mut self, tag: &DETITag, event_tx: &UnboundedSender<EDIEvent>) -> bool {
        // log::debug!("Ensemble::feed: {:?}", tag);

        let mut updated = false;

        for fig in &tag.figs {
            match fig {
                FIG::F0_0(fig) => {
                    updated |= self.eid.replace(fig.eid) != Some(fig.eid);
                    updated |= self.al_flag.replace(fig.al_flag) != Some(fig.al_flag);
                }
                /*
                FIG::F0_1(fig) => {
                    for sc in &fig.subchannels {
                        if sc.id > 30 {
                            // NOTE: not sure - i see strange services that continously change start..
                            continue;
                        }

                        let service = self.services.iter_mut().find(|s| s.id == Some(sc.id));

                        match service {
                            Some(existing_service) => {
                                if existing_service.start.replace(sc.start) != Some(sc.start) {
                                    updated = true;
                                }
                            }
                            None => {
                                self.services.push(Service {
                                    id: Some(sc.id),
                                    start: Some(sc.start),
                                    sid: None,
                                    pl: sc.pl.clone(),
                                    bitrate: sc.bitrate,
                                    label: None,
                                    short_label: None,
                                });
                                updated = true;
                            }
                        }
                    }
                }
                */
                FIG::F0_2(fig) => {
                    // FIG 0/2 - Service organization (MCI)

                    for service in fig.services.iter() {
                        let existing_service = self
                            .services
                            .iter_mut()
                            .find(|s| s.sid == Some(service.sid));

                        match existing_service {
                            Some(existing_service) => {
                                updated |=
                                    existing_service.sid.replace(service.sid) != Some(service.sid);
                                updated |= existing_service.scid.replace(service.scid)
                                    != Some(service.scid);
                                updated |= existing_service.tmid.replace(service.tmid)
                                    != Some(service.tmid);
                            }
                            None => {
                                self.services.push(Service {
                                    sid: Some(service.sid),
                                    scid: Some(service.scid),
                                    tmid: Some(service.tmid),
                                    label: None,
                                    short_label: None,
                                });
                                updated = true;
                            }
                        }
                    }
                }
                FIG::F1_0(fig) => {
                    // Ensemble label
                    updated |= self.label.replace(fig.label.clone()) != Some(fig.label.clone());
                    updated |= self.short_label.replace(fig.short_label.clone())
                        != Some(fig.short_label.clone());
                }
                FIG::F1_1(fig) => {
                    // Programme service label

                    if let Some(service) = self.services.iter_mut().find(|s| s.sid == Some(fig.sid))
                    {
                        updated |=
                            service.label.replace(fig.label.clone()) != Some(fig.label.clone());
                        updated |= service.short_label.replace(fig.short_label.clone())
                            != Some(fig.short_label.clone());
                    }

                    /*
                    match service {
                        Some(existing_service) => {
                            updated |= existing_service.label.replace(fig.label.clone()) != Some(fig.label.clone());
                            updated |= existing_service.short_label.replace(fig.short_label.clone()) != Some(fig.short_label.clone());
                        }
                        _ => {}

                        None => {
                            self.services.push(Service {
                                sid: Some(fig.sid),
                                id: None,
                                start: None,
                                pl: None,
                                bitrate: None,
                                label: Some(fig.label.clone()),
                                short_label: Some(fig.short_label.clone()),
                            });
                            updated = true;
                        }
                    }
                    */
                }
                _ => {}
            }
        }

        if updated {
            // log::debug!("Ensemble updated (local): {:?}", self);

            #[cfg(target_arch = "wasm32")]
            let _ = event_tx.unbounded_send(EDIEvent::EnsembleUpdated(self.clone()));

            #[cfg(not(target_arch = "wasm32"))]
            let _ = event_tx.send(EDIEvent::EnsembleUpdated(self.clone()));
        }

        updated
    }
    pub fn reset(&mut self) {
        self.eid = None;
        self.al_flag = None;
        self.label = None;
        self.short_label = None;
        self.services.clear();
    }
}
