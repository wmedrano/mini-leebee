use log::*;

use crate::audio_buffer::AudioBuffer;

/// Contains JACK ports.
#[derive(Debug)]
pub struct Ports {
    audio_out: [jack::Port<jack::AudioOut>; 2],
    midi_in: jack::Port<jack::MidiIn>,
    midi_urid: lv2_raw::LV2Urid,
    midi_in_as_lv2: livi::event::LV2AtomSequence,
}

impl Ports {
    /// Create a new set of ports.
    pub fn new(client: &jack::Client, features: &livi::Features) -> Result<Ports, jack::Error> {
        Ok(Ports {
            audio_out: [
                client.register_port("audio_out_l", jack::AudioOut)?,
                client.register_port("audio_out_r", jack::AudioOut)?,
            ],
            midi_in: client.register_port("midi_in", jack::MidiIn)?,
            midi_urid: features.midi_urid(),
            midi_in_as_lv2: livi::event::LV2AtomSequence::new(features, 4096),
        })
    }

    /// Mix the contents of src into the output audio.
    pub fn mix_audio_out(&mut self, ps: &jack::ProcessScope, src: &AudioBuffer, volume: f32) {
        for (src, dst) in src.iter_channels().zip(self.audio_out.iter_mut()) {
            for (src, dst) in src.iter().zip(dst.as_mut_slice(ps).iter_mut()) {
                *dst += *src * volume;
            }
        }
        self.midi_in_as_lv2.clear();
        for raw_midi in self.midi_in.iter(ps) {
            const MAX_BYTES: usize = 4;
            if raw_midi.bytes.len() <= MAX_BYTES {
                if let Err(err) = self.midi_in_as_lv2.push_midi_event::<MAX_BYTES>(
                    raw_midi.time as i64,
                    self.midi_urid,
                    raw_midi.bytes,
                ) {
                    error!("Failed to convert midi to lv2 sequence: {:?}", err);
                }
            }
        }
    }

    /// Reset all the output ports to 0 or empty values.
    pub fn reset(&mut self, ps: &jack::ProcessScope) {
        for p in self.audio_out.iter_mut() {
            let buffer = p.as_mut_slice(ps);
            for v in buffer.iter_mut() {
                *v = 0f32;
            }
        }
    }

    /// Get the underlying lv2 atom sequence.
    pub fn lv2_atom_sequence(&self) -> &livi::event::LV2AtomSequence {
        &self.midi_in_as_lv2
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
