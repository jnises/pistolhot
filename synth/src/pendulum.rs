use glam::{vec2, vec4, Vec2, Vec4, Vec4Swizzles};

#[derive(Clone)]
pub struct Pendulum {
    pub g: f32,
    pub step_size: f32,
    pub friction: f32,
    // the mass of the pendulums
    pub mass: Vec2,
    // the length of the pendulums
    pub length: Vec2,
    // simulation state (theta0, theta1, ptheta0, ptheta1) where ptheta are the generalized momenta
    pub t_pt: Vec4,
    pub time_error: f32,
}

impl Default for Pendulum {
    fn default() -> Self {
        Self {
            g: 9.81,
            step_size: 1.0 / 44100.0,
            friction: 0f32,
            mass: vec2(1f32, 1f32),
            length: vec2(1f32, 1f32),
            t_pt: Vec4::ZERO,
            time_error: 0.,
        }
    }
}

impl Pendulum {
    pub fn update(&mut self, elapsed: f32) {
        let Self {
            ref g,
            ref step_size,
            ref friction,
            ref length,
            ref mass,
            t_pt,
            time_error,
            ..
        } = self;
        *time_error += elapsed;
        if *time_error > 0. {
            let iterations = (*time_error / step_size).ceil() as usize;
            for _ in 0..iterations {
                let f = |t_pt: &Vec4| {
                    let theta = t_pt.xy();
                    let pt = t_pt.zw();
                    // TODO revert to simple pendulum if either length is close to 0?
                    // TODO simplify by setting both masses to 1?
                    // TODO extract common calculations
                    let dt0 = (length.y * pt.x - length.x * pt.y * f32::cos(theta.x - theta.y))
                        / (length.x.powi(2)
                            * length.y
                            * (mass.x + mass.y * f32::sin(theta.x - theta.y).powi(2)));
                    let dt1 = (length.x * (mass.x + mass.y) * pt.y
                        - length.y * mass.y * pt.x * f32::cos(theta.x - theta.y))
                        / (length.x
                            * length.y.powi(2)
                            * mass.y
                            * (mass.x + mass.y * f32::sin(theta.x - theta.y).powi(2)));
                    let c0 = pt.x * pt.y * f32::sin(theta.x - theta.y)
                        / (length.x
                            * length.y
                            * (mass.x + mass.y * f32::sin(theta.x - theta.y).powi(2)));
                    let c1 = (length.y.powi(2) * mass.y * pt.x.powi(2)
                        + length.x.powi(2) * (mass.x + mass.y) * pt.y.powi(2)
                        - length.x * length.y * mass.y * pt.x * pt.y * f32::cos(theta.x - theta.y))
                        / (2.
                            * length.x.powi(2)
                            * length.y.powi(2)
                            * (mass.x + mass.y * f32::sin(theta.x - theta.y).powi(2)).powi(2))
                        * f32::sin(2. * (theta.x - theta.y));
                    let mut dp0 = -(mass.x + mass.y) * g * length.x * f32::sin(theta.x) - c0 + c1;
                    let mut dp1 = -mass.y * g * length.y * f32::sin(theta.y) + c0 - c1;
                    // a naive friction. not physical at all
                    // TODO there should at least be something about mass here right?
                    dp0 -= dt0 * friction;
                    dp1 -= dt1 * friction;
                    // TODO add friction
                    const MAX_D: f32 = 999999f32;
                    vec4(
                        dt0.clamp(-MAX_D, MAX_D),
                        dt1.clamp(-MAX_D, MAX_D),
                        dp0.clamp(-MAX_D, MAX_D),
                        dp1.clamp(-MAX_D, MAX_D),
                    )
                };
                let k1 = f(t_pt);
                let k2 = f(&(*t_pt + *step_size * k1 / 2.));
                let k3 = f(&(*t_pt + *step_size * k2 / 2.));
                let k4 = f(&(*t_pt + *step_size * k3));
                let d = 1. / 6. * (k1 + 2. * k2 + 2. * k3 + k4);
                *t_pt += *step_size * d;
            }
            *time_error -= iterations as f32 * step_size;
        }
    }
}
