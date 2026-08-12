use objc2_app_kit::{NSView, NSWindow, NSWindowButton};
use objc2_foundation::NSPoint;
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::Window;

const BUTTONS: [NSWindowButton; 3] = [
    NSWindowButton::CloseButton,
    NSWindowButton::MiniaturizeButton,
    NSWindowButton::ZoomButton,
];

fn ns_window(window: &Window) -> Option<objc2::rc::Retained<NSWindow>> {
    let handle = window.window_handle().ok()?;
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
        return None;
    };
    // Safe on the main thread, which is where winit runs its handlers.
    let view: &NSView = unsafe { handle.ns_view.cast::<NSView>().as_ref() };
    view.window()
}

/// Moves the traffic lights to `(x, y)` measured from the window's top-left.
///
/// AppKit re-lays the titlebar out on resize and on entering or leaving
/// fullscreen, so this has to be called again after those, not once at
/// creation.
pub fn set_traffic_light_inset(window: &Window, x: f64, y: f64) {
    let Some(ns_window) = ns_window(window) else {
        return;
    };
    let Some(close) = ns_window.standardWindowButton(NSWindowButton::CloseButton) else {
        return;
    };
    let Some(container) = (unsafe { close.superview() }) else {
        return;
    };

    let frame = close.frame();
    let dx = x - frame.origin.x;
    let dy = (container.frame().size.height - y - frame.size.height) - frame.origin.y;

    for kind in BUTTONS {
        if let Some(button) = ns_window.standardWindowButton(kind) {
            let origin = button.frame().origin;
            unsafe {
                button.setFrameOrigin(NSPoint::new(origin.x + dx, origin.y + dy));
            }
        }
    }
}
