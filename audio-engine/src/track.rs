use livi::event::LV2AtomSequence;
use log::*;

use crate::audio_buffer::AudioBuffer;

/// A single audio chain.
#[derive(Debug)]
pub struct Track {
    /// The properties of the track. These are not used for processing
    /// internally.
    pub properties: TrackProperties,

    id: i32,
    plugins: Vec<livi::Instance>,
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
    pub fn push_plugin(&mut self, plugin: livi::Instance) {
        self.plugins.push(plugin);
    }

    /// Run processing for the track.
    pub fn process(&mut self, samples: usize, midi_input: &LV2AtomSequence) -> &AudioBuffer {
        self.audio_output.reset_with_buffer_size(samples);
        self.audio_input.reset_with_buffer_size(samples);
        for plugin in self.plugins.iter_mut() {
            std::mem::swap(&mut self.audio_input, &mut self.audio_output);
            let port_counts = plugin.port_counts();
            let ports = livi::EmptyPortConnections::new()
                .with_atom_sequence_inputs(
                    std::iter::once(midi_input).take(port_counts.atom_sequence_inputs),
                )
                .with_audio_inputs(
                    self.audio_input
                        .iter_channels()
                        .take(port_counts.audio_inputs),
                )
                .with_audio_outputs(
                    self.audio_output
                        .iter_channels_mut()
                        .take(port_counts.audio_outputs),
                );
            match unsafe { plugin.run(samples, ports) } {
                Ok(()) => {}
                Err(err) => {
                    self.properties.disabled = true;
                    error!("Processing for plugin {:?} failed! {:?}", plugin, err)
                }
            }
        }
        &self.audio_output
    }
}
