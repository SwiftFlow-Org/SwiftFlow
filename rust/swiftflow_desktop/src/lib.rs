pub mod file_dialog;
mod host;
pub mod input;
pub mod text_input;

#[cfg(target_os = "macos")]
mod macos;

use std::ffi::{c_char, CStr};
use winit::event_loop::EventLoop;

#[repr(C)]
pub struct SFDesktopConfig {
    pub title: *const c_char,

    pub width: f64,
    pub height: f64,
    pub min_width: f64,
    pub min_height: f64,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct SFDesktopSurfaceInfo {
    pub width: u32,
    pub height: u32,

    pub scale: f32,

    pub safe_top: f32,
    pub safe_bottom: f32,
    pub safe_leading: f32,
    pub safe_trailing: f32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct SFDesktopCallbacks {
    pub frame: Option<extern "C" fn(f32)>,
    pub pointer_down: Option<extern "C" fn(f32, f32, f64)>,
    pub pointer_moved: Option<extern "C" fn(f32, f32, f64)>,
    pub pointer_up: Option<extern "C" fn(f32, f32, f64)>,

    pub scroll: Option<extern "C" fn(f32, f32, f32, f32, u32)>,
    pub resized: Option<extern "C" fn(SFDesktopSurfaceInfo)>,

    pub lifecycle: Option<extern "C" fn(u32)>,

    pub key: Option<extern "C" fn(u32, u32, u32, u32)>,

    pub ime_preedit: Option<extern "C" fn(*const c_char, i32, i32)>,

    pub ime_commit: Option<extern "C" fn(*const c_char)>,

    pub ime_enabled: Option<extern "C" fn(u32)>,
}

#[no_mangle]
pub extern "C" fn sf_desktop_set_ime_allowed(allowed: u32) {
    text_input::request_ime_allowed(allowed != 0);
}

#[no_mangle]
pub extern "C" fn sf_desktop_set_ime_cursor_area(x: f32, y: f32, width: f32, height: f32) {
    text_input::request_cursor_area(x, y, width, height);
}

#[no_mangle]
pub extern "C" fn sf_desktop_run(
    config: *const SFDesktopConfig,
    callbacks: *const SFDesktopCallbacks,
) {
    let config = unsafe { &*config };
    let callbacks = unsafe { *callbacks };
    let title = if config.title.is_null() {
        "SwiftFlow".to_string()
    } else {
        unsafe { CStr::from_ptr(config.title) }
            .to_string_lossy()
            .into_owned()
    };

    let event_loop = EventLoop::new().expect("could not create the platform event loop");
    let mut app = host::DesktopApp::new(
        title,
        config.width.max(1.0),
        config.height.max(1.0),
        config.min_width,
        config.min_height,
        callbacks,
    );

    if let Err(e) = event_loop.run_app(&mut app) {
        eprintln!("swiftflow: event loop exited with an error: {e}");
    }
}

#[no_mangle]
pub extern "C" fn sf_desktop_open_folder_dialog() {
    file_dialog::request_open_folder();
}

#[no_mangle]
pub extern "C" fn sf_desktop_take_picked_folder() -> *const c_char {
    file_dialog::take_picked()
}
