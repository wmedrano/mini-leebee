use log::*;

/// Contains JACK ports.
pub struct Ports {
    audio_out: [jack::Port<jack::AudioOut>; 2],
    midi_in: jack::Port<jack::MidiIn>,
}

impl Ports {
    /// Create a new set of ports.
    pub fn new(client: &jack::Client) -> Result<Ports, jack::Error> {
        Ok(Ports {
            audio_out: [
                client.register_port("audio_out_l", jack::AudioOut)?,
                client.register_port("audio_out_r", jack::AudioOut)?,
            ],
            midi_in: client.register_port("midi_in", jack::MidiIn)?,
        })
    }

    /// Reset all the output ports to 0 or empty values.
    pub fn reset_outputs(&mut self, ps: &jack::ProcessScope) {
        for p in self.audio_out.iter_mut() {
            let buffer = p.as_mut_slice(ps);
            for v in buffer.iter_mut() {
                *v = 0f32;
            }
        }
    }

    /// Automatically connect the ports to physical ports.
    pub fn auto_connect_fn(&self) -> Box<dyn Fn(&jack::Client)> {
        let audio_outputs: Vec<_> = self
            .audio_out
            .iter()
            .map(|port| port.name().unwrap())
            .collect();
        let midi_input = self.midi_in.name().unwrap();
        Box::new(move |client: &jack::Client| {
            let srcs = audio_outputs.iter();
            let dsts = client.ports(
                None,
                Some(jack::jack_sys::FLOAT_MONO_AUDIO),
                jack::PortFlags::IS_PHYSICAL | jack::PortFlags::IS_INPUT,
            );
            for (src, dst) in srcs.zip(dsts) {
                match client.connect_ports_by_name(&src, &dst) {
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
                match client.connect_ports_by_name(src, &dst) {
                    Ok(()) => info!("Connected midi port {} to {}.", src, dst),
                    Err(err) => warn!("Failed to connect midi port {} to {}: {:?}", src, dst, err),
                };
            }
        })
    }
}
