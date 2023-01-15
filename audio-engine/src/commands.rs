use livi::Instance;

use crate::track::Track;

/// Commands for the main audio engine to execute.
#[derive(Debug)]
pub enum Command {
    /// Add a new track.
    AddTrack(Track),
    /// Delete tracks.
    DeleteTrack(i32),
    /// Add a plugin to the track.
    AddPluginToTrack(i32, Instance),
}
