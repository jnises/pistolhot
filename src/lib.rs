// this file is used when targeting wasm

#![warn(clippy::all, rust_2018_idioms)]
cfg_if::cfg_if! {
if #[cfg(target_arch = "wasm32")] {

use log::{warn, Level, Metadata, Record};
use web_sys::console;

mod audio;
mod keyboard;
mod midi;
mod synth;
mod periodic_updater;

mod app;
pub use app::Wayfarer;

struct WebLogger;
// isn't there a ready made crate for this functionality somewhere
impl log::Log for WebLogger {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record<'_>) {
        if self.enabled(record.metadata()) {
            match record.level() {
                Level::Error => console::error_1(&format!("{}", record.args()).into()),
                Level::Warn => console::warn_1(&format!("{}", record.args()).into()),
                Level::Info => console::info_1(&format!("{}", record.args()).into()),
                Level::Debug => console::debug_1(&format!("{}", record.args()).into()),
                Level::Trace => console::trace_1(&format!("{}", record.args()).into()),
            }
        }
    }

    fn flush(&self) {}
}
static LOGGER: WebLogger = WebLogger;

use eframe::wasm_bindgen::{self, prelude::*};

#[wasm_bindgen]
pub fn start(canvas_id: &str) -> Result<(), eframe::wasm_bindgen::JsValue> {
    #[cfg(debug_assertions)]
    console_error_panic_hook::set_once();
    log::set_logger(&LOGGER).expect("unable to set logger");
    log::set_max_level(log::LevelFilter::Info);
    let app = Wayfarer::new();
    eframe::start_web(canvas_id, Box::new(app))
}
}}
