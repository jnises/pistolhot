use egui::Ui;

use crate::Params;

pub fn params_gui(ui: &mut Ui, params: &Params) {
    ui.horizontal(|ui| {
        ui.label("chaoticity:");
        let mut chaoticity = params.chaoticity.load();
        ui.add(egui::Slider::new(&mut chaoticity, 0.0001f32..=0.999f32));
        params.chaoticity.store(chaoticity);
    });
}