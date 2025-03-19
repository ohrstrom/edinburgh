use super::ensemble::Ensemble;

/*
    NOTE: not in use at the moment.
    goal is to have a "shared" bus logic to handle update events.
*/

#[derive(Debug)]
pub enum EDIEvent {
    EnsembleUpdated(Ensemble),
}
