use std::path::Path;

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

#[derive(Debug)]
pub struct SampleTrigger {
    sample: Vec<f32>,
    index: Option<usize>,
}

impl SampleTrigger {
    pub fn new(sample: Vec<f32>) -> SampleTrigger {
        SampleTrigger {
            sample,
            index: None,
        }
    }

    pub fn from_wav(p: &Path) -> SampleTrigger {
        let reader = hound::WavReader::open(p).unwrap();
        let specs = reader.spec();
        assert_eq!(specs.channels, 1);
        assert_eq!(specs.bits_per_sample, 16);
        let sample = reader
            .into_samples()
            .map(Result::unwrap)
            .map(|s: i16| s as f64 / i16::MAX as f64)
            .map(|s| s as f32)
            .collect();
        SampleTrigger::new(sample)
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
                    match self.sample.get(index) {
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
