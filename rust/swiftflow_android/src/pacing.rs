#![cfg(target_os = "android")]

use std::os::raw::{c_long, c_void};
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicUsize, Ordering};

static FRAME_READY: AtomicBool = AtomicBool::new(false);

static FRAME_TIME_NS: AtomicI64 = AtomicI64::new(0);

static POSTED: AtomicBool = AtomicBool::new(false);

static RUNNING: AtomicBool = AtomicBool::new(false);

extern "C" {
    fn AChoreographer_getInstance() -> *mut c_void;
    fn AChoreographer_postFrameCallback(
        choreographer: *mut c_void,
        callback: extern "C" fn(frame_time_nanos: c_long, data: *mut c_void),
        data: *mut c_void,
    );

    fn ALooper_forThread() -> *mut c_void;
    fn ALooper_wake(looper: *mut c_void);
}

static LOOPER: AtomicUsize = AtomicUsize::new(0);

extern "C" fn on_vsync(frame_time_nanos: c_long, _data: *mut c_void) {
    FRAME_TIME_NS.store(frame_time_nanos as i64, Ordering::Relaxed);
    FRAME_READY.store(true, Ordering::Release);
    POSTED.store(false, Ordering::Relaxed);

    let looper = LOOPER.load(Ordering::Relaxed);
    if looper != 0 {
        unsafe { ALooper_wake(looper as *mut c_void) };
    }

    post();
}

pub fn start() {

    LOOPER.store(unsafe { ALooper_forThread() } as usize, Ordering::Relaxed);
    RUNNING.store(true, Ordering::Relaxed);
    post();
}

fn post() {
    if !RUNNING.load(Ordering::Relaxed) {
        return;
    }
    if POSTED.swap(true, Ordering::Relaxed) {
        return;
    }
    let instance = unsafe { AChoreographer_getInstance() };
    if instance.is_null() {

        POSTED.store(false, Ordering::Relaxed);
        return;
    }
    unsafe { AChoreographer_postFrameCallback(instance, on_vsync, std::ptr::null_mut()) };
}

pub fn stop() {
    RUNNING.store(false, Ordering::Relaxed);
    FRAME_READY.store(false, Ordering::Release);
}

pub fn is_available() -> bool {
    !unsafe { AChoreographer_getInstance() }.is_null()
}

pub fn take_frame() -> Option<i64> {
    if FRAME_READY.swap(false, Ordering::Acquire) {
        Some(FRAME_TIME_NS.load(Ordering::Relaxed))
    } else {
        None
    }
}
