use crate::ports::Ports;

/// Implements the `jack::ProcessHandler` trait.
pub struct Processor {
    /// The ports to read and write to.
    ports: Ports,
}

impl Processor {
    /// Create a new processor.
    pub fn new(ports: Ports) -> Processor {
        Processor { ports }
    }
}

impl jack::ProcessHandler for Processor {
    fn process(&mut self, _: &jack::Client, ps: &jack::ProcessScope) -> jack::Control {
        self.ports.reset_outputs(ps);
        jack::Control::Continue
    }
}
