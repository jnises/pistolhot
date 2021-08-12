use crate::audio::AudioManager;
use crate::keyboard::OnScreenKeyboard;
use crate::midi::MidiReader;
use crate::periodic_updater::PeriodicUpdater;
use crate::synth::Synth;
use cpal::traits::DeviceTrait;
use crossbeam::channel;
use eframe::{
    egui,
    epi::{self, App},
};
use parking_lot::Mutex;
use std::{collections::VecDeque, sync::Arc};

const NAME: &str = "Wayfärer";
const VIS_SIZE: usize = 512;

pub struct Wayfarer {
    audio: Option<AudioManager<Synth>>,
    midi: Option<MidiReader>,
    status_text: Arc<Mutex<String>>,
    status_clone: Arc<Mutex<String>>,
    keyboard: OnScreenKeyboard,
    forced_buffer_size: Option<u32>,
    left_vis_buffer: VecDeque<f32>,
    synth: Option<Synth>,
    periodic_updater: Option<PeriodicUpdater>,
}

impl Wayfarer {
    pub fn new() -> Self {
        let (midi_tx, midi_rx) = channel::bounded(256);
        let (midi, initial_status) = match MidiReader::new(midi_tx.clone()) {
            Ok(midi) => (Some(midi), String::new()),
            Err(e) => (None, format!("error initializaing midi: {}", e)),
        };
        let mut synth = Some(Synth::new(midi_rx));
        let status_text = Arc::new(Mutex::new(initial_status));
        // can't init audio here for wasm since that gets blocked by chrome's autoplay check
        let audio = if cfg!(target_arch = "wasm32") {
            None
        } else {
            let status_clone = status_text.clone();
            Some(AudioManager::new(synth.take().unwrap(), move |e| {
                *status_clone.lock() = e;
            }))
        };
        Self {
            audio,
            midi,
            status_clone: status_text.clone(),
            status_text,
            keyboard: OnScreenKeyboard::new(midi_tx),
            forced_buffer_size: None,
            left_vis_buffer: VecDeque::with_capacity(VIS_SIZE * 2),
            synth,
            periodic_updater: None,
        }
    }
}

impl App for Wayfarer {
    fn name(&self) -> &str {
        NAME
    }

    fn on_exit(&mut self) {
        self.periodic_updater.take();
    }

    fn update(&mut self, ctx: &egui::CtxRef, frame: &mut epi::Frame<'_>) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading(NAME);
            if self.audio.is_none() {
                if ui.button("start").clicked() {
                    let status_clone = self.status_clone.clone();
                    self.audio = Some(AudioManager::new(self.synth.take().unwrap(), move |e| {
                        *status_clone.lock() = e;
                    }));
                }
            } else {
                // send repaint periodically instead of each frame since the rendering doesn't seem to be vsynced when the window is hidden on mac
                // TODO stop this when not in focus
                if self.periodic_updater.is_none() {
                    let repaint_signal = frame.repaint_signal();
                    self.periodic_updater = Some(PeriodicUpdater::new(repaint_signal));
                }
                let audio = self.audio.as_mut().expect("should have audio by now");
                let midi = &self.midi;
                let left_vis_buffer = &mut self.left_vis_buffer;
                let forced_buffer_size = &mut self.forced_buffer_size;
                let status_text = &self.status_text;
                let keyboard = &mut self.keyboard;
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.label("midi:");
                        ui.label(midi.as_ref().map(|midi| midi.get_name()).unwrap_or("-"));
                    });
                });

                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.label("audio:");
                        let mut selected = audio.get_name().unwrap_or_else(|| "-".to_string());
                        egui::ComboBox::from_id_source("audio combo box")
                            .selected_text(&selected)
                            .show_ui(ui, |ui| {
                                // TODO cache this to not poll too often
                                for device in audio.get_devices() {
                                    if let Ok(name) = device.name() {
                                        ui.selectable_value(&mut selected, name.clone(), name);
                                    }
                                }
                            });
                        if Some(&selected) != audio.get_name().as_ref() {
                            if let Some(device) = audio.get_devices().into_iter().find(|d| {
                                if let Ok(name) = d.name() {
                                    name == selected
                                } else {
                                    false
                                }
                            }) {
                                audio.set_device(device);
                            }
                        }
                    });
                    let buffer_range = audio.get_buffer_size_range();
                    ui.horizontal(|ui| {
                        ui.label("buffer size:");
                        ui.group(|ui| {
                            if buffer_range.is_none() {
                                ui.set_enabled(false);
                                *forced_buffer_size = None;
                            }
                            let mut forced = forced_buffer_size.is_some();
                            ui.horizontal(|ui| {
                                ui.checkbox(&mut forced, "force");
                                ui.set_enabled(forced);
                                let mut size = match forced_buffer_size.to_owned() {
                                    Some(size) => size,
                                    None => audio.get_buffer_size().unwrap_or(0),
                                };
                                let range = match buffer_range {
                                    // limit max to something sensible
                                    Some((min, max)) => min..=max.min(16384),
                                    None => 0..=1,
                                };
                                ui.add(egui::Slider::new(&mut size, range));
                                if forced {
                                    *forced_buffer_size = Some(size);
                                } else {
                                    *forced_buffer_size = None;
                                }
                                audio.set_forced_buffer_size(*forced_buffer_size);
                            });
                        });
                    });

                    audio.pop_each_left_vis_buffer(|value| {
                        left_vis_buffer.push_back(value);
                    });

                    let mut prev = None;
                    let mut it = left_vis_buffer.iter().copied().rev();
                    it.nth(VIS_SIZE / 2 - 1);
                    while let Some(value) = it.next() {
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
                            .line(egui::plot::Line::new(egui::plot::Values::from_values_iter(
                                it.take(VIS_SIZE / 2)
                                    .enumerate()
                                    .map(|(x, y)| egui::plot::Value {
                                        x: x as f64,
                                        y: y as f64,
                                    }),
                            )))
                            .width(ui.available_width().min(200.))
                            .view_aspect(2.0),
                    );
                    if left_vis_buffer.len() > VIS_SIZE {
                        drop(left_vis_buffer.drain(0..left_vis_buffer.len() - VIS_SIZE));
                    }
                    ui.label(&*status_text.lock());
                });
                // put onscreen keyboard at bottom of window
                let height = ui.available_size().y;
                ui.add_space(height - 20f32);
                keyboard.show(ui);
            }
        });
    }
}
