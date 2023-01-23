use crate::{plugin::PluginInstance, track::Track};

/// Commands for the main audio engine to execute.
#[derive(Debug)]
pub enum Command {
    /// Add a new track.
    AddTrack(Track),
    /// Delete tracks.
    DeleteTrack(i32),
    /// Add a plugin to the track.
    AddPluginToTrack(i32, PluginInstance),
    /// Delete a plugin from a track.
    DeletePlugin(i32, usize),
    /// Set metronome properties.
    SetMetronome { volume: f32, beats_per_minute: f32 },
}
