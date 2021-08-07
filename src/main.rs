#![warn(clippy::all, rust_2018_idioms)]
use cpal::traits::DeviceTrait;
use crossbeam::channel::{self, Sender};
use eframe::{
    egui::{self, Vec2},
    epi::{self, App},
};
use slice_deque::SliceDeque;
use std::{sync::Arc, thread::JoinHandle, time::Duration};
mod keyboard;
use keyboard::OnScreenKeyboard;
mod midi;
use midi::MidiReader;
mod audio;
use audio::AudioManager;
mod synth;
use parking_lot::Mutex;
use synth::Synth;
mod pendulum;

const NAME: &str = "Pistolhot";
const VIS_SIZE: usize = 512;

struct Pistolhot {
    audio: AudioManager<Synth>,
    midi: Option<MidiReader>,
    status_text: Arc<Mutex<String>>,
    keyboard: OnScreenKeyboard,
    forced_buffer_size: Option<u32>,
    left_vis_buffer: SliceDeque<f32>,
    periodic_updater: Option<(Sender<()>, JoinHandle<()>)>,
}

impl Pistolhot {
    fn new() -> Self {
        let (midi_tx, midi_rx) = channel::bounded(256);
        let (midi, initial_status) = match MidiReader::new(midi_tx.clone()) {
            Ok(midi) => (Some(midi), String::new()),
            Err(e) => (None, format!("error initializaing midi: {}", e)),
        };
        let synth = Synth::new(midi_rx);
        let status_text = Arc::new(Mutex::new(initial_status));
        let status_clone = status_text.clone();
        let audio = AudioManager::new(synth, move |e| {
            *status_clone.lock() = e;
        });
        Self {
            audio,
            midi,
            status_text,
            keyboard: OnScreenKeyboard::new(midi_tx),
            forced_buffer_size: None,
            left_vis_buffer: SliceDeque::with_capacity(VIS_SIZE),
            periodic_updater: None,
        }
    }
}

impl App for Pistolhot {
    fn name(&self) -> &str {
        NAME
    }

    fn on_exit(&mut self) {
        if let Some((rx, join)) = self.periodic_updater.take() {
            rx.send(()).unwrap();
            join.join().unwrap();
        }
    }

    fn update(&mut self, ctx: &egui::CtxRef, frame: &mut epi::Frame<'_>) {
        // send repaint periodically instead of each frame since the rendering doesn't seem to be vsynced when the window is hidden on mac
        if self.periodic_updater.is_none() {
            let repaint_signal = frame.repaint_signal();
            let (tx, rx) = channel::bounded(1);
            self.periodic_updater = Some((
                tx,
                std::thread::spawn(move || {
                    while rx.try_recv().is_err() {
                        std::thread::sleep(Duration::from_millis(100));
                        repaint_signal.request_repaint();
                    }
                }),
            ));
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading(NAME);
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label("midi:");
                    ui.label(
                        self.midi
                            .as_ref()
                            .map(|midi| midi.get_name())
                            .unwrap_or("-"),
                    );
                });
            });

            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label("audio:");
                    let mut selected = self.audio.get_name().unwrap_or_else(|| "-".to_string());
                    egui::ComboBox::from_id_source("audio combo box")
                        .selected_text(&selected)
                        .show_ui(ui, |ui| {
                            // TODO cache this to not poll too often
                            for device in self.audio.get_devices() {
                                if let Ok(name) = device.name() {
                                    ui.selectable_value(&mut selected, name.clone(), name);
                                }
                            }
                        });
                    if Some(&selected) != self.audio.get_name().as_ref() {
                        if let Some(device) = self.audio.get_devices().into_iter().find(|d| {
                            if let Ok(name) = d.name() {
                                name == selected
                            } else {
                                false
                            }
                        }) {
                            self.audio.set_device(device);
                        }
                    }
                });
                let buffer_range = self.audio.get_buffer_size_range();
                ui.horizontal(|ui| {
                    ui.label("buffer size:");
                    ui.group(|ui| {
                        if buffer_range.is_none() {
                            ui.set_enabled(false);
                            self.forced_buffer_size = None;
                        }
                        let mut forced = self.forced_buffer_size.is_some();
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut forced, "force");
                            ui.set_enabled(forced);
                            let mut size = match self.forced_buffer_size {
                                Some(size) => size,
                                None => self.audio.get_buffer_size().unwrap_or(0),
                            };
                            let range = match buffer_range {
                                Some((min, max)) => min..=max,
                                None => 0..=1,
                            };
                            ui.add(egui::Slider::new(&mut size, range));
                            if forced {
                                self.forced_buffer_size = Some(size);
                            } else {
                                self.forced_buffer_size = None;
                            }
                            self.audio.set_forced_buffer_size(self.forced_buffer_size);
                        });
                    });
                });

                let vis_buf = &mut self.left_vis_buffer;
                self.audio.pop_each_left_vis_buffer(|value| {
                    vis_buf.push_back(value);
                });

                let mut prev = None;
                let mut it = vis_buf.iter().rev();
                it.nth(VIS_SIZE / 2 - 1);
                while let Some(&value) = it.next() {
                    if let Some(prev) = prev {
                        if prev >= 0. && value < 0. {
                            break;
                        }
                    }
                    prev = Some(value);
                }
                ui.add(
                    egui::plot::Plot::new("waveform")
                        .include_y(-1.)
                        .include_y(1.)
                        .line(egui::plot::Line::new(
                            egui::plot::Values::from_ys_f32(&vis_buf[it.len()..it.len() + VIS_SIZE / 2])
                        ))
                        .width(ui.available_width().min(200.))
                        .view_aspect(2.0),
                );
                vis_buf.truncate_front(VIS_SIZE);
                ui.label(&*self.status_text.lock());
            });
            // put onscreen keyboard at bottom of window
            let height = ui.available_size().y;
            ui.add_space(height - 20f32);
            self.keyboard.show(ui);
        });
    }
}

fn main() {
    env_logger::init();
    let app = Box::new(Pistolhot::new());
    eframe::run_native(app, epi::NativeOptions {
        // has to be disabled to work with cpal
        drag_and_drop_support: false,
        initial_window_size: Some(Vec2 {
            x: 400f32,
            y: 300f32,
        }),
        ..Default::default()
    });
}
