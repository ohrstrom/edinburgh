use log;
use serde::Serialize;

use super::bus::{EDIEvent, emit_event};
use super::fic::FIG;
use super::tables;
use super::frame::DETITag;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Subchannel {
    pub id: u8,
    pub start: Option<usize>,
    pub size: Option<usize>,
    pub pl: Option<String>,
    pub bitrate: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ServiceComponent {
    pub scid: u8,
    pub language: Option<tables::Language>,
    pub subchannel_id: Option<u8>,
    pub user_apps: Vec<tables::UserApplication>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Service {
    pub sid: u16,
    pub label: Option<String>,
    pub short_label: Option<String>,
    pub components: Vec<ServiceComponent>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Ensemble {
    pub eid: Option<u16>,
    pub al_flag: Option<bool>,
    pub label: Option<String>,
    pub short_label: Option<String>,
    pub services: Vec<Service>,
    pub subchannels: Vec<Subchannel>,
    pub complete: bool,
}

impl Ensemble {
    pub fn new() -> Self {
        Ensemble {
            eid: None,
            al_flag: None,
            label: None,
            short_label: None,
            services: Vec::new(),
            subchannels: Vec::new(),
            complete: false,
        }
    }

    pub async fn feed(&mut self, tag: &DETITag) -> bool {
        let mut updated = false;

        for fig in &tag.figs {
            match fig {
                FIG::F0_0(fig) => {
                    updated |= self.eid.replace(fig.eid) != Some(fig.eid);
                    updated |= self.al_flag.replace(fig.al_flag) != Some(fig.al_flag);
                }
                FIG::F0_1(fig) => {
                    for sc in &fig.subchannels {
                        let existing_sc = self.subchannels.iter_mut().find(|s| s.id == sc.id);

                        match existing_sc {
                            Some(existing_sc) => {
                                updated |= existing_sc.start.replace(sc.start) != Some(sc.start);
                                updated |= existing_sc.size.replace(sc.size.unwrap_or_default()) != sc.size;
                                updated |= existing_sc.bitrate.replace(sc.bitrate.unwrap_or_default()) != sc.bitrate;
                                if existing_sc.pl != sc.pl {
                                    existing_sc.pl = sc.pl.clone();
                                    updated = true;
                                }
                            }
                            None => {
                                self.subchannels.push(Subchannel {
                                    id: sc.id,
                                    start: Some(sc.start),
                                    size: sc.size,
                                    pl: sc.pl.clone(),
                                    bitrate: sc.bitrate,
                                });
                                updated = true;
                            }
                        }
                    }
                }
                FIG::F0_2(fig) => {
                    for entry in &fig.services {
                        let service = self.services.iter_mut().find(|s| s.sid == entry.sid);

                        match service {
                            Some(existing_service) => {
                                if !existing_service.components.iter().any(|c| c.scid == entry.scid) {
                                    existing_service.components.push(ServiceComponent {
                                        scid: entry.scid,
                                        language: None,
                                        subchannel_id: Some(entry.scid),
                                        user_apps: Vec::new(),
                                    });
                                    updated = true;
                                }
                            }
                            None => {
                                self.services.push(Service {
                                    sid: entry.sid,
                                    label: None,
                                    short_label: None,
                                    components: vec![ServiceComponent {
                                        scid: entry.scid,
                                        language: None,
                                        subchannel_id: Some(entry.scid),
                                        user_apps: Vec::new(),
                                    }],
                                });
                                updated = true;
                            }
                        }
                    }
                }
                FIG::F0_5(fig) => {
                    for lang in &fig.services {
                        let mut matched = 0;
                        for service in &mut self.services {
                            if let Some(component) = service.components.iter_mut().find(|c| c.scid == lang.scid) {
                                matched += 1;
                                updated |= component.language.replace(lang.language) != Some(lang.language);
                            }
                        }
                        if matched > 1 {
                            // log::warn!("FIG0/5: SCId {} matched multiple components across services!", lang.scid);
                        }
                    }
                }
                FIG::F0_13(fig) => {
                    for entry in &fig.services {
                        if let Some(service) = self.services.iter_mut().find(|s| s.sid == entry.sid) {
                            if entry.scids == 0 {
                                // Apply to all components
                                for component in &mut service.components {
                                    if component.user_apps != entry.uas {
                                        component.user_apps = entry.uas.clone();
                                        updated = true;
                                    }
                                }
                            } else {
                                for i in 0..8 {
                                    if (entry.scids & (1 << i)) != 0 {
                                        if let Some(component) = service.components.iter_mut().find(|c| c.scid == i) {
                                            if component.user_apps != entry.uas {
                                                component.user_apps = entry.uas.clone();
                                                updated = true;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                FIG::F1_0(fig) => {
                    updated |= self.label.replace(fig.label.clone()) != Some(fig.label.clone());
                    updated |= self.short_label.replace(fig.short_label.clone()) != Some(fig.short_label.clone());
                }
                FIG::F1_1(fig) => {
                    if let Some(service) = self.services.iter_mut().find(|s| s.sid == fig.sid) {
                        updated |= service.label.replace(fig.label.clone()) != Some(fig.label.clone());
                        updated |= service.short_label.replace(fig.short_label.clone()) != Some(fig.short_label.clone());
                    }
                }
                _ => {}
            }
        }

        if updated {
            // "completeness" means for the moment:
            // - EID and label present
            // - SID and label present on all services
            
            // this is not so nice, as complete could / will set to true
            // when subchannels are not yet completed (e.g. language)

            if self.eid.is_some()
                && self.label.is_some()
                && self.services.iter().all(|s| s.label.is_some())
            {
                self.complete = true;
            } else {
                self.complete = false;
            }

            for s in &self.services {
                // println!("{:?}", s);
                for sc in &s.components {
                    println!("{:?}", sc);
                } 
            } 
        }

        if updated {
            // log::info!("ENSEMBLE: {:#?}", self);
            emit_event(EDIEvent::EnsembleUpdated(self.clone()));
        }

        updated
    }

    pub fn reset(&mut self) {
        self.eid = None;
        self.al_flag = None;
        self.label = None;
        self.short_label = None;
        self.services.clear();
        self.subchannels.clear();
    }
}