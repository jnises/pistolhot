mod pendulum;
use crossbeam::{atomic::AtomicCell, channel};
use glam::{vec2, vec4, Vec2, Vec4};
use pendulum::Pendulum;
use std::{f32::consts::PI, sync::Arc};
use wmidi::MidiMessage;

pub type MidiChannel = channel::Receiver<MidiMessage<'static>>;

#[derive(Clone)]
struct NoteEvent {
    note: wmidi::Note,
}

// TODO handle params using messages instead?
pub struct Params {
    // TODO remove this
    pub distortion: AtomicCell<f32>,
    pub chaoticity: AtomicCell<f32>,
}

#[derive(Clone)]
pub struct Synth {
    midi_events: MidiChannel,

    pendulum: Pendulum,
    note_event: Option<NoteEvent>,
    params: Arc<Params>,
    lowpass: f32,
}

impl Synth {
    pub fn new(midi_events: MidiChannel) -> Self {
        Self {
            midi_events,
            note_event: None,
            params: Arc::new(Params {
                distortion: 2f32.into(),
                chaoticity: 0.67f32.into(),
            }),
            lowpass: 0f32,
            pendulum: Pendulum{
                // higher gravity. for better precision
                g: 9.81f32 * 100000.,
                mass: vec2(1., 1.),
                ..Pendulum::default()
            },
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
                        // / 2.
                        / 2.;
                    let g = self.pendulum.g;
                    // TODO calculate length better. do a few components of the large amplitude equation
                    let length = (1f32 / note.to_freq_f32() / 2f32 / PI).powi(2) * g;
                    let m = vec2(1., 1.);
                    let cm = (m.x - m.y) / m.y;
                    let b = length * (1f32 - chaoticity) / (1f32 + chaoticity * (cm - 1f32));
                    let c = chaoticity * b / (1f32 - chaoticity);
                    let length = vec2(b, c);
                    //dbg!(length);
                    self.pendulum.length = length;
                    self.pendulum.t_pt = vec4(displacement, displacement, 0., 0.);
                }
                wmidi::MidiMessage::NoteOff(_, note, _) => {
                    if let Some(NoteEvent {
                        note: held_note, ..
                    }) = self.note_event
                    {
                        if note == held_note {
                            // TODO increase friction
                            self.pendulum.t_pt = Vec4::ZERO;
                            self.pendulum.d_t_pt = Vec4::ZERO;
                            self.note_event = None;
                        }
                    }
                }
                _ => {}
            }
        }

        // produce sound
        // TODO do a better hipass
        let cutoff = 0.000001f32;
        let pendulum = &mut self.pendulum;
        for frame in output.chunks_exact_mut(channels) {
            // TODO try the other components
            //let a = pendulum.t_pt.z / pendulum.length.y.max(0.000001f32) * 100.;
            //let a = pendulum.t_pt.x + pendulum.t_pt.y;
            //let a = pendulum.t_pt.x;// - pendulum.t_pt.y;
            //let a = pendulum.t_pt.y;
            //let a = pendulum.t_pt.z * 100000000.;
            // let a = pendulum.t_pt.w * 100000000.;
            let tip = Vec2::from(pendulum.t_pt.x.sin_cos()) * pendulum.length.x
                + Vec2::from(pendulum.t_pt.y.sin_cos()) * pendulum.length.y;
            //let a = f32::atan2(tip.x, tip.y);
            //dbg!(tip);
            //dbg!(pendulum.length);
            let full_length = pendulum.length.x + pendulum.length.y;
            //let a = (tip.length() / full_length) * 2. - 1.;
            let a = tip.x / full_length;
            //dbg!(a);
            self.lowpass = a * cutoff + (1f32 - cutoff) * self.lowpass;
            let hipass_a = a - self.lowpass;
            //let clipped = 2. / std::f32::consts::PI * f32::atan(distorsion * hipass_a);
            let clipped = hipass_a.clamp(-1f32, 1f32);
            for sample in frame.iter_mut() {
                *sample = clipped;
            }
            pendulum.update(1. / sample_rate as f32);
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
