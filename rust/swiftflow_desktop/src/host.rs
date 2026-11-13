use crate::file_dialog;
use crate::input::{
    safe_area_points, scroll_delta_pixels, LIFECYCLE_BACKGROUND, LIFECYCLE_FOREGROUND,
    LIFECYCLE_TERMINATE,
};
use crate::text_input;
use crate::{SFDesktopCallbacks, SFDesktopSurfaceInfo};
use std::ffi::CString;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

/// The modifier keys held, as `SF_MOD_*` bits, for `sf_desktop_modifiers`.
static MODIFIERS: AtomicU32 = AtomicU32::new(0);

pub fn current_modifiers() -> u32 {
    MODIFIERS.load(Ordering::Relaxed)
}
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalPosition;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, Ime, MouseButton, Touch, TouchPhase, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::keyboard::ModifiersState;
use winit::window::{Window, WindowId};

#[cfg(target_os = "macos")]
use winit::platform::macos::WindowAttributesExtMacOS;

const FRAME_INTERVAL: Duration = Duration::from_nanos(1_000_000_000 / 60);

pub struct DesktopApp {
    title: String,
    logical_size: (f64, f64),
    min_size: (f64, f64),
    callbacks: SFDesktopCallbacks,

    window: Option<Arc<Window>>,
    started: Instant,
    last_frame: Option<Instant>,

    cursor: (f32, f32),
    pointer_is_down: bool,

    /// The touch (or Windows pen) driving the pointer, while one is down.
    touch_id: Option<u64>,

    /// Whether the middle button is down, panning.
    middle_is_down: bool,

    modifiers: ModifiersState,

    cursor_area_seen: u32,
}

impl DesktopApp {
    pub fn new(
        title: String,
        width: f64,
        height: f64,
        min_width: f64,
        min_height: f64,
        callbacks: SFDesktopCallbacks,
    ) -> Self {
        Self {
            title,
            logical_size: (width, height),
            min_size: (min_width, min_height),
            callbacks,
            window: None,
            started: Instant::now(),
            last_frame: None,
            cursor: (0.0, 0.0),
            pointer_is_down: false,
            touch_id: None,
            middle_is_down: false,
            modifiers: ModifiersState::empty(),
            cursor_area_seen: 0,
        }
    }

    /// A finger on a touchscreen, or on Windows a pen (winit turns Windows
    /// Ink into touches and holds back the mouse messages that would
    /// otherwise follow). The first contact drives the pointer; the others
    /// are ignored until there is a multi-touch path to send them down.
    fn touch(&mut self, touch: Touch) {
        let (x, y) = (touch.location.x as f32, touch.location.y as f32);

        #[cfg(target_os = "windows")]
        match crate::pen::windows::read(touch.id) {
            Some(sample) => crate::pen::store(sample),
            None => crate::pen::clear(),
        }

        match touch.phase {
            TouchPhase::Started => {
                if self.touch_id.is_some() || self.pointer_is_down {
                    return;
                }
                self.touch_id = Some(touch.id);
                self.cursor = (x, y);
                self.pointer_is_down = true;
                if let Some(down) = self.callbacks.pointer_down {
                    down(x, y, self.now());
                }
            }
            TouchPhase::Moved => {
                // A pen hovering over a Windows tablet sends moves under an
                // id that never started; those only move the cursor.
                self.cursor = (x, y);
                if self.touch_id != Some(touch.id) {
                    return;
                }
                if let Some(moved) = self.callbacks.pointer_moved {
                    moved(x, y, self.now());
                }
            }
            TouchPhase::Ended | TouchPhase::Cancelled => {
                if self.touch_id != Some(touch.id) {
                    return;
                }
                self.touch_id = None;
                self.cursor = (x, y);
                self.pointer_is_down = false;
                if let Some(up) = self.callbacks.pointer_up {
                    up(x, y, self.now());
                }
            }
        }
    }

    fn navigate(&self, kind: u32, a: f32, b: f32) {
        if let Some(navigate) = self.callbacks.navigate {
            navigate(kind, self.cursor.0, self.cursor.1, a, b);
        }
    }

    fn now(&self) -> f64 {
        self.started.elapsed().as_secs_f64()
    }

