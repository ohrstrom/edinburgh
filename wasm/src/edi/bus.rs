use super::ensemble::Ensemble;
use super::msc::AACPResult;
use serde::Serialize;

/*
    NOTE: not in use at the moment.
    goal is to have a "shared" bus logic to handle update events.
*/

#[derive(Debug, Serialize)]
pub enum EDIEvent {
    EnsembleUpdated(Ensemble),
    AACPFramesExtracted(AACPResult),
}
