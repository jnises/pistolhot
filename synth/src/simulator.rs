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
    pub fn get_normalized_x(&self) -> f32 {
        let Self { pendulum, .. } = self;
        let tip =
            pendulum.t_pt.x.sin() * pendulum.length.x + pendulum.t_pt.y.sin() * pendulum.length.y;
        tip / (pendulum.length.x + pendulum.length.y)
    }

    pub fn update(&mut self, elapsed: f32, energy: f32, p: f32) {
        debug_assert!(energy >= 0.);
        debug_assert!((0. ..=1.).contains(&p));
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
                Self::adjust_energy(pendulum, energy, p);
                pendulum.update(step_size);
            }
            *time_error -= iterations as f32 * step_size;
        }
    }

    // wolfram alpha kinetic energy in terms for theta and canonical momenta
    // k = (m_2 * l_2^2 * p_1^2  +  (m_1 + m_2) * l_1^2 * p_2^2  -  2 * m_2 * l_1 * l_2 * p_1 * p_2 * cos(theta_1 - theta_2))  /  (2 * m_2 * l_1^2 * l_2^2 * (m_1 + m_2 * sin(theta_1 - theta_2)^2))

    // assume p_2 = p_1 * c
    // k = (m_2 * l_2^2 * p_1^2  +  (m_1 + m_2) * l_1^2 * (p_1 * c)^2  -  2 * m_2 * l_1 * l_2 * p_1 * (p_1 * c) * cos(t_1 - t_2))  /  (2 * m_2 * l_1^2 * l_2^2 * (m_1 + m_2 * sin(t_1 - t_2)^2))

    // k = (m_2 * l_2^2 * (p * d)^2  +  (m_1 + m_2) * l_1^2 * (p * c)^2  -  2 * m_2 * l_1 * l_2 * (p * d) * (p * c) * cos(t_1 - t_2))  /  (2 * m_2 * l_1^2 * l_2^2 * (m_1 + m_2 * sin(t_1 - t_2)^2))
    // k = (m_2 * l_2^2 * (p * d)^2  +  (m_1 + m_2) * l_1^2 * (p * c)^2  -  2 * m_2 * l_1 * l_2 * (p * d) * (p * c) * cos(t))  /  (2 * m_2 * l_1^2 * l_2^2 * (m_1 + m_2 * sin(t)^2))

    /// sets the energy of the pendulum
    /// changes the kinetic energy only
    fn adjust_energy(pendulum: &mut Pendulum, energy: f32, p: f32) {
        let Pendulum {
            g,
            mass,
            length,
            ref mut t_pt,
            ..
        } = *pendulum;
        let mass_sum = mass.x + mass.y;
        let potential = g
            * (mass_sum * length.x * (1. - t_pt.x.cos()) + mass.y * length.y * (1. - t_pt.y.cos()));
        dbg_value!(potential);
        // TODO this will override the simulation all the time right? that's not good.
        // how to handle that better?
        // can we calculate the energy more correctly?
        // have some allowed energy range?
        // lowpass the adjustment?
        let new_t_pt = if energy > potential {
            let kinetic = energy - potential;

            let thetadiff = t_pt.x - t_pt.y;

            // TODO handle t_pt.z == 0 better,
            let c = t_pt.w / (t_pt.z.signum() * t_pt.z.abs().max(f32::EPSILON));
            let pdet = f32::sqrt(
                c.powi(2) * length.x.powi(2) * mass_sum
                    - 2. * c * length.y * length.x * mass.y * thetadiff.cos()
                    + length.y.powi(2) * mass.y,
            );
            let p_theta = std::f32::consts::SQRT_2
                * kinetic.sqrt()
                * length.x
                * length.y
                * mass.y.sqrt()
                * f32::sqrt(mass.y * thetadiff.sin().powi(2) + mass.x)
                / pdet;
            dbg_value!(p_theta);

            let mut new_t_pt = *t_pt;
            new_t_pt.z = if p_theta.is_sign_positive() != t_pt.z.is_sign_positive() {
                -p_theta
            } else {
                p_theta
            };
            new_t_pt.w = c * new_t_pt.z;
            new_t_pt
        } else if energy < f32::EPSILON {
            Vec4::ZERO
        } else {
            *t_pt
        };
        *t_pt = t_pt.lerp(new_t_pt, p);
    }
}
