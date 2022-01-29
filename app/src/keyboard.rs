use crossbeam::channel;
use eframe::egui;
use log::warn;
use std::{collections::HashSet, convert::TryFrom};
use wmidi::MidiMessage;

fn is_key_black(note: wmidi::Note) -> bool {
    [
        false, true, false, true, false, false, true, false, true, false, true, false,
    ][(u8::from(note) % 12) as usize]
}

pub struct OnScreenKeyboard {
    key_pressed: HashSet<egui::Id>,
    midi_tx: channel::Sender<MidiMessage<'static>>,
}

impl OnScreenKeyboard {
    pub fn new(midi_tx: channel::Sender<MidiMessage<'static>>) -> Self {
        Self {
            key_pressed: HashSet::new(),
            midi_tx,
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            // start from middle c
            for note_num in 60.. {
                if ui.available_width() <= 0f32 {
                    break;
                }
                let note = match wmidi::Note::try_from(note_num) {
                    Ok(note) => note,
                    Err(_) => break,
                };
                let b = egui::Button::new(" ").fill(if is_key_black(note) {
                    egui::Color32::BLACK
                } else {
                    egui::Color32::WHITE
                });
                let r = ui.add(b);
                // egui doesn't seem to have any convenient "pressed" or "released" event
                if r.is_pointer_button_down_on() {
                    if self.key_pressed.insert(r.id) {
                        if let Err(e) = self.midi_tx.try_send(MidiMessage::NoteOn(
                            wmidi::Channel::Ch1,
                            note,
                            wmidi::Velocity::from_u8_lossy(127),
                        )) {
                            warn!("error sending note on midi message {}", e);
                        }
                    }
                } else if self.key_pressed.remove(&r.id) {
                    if let Err(e) = self.midi_tx.try_send(MidiMessage::NoteOff(
                        wmidi::Channel::Ch1,
                        note,
                        wmidi::Velocity::from_u8_lossy(0),
                    )) {
                        warn!("error sending midi note off message {}", e);
                    }
                }
            }
        });
    }
}
