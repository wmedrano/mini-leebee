use std::sync::{mpsc::SyncSender, Arc};

use commands::Command;
use log::*;
use ports::Ports;
use processor::Processor;

pub mod audio_buffer;
pub mod commands;
pub mod ports;
pub mod processor;
pub mod track;

/// Manages audio and midi processing.
pub struct AudioEngine {
    /// A channel to send commands to the main processing.
    pub commands: SyncSender<Command>,

    /// Object for managing lv2 plugins.
    livi: Arc<livi::World>,
    /// Object for managing lv2 features.
    lv2_features: Arc<livi::Features>,
    /// The underlying JACK client.
    client: jack::AsyncClient<(), Processor>,
    /// The function to call to automatically connect ports.
    auto_connect_fn: Box<dyn Fn(&jack::Client)>,
}

impl AudioEngine {
    /// Create a new audio engine.
    pub fn new() -> Result<AudioEngine, jack::Error> {
        let livi = Arc::new(livi::World::new());
        let (client, status) =
            jack::Client::new("mini-leebee", jack::ClientOptions::NO_START_SERVER)?;
        info!(
            "Created JACK client {} with status {:?}.",
            client.name(),
            status
        );
        let features_builder = livi::FeaturesBuilder {
            min_block_length: 1,
            max_block_length: client.buffer_size() as usize * 4,
        };
        let lv2_features = features_builder.build(&livi);
        let ports = Ports::new(&client, &lv2_features)?;
        let auto_connect_fn = ports.auto_connect_fn();
        let (processor, commands) = Processor::new(ports);
        let client = client.activate_async((), processor)?;
        Ok(AudioEngine {
            commands,
            livi,
            lv2_features,
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

    /// Return object for managing lv2 plugins.
    pub fn livi(&self) -> Arc<livi::World> {
        self.livi.clone()
    }

    /// Return object for lv2 features.
    pub fn lv2_features(&self) -> Arc<livi::Features> {
        self.lv2_features.clone()
    }
}
