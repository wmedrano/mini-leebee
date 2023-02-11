use std::{path::Path, sync::Arc};

use livi::event::LV2AtomSequence;

use crate::audio_buffer::AudioBuffer;

/// Describes a process failure.
#[derive(Copy, Clone, Debug)]
pub enum PluginProcessError {
    /// An LV2 run error.
    Livi(livi::error::RunError),
}

impl From<livi::error::RunError> for PluginProcessError {
    fn from(value: livi::error::RunError) -> Self {
        PluginProcessError::Livi(value)
    }
}

/// Stores a plugin instance.
#[derive(Debug)]
pub enum PluginInstance {
    /// A sample is triggered for each note.
    Sample(SampleTrigger),
    /// An LV2 plugin instance.
    Lv2(livi::Instance),
}

impl From<livi::Instance> for PluginInstance {
    fn from(value: livi::Instance) -> Self {
        PluginInstance::Lv2(value)
    }
}

impl From<SampleTrigger> for PluginInstance {
    fn from(value: SampleTrigger) -> PluginInstance {
        PluginInstance::Sample(value)
    }
}

impl PluginInstance {
    /// Run the plugin processing.
    pub fn process(
        &mut self,
        samples: usize,
        midi_input: &LV2AtomSequence,
        input: &AudioBuffer,
        output: &mut AudioBuffer,
    ) -> Result<(), PluginProcessError> {
        match self {
            PluginInstance::Sample(sample) => sample.process(midi_input, output),
            PluginInstance::Lv2(instance) => {
                let port_counts = instance.port_counts();
                let ports = livi::EmptyPortConnections::new()
                    .with_atom_sequence_inputs(
                        std::iter::once(midi_input).take(port_counts.atom_sequence_inputs),
                    )
                    .with_audio_inputs(input.iter_channels().take(port_counts.audio_inputs))
                    .with_audio_outputs(output.iter_channels_mut().take(port_counts.audio_outputs));
                unsafe {
                    instance
                        .run(samples, ports)
                        .map_err(PluginProcessError::Livi)
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct SampleTrigger {
    sample: Arc<AudioBuffer>,
    index: Option<usize>,
}

impl SampleTrigger {
    /// Create a sample trigger from an audio buffer.
    pub fn new(sample: Arc<AudioBuffer>) -> SampleTrigger {
        SampleTrigger {
            sample,
            index: None,
        }
    }

    /// Create a sample trigger from a wave path.
    pub fn from_wav(p: &Path) -> SampleTrigger {
        let sample = Arc::new(AudioBuffer::with_wav(p));
        SampleTrigger::new(sample)
    }

    /// Start triggering the sample as opposed to waiting for a midi note on event.
    pub fn start(&mut self) {
        self.index = Some(0);
    }

    /// Returns true if the sample is active or false if it has not been started or is done.
    pub fn is_active(&self) -> bool {
        self.index.is_some()
    }

    /// Processes the sample triggering.
    pub fn process(
        &mut self,
        midi_input: &LV2AtomSequence,
        output: &mut AudioBuffer,
    ) -> Result<(), PluginProcessError> {
        {
            let output = output.iter_channels_mut().next().unwrap();
            let mut midi = midi_input.iter().peekable();
            let sample = self.sample.iter_channels().next().unwrap();
            for (frame, output) in output.iter_mut().enumerate() {
                if midi
                    .peek()
                    .map(|m| m.event.time_in_frames as usize >= frame)
                    .unwrap_or(false)
                {
                    let data = midi.next().unwrap().data;
                    match wmidi::MidiMessage::try_from(data) {
                        Ok(wmidi::MidiMessage::NoteOn(_, _, v)) if u8::from(v) > 0 => {
                            self.index = Some(0)
                        }
                        _ => (),
                    }
                }
                if let Some(index) = self.index {
                    match sample.get(index) {
                        Some(s) => {
                            self.index = Some(index + 1);
                            *output = *s;
                        }
                        None => self.index = None,
                    }
                }
            }
        }
        output.copy_first_channel_to_all();
        Ok(())
    }
}
