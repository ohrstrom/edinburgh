use super::ensemble::Ensemble;
use super::msc::AACPResult;
use super::pad::mot::MOTImage;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub enum EDIEvent {
    // FIC
    EnsembleUpdated(Ensemble),
    // Audio
    AACPFramesExtracted(AACPResult),
    // PAD
    MOTImageReceived(MOTImage),
}
