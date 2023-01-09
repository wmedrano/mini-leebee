use audio_engine::Communicator;
use log::*;
use ports::Ports;
use processor::Processor;

pub mod ports;
pub mod processor;

/// Manages audio and midi processing.
pub struct JackAdapter {
    pub audio_engine: Communicator,
    /// The underlying JACK client.
    client: jack::AsyncClient<(), Processor>,
    /// The function to call to automatically connect ports.
    auto_connect_fn: Box<dyn Fn(&jack::Client)>,
}

impl JackAdapter {
    /// Create a new audio engine.
    pub fn new() -> Result<JackAdapter, jack::Error> {
        let (client, status) =
            jack::Client::new("mini-leebee", jack::ClientOptions::NO_START_SERVER)?;
        info!(
            "Created JACK client {} with status {:?}.",
            client.name(),
            status
        );
        // JACK on pipewire may sometimes increase the buffer size. To combat
        // this, we artifically increase the buffer size.
        let buffer_size = client.buffer_size() as usize * 4;
        let ports = Ports::new(&client)?;
        let auto_connect_fn = ports.auto_connect_fn();
        let (processor, communicator) = Processor::new(ports, buffer_size);
        let client = client.activate_async((), processor)?;
        Ok(JackAdapter {
            audio_engine: communicator,
            client,
            auto_connect_fn,
        })
    }

    /// Automatically connect io ports.
    pub fn auto_connect(&self) {
        (self.auto_connect_fn)(self.client.as_client());
    }

    /// Get the buffer size.
    pub fn buffer_size(&self) -> usize {
        self.client.as_client().buffer_size() as usize
    }

    /// Get the sample rate.
    pub fn sample_rate(&self) -> f64 {
        self.client.as_client().sample_rate() as f64
    }
}
