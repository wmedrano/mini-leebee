use std::sync::mpsc::{Receiver, SyncSender};

use crate::{commands::Command, ports::Ports, track::Track};

/// Implements the `jack::ProcessHandler` trait.
#[derive(Debug)]
pub struct Processor {
    /// The ports to read and write to.
    ports: Ports,
    /// The tracks to process.
    tracks: Vec<Track>,
    /// A channel to receive commands from.
    commands: Receiver<Command>,
}

impl Processor {
    /// Create a new processor.
    pub fn new(ports: Ports) -> (Processor, SyncSender<Command>) {
        let tracks = Vec::with_capacity(32);
        let (commands_tx, commands_rx) = std::sync::mpsc::sync_channel(1024);
        let processor = Processor {
            ports,
            tracks,
            commands: commands_rx,
        };
        (processor, commands_tx)
    }

    fn handle_commands(&mut self) {
        for cmd in self.commands.try_iter() {
            match cmd {
                Command::AddTrack(track) => self.tracks.push(track),
            }
        }
    }
}

impl jack::ProcessHandler for Processor {
    fn process(&mut self, _: &jack::Client, ps: &jack::ProcessScope) -> jack::Control {
        self.handle_commands();
        let samples = ps.n_frames() as usize;
        self.ports.reset(ps);
        for track in self.tracks.iter_mut() {
            let volume = track.properties.volume;
            let output = track.process(samples, self.ports.lv2_atom_sequence());
            self.ports.mix_audio_out(ps, output, volume);
        }
        jack::Control::Continue
    }
}
