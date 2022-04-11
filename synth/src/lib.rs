/*
TODO how to determine float precision?
TODO change G to improve precision?
TODO change friction to instead be some sort of energy dissipation
TODO calculate length only using the first part of pendulum?
*/

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
enum NoteState {
    Pressed(u32),
    Released { pressed_time: u32, elapsed: u32 },
}

impl NoteState {
    fn update(&mut self, time: u32) {
        match self {
            NoteState::Pressed(elapsed) | NoteState::Released { elapsed, .. } => *elapsed += time,
        }
    }

    fn get_pressed_time(&self) -> u32 {
        match *self {
            NoteState::Pressed(time) => time,
            NoteState::Released { pressed_time, .. } => pressed_time,
        }
    }
}

#[derive(Clone)]
struct NoteEvent {
    note: wmidi::Note,
    state: NoteState,
    velocity: f32,
}

pub const CHAOTICITY_RANGE: RangeInclusive<f32> = 0.1f32..=1f32;
pub const ATTACK_RANGE: RangeInclusive<f32> = 0f32..=10f32;
pub const DECAY_RANGE: RangeInclusive<f32> = 0f32..=10f32;
pub const SUSTAIN_RANGE: RangeInclusive<f32> = 0f32..=1f32;
pub const RELEASE_RANGE: RangeInclusive<f32> = 0f32..=10f32;

// TODO handle params using messages instead?
pub struct Params {
    pub chaoticity: AtomicCell<f32>,
    pub attack: AtomicCell<f32>,
    pub decay: AtomicCell<f32>,
    pub sustain: AtomicCell<f32>,
    pub release: AtomicCell<f32>,
}

impl Params {
    fn get_attack(&self) -> f32 {
        self.attack
            .load()
            .clamp(*ATTACK_RANGE.start(), *ATTACK_RANGE.end())
    }

    fn get_decay(&self) -> f32 {
        self.decay
            .load()
            .clamp(*DECAY_RANGE.start(), *DECAY_RANGE.end())
    }

    fn get_sustain(&self) -> f32 {
        self.sustain
            .load()
            .clamp(*SUSTAIN_RANGE.start(), *SUSTAIN_RANGE.end())
    }

    fn get_release(&self) -> f32 {
        self.release
            .load()
            .clamp(*RELEASE_RANGE.start(), *RELEASE_RANGE.end())
    }
}

const LOWPASS_FREQ: f32 = 10000f32;

fn lerp(from: f32, to: f32, mix: f32) -> f32 {
    let mixclamp = mix.clamp(0., 1.);
    from * (1. - mixclamp) + to * mixclamp
}

fn get_pendulum_x(pendulum: &Pendulum) -> f32 {
    let tip = pendulum.t_pt.x.sin() * pendulum.length.x + pendulum.t_pt.y.sin() * pendulum.length.y;
    tip
}

/// sets the energy of the pendulum
/// changes the kinetic energy as much as possible, if not enough also adjusts potential
fn adjust_energy(pendulum: &mut Pendulum, energy: f32) {
    let oldx = get_pendulum_x(pendulum);
    let Pendulum {
        g,
        mass,
        length,
        ref mut t_pt,
        ..
    } = *pendulum;
    let mass_sum = mass.x + mass.y;
    let potential =
        g * (mass_sum * length.x * (1. - t_pt.x.cos()) + mass.y * length.y * (1. - t_pt.y.cos()));
    if energy > potential {
        let kinetic = energy - potential;
        let p = f32::sqrt(kinetic * 2f32 / mass_sum) * mass_sum;
        let psum = t_pt.z + t_pt.w;
        // TODO should the momentum be split up like this?
        if psum > f32::EPSILON {
            t_pt.z *= 1f32 / psum * p;
            t_pt.w *= 1f32 / psum * p;
        } else {
            let pd2 = p / 2f32;
            t_pt.z = pd2;
            t_pt.w = pd2;
        }
    } else {
        t_pt.z = 0.;
        t_pt.w = 0.;
        // TODO calculate theta to make the pendulum tip x as close to the old x as possible
        // simplify by setting both theta to the same value
        let den = g * (mass_sum * length.x + mass.y * length.y);
        let theta = if den != 0. {
            // TODO should be 0 if energy is 0
            let theta = f32::acos(1. - energy / den);
            if oldx < 0. {
                -theta
            } else {
                theta
            }
        } else {
            0.
        };
        t_pt.x = theta;
        t_pt.y = theta;
    };
}

fn get_lengths(center_length: f32, chaoticity: f32) -> Vec2 {
    let b = center_length / (1f32 + chaoticity / 2f32);
    let c = b * chaoticity;
    let length = vec2(b, c);
    length
}

#[derive(Clone)]
pub struct Synth {
    midi_events: MidiChannel,

    pendulum: Pendulum,
    note_event: Option<NoteEvent>,
    params: Arc<Params>,
    lowpass: (u32, biquad::DirectForm1<f32>),
    center_length: f32,
    sample_rate: u32,
}

