use log::*;

/// Controls and information for transport.
#[derive(Debug)]
pub struct Transport {
    beats_per_sample: f64,
    beats: f64,
    beats_buffer: Vec<u32>,
}

impl Transport {
    pub fn new(sample_rate: f64) -> Transport {
        let beats_per_minute = 120.0;
        let minutes_per_second = 1.0 / 60.0;
        let samples_per_second = sample_rate;
        let seconds_per_sample = 1.0 / samples_per_second;
        let beats_per_sample = beats_per_minute * minutes_per_second * seconds_per_sample;
        error!("{}", beats_per_minute * minutes_per_second);
        Transport {
            beats_per_sample,
            beats: 0.0,
            beats_buffer: Vec::with_capacity(128),
        }
    }

    pub fn compute_beats(&mut self, samples: usize) -> &[u32] {
        self.beats_buffer.clear();
        for i in 0..samples as u32 {
            self.beats += self.beats_per_sample;
            while self.beats >= 1.0 {
                self.beats -= 1.0;
                self.beats_buffer.push(i);
            }
        }
        &self.beats_buffer
    }
}
