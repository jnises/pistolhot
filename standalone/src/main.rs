#![warn(clippy::all, rust_2018_idioms)]

use app::Pistolhot;

fn main() {
    use eframe::{egui::Vec2, epi};

    env_logger::init();
    let app = Box::new(Pistolhot::new());
    eframe::run_native(
        app,
        epi::NativeOptions {
            // has to be disabled to work with cpal
            drag_and_drop_support: false,
            initial_window_size: Some(Vec2 {
                x: 400f32,
                y: 700f32,
            }),
            ..Default::default()
        },
    );
}
