use eframe::epi::RepaintSignal;

type Repainter = std::sync::Arc<dyn RepaintSignal>;
cfg_if::cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        use eframe::wasm_bindgen::{prelude::Closure, JsCast};
        pub struct PeriodicUpdater {
            _closure: Closure<dyn Fn()>,
            handle: i32,
        }

        impl PeriodicUpdater {
            pub fn new(repaint_signal: Repainter) -> Self {
                let win = web_sys::window().unwrap();
                let f = Closure::wrap(Box::new(move || {
                    repaint_signal.request_repaint();
                }) as Box<dyn Fn()>);
                let handle = win.set_interval_with_callback_and_timeout_and_arguments_0(f.as_ref().unchecked_ref(), 100).unwrap();
                PeriodicUpdater {
                    _closure: f,
                    handle
                }
            }
        }


        impl Drop for PeriodicUpdater {
            fn drop(&mut self) {
                web_sys::window().unwrap().clear_interval_with_handle(self.handle);
            }
        }
    } else {
        use crossbeam::channel::{self, Sender};
        use std::{thread::JoinHandle, time::Duration};

        pub struct PeriodicUpdater {
            quitter: Sender<()>,
            join_handle: Option<JoinHandle<()>>,
        }

        impl PeriodicUpdater {
            pub fn new(repaint_signal: Repainter) -> Self {
                let (tx, rx) = channel::bounded(1);
                let join_handle = Some(std::thread::spawn(move || {
                        while rx.try_recv().is_err() {
                            std::thread::sleep(Duration::from_millis(100));
                            repaint_signal.request_repaint();
                        }
                    }));
                PeriodicUpdater{quitter: tx, join_handle}
            }
        }

        impl Drop for PeriodicUpdater {
            fn drop(&mut self) {
                self.quitter.send(()).unwrap();
                self.join_handle.take().unwrap().join().unwrap();
            }
        }
    }
}
