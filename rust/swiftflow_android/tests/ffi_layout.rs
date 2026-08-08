use std::mem::{align_of, offset_of, size_of};
use swiftflow_android::{SFAndroidCallbacks, SFAndroidSurfaceInfo};

#[test]
fn surface_info_matches_the_c_header() {
    assert_eq!(size_of::<SFAndroidSurfaceInfo>(), 32, "struct size");
    assert_eq!(align_of::<SFAndroidSurfaceInfo>(), 4, "struct alignment");

    let expected: &[(&str, usize)] = &[
        ("width", 0),
        ("height", 4),
        ("scale", 8),
        ("safe_top", 12),
        ("safe_bottom", 16),
        ("safe_leading", 20),
        ("safe_trailing", 24),
        ("corner_radius", 28),
    ];
    let actual: &[(&str, usize)] = &[
        ("width", offset_of!(SFAndroidSurfaceInfo, width)),
        ("height", offset_of!(SFAndroidSurfaceInfo, height)),
        ("scale", offset_of!(SFAndroidSurfaceInfo, scale)),
        ("safe_top", offset_of!(SFAndroidSurfaceInfo, safe_top)),
        ("safe_bottom", offset_of!(SFAndroidSurfaceInfo, safe_bottom)),
        ("safe_leading", offset_of!(SFAndroidSurfaceInfo, safe_leading)),
        ("safe_trailing", offset_of!(SFAndroidSurfaceInfo, safe_trailing)),
        ("corner_radius", offset_of!(SFAndroidSurfaceInfo, corner_radius)),
    ];
    assert_eq!(actual, expected);
}

#[test]
fn callbacks_match_the_c_header() {

    assert_eq!(size_of::<SFAndroidCallbacks>(), 88, "struct size");
    assert_eq!(align_of::<SFAndroidCallbacks>(), 8, "struct alignment");

    let expected: &[(&str, usize)] = &[
        ("frame", 0),
        ("pointer_down", 8),
        ("pointer_moved", 16),
        ("pointer_up", 24),
        ("resized", 32),
        ("lifecycle", 40),
        ("assets_path", 48),
        ("key", 56),
        ("ime_preedit", 64),
        ("ime_commit", 72),
        ("ime_enabled", 80),
    ];
    let actual: &[(&str, usize)] = &[
        ("frame", offset_of!(SFAndroidCallbacks, frame)),
        ("pointer_down", offset_of!(SFAndroidCallbacks, pointer_down)),
        ("pointer_moved", offset_of!(SFAndroidCallbacks, pointer_moved)),
        ("pointer_up", offset_of!(SFAndroidCallbacks, pointer_up)),
        ("resized", offset_of!(SFAndroidCallbacks, resized)),
        ("lifecycle", offset_of!(SFAndroidCallbacks, lifecycle)),
        ("assets_path", offset_of!(SFAndroidCallbacks, assets_path)),
        ("key", offset_of!(SFAndroidCallbacks, key)),
        ("ime_preedit", offset_of!(SFAndroidCallbacks, ime_preedit)),
        ("ime_commit", offset_of!(SFAndroidCallbacks, ime_commit)),
        ("ime_enabled", offset_of!(SFAndroidCallbacks, ime_enabled)),
    ];
    assert_eq!(actual, expected);
}

#[test]
fn an_absent_callback_is_a_null_pointer() {

    assert_eq!(
        size_of::<Option<extern "C" fn(f32)>>(),
        size_of::<extern "C" fn(f32)>()
    );
}
