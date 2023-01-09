use std::sync::{
    mpsc::{Receiver, SyncSender},
    Arc,
};

use audio_buffer::AudioBuffer;
use commands::Command;
use livi::event::LV2AtomSequence;
use log::*;
use track::Track;

pub mod audio_buffer;
pub mod commands;
pub mod track;

/// Manages audio and midi processing.
pub struct Communicator {
    /// A channel to send commands to the main processing.
    pub commands: SyncSender<Command>,
    /// Object for managing lv2 plugins.
    pub livi: Arc<livi::World>,
    /// Object for managing lv2 features.
    pub lv2_features: Arc<livi::Features>,
}

/// Implements the `jack::ProcessHandler` trait.
#[derive(Debug)]
pub struct Processor {
    /// The tracks to process.
    tracks: Vec<Track>,
    /// URID for midi.
    midi_urid: lv2_raw::LV2Urid,
    /// Buffer for midi input.
    midi_input: LV2AtomSequence,
    /// Buffer to write output to.
    audio_out: AudioBuffer,
    /// A channel to receive commands from.
    commands: Receiver<Command>,
}

impl Processor {
    /// Create a new processor.
    pub fn new(buffer_size: usize) -> (Processor, Communicator) {
        let (commands_tx, commands_rx) = std::sync::mpsc::sync_channel(1024);
        let livi = Arc::new(livi::World::new());
        let lv2_features = livi::FeaturesBuilder {
            min_block_length: 1,
            max_block_length: buffer_size,
        }
        .build(&livi);
        let processor = Processor {
            tracks: Vec::with_capacity(32),
            midi_urid: lv2_features.midi_urid(),
            midi_input: LV2AtomSequence::new(&lv2_features, 1024 * 1024 /*1 MiB*/),
            audio_out: AudioBuffer::with_stereo(buffer_size),
            commands: commands_rx,
        };
        let communicator = Communicator {
            commands: commands_tx,
            livi,
            lv2_features,
        };
        (processor, communicator)
    }

    /// Do processing and return the results in an audio buffer.
    pub fn process<'a, I>(&mut self, samples: usize, input_midi: I) -> &AudioBuffer
    where
        I: Iterator<Item = (u32, &'a [u8])>,
    {
        self.handle_commands();
        self.reset_midi_input(input_midi);
        self.audio_out.reset_with_buffer_size(samples);
        for track in self.tracks.iter_mut() {
            if track.properties.disabled {
                continue;
            }
            let volume = track.properties.volume;
            let output = track.process(samples, &self.midi_input);
            self.audio_out.mix_from(output, volume);
        }
        &self.audio_out
    }

    /// Handle all commands in `self.commands`.
    fn handle_commands(&mut self) {
        for cmd in self.commands.try_iter() {
            match cmd {
                Command::AddTrack(track) => self.tracks.push(track),
            }
        }
    }

    /// Reset the midi input with the contents of `midi_input.`
    fn reset_midi_input<'a, I>(&mut self, midi_input: I)
    where
        I: Iterator<Item = (u32, &'a [u8])>,
    {
        self.midi_input.clear();
        for (frame, data) in midi_input {
            if let Err(err) =
                self.midi_input
                    .push_midi_event::<4>(frame as i64, self.midi_urid, data)
            {
                warn!("Dropping midi message: {:?}", err);
            };
        }
    }
}
