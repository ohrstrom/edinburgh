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
pub struct Service {
    pub sid: Option<u16>,
    pub scid: Option<u8>,
    pub tmid: Option<u8>,
    pub label: Option<String>,
    pub short_label: Option<String>,
    pub subchannel: Option<Subchannel>,
    pub language: Option<tables::Language>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Ensemble {
    pub eid: Option<u16>,
    pub al_flag: Option<bool>,
    pub label: Option<String>,
    pub short_label: Option<String>,
    pub services: Vec<Service>,
    pub subchannels: Vec<Subchannel>,
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
        }
    }
    pub async fn feed(&mut self, tag: &DETITag) -> bool {
        // log::debug!("Ensemble::feed: {:?}", tag);

        let mut updated = false;

        for fig in &tag.figs {

            // log::debug!("FIG: {:?}", fig);

            match fig {
                FIG::F0_0(fig) => {
                    updated |= self.eid.replace(fig.eid) != Some(fig.eid);
                    updated |= self.al_flag.replace(fig.al_flag) != Some(fig.al_flag);
                }
                FIG::F0_1(fig) => {
                    // FIG 0/1 - Sub-channel organization (MCI)
                    for sc in &fig.subchannels {
                        // log::debug!("SC: {:?}", sc);
                        let existing_sc = self
                            .subchannels
                            .iter_mut()
                            .find(|s| s.id == sc.id);

                        match existing_sc {
                            Some(existing_sc) => {
                                updated |= existing_sc.start.replace(sc.start) != Some(sc.start);
                                updated |= existing_sc.size.replace(sc.size.unwrap_or_default()) != sc.size;
                                // updated |= existing_sc.pl.replace(sc.pl.clone()) != sc.pl.clone();
                                updated |= existing_sc.bitrate.replace(sc.bitrate.unwrap_or_default()) != sc.bitrate;

                                // NOTE: i think pl should not be string until here..
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
                    // FIG 0/2 - Service organization (MCI)
                    for service in fig.services.iter() {
                        let existing_service = self
                            .services
                            .iter_mut()
                            .find(|s| s.sid == Some(service.sid));

                        // check if we already have a subchannel for the service
                        let service_sc = self.subchannels
                            .iter()
                            .find(|s| s.id == service.scid);

                        match existing_service {
                            Some(existing_service) => {
                                updated |=
                                    existing_service.sid.replace(service.sid) != Some(service.sid);
                                updated |= existing_service.scid.replace(service.scid)
                                    != Some(service.scid);
                                updated |= existing_service.tmid.replace(service.tmid)
                                    != Some(service.tmid);

                               // set or update subchannel
                                if let Some(service_sc) = service_sc {
                                    if existing_service.subchannel != Some(service_sc.clone()) {
                                        existing_service.subchannel = Some(service_sc.clone());
                                        updated = true;
                                    }
                                }

                            }
                            None => {
                                self.services.push(Service {
                                    sid: Some(service.sid),
                                    scid: Some(service.scid),
                                    tmid: Some(service.tmid),
                                    label: None,
                                    short_label: None,
                                    subchannel: service_sc.cloned(),
                                    language: None,
                                });
                                updated = true;
                            }
                        }
                    }
                }
                FIG::F0_5(fig) => {
                    // FIG 0/5 - Service component language (SI)
                    for service in fig.services.iter() {
                        if let Some(existing_service) = self.services.iter_mut().find(|s| s.scid == Some(service.scid)) {
                            //
                            // log::debug!("S: {:?}", existing_service);
                            updated |= existing_service.language.replace(service.language)
                            != Some(service.language);
                        }
                    }

                }
                FIG::F0_9(fig) => {
                    // FIG 0/9 - Country, LTO & International table (SI)
                    // log::debug!("FIG 0/9: {:?}", fig);
                }
                FIG::F0_10(fig) => {
                    // FIG 0/10 - Date & time (SI)
                    // log::debug!("FIG 0/10: {:?}", fig);
                }
                FIG::F0_13(fig) => {
                    // FIG 0/13 - User Application information (MCI)
                    // log::debug!("FIG 0/13: {:?}", fig);
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
                }
                _ => {}
            }
        }

    
        if updated {
            log::info!("ENSEMBLE: {:#?}", self);
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
    }
}
