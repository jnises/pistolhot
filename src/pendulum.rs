use glam::{vec2, vec4, Vec2, Vec4, Vec4Swizzles};

#[derive(Clone)]
pub struct Pendulum {
    pub g: f32,
    pub step_size: f32,
    // the mass of the pendulums
    pub mass: Vec2,
    // the length of the pendulums
    pub length: Vec2,
    // simulation state (theta0, theta1, ...)
    pub t_pt: Vec4,
    pub time_error: f32,
}

impl Default for Pendulum {
    fn default() -> Self {
        Self {
            g: 9.81,
            step_size: 1.0 / 44100.0,
            mass: vec2(1f32, 1f32),
            length: vec2(1f32, 1f32),
            t_pt: vec4(0., 0., 0., 0.),
            time_error: 0.,
        }
    }
}

impl Pendulum {
    pub fn update(&mut self, elapsed: f32) {
        let Self {
            ref g,
            ref step_size,
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
                    let pt: (f32, f32) = t_pt.zw().into();
                    let dt0 = (length.y * pt.0 - length.x * pt.1 * f32::cos(theta.x - theta.y))
                        / (length.x.powi(2) * length.y * (mass.x + mass.y * f32::sin(theta.x - theta.y).powi(2)));
                    let dt1 = (length.x * (mass.x + mass.y) * pt.1 - length.y * mass.y * pt.0 * f32::cos(theta.x - theta.y))
                        / (length.x * length.y.powi(2) * mass.y * (mass.x + mass.y * f32::sin(theta.x - theta.y).powi(2)));
                    let c0 = pt.0 * pt.1 * f32::sin(theta.x - theta.y)
                        / (length.x * length.y * (mass.x + mass.y * f32::sin(theta.x - theta.y).powi(2)));
                    let c1 = (length.y.powi(2) * mass.y * pt.0.powi(2)
                        + length.x.powi(2) * (mass.x + mass.y) * pt.1.powi(2)
                        - length.x * length.y * mass.y * pt.0 * pt.1 * f32::cos(theta.x - theta.y))
                        / (2.
                            * length.x.powi(2)
                            * length.y.powi(2)
                            * (mass.x + mass.y * f32::sin(theta.x - theta.y).powi(2)).powi(2))
                        * f32::sin(2. * (theta.x - theta.y));
                    let dp0 = -(mass.x + mass.y) * g * length.x * f32::sin(theta.x) - c0 + c1;
                    let dp1 = -mass.y * g * length.y * f32::sin(theta.y) + c0 - c1;
                    // TODO add friction
                    vec4(dt0, dt1, dp0, dp1)
                };
                let k1 = f(t_pt);
                let k2 = f(&(*t_pt + *step_size * k1 / 2.));
                let k3 = f(&(*t_pt + *step_size * k2 / 2.));
                let k4 = f(&(*t_pt + *step_size * k3));
                *t_pt += step_size / 6. * (k1 + 2. * k2 + 2. * k3 + k4);
            }
            *time_error -= iterations as f32 * step_size;
        }
    }
}
