use livi::event::LV2AtomSequence;

use crate::{audio_buffer::AudioBuffer, plugin::PluginInstance};

/// A single audio chain.
#[derive(Debug)]
pub struct Track {
    /// The properties of the track. These are not used for processing
    /// internally.
    pub properties: TrackProperties,

    id: i32,
    plugins: Vec<PluginInstance>,
    audio_input: AudioBuffer,
    audio_output: AudioBuffer,
}

/// Properties for the track.
#[derive(Copy, Clone, Debug)]
pub struct TrackProperties {
    /// True if the track should be disabled.
    pub disabled: bool,
    /// The volume multiplier.
    pub volume: f32,
}

impl Default for TrackProperties {
    fn default() -> Self {
        TrackProperties {
            disabled: false,
            volume: 0.5,
        }
    }
}

impl Track {
    /// Create a new track.
    pub fn new(id: i32, buffer_size: usize) -> Track {
        Track {
            properties: TrackProperties::default(),
            id,
            plugins: Vec::with_capacity(16),
            audio_input: AudioBuffer::with_stereo(buffer_size),
            audio_output: AudioBuffer::with_stereo(buffer_size),
        }
    }

    /// Get the `id` of the track.
    pub fn id(&self) -> i32 {
        self.id
    }

    /// Push a new plugin.
    pub fn push_plugin(&mut self, plugin: PluginInstance) {
        self.plugins.push(plugin);
    }

    /// Remove a plugin.
    pub fn remove_plugin(&mut self, index: usize) -> Option<PluginInstance> {
        if index < self.plugins.len() {
            Some(self.plugins.remove(index))
        } else {
            None
        }
    }

    /// Run processing for the track.
    pub fn process(&mut self, samples: usize, midi_input: &LV2AtomSequence) -> &AudioBuffer {
        self.audio_output.reset_with_buffer_size(samples);
        self.audio_input.reset_with_buffer_size(samples);
        for plugin in self.plugins.iter_mut() {
            std::mem::swap(&mut self.audio_input, &mut self.audio_output);
            let is_good = plugin.process(
                samples,
                midi_input,
                &self.audio_input,
                &mut self.audio_output,
            );
            if !is_good {
                self.properties.disabled = true;
            }
        }
        &self.audio_output
    }
}
