use std::ops::RangeInclusive;

use crossbeam::atomic::AtomicCell;
use egui::Ui;

use crate::Params;

fn param(ui: &mut Ui, param: &AtomicCell<f32>, name: &str, range: RangeInclusive<f32>) {
    ui.label(name);
    let mut p = param.load();
    ui.add(egui::Slider::new(&mut p, range));
    param.store(p);
}

pub fn params_gui(ui: &mut Ui, params: &Params) {
    ui.horizontal(|ui| {
        param(ui, &params.chaoticity, "chaoticity:", 0.0001f32..=0.999f32);
    });
}
