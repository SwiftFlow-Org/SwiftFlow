//! The pen, as the host last read it.
//!
//! winit 0.30 reports a tablet pen as a plain mouse on macOS and as a touch
//! with a force on Windows; neither carries tilt or rotation, and neither
//! says whether the eraser end is down. So each platform reads its own
//! tablet API beside winit (`macos` and `windows` below) and stores the
//! reading here. When winit then delivers the matching pointer event and
//! the host forwards it to Swift, Swift calls `sf_desktop_pen_sample` and
//! attaches the reading to the event.
//!
//! Only the latest reading is kept. That is enough because the tablet
//! readers see each event before winit dispatches it, and the host hands
//! that event to Swift synchronously, before the next one is read.

use std::sync::Mutex;

pub const SF_PEN_NONE: u32 = 0;
pub const SF_PEN_PEN: u32 = 1;
pub const SF_PEN_ERASER: u32 = 2;

/// One pen reading, as Swift receives it. Mirrors `SFPenSample` in
/// SwiftFlowMetal.h.
#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct SFPenSample {
    /// `SF_PEN_PEN` or `SF_PEN_ERASER`; `SF_PEN_NONE` when no pen is near.
    pub tool: u32,
    /// 0 to 1.
    pub pressure: f32,
    /// Lean from upright, in radians: positive x leans right, positive y
    /// leans toward the bottom of the screen.
    pub tilt_x: f32,
    pub tilt_y: f32,
    /// Turn about the pen's own axis, in radians.
    pub rotation: f32,
}

static CURRENT: Mutex<SFPenSample> = Mutex::new(SFPenSample {
    tool: SF_PEN_NONE,
    pressure: 0.0,
    tilt_x: 0.0,
    tilt_y: 0.0,
    rotation: 0.0,
});

/// Records the pen's latest reading.
pub fn store(sample: SFPenSample) {
    *CURRENT.lock().unwrap() = sample;
}

/// Records that the pen has left the tablet's range.
pub fn clear() {
    *CURRENT.lock().unwrap() = SFPenSample::default();
}

/// The latest reading, or `None` when no pen is near.
pub fn current() -> Option<SFPenSample> {
    let sample = *CURRENT.lock().unwrap();
    (sample.tool != SF_PEN_NONE).then_some(sample)
}

/// Copies the pen's latest reading into `out` and returns 1, or returns 0
/// and leaves `out` alone when no pen is near.
#[no_mangle]
pub extern "C" fn sf_desktop_pen_sample(out: *mut SFPenSample) -> u32 {
    match current() {
        Some(sample) if !out.is_null() => {
            unsafe { *out = sample };
            1
        }
        _ => 0,
    }
}

/// Converts a tilt given as a fraction of the device's maximum lean (what
/// AppKit reports, -1 to 1) to radians. Wacom pens report up to 60° of lean,
/// so ±1 maps to ±60°.
pub fn tilt_fraction_to_radians(fraction: f32) -> f32 {
    fraction.clamp(-1.0, 1.0) * MAX_TILT
}

/// Wacom's pens measure up to 60° from upright.
pub const MAX_TILT: f32 = std::f32::consts::FRAC_PI_3;

#[cfg(target_os = "macos")]
pub mod macos {
    //! Reads the tablet from AppKit.
    //!
    //! A local event monitor sees every event the app dequeues before the
    //! window gets it, so it runs before winit's own handler for the same
    //! event. It reads the pen fields off mouse events that came from a
    //! tablet (subtype `TabletPoint`) and off pure tablet events, notes
    //! which end of the pen is in use from proximity events, and passes
    //! every event on unchanged.

    use super::*;
    use block2::RcBlock;
    use objc2_app_kit::{NSEvent, NSEventMask, NSEventSubtype, NSEventType, NSPointingDeviceType};
    use std::ptr::NonNull;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// The tool the last proximity event announced.
    static TOOL: AtomicU32 = AtomicU32::new(SF_PEN_PEN);

    /// Installs the monitor. Call once, on the main thread, after the app
    /// has started (winit's `resumed`).
    pub fn install() {
        let mask = NSEventMask::LeftMouseDown
            | NSEventMask::LeftMouseUp
            | NSEventMask::LeftMouseDragged
            | NSEventMask::MouseMoved
            | NSEventMask::TabletPoint
            | NSEventMask::TabletProximity;

        let block = RcBlock::new(|event: NonNull<NSEvent>| -> *mut NSEvent {
            read(unsafe { event.as_ref() });
            event.as_ptr()
        });

        // The monitor lives as long as the app; dropping the returned
        // token would not remove it anyway, removeMonitor: does.
        let monitor = unsafe { NSEvent::addLocalMonitorForEventsMatchingMask_handler(mask, &block) };
        std::mem::forget(monitor);
    }

