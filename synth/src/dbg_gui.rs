use std::{collections::HashMap, ops::RangeInclusive, sync::Mutex};

use crossbeam::atomic::AtomicCell;
use egui::Ui;

use crate::{Params, ATTACK_RANGE, CHAOTICITY_RANGE, DECAY_RANGE, RELEASE_RANGE, SUSTAIN_RANGE};

use once_cell::sync::Lazy;

#[cfg(debug_assertions)]
static DBG_VALUES: Lazy<Mutex<HashMap<&'static str, f32>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

#[cfg(debug_assertions)]
pub fn dbg_value(name: &'static str, value: f32) {
    DBG_VALUES.lock().unwrap().insert(name, value);
}

#[cfg(not(debug_assertions))]
pub fn dbg_value(_name: &'static str, _value: f32) {}

#[cfg(debug_assertions)]
#[macro_export]
macro_rules! dbg_value {
    // () => {
    //     $crate::eprintln!("[{}:{}]", $crate::file!(), $crate::line!())
    // };
    ($val:expr $(,)?) => {
        // Use of `match` here is intentional because it affects the lifetimes
        // of temporaries - https://stackoverflow.com/a/48732525/1063961
        match $val {
            tmp => {
                dbg_value(stringify!($val), tmp);
                tmp
            }
        }
    };
    ($($val:expr),+ $(,)?) => {
        ($($crate::dbg_value!($val)),+,)
    };
}

#[cfg(not(debug_assertions))]
#[macro_export]
macro_rules! dbg_value {
    // () => {
    //     $crate::eprintln!("[{}:{}]", $crate::file!(), $crate::line!())
    // };
    ($val:expr $(,)?) => {
        tmp
    };
    ($($val:expr),+ $(,)?) => {
        ($($crate::dbg_value!($val)),+,)
    };
}

#[cfg(debug_assertions)]
pub fn dbg_gui(ui: &mut Ui) {
    ui.vertical(|ui| {
        for (&key, value) in DBG_VALUES.lock().unwrap().iter() {
            ui.label(format!("{}: {}", key, value));
        }
    });
}

#[cfg(not(debug_assertions))]
pub fn dbg_gui(_ui: &mut Ui) {}
