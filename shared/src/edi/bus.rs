use super::ensemble::Ensemble;
use super::msc::AACPResult;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub enum EDIEvent {
    EnsembleUpdated(Ensemble),
    AACPFramesExtracted(AACPResult),
}
