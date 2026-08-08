use std::ffi::CString;

#[cfg(any(target_os = "ios", target_os = "macos"))]
mod imp {
    use std::ffi::CString;

    extern "C" {
        fn asl_log(
            asl: *mut std::ffi::c_void,
            msg: *mut std::ffi::c_void,
            level: i32,
            format: *const std::ffi::c_char,
            ...
        );
    }

    const ASL_LEVEL_INFO: i32 = 5;

    pub fn write(msg: CString) {
        unsafe {
            asl_log(
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                ASL_LEVEL_INFO,
                msg.as_ptr(),
            );
        }
    }
}

#[cfg(target_os = "android")]
mod imp {
    use std::ffi::CString;

    extern "C" {
        fn __android_log_write(
            prio: std::ffi::c_int,
            tag: *const std::ffi::c_char,
            text: *const std::ffi::c_char,
        ) -> std::ffi::c_int;
    }

    const ANDROID_LOG_INFO: std::ffi::c_int = 4;

    pub fn write(msg: CString) {

        const TAG: &[u8] = b"SwiftFlow\0";
        unsafe {
            __android_log_write(
                ANDROID_LOG_INFO,
                TAG.as_ptr() as *const std::ffi::c_char,
                msg.as_ptr(),
            );
        }
    }
}

#[cfg(not(any(target_os = "ios", target_os = "macos", target_os = "android")))]
mod imp {
    use std::ffi::CString;

    pub fn write(msg: CString) {
        eprintln!("{}", msg.to_string_lossy());
    }
}

pub fn nslog(msg: &str) {
    let prefixed = format!("[SF] {}", msg);
    if let Ok(s) = CString::new(prefixed) {
        imp::write(s);
    }
}

#[macro_export]
macro_rules! sflog {
    ($($arg:tt)*) => {
        $crate::log::nslog(&format!($($arg)*));
    }
}
