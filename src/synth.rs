use std::f32::consts::PI;

use crossbeam::channel;
use glam::{vec2, vec4};
use wmidi::MidiMessage;
use crate::pendulum::Pendulum;

// super simple synth
// TODO make interesting

type MidiChannel = channel::Receiver<MidiMessage<'static>>;

#[derive(Clone)]
struct NoteEvent {
    note: wmidi::Note,
    pendulum: Pendulum,
}

#[derive(Clone)]
pub struct Synth {
    clock: u64,
    midi_events: MidiChannel,

    note_event: Option<NoteEvent>,
}

impl Synth {
    pub fn new(midi_events: MidiChannel) -> Self {
        Self {
            clock: 0,
            midi_events,
            note_event: None,
        }
    }
}

pub trait SynthPlayer {
    fn play(&mut self, sample_rate: u32, channels: usize, output: &mut [f32]);
}

impl SynthPlayer for Synth {
    fn play(&mut self, sample_rate: u32, channels: usize, output: &mut [f32]) {
        // pump midi messages
        for message in self.midi_events.try_iter() {
            match message {
                wmidi::MidiMessage::NoteOn(_, note, velocity) => {
                    let displacement = (u8::from(velocity) - u8::from(wmidi::U7::MIN)) as f32
                    / (u8::from(wmidi::U7::MAX) - u8::from(wmidi::U7::MIN)) as f32
                    * PI / 2. / 2.;
                    self.note_event = Some(NoteEvent {
                        note,
                        pendulum: Pendulum {
                            m: vec2(1., 1.),
                            l: 0.01 * vec2(1., 3.) / note.to_freq_f32(),
                            t_pt: vec4(displacement, displacement, 0., 0.),
                            ..Pendulum::default()
                        },
                    });
                }
                wmidi::MidiMessage::NoteOff(_, note, _) => {
                    if let Some(NoteEvent {
                        note: held_note,
                        ..
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
        if let Some(NoteEvent {
            pendulum,
            ..
        }) = &mut self.note_event
        {
            for frame in output.chunks_exact_mut(channels) {
                for sample in frame.iter_mut() {
                    // TODO try the other components
                    //*sample = (pendulum.t_pt.x / (PI / 2.)).clamp(-1., 1.);
                    *sample = (pendulum.t_pt.z / pendulum.l.y * 100.).clamp(-1., 1.);
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