    fn notify_resize(&self, window: &Window) {
        let Some(resized) = self.callbacks.resized else {
            return;
        };
        let size = window.inner_size();
        let (safe_top, safe_bottom, safe_leading, safe_trailing) = safe_area_points();
        resized(SFDesktopSurfaceInfo {
            width: size.width.max(1),
            height: size.height.max(1),
            scale: window.scale_factor() as f32,
            safe_top,
            safe_bottom,
            safe_leading,
            safe_trailing,
        });
    }
}

impl ApplicationHandler for DesktopApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let mut attributes = Window::default_attributes()
            .with_title(&self.title)
            .with_inner_size(LogicalSize::new(self.logical_size.0, self.logical_size.1));

        if self.min_size.0 > 0.0 && self.min_size.1 > 0.0 {
            attributes =
                attributes.with_min_inner_size(LogicalSize::new(self.min_size.0, self.min_size.1));
        }

        #[cfg(target_os = "macos")]
        let attributes = {
            use winit::platform::macos::WindowAttributesExtMacOS;
            attributes
                .with_titlebar_transparent(true)
                .with_fullsize_content_view(true)
                .with_title_hidden(true)
        };

        let window = Arc::new(
            event_loop
                .create_window(attributes)
                .expect("could not create the platform window"),
        );

        println!("Offsetting traffic lights on macos.");
        #[cfg(target_os = "macos")]
        crate::macos::set_traffic_light_inset(&window, 20.0, 20.0);

        #[cfg(target_os = "macos")]
        crate::pen::macos::install();

        let size = window.inner_size();

        swiftflow_wgpu::init_with_target(
            window.clone().into(),
            size.width.max(1),
            size.height.max(1),
        );

        self.notify_resize(&window);
        self.window = Some(window);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                if let Some(lifecycle) = self.callbacks.lifecycle {
                    lifecycle(LIFECYCLE_TERMINATE);
                }
                event_loop.exit();
            }

            WindowEvent::Resized(size) => {
                swiftflow_wgpu::swiftflow_resize(size.width.max(1), size.height.max(1));
                if let Some(window) = self.window.clone() {
                    self.notify_resize(&window);
                }
            }

            WindowEvent::ScaleFactorChanged { .. } => {
                if let Some(window) = self.window.clone() {
                    self.notify_resize(&window);
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                let previous = self.cursor;
                self.cursor = (position.x as f32, position.y as f32);

                if self.middle_is_down {
                    self.navigate(
                        crate::SF_NAVIGATE_PAN,
                        self.cursor.0 - previous.0,
                        self.cursor.1 - previous.1,
                    );
                } else if self.pointer_is_down {
                    if let Some(moved) = self.callbacks.pointer_moved {
                        moved(self.cursor.0, self.cursor.1, self.now());
                    }
                } else if let Some(hover) = self.callbacks.hover {
                    hover(self.cursor.0, self.cursor.1, self.now());
                }
            }

            WindowEvent::MouseInput { state, button: MouseButton::Middle, .. } => {
                self.middle_is_down = state == ElementState::Pressed;
                if !self.middle_is_down {
                    self.navigate(crate::SF_NAVIGATE_END, 0.0, 0.0);
                }
            }

            WindowEvent::PinchGesture { delta, phase, .. } => {
                if delta.is_finite() {
                    self.navigate(crate::SF_NAVIGATE_MAGNIFY, delta as f32, 0.0);
                }
                if matches!(phase, TouchPhase::Ended | TouchPhase::Cancelled) {
                    self.navigate(crate::SF_NAVIGATE_END, 0.0, 0.0);
                }
            }

            WindowEvent::RotationGesture { delta, phase, .. } => {
                // winit gives degrees, counterclockwise; navigation turns
                // clockwise in radians, the way the screen's y runs down.
                if delta.is_finite() {
                    self.navigate(crate::SF_NAVIGATE_ROTATE, -delta.to_radians(), 0.0);
                }
                if matches!(phase, TouchPhase::Ended | TouchPhase::Cancelled) {
                    self.navigate(crate::SF_NAVIGATE_END, 0.0, 0.0);
                }
            }

            WindowEvent::Touch(touch) => self.touch(touch),

            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } => {
                let (x, y) = self.cursor;
                let t = self.now();
                match state {
                    ElementState::Pressed => {
                        self.pointer_is_down = true;
                        if let Some(down) = self.callbacks.pointer_down {
                            down(x, y, t);
                        }
                    }
                    ElementState::Released => {
                        self.pointer_is_down = false;
                        if let Some(up) = self.callbacks.pointer_up {
                            up(x, y, t);
                        }
                    }
                }
            }

            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Right,
                ..
            } => {
                if let Some(secondary) = self.callbacks.secondary_click {
                    let (x, y) = self.cursor;
                    secondary(x, y, self.now());
                }
            }

            WindowEvent::MouseWheel { delta, phase, .. } => {
                if let Some(scroll) = self.callbacks.scroll {
                    let scale = self
                        .window
                        .as_ref()
                        .map(|w| w.scale_factor() as f32)
                        .unwrap_or(1.0);
                    let (dx, dy) = scroll_delta_pixels(delta, scale);
                    scroll(self.cursor.0, self.cursor.1, dx, dy, phase as u32);
                }
            }

            WindowEvent::Focused(focused) => {
                if let Some(lifecycle) = self.callbacks.lifecycle {
                    lifecycle(if focused {
                        LIFECYCLE_FOREGROUND
                    } else {
                        LIFECYCLE_BACKGROUND
                    });
                }
            }

            WindowEvent::ModifiersChanged(new) => {
                self.modifiers = new.state();
                MODIFIERS.store(text_input::modifier_mask(self.modifiers), Ordering::Relaxed);
            }

            WindowEvent::KeyboardInput { event, .. } => {
                let Some(key) = self.callbacks.key else {
                    return;
                };
                let code = text_input::map_key(&event.logical_key);

                if code == text_input::SF_KEY_OTHER {
                    return;
                }
                key(
                    code,
                    text_input::modifier_mask(self.modifiers),
                    u32::from(event.state == ElementState::Pressed),
                    u32::from(event.repeat),
                );
            }

            WindowEvent::Ime(ime) => match ime {
                Ime::Enabled => {
                    if let Some(cb) = self.callbacks.ime_enabled {
                        cb(1);
                    }
                }
                Ime::Disabled => {
                    if let Some(cb) = self.callbacks.ime_enabled {
                        cb(0);
                    }
                }
                Ime::Preedit(text, cursor) => {
                    let Some(cb) = self.callbacks.ime_preedit else {
                        return;
                    };

                    let (begin, end) = cursor.map_or((-1, -1), |(b, e)| (b as i32, e as i32));
                    match CString::new(text) {
                        Ok(c) => cb(c.as_ptr(), begin, end),
                        Err(_) => cb(c"".as_ptr(), -1, -1),
                    }
                }
                Ime::Commit(text) => {
                    let Some(cb) = self.callbacks.ime_commit else {
                        return;
                    };
                    if let Ok(c) = CString::new(text) {
                        cb(c.as_ptr());
                    }
                }
            },

            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let dt = self
                    .last_frame
                    .map(|last| now.duration_since(last).as_secs_f32())
                    .unwrap_or(FRAME_INTERVAL.as_secs_f32());
                self.last_frame = Some(now);

                if let Some(frame) = self.callbacks.frame {
                    frame(dt);
                }
            }

            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(ControlFlow::WaitUntil(Instant::now() + FRAME_INTERVAL));
        if let Some(window) = &self.window {
            if let Some(allowed) = text_input::take_ime_allowed() {
                window.set_ime_allowed(allowed);
            }
            if let Some((x, y, w, h)) = text_input::take_cursor_area(&mut self.cursor_area_seen) {
                window.set_ime_cursor_area(
                    winit::dpi::PhysicalPosition::new(x, y),
                    winit::dpi::PhysicalSize::new(w, h),
                );
            }
            if file_dialog::take_request() {
                // Blocks the event loop while the chooser is up. That's what a
                // native modal does anyway, but rendering stops with it — so
                // this belongs here and nowhere hot.
                let picked = rfd::FileDialog::new()
                    .set_title("Open Folder")
                    .pick_folder()
                    .map(|path| path.to_string_lossy().into_owned());
                file_dialog::store_picked(picked);
            }
            if let Some(extensions) = file_dialog::take_file_request() {
                let mut dialog = rfd::FileDialog::new().set_title("Open");
                if !extensions.is_empty() {
                    dialog = dialog.add_filter("Documents", &extensions);
                }
                let picked = dialog.pick_file().map(|path| path.to_string_lossy().into_owned());
                file_dialog::store_picked_file(picked);
            }
            window.request_redraw();
        }
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        self.window = None;
    }
}
