mod keyboard;
mod midi;
mod periodic_updater;
mod audio;
mod timer;
use crate::keyboard::OnScreenKeyboard;
use crate::midi::MidiReader;
use crate::periodic_updater::PeriodicUpdater;
use synth::{self, Synth, params_gui};
use crate::{audio::AudioManager, synth::Params};
use cpal::traits::DeviceTrait;
use crossbeam::channel;
use eframe::{
    egui::{self, emath, epaint, pos2, vec2, Color32, Rect, Stroke},
    epi::{self, App},
};
use log::warn;
use parking_lot::Mutex;
use std::{collections::VecDeque, sync::Arc};

const NAME: &str = "Pistolhot";
const VIS_SIZE: usize = 512;

pub struct Data {
    audio: AudioManager<Synth>,
    midi: Arc<MidiReader>,
    status_text: Arc<Mutex<String>>,
    status_clone: Arc<Mutex<String>>,
    keyboard: OnScreenKeyboard,
    forced_buffer_size: Option<u32>,
    left_vis_buffer: VecDeque<f32>,
    synth: Option<Synth>,
    synth_params: Arc<Params>,
    periodic_updater: Option<PeriodicUpdater>,
}

pub enum Pistolhot {
    Initialized(Data),
    Uninitialized,
}

impl Pistolhot {
    fn init(&mut self) {
        let (midi_tx, midi_rx) = channel::bounded(256);
        let midi = MidiReader::new(midi_tx.clone());

        let mut synth = Some(Synth::new(midi_rx));
        let synth_params = synth.as_ref().unwrap().get_params();
        let status_text = Arc::new(Mutex::new("".to_string()));
        let status_clone = status_text.clone();
        let audio = AudioManager::new(synth.take().unwrap(), move |e| {
            *status_clone.lock() = e;
        });
        *self = Self::Initialized(Data {
            audio,
            midi,
            status_clone: status_text.clone(),
            status_text,
            keyboard: OnScreenKeyboard::new(midi_tx),
            forced_buffer_size: None,
            left_vis_buffer: VecDeque::with_capacity(VIS_SIZE * 2),
            synth,
            synth_params,
            periodic_updater: None,
        });
    }

    pub fn new() -> Self {
        let mut s = Self::Uninitialized;
        // need to defer initializion in wasm due to chrome's autoplay blocking and such
        if cfg!(not(target_arch = "wasm32")) {
            s.init();
        }
        s
    }
}

impl App for Pistolhot {
    fn name(&self) -> &str {
        NAME
    }

    fn on_exit(&mut self) {
        if let Self::Initialized(Data {
            periodic_updater, ..
        }) = self
        {
            periodic_updater.take();
        }
    }

    fn update(&mut self, ctx: &egui::Context, frame: &epi::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading(NAME);
            match self {
                Self::Uninitialized => {
                    if ui.button("start").clicked() {
                        self.init();
                    }
                }
                Self::Initialized(data) => {
                    // send repaint periodically instead of each frame since the rendering doesn't seem to be vsynced when the window is hidden on mac
                    // TODO stop this when not in focus
                    if data.periodic_updater.is_none() {
                        data.periodic_updater = Some(PeriodicUpdater::new(frame.clone()));
                    }
                    let audio = &mut data.audio;
                    let midi = &data.midi;
                    let left_vis_buffer = &mut data.left_vis_buffer;
                    let forced_buffer_size = &mut data.forced_buffer_size;
                    let status_text = &data.status_text;
                    let keyboard = &mut data.keyboard;
                    let params = data.synth_params.as_ref();
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.label("midi:");
                            ui.label(midi.get_name());
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
                        let mut it = left_vis_buffer.iter().copied();//.rev();
                        it.nth(VIS_SIZE / 2 - 1);
                        for value in &mut it {
                            if let Some(prev) = prev {
                                if prev >= 0. && value < 0. {
                                    break;
                                }
                            }
                            prev = Some(value);
                        }
                        let plot_width = ui.available_width().min(300.);
                        let (_, rect) = ui.allocate_space(vec2(plot_width, plot_width * 0.5));
                        let p = ui.painter_at(rect);
                        p.rect_filled(rect, 10f32, Color32::BLACK);
                        let to_rect = emath::RectTransform::from_to(
                            Rect::from_x_y_ranges(0.0..=(VIS_SIZE / 2) as f32, -1.0..=1.0),
                            rect,
                        );
                        p.add(epaint::Shape::line(
                            it.take(VIS_SIZE / 2)
                                .enumerate()
                                .map(|(x, y)| to_rect * pos2(x as f32, y))
                                .collect(),
                            Stroke::new(1f32, Color32::GRAY),
                        ));
                        if left_vis_buffer.len() > VIS_SIZE {
                            drop(left_vis_buffer.drain(0..left_vis_buffer.len() - VIS_SIZE));
                        }
                        ui.label(&*status_text.lock());
                    });
                    ui.group(|ui| {
                        params_gui(ui, &params);
                    });
                    // put onscreen keyboard at bottom of window
                    let height = ui.available_size().y;
                    ui.add_space(height - 20f32);
                    keyboard.show(ui);
                }
            }
        });
    }
}
