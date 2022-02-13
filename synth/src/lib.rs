mod params_gui;
mod pendulum;
use biquad::{Biquad, ToHertz};
use crossbeam::{atomic::AtomicCell, channel};
use glam::{vec2, vec4, Vec2, Vec4, Vec4Swizzles};
pub use params_gui::params_gui;
use pendulum::Pendulum;
use std::{f32::consts::PI, ops::RangeInclusive, sync::Arc};
use wmidi::MidiMessage;

pub type MidiChannel = channel::Receiver<MidiMessage<'static>>;

#[derive(Clone)]
struct NoteEvent {
    note: wmidi::Note,
}

// TODO handle params using messages instead?
pub struct Params {
    pub chaoticity: AtomicCell<f32>,
}

pub const CHAOTICITY_RANGE: RangeInclusive<f32> = 0.1f32..=1f32;

const LOWPASS_FREQ: f32 = 10000f32;

#[derive(Clone)]
pub struct Synth {
    midi_events: MidiChannel,

    pendulum: Pendulum,
    note_event: Option<NoteEvent>,
    params: Arc<Params>,
    //lowpass: f32,
    lowpass: Option<(u32, biquad::DirectForm1<f32>)>,
    center_length: f32,
}

impl Synth {
    pub fn new(midi_events: MidiChannel) -> Self {
        Self {
            midi_events,
            note_event: None,
            params: Arc::new(Params {
                chaoticity: 0.67f32.into(),
            }),
            lowpass: None,
            pendulum: Pendulum {
                // higher gravity. for better precision
                g: 9.81f32 * 100000.,
                mass: vec2(1., 1.),
                ..Pendulum::default()
            },
            center_length: 1f32,
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
        let chaoticity = self
            .params
            .chaoticity
            .load()
            .clamp(*CHAOTICITY_RANGE.start(), *CHAOTICITY_RANGE.end());
        // pump midi messages
        for message in self.midi_events.try_iter() {
            match message {
                wmidi::MidiMessage::NoteOn(_, note, velocity) => {
                    let norm_vel = (u8::from(velocity) - u8::from(wmidi::U7::MIN)) as f32
                        / (u8::from(wmidi::U7::MAX) - u8::from(wmidi::U7::MIN)) as f32;
                    let displacement = norm_vel * PI
                        // / 2.
                        / 2.;
                    let g = self.pendulum.g;
                    // TODO calculate length better. do a few components of the large amplitude equation
                    self.center_length = (1f32 / note.to_freq_f32() / 2f32 / PI).powi(2) * g;
                    // TODO set momenta instead?
                    //self.pendulum.t_pt = vec4(displacement, displacement, 0., 0.);
                    // TOOD these should depend on t_pt.xy somehow
                    self.pendulum.t_pt.z = norm_vel * 10f32;
                    self.pendulum.t_pt.w = norm_vel * 10f32;

                    // self.pendulum.t_pt.z = displacement * g;
                    // self.pendulum.t_pt.w = displacement * g;
                    //self.pendulum.d_t_pt = Vec4::ZERO;
                    //self.pendulum.d_t_pt = vec4(displacement, displacement, 0., 0.) / 44100f32;
                    //self.pendulum.d_t_pt = vec4(1., 1., 1., 1.) * 0.0001;//1., 1.);
                    self.note_event = Some(NoteEvent { note });
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

        match self.lowpass {
            Some((srate, _)) if srate == sample_rate => {}
            _ => {
                self.lowpass = Some((
                    sample_rate,
                    biquad::DirectForm1::<f32>::new(
                        biquad::Coefficients::<f32>::from_params(
                            // TODO use singlepole instead?
                            biquad::Type::LowPass,
                            //biquad::Type::SinglePoleLowPass,
                            sample_rate.hz(),
                            LOWPASS_FREQ.min(sample_rate as f32 / 2.001f32).hz(),
                            biquad::Q_BUTTERWORTH_F32,
                        )
                        .unwrap(),
                    ),
                ));
            }
        }
        let lowpass = &mut self.lowpass.as_mut().unwrap().1;

        // TODO m?? is that mass?
        //let m = vec2(1., 1.);
        //let cm = (m.x - m.y) / m.y;
        // TODO make the lengths the same, and change the mass instead?
        // TODO is it perhaps only the first length that should be used to calculate the center of mass?
        // TODO figure this out
        let b = self.center_length / (1f32 + chaoticity / 2f32);
        let c = b * chaoticity;
        //let b = self.full_length * (1f32 - chaoticity) / (1f32 + chaoticity * (cm - 1f32));
        //let c = chaoticity * b / (1f32 - chaoticity);
        let length = vec2(b, c);
        //dbg!(length);
        self.pendulum.length = length;
        // TODO recalculate the momenta depending on the chaoticity?

        // produce sound
        // TODO do a better lowpass
        //let cutoff = 0.1f32;
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
            let lowpassed = lowpass.run(a);
            //self.lowpass = a * cutoff + (1f32 - cutoff) * self.lowpass;
            //let hipass_a = a - self.lowpass;
            //let clipped = 2. / std::f32::consts::PI * f32::atan(distorsion * hipass_a);
            //let clipped = hipass_a.clamp(-1f32, 1f32);
            let clipped = lowpassed.clamp(-1f32, 1f32);
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
