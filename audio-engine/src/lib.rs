use log::*;
use ports::Ports;

pub struct AudioEngine {
    client: jack::AsyncClient<(), ()>,
    ports: Ports,
}

impl AudioEngine {
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
        ports.auto_connect(client.as_client());
        Ok(AudioEngine {
            client,
            ports: ports,
        })
    }

    pub fn buffer_size(&self) -> usize {
        self.client.as_client().buffer_size() as usize
    }

    pub fn sample_rate(&self) -> usize {
        self.client.as_client().sample_rate() as usize
    }
}

pub mod ports;
