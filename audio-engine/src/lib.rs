use log::*;
use ports::Ports;

pub mod ports;

/// Manages audio and midi processing.
pub struct AudioEngine {
    /// The underlying JACK client.
    client: jack::AsyncClient<(), ()>,
    /// The JACK ports.
    ports: Ports,
}

impl AudioEngine {
    /// Create a new audio engine.
    pub fn new() -> Result<AudioEngine, jack::Error> {
        let (client, status) =
            jack::Client::new("mini-leebee", jack::ClientOptions::NO_START_SERVER)?;
        info!(
            "Created JACK client {} with status {:?}.",
            client.name(),
            status
        );
        let ports = Ports::new(&client)?;
        let client = client.activate_async((), ())?;
        Ok(AudioEngine { client, ports })
    }

    /// Automatically connect io ports.
    pub fn auto_connect(&self) {
        self.ports.auto_connect(self.client.as_client());
    }

    /// Get the buffer size.
    pub fn buffer_size(&self) -> usize {
        self.client.as_client().buffer_size() as usize
    }

    /// Get the sample rate.
    pub fn sample_rate(&self) -> usize {
        self.client.as_client().sample_rate()
    }
}
