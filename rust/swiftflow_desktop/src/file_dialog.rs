use std::ffi::CString;
use std::os::raw::c_char;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

static REQUESTED: AtomicBool = AtomicBool::new(false);
static PICKED: Mutex<Option<CString>> = Mutex::new(None);

/// Keeps the last returned string alive. The pointer handed to Swift has to
/// outlive the lock guard, so it lives here until the next pick replaces it —
/// Swift copies on the line after the call, so one generation is enough.
static RETURNED: Mutex<Option<CString>> = Mutex::new(None);

/// Asks for a chooser. Picked up by the event loop rather than run here: the
/// dialog has to be on the thread that owns the window.
pub fn request_open_folder() {
    REQUESTED.store(true, Ordering::Release);
}

pub fn take_request() -> bool {
    REQUESTED.swap(false, Ordering::Acquire)
}

pub fn store_picked(path: Option<String>) {
    let value = path.and_then(|p| CString::new(p).ok());
    if let Ok(mut slot) = PICKED.lock() {
        *slot = value;
    }
}

/// Null until a folder is chosen, then the path exactly once.
pub fn take_picked() -> *const c_char {
    let picked = PICKED.lock().ok().and_then(|mut slot| slot.take());
    let Some(value) = picked else {
        return std::ptr::null();
    };
    let Ok(mut returned) = RETURNED.lock() else {
        return std::ptr::null();
    };
    *returned = Some(value);
    returned.as_ref().map_or(std::ptr::null(), |c| c.as_ptr())
}
