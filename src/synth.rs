use std::{f32::consts::PI, sync::Arc};

use crate::pendulum::Pendulum;
use crossbeam::{atomic::AtomicCell, channel};
use glam::{vec2, vec4};
use wmidi::MidiMessage;

// super simple synth
// TODO make interesting

type MidiChannel = channel::Receiver<MidiMessage<'static>>;

#[derive(Clone)]
struct NoteEvent {
    note: wmidi::Note,
    pendulum: Pendulum,
}

// TODO handle params using messages instead?
pub struct Params {
    pub distorsion: AtomicCell<f32>,
    pub chaoticity: AtomicCell<f32>,
}

#[derive(Clone)]
pub struct Synth {
    clock: u64,
    midi_events: MidiChannel,

    note_event: Option<NoteEvent>,
    params: Arc<Params>,
}

impl Synth {
    pub fn new(midi_events: MidiChannel) -> Self {
        Self {
            clock: 0,
            midi_events,
            note_event: None,
            params: Arc::new(Params {
                distorsion: 1f32.into(),
                chaoticity: 0.1f32.into(),
            }),
        }
    }

    pub fn get_params(&self) -> Arc<Params> {
        self.params.clone()
    }
}

pub trait SynthPlayer {
    fn play(&mut self, sample_rate: u32, channels: usize, output: &mut [f32]);
}

impl SynthPlayer for Synth {
    fn play(&mut self, sample_rate: u32, channels: usize, output: &mut [f32]) {
        let chaoticity = self.params.chaoticity.load().clamp(0f32, 1f32);
        // pump midi messages
        for message in self.midi_events.try_iter() {
            match message {
                wmidi::MidiMessage::NoteOn(_, note, velocity) => {
                    let displacement = (u8::from(velocity) - u8::from(wmidi::U7::MIN)) as f32
                        / (u8::from(wmidi::U7::MAX) - u8::from(wmidi::U7::MIN)) as f32
                        * PI
                        / 2.
                        / 2.;
                    let g = 9.81f32;
                    // TODO calculate length better. do a few components of the large amplitude equation
                    let length = (1f32 / note.to_freq_f32() / 2f32 / PI).powi(2) * g;
                    let m = vec2(1., 1.);
                    let cm = (m.x - m.y) / m.y;
                    let b = length * (1f32 - chaoticity) / (1f32 + chaoticity * (cm - 1f32));
                    let c = chaoticity * b / (1f32 - chaoticity);
                    let lengths = vec2(b, c);
                    self.note_event = Some(NoteEvent {
                        note,
                        pendulum: Pendulum {
                            m: vec2(1., 1.),
                            l: lengths,
                            t_pt: vec4(displacement, displacement, 0., 0.),
                            g,
                            ..Pendulum::default()
                        },
                    });
                }
                wmidi::MidiMessage::NoteOff(_, note, _) => {
                    if let Some(NoteEvent {
                        note: held_note, ..
                    }) = self.note_event
                    {
                        if note == held_note {
                            // TODO increase friction
                            self.note_event = None;
                        }
                    }
                }
                _ => {}
            }
        }

        // produce sound
        if let Some(NoteEvent { pendulum, .. }) = &mut self.note_event {
            let distorsion = self.params.distorsion.load();
            for frame in output.chunks_exact_mut(channels) {
                // TODO try the other components
                let a = pendulum.t_pt.z / pendulum.l.y * 100.;
                let clipped = 2. / std::f32::consts::PI * f32::atan(distorsion * a);
                for sample in frame.iter_mut() {
                    *sample = clipped;
                }
                pendulum.update(1. / sample_rate as f32);
                self.clock += 1;
            }
        } else {
            output.fill(0f32);
            self.clock += (output.len() / channels) as u64;
        }
    }
}

#[cfg(test)]
mod test {
    use super::{Synth, SynthPlayer};
    use crossbeam::channel;

    #[test]
    fn silence() {
        let (_tx, rx) = channel::bounded(1);
        let mut synth = Synth::new(rx);
        let mut data = [0f32; 512];
        synth.play(48000, 2, &mut data);
        assert_eq!([0f32; 512], data);
    }
}
