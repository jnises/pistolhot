use std::ops::RangeInclusive;

use crossbeam::atomic::AtomicCell;
use egui::Ui;

use crate::{
    Params, ATTACK_RANGE, CHAOTICITY_RANGE, DECAY_DELAY_RANGE, DECAY_RANGE, RELEASE_RANGE,
    SUSTAIN_RANGE,
};

fn param(ui: &mut Ui, param: &AtomicCell<f32>, name: &str, range: RangeInclusive<f32>) {
    ui.label(name);
    let mut p = param.load();
    ui.add(egui::Slider::new(&mut p, range));
    param.store(p);
}

pub fn params_gui(ui: &mut Ui, params: &Params) {
    ui.vertical(|ui| {
        param(ui, &params.chaoticity, "chaoticity:", CHAOTICITY_RANGE);
        param(ui, &params.attack, "attack:", ATTACK_RANGE);
        param(ui, &params.decay, "decay:", DECAY_RANGE);
        param(ui, &params.decay_delay, "decay_delay:", DECAY_DELAY_RANGE);
        param(ui, &params.sustain, "sustain:", SUSTAIN_RANGE);
        param(ui, &params.release, "release:", RELEASE_RANGE);
    });
}
