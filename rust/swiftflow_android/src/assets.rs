#![cfg(target_os = "android")]

use android_activity::AndroidApp;
use std::ffi::{CStr, CString};
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::sync::OnceLock;
use swiftflow_core::sflog;

static UNPACKED: OnceLock<CString> = OnceLock::new();

pub fn unpacked_path() -> Option<&'static CStr> {
    UNPACKED.get().map(|s| s.as_c_str())
}

pub fn unpack(app: &AndroidApp) {
    let Some(internal) = app.internal_data_path() else {
        sflog!("assets: no internal data path — images will not resolve");
        return;
    };
    let dest = internal.join("assets");
    if let Err(e) = fs::create_dir_all(&dest) {
        sflog!("assets: could not create {}: {e}", dest.display());
        return;
    }

    let manager = app.asset_manager();
    let root = CString::new("").expect("an empty string has no interior NUL");
    let Some(dir) = manager.open_dir(&root) else {
        sflog!("assets: the APK has no assets/ directory");
        publish(dest);
        return;
    };

    let mut copied = 0usize;
    for name in dir {
        let Some(asset) = manager.open(&name) else {
            sflog!("assets: could not open {:?}", name);
            continue;
        };
        let filename = name.to_string_lossy().into_owned();
        if copy_one(asset, &dest.join(&filename)) {
            copied += 1;
        }
    }
    sflog!("assets: {copied} unpacked to {}", dest.display());
    publish(dest);
}

fn publish(dest: PathBuf) {
    match CString::new(dest.to_string_lossy().as_bytes()) {
        Ok(path) => {
            let _ = UNPACKED.set(path);
        }
        Err(_) => sflog!("assets: the data path contains a NUL — refusing to pass it on"),
    }
}

fn copy_one(mut asset: ndk::asset::Asset, dest: &std::path::Path) -> bool {
    let expected = asset.length();
    if let Ok(existing) = fs::metadata(dest) {
        if existing.len() as usize == expected {
            return false;
        }
    }

    let mut bytes = Vec::with_capacity(expected);
    if let Err(e) = asset.read_to_end(&mut bytes) {
        sflog!("assets: could not read {}: {e}", dest.display());
        return false;
    }
    if let Err(e) = fs::write(dest, &bytes) {
        sflog!("assets: could not write {}: {e}", dest.display());
        return false;
    }
    true
}
