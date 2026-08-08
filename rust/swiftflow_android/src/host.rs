#![cfg(target_os = "android")]

use crate::ime::{ImeBridge, ImeState};
use crate::input::{
    frame_delta_seconds, frame_interval_seconds, Phase, PointerAction, TouchTracker,
    LIFECYCLE_BACKGROUND, LIFECYCLE_FOREGROUND, LIFECYCLE_TERMINATE,
};
use crate::jni_metrics::{self, AndroidMetrics};
use crate::pacing;
use crate::text_input;
use crate::{SFAndroidCallbacks, SFAndroidSurfaceInfo};
use std::sync::Arc;
use std::time::{Duration, Instant};
use swiftflow_core::sflog;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, TouchPhase, WindowEvent};
use winit::keyboard::ModifiersState;
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::window::{Window, WindowId};

pub struct AndroidHost {
    callbacks: SFAndroidCallbacks,

    window: Option<Arc<Window>>,

    surface_live: bool,

    metrics: AndroidMetrics,
    touches: TouchTracker,

    started: Instant,

    last_vsync_ns: Option<i64>,

    pending_vsync_ns: Option<i64>,

    last_frame: Option<Instant>,
    paced_by_choreographer: bool,

    modifiers: ModifiersState,

    ime: ImeBridge,
}

impl AndroidHost {
    pub fn new(callbacks: SFAndroidCallbacks) -> Self {
        Self {
            callbacks,
            window: None,
            surface_live: false,
            metrics: AndroidMetrics::default(),
            touches: TouchTracker::default(),
            started: Instant::now(),
            last_vsync_ns: None,
            pending_vsync_ns: None,
            last_frame: None,

            paced_by_choreographer: pacing::is_available(),
            modifiers: ModifiersState::empty(),
            ime: ImeBridge::new(),
        }
    }

    #[cfg(target_os = "android")]
    fn pump_ime(&mut self, focus_changed: Option<bool>) {
        let Some(app) = crate::entry::android_app() else {
            return;
        };
        if let Some(allowed) = focus_changed {

            let update = self.ime.set_enabled(allowed);
            self.dispatch_ime(app, update);
        }
        if !self.ime.is_enabled() {
            return;
        }

        let state = app.text_input_state();
        let update = self.ime.poll(&ImeState {
            text: state.text,
            compose: state
                .compose_region
                .map(|region| (region.start, region.end)),
        });
        self.dispatch_ime(app, update);
    }

    #[cfg(target_os = "android")]
    fn dispatch_ime(&mut self, app: &android_activity::AndroidApp, update: crate::ime::ImeUpdate) {
        use std::ffi::CString;

        if let (Some(text), Some(cb)) = (&update.commit, self.callbacks.ime_commit) {
            if let Ok(c) = CString::new(text.as_str()) {
                cb(c.as_ptr());
            }
        }
        if let (Some(text), Some(cb)) = (&update.preedit, self.callbacks.ime_preedit) {

            let end = text.len() as i32;
            match CString::new(text.as_str()) {
                Ok(c) => cb(c.as_ptr(), end, end),

                Err(_) => cb(c"".as_ptr(), -1, -1),
            }
        }
        if update.reset_buffer {

            app.set_text_input_state(android_activity::input::TextInputState {
                text: String::new(),
                selection: android_activity::input::TextSpan { start: 0, end: 0 },
                compose_region: None,
            });
        }
    }

    fn now(&self) -> f64 {
        self.started.elapsed().as_secs_f64()
    }

    fn fallback_interval(&self) -> Duration {
        Duration::from_secs_f32(frame_interval_seconds(self.metrics.refresh_rate))
    }

    fn refresh_metrics(&mut self, scale: f32) {
        let Some(app) = crate::entry::android_app() else {
            return;
        };
        self.metrics = jni_metrics::read(app.vm_as_ptr(), app.activity_as_ptr().cast(), scale);
    }

    fn notify_resize(&self, window: &Window) {
        let Some(resized) = self.callbacks.resized else {
            return;
        };
        let size = window.inner_size();
        resized(SFAndroidSurfaceInfo {
            width: size.width.max(1),
            height: size.height.max(1),
            scale: window.scale_factor() as f32,
            safe_top: self.metrics.safe_top,
            safe_bottom: self.metrics.safe_bottom,
            safe_leading: self.metrics.safe_leading,
            safe_trailing: self.metrics.safe_trailing,
            corner_radius: self.metrics.corner_radius,
        });
    }

    fn lifecycle(&self, event: u32) {
        if let Some(cb) = self.callbacks.lifecycle {
            cb(event);
        }
    }

    fn draw(&mut self, dt: f32) {
        if let Some(frame) = self.callbacks.frame {
            frame(dt);
        }
    }
}

impl ApplicationHandler for AndroidHost {

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let window = match event_loop.create_window(Window::default_attributes()) {
            Ok(window) => Arc::new(window),
            Err(e) => {
                sflog!("android host: could not adopt the native window: {e}");
                return;
            }
        };

