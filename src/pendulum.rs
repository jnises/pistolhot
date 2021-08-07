use glam::{vec2, vec4, Vec2, Vec4, Vec4Swizzles};

#[derive(Clone)]
pub struct Pendulum {
    pub g: f32,
    pub step_size: f32,
    // the mass of the pendulums
    pub m: Vec2,
    // the length of the pendulums
    pub l: Vec2,
    // derivatives
    pub t_pt: Vec4,
    pub time_error: f32,
}

impl Default for Pendulum {
    fn default() -> Self {
        Self {
            g: 9.81,
            step_size: 1.0 / 44100.0,
            m: vec2(1f32, 1f32),
            l: vec2(1f32, 1f32),
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
            ref l,
            ref m,
            t_pt,
            time_error,
            ..
        } = self;
        *time_error += elapsed;
        if *time_error > 0. {
            let iterations = (*time_error / step_size).ceil() as usize;
            for _ in 0..iterations {
                let f = |t_pt: &Vec4| {
                    let t: (f32, f32) = t_pt.xy().into();
                    let pt: (f32, f32) = t_pt.zw().into();
                    let dt0 = (l.y * pt.0 - l.x * pt.1 * f32::cos(t.0 - t.1))
                        / (l.x.powi(2) * l.y * (m.x + m.y * f32::sin(t.0 - t.1).powi(2)));
                    let dt1 = (l.x * (m.x + m.y) * pt.1 - l.y * m.y * pt.0 * f32::cos(t.0 - t.1))
                        / (l.x * l.y.powi(2) * m.y * (m.x + m.y * f32::sin(t.0 - t.1).powi(2)));
                    let c0 = pt.0 * pt.1 * f32::sin(t.0 - t.1)
                        / (l.x * l.y * (m.x + m.y * f32::sin(t.0 - t.1).powi(2)));
                    let c1 = (l.y.powi(2) * m.y * pt.0.powi(2)
                        + l.x.powi(2) * (m.x + m.y) * pt.1.powi(2)
                        - l.x * l.y * m.y * pt.0 * pt.1 * f32::cos(t.0 - t.1))
                        / (2.
                            * l.x.powi(2)
                            * l.y.powi(2)
                            * (m.x + m.y * f32::sin(t.0 - t.1).powi(2)).powi(2))
                        * f32::sin(2. * (t.0 - t.1));
                    let dp0 = -(m.x + m.y) * g * l.x * f32::sin(t.0) - c0 + c1;
                    let dp1 = -m.y * g * l.y * f32::sin(t.1) + c0 - c1;
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
