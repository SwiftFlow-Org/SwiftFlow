use crate::input::{
    safe_area_points, scroll_delta_pixels, LIFECYCLE_BACKGROUND, LIFECYCLE_FOREGROUND,
    LIFECYCLE_TERMINATE,
};
use crate::text_input;
use crate::{SFDesktopCallbacks, SFDesktopSurfaceInfo};
use std::ffi::CString;
use std::sync::Arc;
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, Ime, MouseButton, WindowEvent};
use winit::keyboard::ModifiersState;
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::window::{Window, WindowId};

const FRAME_INTERVAL: Duration = Duration::from_nanos(1_000_000_000 / 60);

pub struct DesktopApp {
    title: String,
    logical_size: (u32, u32),
    callbacks: SFDesktopCallbacks,

    window: Option<Arc<Window>>,
    started: Instant,
    last_frame: Option<Instant>,

    cursor: (f32, f32),
    pointer_is_down: bool,

    modifiers: ModifiersState,

    cursor_area_seen: u32,
}

impl DesktopApp {
    pub fn new(
        title: String,
        width: u32,
        height: u32,
        callbacks: SFDesktopCallbacks,
    ) -> Self {
        Self {
            title,
            logical_size: (width, height),
            callbacks,
            window: None,
            started: Instant::now(),
            last_frame: None,
            cursor: (0.0, 0.0),
            pointer_is_down: false,
            modifiers: ModifiersState::empty(),
            cursor_area_seen: 0,
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

        let attributes = Window::default_attributes()
            .with_title(&self.title)
            .with_inner_size(LogicalSize::new(self.logical_size.0, self.logical_size.1));

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

        let size = window.inner_size();

        swiftflow_wgpu::init_with_target(
            window.clone().into(),
            size.width.max(1),
            size.height.max(1),
        );

        self.notify_resize(&window);
        self.window = Some(window);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _id: WindowId,
        event: WindowEvent,
    ) {
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
                self.cursor = (position.x as f32, position.y as f32);

                if self.pointer_is_down {
                    if let Some(moved) = self.callbacks.pointer_moved {
                        moved(self.cursor.0, self.cursor.1, self.now());
                    }
                }
            }

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
            }

            WindowEvent::KeyboardInput { event, .. } => {
                let Some(key) = self.callbacks.key else { return };
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
                    let Some(cb) = self.callbacks.ime_preedit else { return };

                    let (begin, end) = cursor.map_or((-1, -1), |(b, e)| (b as i32, e as i32));
                    match CString::new(text) {
                        Ok(c) => cb(c.as_ptr(), begin, end),
                        Err(_) => cb(c"".as_ptr(), -1, -1),
                    }
                }
                Ime::Commit(text) => {
                    let Some(cb) = self.callbacks.ime_commit else { return };
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
            window.request_redraw();
        }
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {

        self.window = None;
    }
}