        let size = window.inner_size();
        let scale = window.scale_factor() as f32;
        self.refresh_metrics(scale);

        swiftflow_wgpu::init_with_target(
            window.clone().into(),
            size.width.max(1),
            size.height.max(1),
        );
        self.surface_live = true;

        self.notify_resize(&window);
        self.window = Some(window);

        if self.paced_by_choreographer {
            pacing::start();
        }
        self.last_vsync_ns = None;
        self.last_frame = None;
        self.lifecycle(LIFECYCLE_FOREGROUND);
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        pacing::stop();

        self.touches.reset();
        self.surface_live = false;

        swiftflow_core::release_backend();
        self.window = None;

        self.lifecycle(LIFECYCLE_BACKGROUND);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested | WindowEvent::Destroyed => {
                self.lifecycle(LIFECYCLE_TERMINATE);
                event_loop.exit();
            }

            WindowEvent::Resized(size) => {
                if self.surface_live {
                    swiftflow_wgpu::swiftflow_resize(size.width.max(1), size.height.max(1));
                }
                if let Some(window) = self.window.clone() {

                    let scale = window.scale_factor() as f32;
                    self.refresh_metrics(scale);
                    self.notify_resize(&window);
                }
            }

            WindowEvent::ScaleFactorChanged { .. } => {
                if let Some(window) = self.window.clone() {
                    let scale = window.scale_factor() as f32;
                    self.refresh_metrics(scale);
                    self.notify_resize(&window);
                }
            }

            WindowEvent::Touch(touch) => {
                let phase = match touch.phase {
                    TouchPhase::Started => Phase::Started,
                    TouchPhase::Moved => Phase::Moved,
                    TouchPhase::Ended => Phase::Ended,
                    TouchPhase::Cancelled => Phase::Cancelled,
                };

                let (x, y) = (touch.location.x as f32, touch.location.y as f32);
                let t = self.now();
                match self.touches.update(touch.id, phase) {
                    PointerAction::Down => {
                        if let Some(cb) = self.callbacks.pointer_down {
                            cb(x, y, t);
                        }
                    }
                    PointerAction::Moved => {
                        if let Some(cb) = self.callbacks.pointer_moved {
                            cb(x, y, t);
                        }
                    }
                    PointerAction::Up => {
                        if let Some(cb) = self.callbacks.pointer_up {
                            cb(x, y, t);
                        }
                    }
                    PointerAction::Ignore => {}
                }
            }

            WindowEvent::ModifiersChanged(new) => {
                self.modifiers = new.state();
            }

            WindowEvent::KeyboardInput { event, .. } => {
                let code = text_input::map_key(&event.logical_key);
                let pressed = event.state == ElementState::Pressed;

                if pressed && !self.ime.is_enabled() {
                    if let Some(text) =
                        text_input::printable_text(code, event.text.as_deref())
                    {
                        if let Some(commit) = self.callbacks.ime_commit {
                            if let Ok(c) = std::ffi::CString::new(text) {
                                commit(c.as_ptr());
                            }
                        }
                    }
                }

                let Some(key) = self.callbacks.key else { return };
                if code == text_input::SF_KEY_OTHER {
                    return;
                }
                key(
                    code,
                    text_input::modifier_mask(self.modifiers),
                    u32::from(pressed),
                    u32::from(event.repeat),
                );
            }

            WindowEvent::RedrawRequested => {

                if !self.surface_live {
                    return;
                }
                let fallback = frame_interval_seconds(self.metrics.refresh_rate);
                let dt = if let Some(now_ns) = self.pending_vsync_ns.take() {
                    let dt = frame_delta_seconds(self.last_vsync_ns, now_ns, fallback);
                    self.last_vsync_ns = Some(now_ns);
                    dt
                } else if self.paced_by_choreographer {

                    fallback
                } else {
                    let now = Instant::now();
                    let dt = self
                        .last_frame
                        .map(|last| now.duration_since(last).as_secs_f32())
                        .unwrap_or(fallback);
                    self.last_frame = Some(now);
                    dt
                };
                self.draw(dt);
            }

            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {

        let Some(window) = self.window.clone() else {

            event_loop.set_control_flow(ControlFlow::Wait);
            return;
        };

        let ime_allowed = text_input::take_ime_allowed();
        if let Some(allowed) = ime_allowed {
            window.set_ime_allowed(allowed);
        }

        self.pump_ime(ime_allowed);

        if self.paced_by_choreographer {

            if let Some(ts) = pacing::take_frame() {
                self.pending_vsync_ns = Some(ts);
                window.request_redraw();
            }

            let backstop = self.fallback_interval() * 2;
            event_loop.set_control_flow(ControlFlow::WaitUntil(Instant::now() + backstop));
        } else {
            event_loop
                .set_control_flow(ControlFlow::WaitUntil(Instant::now() + self.fallback_interval()));
            window.request_redraw();
        }
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        pacing::stop();
        self.window = None;
        self.surface_live = false;
    }
}
