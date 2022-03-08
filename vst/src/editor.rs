use core::ffi::c_void;
use egui_baseview::EguiWindow;
use raw_window_handle::{HasRawWindowHandle, RawWindowHandle};
use std::sync::Arc;
use vst::editor::Editor;
use synth::params_gui;

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

const WINDOW_WIDTH: i32 = 300;
const WINDOW_HEIGHT: i32 = 200;

impl Editor for PistolhotEditor {
    fn size(&self) -> (i32, i32) {
        (WINDOW_WIDTH, WINDOW_HEIGHT)
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
        let settings = baseview::WindowOpenOptions {
            title: "Pistolhot".to_string(),
            size: baseview::Size::new(WINDOW_WIDTH as f64, WINDOW_HEIGHT as f64),
            scale: baseview::WindowScalePolicy::SystemScaleFactor,
            gl_config: Some(baseview::gl::GlConfig::default()),
        };
        let params = self.params.clone();
        self.window_handle = EguiWindow::open_parented(
            &VstParent(parent),
            settings,
            (),
            // build
            |_ctx: &egui::Context, _queue: &mut egui_baseview::Queue, _state: &mut ()| {},
            // update
            move |egui_ctx: &egui::Context, _queue: &mut egui_baseview::Queue, _state: &mut ()| {
                egui::CentralPanel::default().show(&egui_ctx, |ui| {
                    ui.heading("Pistolhot");
                    ui.group(|ui| {
                        params_gui(ui, &params);
                    });
                });
            },
        );
        assert!(self.window_handle.is_some());
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
        handle.ns_view = self.0;
        RawWindowHandle::AppKit(handle)
    }
}

#[cfg(target_os = "windows")]
unsafe impl HasRawWindowHandle for VstParent {
    fn raw_window_handle(&self) -> RawWindowHandle {
        let mut handle = raw_window_handle::Win32Handle::empty();
        handle.hwnd = self.0;
        RawWindowHandle::AppKit(handle)
    }
}
