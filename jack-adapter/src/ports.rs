use log::*;

use audio_engine::audio_buffer::AudioBuffer;

/// Contains JACK ports.
#[derive(Debug)]
pub struct Ports {
    /// The audio outputs.
    pub audio_out: [jack::Port<jack::AudioOut>; 2],
    /// The midi input.
    pub midi_input: jack::Port<jack::MidiIn>,
}

impl Ports {
    /// Create a new set of ports.
    pub fn new(client: &jack::Client) -> Result<Ports, jack::Error> {
        Ok(Ports {
            audio_out: [
                client.register_port("audio_out_l", jack::AudioOut)?,
                client.register_port("audio_out_r", jack::AudioOut)?,
            ],
            midi_input: client.register_port("midi_in", jack::MidiIn)?,
        })
    }

    /// Mix the contents of src into the output audio.
    pub fn copy_audio_out(&mut self, ps: &jack::ProcessScope, src: &AudioBuffer) {
        for (src, dst) in src.iter_channels().zip(self.audio_out.iter_mut()) {
            for (src, dst) in src.iter().zip(dst.as_mut_slice(ps).iter_mut()) {
                *dst = *src;
            }
        }
    }

    /// Automatically connect the ports to physical ports.
    pub fn auto_connect_fn(&self) -> Box<dyn Send + Sync + Fn(&jack::Client)> {
        let audio_outputs: Vec<_> = self
            .audio_out
            .iter()
            .map(|port| port.name().unwrap())
            .collect();
        let midi_input = self.midi_input.name().unwrap();
        Box::new(move |client: &jack::Client| {
            let srcs = audio_outputs.iter();
            let dsts = client.ports(
                None,
                Some(jack::jack_sys::FLOAT_MONO_AUDIO),
                jack::PortFlags::IS_PHYSICAL | jack::PortFlags::IS_INPUT,
            );
            for (src, dst) in srcs.zip(dsts) {
                match client.connect_ports_by_name(src, &dst) {
                    Ok(()) => info!("Connected audio port {} to {}.", src, dst),
                    Err(err) => warn!("Failed to connect audio port {} to {}: {:?}", src, dst, err),
                };
            }

            let srcs = client.ports(
                None,
                Some(jack::jack_sys::RAW_MIDI_TYPE),
                jack::PortFlags::IS_PHYSICAL | jack::PortFlags::IS_OUTPUT,
            );
            let dsts = std::iter::once(&midi_input).cycle();
            for (src, dst) in srcs.iter().zip(dsts) {
                match client.connect_ports_by_name(src, dst) {
                    Ok(()) => info!("Connected midi port {} to {}.", src, dst),
                    Err(err) => warn!("Failed to connect midi port {} to {}: {:?}", src, dst, err),
                };
            }
        })
    }
}