impl Synth {
    pub fn new(midi_events: MidiChannel) -> Self {
        let sample_rate = 44100;
        Self {
            midi_events,
            note_event: None,
            params: Arc::new(Params {
                chaoticity: 0.67f32.into(),
                attack: 0.1f32.into(),
                decay: 0.5f32.into(),
                sustain: 0.5f32.into(),
                release: 0.5f32.into(),
            }),
            lowpass: (
                0, //< to make sure it is recalculated
                biquad::DirectForm1::<f32>::new(
                    biquad::Coefficients::<f32>::from_params(
                        biquad::Type::LowPass,
                        sample_rate.hz(),
                        LOWPASS_FREQ.min(sample_rate as f32 / 2.001f32).hz(),
                        biquad::Q_BUTTERWORTH_F32,
                    )
                    .unwrap(),
                ),
            ),
            pendulum: Pendulum {
                // higher gravity. for better precision. (is it really?)
                g: 9.81f32 * 100000.,
                mass: vec2(1., 1.),
                ..Pendulum::default()
            },
            center_length: 1f32,
            sample_rate,
        }
    }

    pub fn get_params(&self) -> Arc<Params> {
        self.params.clone()
    }

    fn calculate_energy(&self) -> f32 {
        if let Some(event) = &self.note_event {
            const VELOCITY_GAIN: f32 = 1.;
            let length = get_lengths(self.center_length, self.params.chaoticity.load());
            let Pendulum { g, mass, t_pt, .. } = self.pendulum;
            let mass_sum = mass.x + mass.y;
            let desired_potential =
                g * VELOCITY_GAIN * event.velocity * (mass_sum * length.x + mass.y * length.y);
            // let potential =
            //     -g * (mass_sum * length.x * t_pt.x.cos() + mass.y * length.y * t_pt.y.cos());
            let pressed_time = match event.state {
                NoteState::Pressed(elapsed) => elapsed,
                NoteState::Released { pressed_time, .. } => pressed_time,
            };
            let pressed_seconds = pressed_time as f32 / self.sample_rate as f32;
            let attack = self.params.get_attack();
            let pressed_value = if pressed_seconds < attack {
                pressed_seconds / attack
            } else {
                let remain = pressed_seconds - attack;
                let a = remain / self.params.get_decay();
                lerp(1., self.params.get_sustain(), a)
            };
            let adsr = event.velocity
                * match event.state {
                    NoteState::Pressed(_) => 1.,
                    NoteState::Released { elapsed, .. } => {
                        let elapsed_seconds = elapsed as f32 / self.sample_rate as f32;
                        let release = self.params.get_release().max(f32::EPSILON);
                        lerp(pressed_value, 0., elapsed_seconds / release)
                    }
                };
            desired_potential * adsr
        } else {
            0.
        }
    }
}

pub trait SynthPlayer {
    fn play(&mut self, sample_rate: u32, channels: usize, output: &mut [f32]);
}

impl SynthPlayer for Synth {
    fn play(&mut self, sample_rate: u32, channels: usize, output: &mut [f32]) {
        debug_assert!(sample_rate > 0);
        // TODO check if sample_rate has changed, and recalculate stuff?
        self.sample_rate = sample_rate;
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
                    // TODO make g a constant
                    let Pendulum {
                        ref mut t_pt,
                        ref mut mass,
                        g,
                        ..
                    } = self.pendulum;
                    // TODO calculate length better. do a few components of the large amplitude equation
                    self.center_length = (1f32 / note.to_freq_f32() / 2f32 / PI).powi(2) * g;
                    self.pendulum.friction = 0f32;
                    self.note_event = Some(NoteEvent {
                        note,
                        state: NoteState::Pressed(0),
                        velocity: norm_vel,
                    });
                }
                wmidi::MidiMessage::NoteOff(_, note, _) => {
                    if let Some(NoteEvent {
                        note: held_note,
                        ref mut state,
                        ..
                    }) = self.note_event
                    {
                        if note == held_note {
                            *state = NoteState::Released {
                                pressed_time: state.get_pressed_time(),
                                elapsed: 0,
                            };
                        }
                    }
                }
                _ => {}
            }
        }

        if self.lowpass.0 != sample_rate {
            self.lowpass = (
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
            );
        }

        // TODO m?? is that mass?
        //let m = vec2(1., 1.);
        //let cm = (m.x - m.y) / m.y;
        // TODO make the lengths the same, and change the mass instead?
        // TODO is it perhaps only the first length that should be used to calculate the center of mass?
        // TODO figure this out
        let length = get_lengths(self.center_length, chaoticity);
        self.pendulum.length = length;
        // TODO recalculate the momenta depending on the chaoticity?

        // if self.note_event.is_none() {
        //     // TODO don't use friction. just set the momentum and theta for more control
        //     self.pendulum.friction = release.powi(2);
        // }

        // produce sound
        for frame in output.chunks_exact_mut(channels) {
            // TODO should this be done in the rk4 loop in the pendulum code instead?
            let energy = self.calculate_energy();
            adjust_energy(&mut self.pendulum, energy);
            let tip = get_pendulum_x(&self.pendulum);
            let full_length = self.pendulum.length.x + self.pendulum.length.y;
            let a = tip / full_length;
            let lowpassed = self.lowpass.1.run(a);
            let clipped = lowpassed.clamp(-1f32, 1f32);
            for sample in frame.iter_mut() {
                *sample = clipped;
            }
            self.pendulum.update(1. / sample_rate as f32);
            if let Some(event) = &mut self.note_event {
                event.state.update(1);
            }
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