    fn read(event: &NSEvent) {
        let kind = event.r#type();

        if kind == NSEventType::TabletProximity
            || (is_mouse(kind) && event.subtype() == NSEventSubtype::TabletProximity)
        {
            if event.isEnteringProximity() {
                let tool = if event.pointingDeviceType() == NSPointingDeviceType::Eraser {
                    SF_PEN_ERASER
                } else {
                    SF_PEN_PEN
                };
                TOOL.store(tool, Ordering::Relaxed);
            } else {
                clear();
            }
            return;
        }

        let from_tablet = kind == NSEventType::TabletPoint
            || (is_mouse(kind) && event.subtype() == NSEventSubtype::TabletPoint);
        if !from_tablet {
            // A mouse or trackpad moved while no pen was down. If the pen
            // left without a proximity event (some drivers skip it when
            // the app was not frontmost), this is where it stops counting.
            if is_mouse(kind) {
                clear();
            }
            return;
        }

        let tilt = event.tilt();
        store(SFPenSample {
            tool: TOOL.load(Ordering::Relaxed),
            pressure: event.pressure().clamp(0.0, 1.0),
            tilt_x: tilt_fraction_to_radians(tilt.x as f32),
            // AppKit's tilt y is positive toward the top of the tablet;
            // SwiftFlow's is positive toward the bottom of the screen.
            tilt_y: -tilt_fraction_to_radians(tilt.y as f32),
            rotation: event.rotation().to_radians(),
        });
    }

    fn is_mouse(kind: NSEventType) -> bool {
        kind == NSEventType::LeftMouseDown
            || kind == NSEventType::LeftMouseUp
            || kind == NSEventType::LeftMouseDragged
            || kind == NSEventType::MouseMoved
    }
}

#[cfg(target_os = "windows")]
pub mod windows {
    //! Reads the pen from Windows Ink.
    //!
    //! winit already handles `WM_POINTER*` and turns a pen into
    //! `WindowEvent::Touch` with its pressure as the force, and the pointer
    //! id as the touch id. The rest of the pen's state (tilt, rotation,
    //! eraser) is one `GetPointerPenInfo` call on that id away.

    use super::*;
    use windows_sys::Win32::UI::Input::Pointer::{GetPointerPenInfo, POINTER_PEN_INFO};
    use windows_sys::Win32::UI::WindowsAndMessaging::{PEN_FLAG_ERASER, PEN_FLAG_INVERTED};

    /// Pressure in `POINTER_PEN_INFO` runs from 0 to 1024.
    const MAX_PRESSURE: f32 = 1024.0;

    /// The pen reading for pointer `id`, or `None` when it is not a pen
    /// (a finger on a touchscreen).
    pub fn read(id: u64) -> Option<SFPenSample> {
        let mut info: POINTER_PEN_INFO = unsafe { std::mem::zeroed() };
        if unsafe { GetPointerPenInfo(id as u32, &mut info) } == 0 {
            return None;
        }
        let erasing = info.penFlags & (PEN_FLAG_ERASER | PEN_FLAG_INVERTED) != 0;
        Some(SFPenSample {
            tool: if erasing { SF_PEN_ERASER } else { SF_PEN_PEN },
            pressure: (info.pressure as f32 / MAX_PRESSURE).clamp(0.0, 1.0),
            // Windows gives whole degrees, -90 to 90, positive to the right
            // and toward the user.
            tilt_x: (info.tiltX as f32).to_radians(),
            tilt_y: (info.tiltY as f32).to_radians(),
            rotation: (info.rotation as f32).to_radians(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // One test, because the reading is a process-wide global.
    #[test]
    fn a_stored_reading_comes_back_until_cleared() {
        clear();
        let mut out = SFPenSample { tool: 9, ..Default::default() };
        assert_eq!(sf_desktop_pen_sample(&mut out), 0, "no pen should read as none");
        assert_eq!(out.tool, 9, "out should be left alone");

        let sample = SFPenSample { tool: SF_PEN_ERASER, pressure: 0.5, tilt_x: 0.1, tilt_y: -0.2, rotation: 1.0 };
        store(sample);
        assert_eq!(sf_desktop_pen_sample(&mut out), 1);
        assert_eq!(out, sample);
        clear();
        assert_eq!(sf_desktop_pen_sample(&mut out), 0);
    }

    #[test]
    fn full_tilt_is_sixty_degrees_either_way() {
        assert!((tilt_fraction_to_radians(1.0) - 60f32.to_radians()).abs() < 1e-6);
        assert!((tilt_fraction_to_radians(-2.0) + 60f32.to_radians()).abs() < 1e-6);
    }
}
