use std::{collections::HashSet, path::Path};

use audio_engine::commands::Command;

/// Implements the MiniLeebee gRPC service.
#[derive(Debug)]
pub struct State {
    jack_adapter: jack_adapter::JackAdapter,
    state: InnerState,
    ok_sound: audio_engine::plugin::SampleTrigger,
}

#[derive(Debug)]
struct InnerState {
    metronome: Metronome,
    tracks: Vec<Track>,
    armed_track: Option<i32>,
    next_track_id: i32,
}

impl State {
    /// Create a new server.
    pub fn new(jack_adapter: jack_adapter::JackAdapter) -> State {
        let mut ok_sound =
            audio_engine::plugin::SampleTrigger::from_wav(Path::new("resources/beep.wav"));
        ok_sound.start();
        State {
            jack_adapter,
            state: InnerState {
                metronome: Metronome {
                    beats_per_minute: 120.0,
                    volume: 0.0,
                },
                tracks: Vec::new(),
                armed_track: None,
                next_track_id: 1,
            },
            ok_sound,
        }
    }

    pub fn play_sound(&self) {
        self.jack_adapter
            .audio_engine
            .commands
            .send(Command::PlaySound(self.ok_sound.clone()))
            .unwrap();
    }

    /// Get the plugins.
    pub fn get_plugins(&self) -> Vec<Plugin> {
        let plugins = self
            .jack_adapter
            .audio_engine
            .livi
            .iter_plugins()
            .map(|p| Plugin {
                id: id_for_plugin(&p),
                name: p.name(),
                class: if p.is_instrument() {
                    PluginClass::Instrument
                } else {
                    PluginClass::Effect
                },
            })
            .collect();
        plugins
    }

    /// Set the metronome parameters.
    pub fn set_metronome(&mut self, metronome: Metronome) {
        self.jack_adapter
            .audio_engine
            .commands
            .send(Command::SetMetronome {
                volume: metronome.volume,
                beats_per_minute: metronome.beats_per_minute,
            })
            .unwrap();
        self.state.metronome = metronome;
    }

    /// Set a track to be armed.
    pub fn set_armed(&mut self, track_id: Option<i32>) {
        self.state.armed_track = track_id;
        let track_id = track_id.unwrap_or(-1);
        self.jack_adapter
            .audio_engine
            .commands
            .send(Command::ArmTrack(track_id))
            .unwrap();
    }

    /// Add a plugin to a track.
    pub fn add_plugin_to_track(&mut self, track_id: i32, plugin_id: &str) -> Result<(), String> {
        let plugin = match self
            .jack_adapter
            .audio_engine
            .livi
            .iter_plugins()
            .find(|p| id_for_plugin(p) == plugin_id)
        {
            Some(p) => p,
            None => {
                return Err(format!("plugin {} not found", plugin_id));
            }
        };
        let track = match self.state.tracks.iter_mut().find(|t| t.id == track_id) {
            Some(t) => t,
            None => {
                return Err(format!("track {} not found", track_id));
            }
        };
        let instance_or_err = unsafe {
            plugin.instantiate(
                self.jack_adapter.audio_engine.lv2_features.clone(),
                self.jack_adapter.sample_rate(),
            )
        };
        let instance = match instance_or_err {
            Ok(i) => i,
            Err(err) => {
                return Err(format!(
                    "failed to instantiate plugin {}: {}",
                    plugin.name(),
                    err
                ))
            }
        };
        self.jack_adapter
            .audio_engine
            .commands
            .send(Command::AddPluginToTrack(track.id, instance.into()))
            .unwrap();
        track.plugins.push(TrackPlugin {
            plugin_id: plugin_id.to_string(),
        });
        self.play_sound();
        Ok(())
    }

