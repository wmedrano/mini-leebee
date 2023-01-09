use crate::track::Track;

/// Commands for the main audio engine to execute.
#[derive(Debug)]
pub enum Command {
    /// Add a new track.
    AddTrack(Track),
}
