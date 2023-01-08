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

    /// Automatically connect the ports to physical ports.
    pub fn auto_connect(&self, client: &jack::Client) {
        let srcs = self.audio_out.iter();
        let dsts = client.ports(
            None,
            Some(jack::jack_sys::FLOAT_MONO_AUDIO),
            jack::PortFlags::IS_PHYSICAL | jack::PortFlags::IS_INPUT,
        );
        for (src, dst) in srcs.zip(dsts) {
            let src = match src.name() {
                Ok(n) => n,
                Err(err) => {
                    warn!("Failed to get name for audio port: {:?}.", err);
                    continue;
                }
            };
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
        let dsts = std::iter::once(&self.midi_in).cycle();
        for (src, dst) in srcs.iter().zip(dsts) {
            let dst = match dst.name() {
                Ok(n) => n,
                Err(err) => {
                    warn!("Failed to get name for midi port: {:?}.", err);
                    continue;
                }
            };
            match client.connect_ports_by_name(src, &dst) {
                Ok(()) => info!("Connected midi port {} to {}.", src, dst),
                Err(err) => warn!("Failed to connect midi port {} to {}: {:?}", src, dst, err),
            };
        }
    }
}
