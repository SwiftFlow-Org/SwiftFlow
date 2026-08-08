use eframe::egui;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub struct Preview {
    pub texture: egui::TextureHandle,
    pub width: u32,
    pub height: u32,
    pub file_size: u64,
}

#[derive(Default)]
pub struct PreviewCache {
    entries: HashMap<PathBuf, Option<Preview>>,
}

impl PreviewCache {

    pub fn get(&mut self, ctx: &egui::Context, path: &Path) -> Option<&Preview> {
        if !self.entries.contains_key(path) {
            let loaded = load(ctx, path);
            self.entries.insert(path.to_path_buf(), loaded);
        }
        self.entries.get(path).and_then(|e| e.as_ref())
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

fn load(ctx: &egui::Context, path: &Path) -> Option<Preview> {
    let file_size = std::fs::metadata(path).ok()?.len();
    let decoded = image::open(path).ok()?.to_rgba8();
    let (width, height) = decoded.dimensions();

    let image = egui::ColorImage::from_rgba_unmultiplied(
        [width as usize, height as usize],
        decoded.as_raw(),
    );

    let texture = ctx.load_texture(
        path.to_string_lossy().to_string(),
        image,
        egui::TextureOptions::NEAREST,
    );

    Some(Preview {
        texture,
        width,
        height,
        file_size,
    })
}

pub fn format_size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * KB;
    let b = bytes as f64;
    if b >= MB {
        format!("{:.1} MB", b / MB)
    } else if b >= KB {
        format!("{:.0} KB", b / KB)
    } else {
        format!("{bytes} bytes")
    }
}
