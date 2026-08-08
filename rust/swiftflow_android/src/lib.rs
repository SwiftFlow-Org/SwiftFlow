pub mod ime;
pub mod input;
pub mod text_input;

#[cfg(target_os = "android")]
mod assets;
#[cfg(target_os = "android")]
mod host;
#[cfg(target_os = "android")]
mod jni_metrics;
#[cfg(target_os = "android")]
mod pacing;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct SFAndroidSurfaceInfo {

    pub width: u32,
    pub height: u32,

    pub scale: f32,

    pub safe_top: f32,
    pub safe_bottom: f32,
    pub safe_leading: f32,
    pub safe_trailing: f32,

    pub corner_radius: f32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct SFAndroidCallbacks {

    pub frame: Option<extern "C" fn(f32)>,
    pub pointer_down: Option<extern "C" fn(f32, f32, f64)>,
    pub pointer_moved: Option<extern "C" fn(f32, f32, f64)>,
    pub pointer_up: Option<extern "C" fn(f32, f32, f64)>,
    pub resized: Option<extern "C" fn(SFAndroidSurfaceInfo)>,

    pub lifecycle: Option<extern "C" fn(u32)>,

    pub assets_path: Option<extern "C" fn(*const std::ffi::c_char)>,

    pub key: Option<extern "C" fn(u32, u32, u32, u32)>,

    pub ime_preedit: Option<extern "C" fn(*const std::ffi::c_char, i32, i32)>,

    pub ime_commit: Option<extern "C" fn(*const std::ffi::c_char)>,

    pub ime_enabled: Option<extern "C" fn(u32)>,
}

#[cfg(target_os = "android")]
mod entry {
    use super::SFAndroidCallbacks;
    use android_activity::AndroidApp;
    use std::sync::OnceLock;
    use swiftflow_core::sflog;
    use winit::event_loop::EventLoop;
    use winit::platform::android::EventLoopBuilderExtAndroid;

    static ANDROID_APP: OnceLock<AndroidApp> = OnceLock::new();

    pub fn android_app() -> Option<&'static AndroidApp> {
        ANDROID_APP.get()
    }

    extern "C" {

        fn sf_android_main();
    }

    #[no_mangle]
    fn android_main(app: AndroidApp) {
        sflog!("android_main: starting");

        super::assets::unpack(&app);
        let _ = ANDROID_APP.set(app);
        unsafe { sf_android_main() };
    }

    #[no_mangle]
    pub extern "C" fn sf_android_set_ime_allowed(allowed: u32) {
        super::text_input::request_ime_allowed(allowed != 0);
    }

    #[no_mangle]
    pub extern "C" fn sf_android_set_ime_cursor_area(_x: f32, _y: f32, _w: f32, _h: f32) {}

    #[no_mangle]
    pub extern "C" fn sf_android_run(callbacks: SFAndroidCallbacks) {
        let Some(app) = android_app() else {
            sflog!("sf_android_run: called before android_main — nothing to run");
            return;
        };

        if let Some(push) = callbacks.assets_path {
            if let Some(path) = super::assets::unpacked_path() {
                push(path.as_ptr());
            }
        }

        let event_loop = match EventLoop::builder().with_android_app(app.clone()).build() {
            Ok(loop_) => loop_,
            Err(e) => {
                sflog!("sf_android_run: could not create the event loop: {e}");
                return;
            }
        };

        let mut host = super::host::AndroidHost::new(callbacks);
        if let Err(e) = event_loop.run_app(&mut host) {
            sflog!("sf_android_run: event loop exited with an error: {e}");
        }
    }
}
