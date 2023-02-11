/// Contains audio data for several channels.
pub struct AudioBuffer {
    buffer: Vec<f32>,
    buffer_size: usize,
}

impl AudioBuffer {
    /// Creates a new audio buffer with the given number of channels.
    pub fn new(channels: usize, buffer_size: usize) -> AudioBuffer {
        AudioBuffer {
            buffer: vec![0f32; channels * buffer_size],
            buffer_size,
        }
    }

    /// Create a new audio buffer with 2 channels and the given buffer size.
    pub fn with_stereo(buffer_size: usize) -> AudioBuffer {
        AudioBuffer::new(2, buffer_size)
    }

    /// Create a new audio buffer from a wave file.
    pub fn with_wav(p: &std::path::Path) -> AudioBuffer {
        let reader = hound::WavReader::open(p).unwrap();
        let specs = reader.spec();
        if specs.channels != 1 {
            unimplemented!(
                "Only a single channel is supported bug {p:?} contains {} channels.",
                specs.channels
            );
        }
        if specs.bits_per_sample != 16 {
            unimplemented!(
                "Only 16 bits per sample supported for wav but {p:?} contains {} bits per channel.",
                specs.bits_per_sample
            );
        }
        let buffer: Vec<f32> = reader
            .into_samples()
            .map(Result::unwrap)
            .map(|s: i16| s as f64 / i16::MAX as f64)
            .map(|s| s as f32)
            .collect();
        let buffer_size = buffer.len();
        AudioBuffer {
            buffer,
            buffer_size,
        }
    }

    /// Returns the number of channels.
    pub fn channels(&self) -> usize {
        self.buffer.len() / self.buffer_size
    }

    /// Sets all the values to 0.
    pub fn reset(&mut self) {
        for v in self.buffer.iter_mut() {
            *v = 0f32;
        }
    }

    /// Resizes by `buffer_size` and rests the values.
    pub fn reset_with_buffer_size(&mut self, buffer_size: usize) {
        if buffer_size != self.buffer_size {
            self.buffer_size = buffer_size;
            let desired_len = buffer_size * self.channels();
            self.buffer.resize(desired_len, 0.0);
        }
        self.reset();
    }

    /// Mixes the buffers from `src` onto `self`.
    pub fn mix_from(&mut self, src: &AudioBuffer, volume: f32) {
        for (src, dst) in src.iter_channels().zip(self.iter_channels_mut()) {
            for (src, dst) in src.iter().zip(dst.iter_mut()) {
                *dst += *src * volume;
            }
        }
    }

    /// Iterate over all the channels.
    pub fn iter_channels(&self) -> impl ExactSizeIterator + Iterator<Item = &[f32]> {
        self.buffer.chunks_exact(self.buffer_size)
    }

    /// Iterate over all the channels mutably.
    pub fn iter_channels_mut(&mut self) -> impl ExactSizeIterator + Iterator<Item = &mut [f32]> {
        self.buffer.chunks_exact_mut(self.buffer_size)
    }

    /// Copies the data from the first channel to all other channels.
    pub fn copy_first_channel_to_all(&mut self) {
        let (src, dsts) = self.buffer.split_at_mut(self.buffer_size);
        for dst in dsts.chunks_exact_mut(self.buffer_size) {
            dst.copy_from_slice(src);
        }
    }
}

impl std::fmt::Debug for AudioBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let channels = self.buffer.len() / self.buffer_size;
        f.debug_struct("AudioBuffer")
            .field("channels", &channels)
            .field("buffer_size", &self.buffer_size)
            .finish()
    }
}
