use std::path::Path;

use livi::event::LV2AtomSequence;

use crate::{audio_buffer::AudioBuffer, plugin::SampleTrigger, track::Track};

/// Produces metrenome ticks and timing information.
#[derive(Debug)]
pub struct Metrenome {
    track: Track,
    midi_urid: lv2_raw::LV2Urid,
    events: LV2AtomSequence,
    current_time_info: SampleTimeInfo,
    time_info: Vec<SampleTimeInfo>,
    beats_per_sample: f64,
}

/// Contains information for the timing of a frame.
#[derive(Copy, Clone, Debug)]
pub struct SampleTimeInfo {
    /// The beat.
    pub beat: u32,
    /// The time after a bit. Between [0.0, 1.0).
    pub sub_beat: f64,
}

impl Metrenome {
    const NOTE: wmidi::MidiMessage<'static> =
        wmidi::MidiMessage::NoteOn(wmidi::Channel::Ch1, wmidi::Note::A0, wmidi::U7::MAX);

    /// Create a new metrenome.
    pub fn new(sample_rate: f64, features: &livi::Features) -> Metrenome {
        let mut track = Track::new(-1, features.max_block_length());
        track.push_plugin(SampleTrigger::from_wav(Path::new("resources/click.wav")).into());
        let events = LV2AtomSequence::new(features, 1024 /*1 KiB*/);
        let bpm = 120.0;
        let beats_per_sample = bpm_to_beats_per_sample(sample_rate, bpm);
        Metrenome {
            track,
            midi_urid: features.midi_urid(),
            events,
            current_time_info: SampleTimeInfo {
                beat: 0,
                sub_beat: -beats_per_sample,
            },
            time_info: Vec::with_capacity(features.max_block_length()),
            beats_per_sample,
        }
    }

    /// Process the metrenome for the given number of samples.
    pub fn process(&mut self, samples: usize) -> (&AudioBuffer, &[SampleTimeInfo]) {
        self.time_info.clear();
        self.events.clear();
        for frame in 0..samples {
            self.current_time_info.sub_beat += self.beats_per_sample;
            if self.current_time_info.sub_beat >= 1.0 {
                self.current_time_info.beat += 1;
                self.current_time_info.sub_beat -= 1.0;
                let mut data = [0u8; 3];
                Metrenome::NOTE.copy_to_slice(&mut data).unwrap();
                self.events
                    .push_midi_event::<3>(frame as i64, self.midi_urid, &data)
                    .unwrap();
            }
            self.time_info.push(self.current_time_info);
        }
        let audio_out = self.track.process(samples, &self.events);
        (audio_out, &self.time_info)
    }
}

fn bpm_to_beats_per_sample(sample_rate: f64, bpm: f64) -> f64 {
    let beats_per_minute = bpm;
    let minutes_per_second = 1.0 / 60.0;
    let seconds_per_sample = 1.0 / sample_rate;
    beats_per_minute * minutes_per_second * seconds_per_sample
}
