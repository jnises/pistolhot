use glam::Vec4;

use crate::dbg_gui::dbg_value;
use crate::pendulum::Pendulum;

#[derive(Clone)]
pub struct Simulator {
    pub pendulum: Pendulum,
    pub step_size: f32,
    pub time_error: f32,
}

impl Default for Simulator {
    fn default() -> Self {
        Self {
            step_size: 1.0 / 44100.0,
            time_error: 0.,
            pendulum: Pendulum::default(),
        }
    }
}

impl Simulator {
    pub fn update(&mut self, elapsed: f32) {
        let Self {
            ref mut pendulum,
            step_size,
            ref mut time_error,
        } = *self;
        *time_error += elapsed;
        if *time_error > 0. {
            let iterations = (*time_error / step_size).ceil() as usize;
            for _ in 0..iterations {
                // TODO do the adsr stuff here
                pendulum.update(step_size);
            }
            *time_error -= iterations as f32 * step_size;
        }
    }

    /// sets the energy of the pendulum
    /// changes the kinetic energy only
    pub fn adjust_energy(&mut self, energy: f32) {
        //let oldx = get_pendulum_x(pendulum);
        let Pendulum {
            g,
            mass,
            length,
            ref mut t_pt,
            ..
        } = self.pendulum;
        let mass_sum = mass.x + mass.y;
        let potential = g
            * (mass_sum * length.x * (1. - t_pt.x.cos()) + mass.y * length.y * (1. - t_pt.y.cos()));
        dbg_value!(potential);
        // TODO this will override the simulation all the time right? that's not good.
        // how to handle that better?
        // can we calculate the energy more correctly?
        // have some allowed energy range?
        // lowpass the adjustment?
        if energy > potential {
            let kinetic = energy - potential;

            let thetadiff = t_pt.x - t_pt.y;

            // TODO handle t_pt.z == 0 better,
            let c = t_pt.w / (t_pt.z.signum() * t_pt.z.abs().max(f32::EPSILON));
            let pdet = f32::sqrt(
                c.powi(2) * length.x.powi(2) * mass_sum
                    - 2. * c * length.y * length.x * mass.y * thetadiff.cos()
                    + length.y.powi(2) * mass.y,
            );
            let p = std::f32::consts::SQRT_2
                * kinetic.sqrt()
                * length.x
                * length.y
                * mass.y.sqrt()
                * f32::sqrt(mass.y * thetadiff.sin().powi(2) + mass.x)
                / pdet;
            dbg_value!(p);

            t_pt.z = if p.is_sign_positive() != t_pt.z.is_sign_positive() {
                -p
            } else {
                p
            };
            t_pt.w = c * t_pt.z;
        } else if energy < f32::EPSILON {
            *t_pt = Vec4::ZERO;
        }
    }
}
