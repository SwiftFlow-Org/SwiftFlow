#![cfg(target_os = "android")]

use jni::objects::{JObject, JValue};
use jni::sys::jobject;
use jni::{jni_sig, jni_str, Env, JavaVM};

use swiftflow_core::sflog;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct AndroidMetrics {
    pub safe_top: f32,
    pub safe_bottom: f32,
    pub safe_leading: f32,
    pub safe_trailing: f32,

    pub corner_radius: f32,

    pub refresh_rate: f32,
}

impl Default for AndroidMetrics {
    fn default() -> Self {
        Self {
            safe_top: 0.0,
            safe_bottom: 0.0,
            safe_leading: 0.0,
            safe_trailing: 0.0,
            corner_radius: 0.0,
            refresh_rate: 60.0,
        }
    }
}

pub fn read(vm_ptr: *mut std::ffi::c_void, activity_ptr: jobject, scale: f32) -> AndroidMetrics {
    let fallback = AndroidMetrics::default();
    if vm_ptr.is_null() || activity_ptr.is_null() {
        return fallback;
    }
    let scale = if scale > 0.0 { scale } else { 1.0 };

    let vm = unsafe { JavaVM::from_raw(vm_ptr.cast()) };

    vm.attach_current_thread(|env| {

        let activity = unsafe { JObject::from_raw(env, activity_ptr) };
        let metrics = collect(env, &activity, scale);
        Ok::<_, jni::errors::Error>(metrics)
    })
    .unwrap_or_else(|e| {
        sflog!("android metrics: could not attach to the JVM ({e})");
        fallback
    })
}

fn collect<'local>(env: &mut Env<'local>, activity: &JObject, scale: f32) -> AndroidMetrics {
    let mut out = AndroidMetrics::default();

    if let Some(hz) = refresh_rate(env, activity) {
        out.refresh_rate = hz;
    }

    let Some(insets) = root_insets(env, activity) else {
        return out;
    };

    if let Some((l, t, r, b)) = system_bar_insets(env, &insets) {
        out.safe_leading = l as f32 / scale;
        out.safe_top = t as f32 / scale;
        out.safe_trailing = r as f32 / scale;
        out.safe_bottom = b as f32 / scale;
    }

    if let Some(radius) = corner_radius(env, &insets) {
        out.corner_radius = radius as f32 / scale;
    }

    out
}

fn refresh_rate<'local>(env: &mut Env<'local>, activity: &JObject) -> Option<f32> {

    let display = checked(env, |env| {
        env.call_method(
            activity,
            jni_str!("getDisplay"),
            jni_sig!("()Landroid/view/Display;"),
            &[],
        )?
        .l()
    })?;
    if display.as_raw().is_null() {
        return None;
    }
    checked(env, |env| {
        env.call_method(&display, jni_str!("getRefreshRate"), jni_sig!("()F"), &[])?
            .f()
    })
}

fn root_insets<'local>(env: &mut Env<'local>, activity: &JObject) -> Option<JObject<'local>> {
    let window = checked(env, |env| {
        env.call_method(
            activity,
            jni_str!("getWindow"),
            jni_sig!("()Landroid/view/Window;"),
            &[],
        )?
        .l()
    })?;
    let decor = checked(env, |env| {
        env.call_method(
            &window,
            jni_str!("getDecorView"),
            jni_sig!("()Landroid/view/View;"),
            &[],
        )?
        .l()
    })?;

    let insets = checked(env, |env| {
        env.call_method(
            &decor,
            jni_str!("getRootWindowInsets"),
            jni_sig!("()Landroid/view/WindowInsets;"),
            &[],
        )?
        .l()
    })?;
    if insets.as_raw().is_null() {
        None
    } else {
        Some(insets)
    }
}

fn system_bar_insets<'local>(
    env: &mut Env<'local>,
    insets: &JObject,
) -> Option<(i32, i32, i32, i32)> {
    if let Some(v) = modern_insets(env, insets) {
        return Some(v);
    }
    fn legacy<'local>(env: &mut Env<'local>, insets: &JObject, name: &jni::strings::JNIStr) -> Option<i32> {
        checked(env, |env| {
            env.call_method(insets, name, jni_sig!("()I"), &[])?.i()
        })
    }
    Some((
        legacy(env, insets, jni_str!("getSystemWindowInsetLeft"))?,
        legacy(env, insets, jni_str!("getSystemWindowInsetTop"))?,
        legacy(env, insets, jni_str!("getSystemWindowInsetRight"))?,
        legacy(env, insets, jni_str!("getSystemWindowInsetBottom"))?,
    ))
}

fn modern_insets<'local>(env: &mut Env<'local>, insets: &JObject) -> Option<(i32, i32, i32, i32)> {
    let bars = checked(env, |env| {
        env.call_static_method(
            jni_str!("android/view/WindowInsets$Type"),
            jni_str!("systemBars"),
            jni_sig!("()I"),
            &[],
        )?
        .i()
    })?;

    let cutout = checked(env, |env| {
        env.call_static_method(
            jni_str!("android/view/WindowInsets$Type"),
            jni_str!("displayCutout"),
            jni_sig!("()I"),
            &[],
        )?
        .i()
    })
    .unwrap_or(0);

    let insets_obj = checked(env, |env| {
        env.call_method(
            insets,
            jni_str!("getInsets"),
            jni_sig!("(I)Landroid/graphics/Insets;"),
            &[JValue::Int(bars | cutout)],
        )?
        .l()
    })?;
    if insets_obj.as_raw().is_null() {
        return None;
    }

    fn field<'local>(
        env: &mut Env<'local>,
        obj: &JObject,
        name: &jni::strings::JNIStr,
    ) -> Option<i32> {
        checked(env, |env| env.get_field(obj, name, jni_sig!("I"))?.i())
    }
    Some((
        field(env, &insets_obj, jni_str!("left"))?,
        field(env, &insets_obj, jni_str!("top"))?,
        field(env, &insets_obj, jni_str!("right"))?,
        field(env, &insets_obj, jni_str!("bottom"))?,
    ))
}

fn corner_radius<'local>(env: &mut Env<'local>, insets: &JObject) -> Option<i32> {
    const POSITION_TOP_LEFT: i32 = 0;
    let corner = checked(env, |env| {
        env.call_method(
            insets,
            jni_str!("getRoundedCorner"),
            jni_sig!("(I)Landroid/view/RoundedCorner;"),
            &[JValue::Int(POSITION_TOP_LEFT)],
        )?
        .l()
    })?;

    if corner.as_raw().is_null() {
        return None;
    }
    checked(env, |env| {
        env.call_method(&corner, jni_str!("getRadius"), jni_sig!("()I"), &[])?
            .i()
    })
}

fn checked<'local, T>(
    env: &mut Env<'local>,
    f: impl FnOnce(&mut Env<'local>) -> Result<T, jni::errors::Error>,
) -> Option<T> {
    let result = f(env);
    if env.exception_check() {
        env.exception_clear();
        return None;
    }
    result.ok()
}
