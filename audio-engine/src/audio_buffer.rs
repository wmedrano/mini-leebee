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

    /// Sets all the values to 0.
    pub fn reset(&mut self) {
        for v in self.buffer.iter_mut() {
            *v = 0f32;
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
