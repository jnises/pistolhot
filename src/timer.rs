use std::thread;

use chrono::Duration;

// util to be able to set timers on both wasm and native targets

cfg_if::cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        use eframe::wasm_bindgen::{prelude::Closure, JsCast};
        use std::{rc::Rc, cell::RefCell};
        pub struct Timer;
        impl Timer {
            pub fn new() -> Self {
                Self{}
            }

            pub fn schedule_with_delay<T: Fn() + 'static>(&self, delay: &Duration, callback: T) {
                let win = web_sys::window().unwrap();
                let rcf = Rc::new(RefCell::new(None));
                let rcfclone = rcf.clone();
                let f = Closure::wrap(Box::new(move || {
                    callback();
                    rcfclone.borrow_mut().take();
                }) as Box<dyn Fn()>);
                *rcf.borrow_mut() = Some(f);
                win.set_timeout_with_callback_and_timeout_and_arguments_0(rcf.borrow().as_ref().unwrap().as_ref().unchecked_ref(), 100).unwrap();
            }
        }
    } else {
        // TODO use a proper timer implementation instead. use the Timer crate?
        pub struct Timer;
        impl Timer {
            pub fn new() -> Self {
                Self{}
            }

            pub fn schedule_with_delay<T: Fn() + Send + 'static>(&self, delay: &Duration, callback: T) {
                let d2 = delay.clone();
                thread::spawn(move || {
                    thread::sleep(d2.to_std().unwrap());
                    callback();
                });
            }
        }
    }
}