    /// Remove a plugin from a track.
    pub fn remove_plugin_from_track(
        &mut self,
        track_id: i32,
        plugin_index: usize,
    ) -> Result<(), String> {
        let track = match self.state.tracks.iter_mut().find(|t| t.id == track_id) {
            Some(t) => t,
            None => return Err(format!("track {} not found", track_id)),
        };
        if plugin_index >= track.plugins.len() {
            return Err(format!(
                "track {} does not a plugin at index {}",
                track_id, plugin_index
            ));
        }
        self.jack_adapter
            .audio_engine
            .commands
            .send(Command::DeletePlugin(track_id, plugin_index))
            .unwrap();
        track.plugins.remove(plugin_index);
        self.play_sound();
        Ok(())
    }

    /// Get the tracks.
    pub fn iter_tracks(&self) -> impl Iterator<Item = &Track> {
        self.state.tracks.iter()
    }

    /// Get the metronome.
    pub fn metronome(&self) -> &Metronome {
        &self.state.metronome
    }

    /// Create a track.
    pub fn create_track(&mut self, name: Option<String>) -> Result<i32, String> {
        let track_id = self.state.next_track_id;
        let track = Track {
            name: name.unwrap_or_else(|| format!("Track {track_id}")),
            id: track_id,
            plugins: Vec::new(),
            properties: TrackProperties { armed: false },
        };
        let audio_engine_track =
            audio_engine::track::Track::new(track_id, self.jack_adapter.buffer_size());
        self.jack_adapter
            .audio_engine
            .commands
            .send(Command::AddTrack(audio_engine_track))
            .unwrap();
        self.state.tracks.push(track);
        self.state.next_track_id += 1;
        self.play_sound();
        Ok(track_id)
    }

    /// Delete tracks.
    pub fn delete_tracks(
        &mut self,
        ids_requested_for_deletion: HashSet<i32>,
    ) -> Result<(), String> {
        let existing_ids = self.state.tracks.iter().map(|t| t.id);
        let delete_targets: HashSet<i32> = existing_ids
            .filter(|id| ids_requested_for_deletion.contains(id))
            .collect();
        for t in delete_targets.iter() {
            self.jack_adapter
                .audio_engine
                .commands
                .send(Command::DeleteTrack(*t))
                .unwrap();
        }
        self.state
            .tracks
            .retain(|t| !delete_targets.contains(&t.id));
        self.play_sound();
        Ok(())
    }

    /// Create a pprof report and call `callback` with the SVG data.
    pub fn pprof_report(
        &self,
        duration: std::time::Duration,
        callback: Box<dyn Send + FnOnce(Vec<u8>)>,
    ) {
        std::thread::spawn(move || {
            let guard = pprof::ProfilerGuardBuilder::default()
                .frequency(1000)
                .build()
                .unwrap();
            std::thread::sleep(duration);
            let report = guard.report().build().unwrap();
            let mut flamegraph_svg = Vec::new();
            report.flamegraph(&mut flamegraph_svg).unwrap();
            callback(flamegraph_svg);
        });
    }
}

/// Get the id for the plugin.
fn id_for_plugin(p: &livi::Plugin) -> String {
    format!("lv2:{}", p.uri())
}

/// A plugin.
#[derive(Clone, Debug)]
pub struct Plugin {
    pub id: String,
    pub name: String,
    pub class: PluginClass,
}

#[derive(Clone, Copy, Debug)]
pub enum PluginClass {
    Instrument,
    Effect,
}

/// A track.
#[derive(Clone, Debug)]
pub struct Track {
    /// The unique identifier for the track.
    pub id: i32,

    /// The name of the track.
    pub name: String,

    /// The plugins on the track.
    pub plugins: Vec<TrackPlugin>,

    /// The track properties.
    pub properties: TrackProperties,
}

#[derive(Copy, Clone, Debug)]
pub struct TrackProperties {
    pub armed: bool,
}

// A plugin within a track.
#[derive(Clone, Debug)]
pub struct TrackPlugin {
    // The id of the plugin.
    pub plugin_id: String,
}

#[derive(Clone, Debug)]
pub struct Metronome {
    /// The beats per minute.
    pub beats_per_minute: f32,

    // The volume of the metronome.
    pub volume: f32,
}
