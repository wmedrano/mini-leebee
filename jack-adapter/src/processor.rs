use audio_engine::Communicator;

use crate::ports::Ports;

/// Implements the `jack::ProcessHandler` trait.
#[derive(Debug)]
pub struct Processor {
    /// The undelrying processor.
    inner: audio_engine::Processor,
    /// The ports to read and write to.
    ports: Ports,
}

impl Processor {
    /// Create a new processor.
    pub fn new(ports: Ports, buffer_size: usize) -> (Processor, Communicator) {
        let (inner, communicator) = audio_engine::Processor::new(buffer_size);
        let processor = Processor { inner, ports };
        (processor, communicator)
    }
}

impl jack::ProcessHandler for Processor {
    fn process(&mut self, _: &jack::Client, ps: &jack::ProcessScope) -> jack::Control {
        let samples = ps.n_frames() as usize;
        self.ports.copy_audio_out(
            ps,
            self.inner.process(
                samples,
                self.ports
                    .midi_input
                    .iter(ps)
                    .map(|raw| (raw.time, raw.bytes)),
            ),
        );
        jack::Control::Continue
    }
}
