use core::ffi::c_void;
use egui_baseview::EguiWindow;
use raw_window_handle::{HasRawWindowHandle, RawWindowHandle};
use std::sync::Arc;
use vst::editor::Editor;

pub struct PistolhotEditor {
    window_handle: Option<baseview::WindowHandle>,
    params: Arc<synth::Params>,
}

impl PistolhotEditor {
    pub fn new(params: Arc<synth::Params>) -> Self {
        Self {
            window_handle: None,
            params,
        }
    }
}

const WINDOW_DIMENSIONS: (i32, i32) = (300, 200);

struct State;

impl Editor for PistolhotEditor {
    fn size(&self) -> (i32, i32) {
        (WINDOW_DIMENSIONS.0 as i32, WINDOW_DIMENSIONS.1 as i32)
    }

    fn position(&self) -> (i32, i32) {
        (0, 0)
    }

    fn open(&mut self, parent: *mut c_void) -> bool {
        // TODO also check the WindowHandle is_some method?
        if self.window_handle.is_some() {
            return false;
        }
        assert!(parent != std::ptr::null_mut());
        let settings = egui_baseview::Settings {
            window: baseview::WindowOpenOptions {
                title: "Pistolhot".to_string(),
                size: baseview::Size::new(WINDOW_DIMENSIONS.0 as f64, WINDOW_DIMENSIONS.1 as f64),
                scale: baseview::WindowScalePolicy::SystemScaleFactor,
            },
            render_settings: egui_baseview::RenderSettings::default(),
        };
        let params = self.params.clone();
        self.window_handle = Some(EguiWindow::open_parented(
            &VstParent(parent),
            settings,
            (),
            // build
            |_ctx: &egui::CtxRef, _queue: &mut egui_baseview::Queue, _state: &mut ()| {},
            // update
            move |egui_ctx: &egui::CtxRef, queue: &mut egui_baseview::Queue, _state: &mut ()| {
                egui::CentralPanel::default().show(&egui_ctx, |ui| {
                    ui.heading("Pistolhot");
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.label("distortion:");
                            let mut distorsion = params.distortion.load();
                            ui.add(egui::Slider::new(&mut distorsion, 1f32..=10f32));
                            params.distortion.store(distorsion);
                        });
                        ui.horizontal(|ui| {
                            ui.label("chaoticity:");
                            let mut chaoticity = params.chaoticity.load();
                            ui.add(egui::Slider::new(&mut chaoticity, 0.0001f32..=0.999f32));
                            params.chaoticity.store(chaoticity);
                        });
                    });
                });
            },
        ));
        true
    }

    fn is_open(&mut self) -> bool {
        self.window_handle.is_some()
    }

    fn close(&mut self) {
        if let Some(mut window_handle) = self.window_handle.take() {
            window_handle.close();
        }
    }
}

struct VstParent(*mut ::std::ffi::c_void);

#[cfg(target_os = "macos")]
unsafe impl HasRawWindowHandle for VstParent {
    fn raw_window_handle(&self) -> RawWindowHandle {
        let mut handle = raw_window_handle::AppKitHandle::empty();
        handle.ns_view = self.0; // as *mut ::std::ffi::c_void;
        RawWindowHandle::AppKit(handle)
    }
}

#[cfg(target_os = "windows")]
unsafe impl HasRawWindowHandle for VstParent {
    fn raw_window_handle(&self) -> RawWindowHandle {
        let mut handle = raw_window_handle::Win32Handle::empty();
        handle.hwnd = self.0; // as *mut ::std::ffi::c_void;
        RawWindowHandle::AppKit(handle)
    }
}
