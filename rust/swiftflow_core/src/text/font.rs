use crate::glyph::{with_glyph_loader, GlyphData};
use crate::node::SFFontFamily;
use crate::rasterizer::rasterize_glyph;
use fontdue::{Font, FontSettings};
use std::collections::HashMap;
use ttf_parser::{Face, Tag};

const WEIGHT_AXIS: Tag = Tag::from_bytes(b"wght");

static SANS_DATA: &[u8] = include_bytes!("../../fonts/Inter.ttf");
static MONO_DATA: &[u8] = include_bytes!("../../fonts/JetBrainsMono.ttf");

#[repr(C)]
struct BezierCurve {
    p0: [f32; 2],
    p1: [f32; 2],
    p2: [f32; 2],
}

struct GlyphEntry {
    curve_offset: u32,
    curve_count: u32,
    bbox_min: [f32; 2],
    bbox_max: [f32; 2],
    advance: f32,
}

#[derive(Copy, Clone, Debug)]
pub struct GlyphInfo {

    pub uv_x: f32,
    pub uv_y: f32,
    pub uv_width: f32,
    pub uv_height: f32,

    pub width: f32,
    pub height: f32,
    pub offset_x: f32,
    pub offset_y: f32,
    pub advance: f32,
    pub ascender: f32,
}

pub struct GlyphAccumulator {
    pub coverage: Vec<f32>,
    pub total_samples: u32,
    pub width: u32,
    pub height: u32,

    pub raster_size: f32,
}

pub struct FontAtlas {
    pub data: Vec<u8>,
    pub glyphs: HashMap<GlyphKey, GlyphInfo>,
    pub accum: HashMap<GlyphKey, GlyphAccumulator>,

    pub overflowed: std::collections::HashSet<GlyphKey>,
}

const ATLAS_SIZE: u32 = 2048;

const REFINE_TARGET_SAMPLES: u32 = 128;

const REFINE_SAMPLES_PER_PASS: u32 = 8;

const REFINE_BUDGET_PER_FRAME: usize = 8;
const PADDING: u32 = 2;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GlyphKey {
    pub c: char,

    pub size: u32,
    pub weight: u32,
    pub family: SFFontFamily,
}

fn glyph_cache_key(c: char, font_size: f32, weight: f32, family: SFFontFamily) -> GlyphKey {
    GlyphKey {
        c,
        size: (font_size / 2.0).round() as u32 * 2,
        weight: weight.round() as u32,
        family,
    }
}

impl FontAtlas {
    pub fn new() -> Self {
        Self {
            data: vec![0u8; (ATLAS_SIZE * ATLAS_SIZE * 4) as usize],
            glyphs: HashMap::new(),
            accum: HashMap::new(),
            overflowed: std::collections::HashSet::new(),
        }
    }
}

pub struct FontSystem {
    sans: Vec<u8>,
    monospaced: Vec<u8>,
    pub atlas: FontAtlas,
    pub cursor_x: u32,
    pub cursor_y: u32,
    pub row_height: u32,
    pub dirty: bool,

    pub dirty_rows: Option<(u32, u32)>,

    refine_cursor: usize,
}

impl FontSystem {
    pub fn new() -> Self {
        Self {
            sans: SANS_DATA.to_vec(),
            monospaced: MONO_DATA.to_vec(),
            atlas: FontAtlas::new(),
            cursor_x: PADDING,
            cursor_y: PADDING,
            row_height: 0,
            dirty: true,

            dirty_rows: Some((0, ATLAS_SIZE)),
            refine_cursor: 0,
        }
    }

    fn family_data(&self, family: SFFontFamily) -> &[u8] {
        match family {
            SFFontFamily::Monospaced => &self.monospaced,
            _ => &self.sans,
        }
    }

    pub fn face(&self, family: SFFontFamily) -> Face<'_> {
        Face::parse(self.family_data(family), 0).expect("Failed to parse font")
    }

    pub fn face_for_weight(&self, weight: f32, family: SFFontFamily) -> Face<'_> {
        let mut face = self.face(family);
        face.set_variation(WEIGHT_AXIS, weight);
        face
    }

    pub fn face_for_char(&self, c: char, weight: f32, family: SFFontFamily) -> Face<'_> {
        let icon = || {
            crate::icon::face_data_for(c, weight).and_then(|data| Face::parse(data, 0).ok())
        };
        if family == SFFontFamily::Icon {
            return icon().unwrap_or_else(|| self.face_for_weight(weight, SFFontFamily::Sans));
        }
        let own = self.face_for_weight(weight, family);
        if own.glyph_index(c).is_some() {
            return own;
        }
        icon().unwrap_or(own)
    }

    pub fn ascender_for(
        &self,
        c: char,
        font_size: f32,
        weight: f32,
        family: SFFontFamily,
    ) -> f32 {
        let face = self.face_for_char(c, weight, family);
        let upem = face.units_per_em() as f32;
        if upem <= 0.0 {
            return font_size;
        }
        face.ascender() as f32 * (font_size / upem)
    }

    pub fn units_per_em(&self, family: SFFontFamily) -> f32 {
        self.face(family).units_per_em() as f32
    }

    pub fn space_width(&self, font_size: f32, weight: f32, family: SFFontFamily) -> f32 {
        let face = self.face_for_weight(weight, family);
        let scale = font_size / face.units_per_em() as f32;

        face.glyph_index(' ')
            .and_then(|id| face.glyph_hor_advance(id))
            .map(|adv| adv as f32 * scale)
            .unwrap_or(font_size * 0.25)
    }

    pub fn kerning(
        &self,
        left: char,
        right: char,
        font_size: f32,
        weight: f32,
        family: SFFontFamily,
    ) -> f32 {

        if crate::icon::is_icon(left) || crate::icon::is_icon(right) {
            return 0.0;
        }
        let face = self.face_for_weight(weight, family);
        let left_id = match face.glyph_index(left) {
            Some(id) => id,
            None => return 0.0,
        };
        let right_id = match face.glyph_index(right) {
            Some(id) => id,
            None => return 0.0,
        };
        let scale = font_size / face.units_per_em() as f32;

        let kern = face
            .tables()
            .kern
            .and_then(|kern| {
                kern.subtables
                    .into_iter()
                    .find_map(|st| st.glyphs_kerning(left_id, right_id))
            })
            .unwrap_or(0);

        kern as f32 * scale
    }

    pub fn glyph(
        &mut self,
        c: char,
        font_size: f32,
        weight: f32,
        family: SFFontFamily,
    ) -> Option<GlyphInfo> {
        let key = glyph_cache_key(c, font_size, weight, family);
        if let Some(info) = self.atlas.glyphs.get(&key) {
            return Some(*info);
        }
        if self.atlas.overflowed.contains(&key) {
            return None;
        }
        self.rasterize(c, font_size, weight, family);
        self.atlas.glyphs.get(&key).copied()
    }

    pub fn measure(
        &mut self,
        text: &str,
        font_size: f32,
        weight: f32,
        family: SFFontFamily,
    ) -> (f32, f32) {
        let mut width = 0.0f32;
        let chars: Vec<char> = text.chars().collect();

        for (i, &c) in chars.iter().enumerate() {
            if c == ' ' {
                width += self.space_width(font_size, weight, family);
                continue;
            }
            if let Some(info) = self.glyph(c, font_size, weight, family) {
                width += info.advance;

                if i + 1 < chars.len() {
                    width += self.kerning(c, chars[i + 1], font_size, weight, family);
                }
            }
        }

        let cap = self
            .glyph('H', font_size, weight, family)
            .map(|g| g.height)
            .unwrap_or(0.0);
        let height = cap + self.descent(font_size, family);

        if height <= 0.0 {

            return (width, font_size * 1.2);
        }
        (width, height)
    }

    fn rasterize(&mut self, c: char, font_size: f32, weight: f32, family: SFFontFamily) {
        let glyph_data = with_glyph_loader(|l| l.load(c, weight, family).cloned());
        let Some(glyph_data) = glyph_data else {
            return;
        };

        let (bitmap, glyph_w, glyph_h) = rasterize_glyph(&glyph_data, font_size, 32, 0);

        let key = glyph_cache_key(c, font_size, weight, family);

        if glyph_w == 0 || glyph_h == 0 {
            self.atlas.overflowed.insert(key);
            return;
        }

        let mut cursor_x = self.cursor_x;
        let mut cursor_y = self.cursor_y;
        let mut row_height = self.row_height;
        if cursor_x + glyph_w + PADDING > ATLAS_SIZE {
            cursor_x = PADDING;
            cursor_y += row_height + PADDING;
            row_height = 0;
        }
        if cursor_y + glyph_h + PADDING > ATLAS_SIZE {
            self.atlas.overflowed.insert(key);
            return;
        }
        self.cursor_x = cursor_x;
        self.cursor_y = cursor_y;
        self.row_height = row_height;

        let coverage: Vec<f32> = bitmap.iter().map(|&v| v as f32 / 255.0).collect();

        self.atlas.accum.insert(
            key,
            GlyphAccumulator {
                coverage,
                total_samples: 32,
                raster_size: font_size,
                width: glyph_w,
                height: glyph_h,
            },
        );

        let scale_factor = font_size / glyph_data.units_per_em;

        self.atlas.glyphs.insert(
            key,
            GlyphInfo {
                uv_x: self.cursor_x as f32 / ATLAS_SIZE as f32,
                uv_y: self.cursor_y as f32 / ATLAS_SIZE as f32,
                uv_width: glyph_w as f32 / ATLAS_SIZE as f32,
                uv_height: glyph_h as f32 / ATLAS_SIZE as f32,
                width: glyph_w as f32,
                height: glyph_h as f32,
                offset_x: glyph_data.bbox_min[0] * scale_factor,
                offset_y: glyph_data.bbox_min[1] * scale_factor,
                advance: glyph_data.advance * scale_factor,
                ascender: glyph_data.bbox_max[1] * scale_factor,
            },
        );
        self.write_accum_to_atlas(key, glyph_w, glyph_h);

        self.cursor_x += glyph_w + PADDING;
        self.row_height = self.row_height.max(glyph_h);
        self.dirty = true;
    }

    pub fn refine(&mut self, frame: u32) -> usize {
        let mut pending: Vec<GlyphKey> = self
            .atlas
            .accum
            .iter()
            .filter(|(_, a)| a.total_samples < REFINE_TARGET_SAMPLES)
            .map(|(k, _)| *k)
            .collect();

        if pending.is_empty() {
            self.refine_cursor = 0;
            return 0;
        }
        pending.sort_unstable();

        let take = REFINE_BUDGET_PER_FRAME.min(pending.len());
        let start = self.refine_cursor % pending.len();
        let mut refined = 0;

        for i in 0..take {
            let key = pending[(start + i) % pending.len()];
            if self.refine_one(key, frame) {
                refined += 1;
            }
        }

        self.refine_cursor = self.refine_cursor.wrapping_add(take);
        refined
    }

    fn refine_one(&mut self, key: GlyphKey, frame: u32) -> bool {

        if !self.atlas.glyphs.contains_key(&key) {
            return false;
        }

        let c = key.c;
        let weight = key.weight as f32;
        let family = key.family;

        let Some(accum) = self.atlas.accum.get(&key) else {
            return false;
        };
        let samples_so_far = accum.total_samples;
        let w = accum.width;
        let h = accum.height;

        let raster_size = accum.raster_size;

        let rasterized = with_glyph_loader(|l| {
            l.load(c, weight, family)
                .map(|g| rasterize_glyph(g, raster_size, REFINE_SAMPLES_PER_PASS, frame))
        });
        let Some((new_bitmap, _, _)) = rasterized else {
            return false;
        };

        let Some(accum) = self.atlas.accum.get_mut(&key) else {
            return false;
        };
        let added = REFINE_SAMPLES_PER_PASS as f32;
        for i in 0..accum.coverage.len() {
            if i >= new_bitmap.len() {
                continue;
            }
            let old = accum.coverage[i] * samples_so_far as f32;
            let new = new_bitmap[i] as f32 / 255.0 * added;
            accum.coverage[i] = (old + new) / (samples_so_far + REFINE_SAMPLES_PER_PASS) as f32;
        }
        accum.total_samples += REFINE_SAMPLES_PER_PASS;

        self.write_accum_to_atlas(key, w, h);
        true
    }

    fn mark_rows(&mut self, y: u32, h: u32) {
        let lo = y.min(ATLAS_SIZE);
        let hi = (y + h + 1).min(ATLAS_SIZE);
        self.dirty_rows = Some(match self.dirty_rows {
            Some((a, b)) => (a.min(lo), b.max(hi)),
            None => (lo, hi),
        });
        self.dirty = true;
    }

    pub fn sample_count(
        &self,
        c: char,
        font_size: f32,
        weight: f32,
        family: SFFontFamily,
    ) -> Option<u32> {
        self.atlas
            .accum
            .get(&glyph_cache_key(c, font_size, weight, family))
            .map(|a| a.total_samples)
    }

    pub fn atlas_dirty_rows(&mut self) -> Option<(u32, u32)> {
        self.dirty = false;
        self.dirty_rows.take().filter(|(a, b)| b > a)
    }

    fn write_accum_to_atlas(&mut self, key: GlyphKey, w: u32, h: u32) {

        if let Some(y) = self
            .atlas
            .glyphs
            .get(&key)
            .map(|info| (info.uv_y * ATLAS_SIZE as f32) as u32)
        {
            self.mark_rows(y, h);
        }

        let (Some(&info), Some(accum)) = (self.atlas.glyphs.get(&key), self.atlas.accum.get(&key))
        else {
            return;
        };
        let base_x = (info.uv_x * ATLAS_SIZE as f32) as u32;
        let base_y = (info.uv_y * ATLAS_SIZE as f32) as u32;

        for row in 0..h {
            for col in 0..w {
                let src_idx = (row * w + col) as usize;
                let dst_x = base_x + col;
                let dst_y = base_y + row;
                let dst_idx = ((dst_y * ATLAS_SIZE + dst_x) * 4) as usize;

                let val = (accum.coverage[src_idx] * 255.0) as u8;
                if val > 0 {
                    self.atlas.data[dst_idx] = 255;
                    self.atlas.data[dst_idx + 1] = 255;
                    self.atlas.data[dst_idx + 2] = 255;
                    self.atlas.data[dst_idx + 3] = val;
                } else {
                    self.atlas.data[dst_idx] = 0;
                    self.atlas.data[dst_idx + 1] = 0;
                    self.atlas.data[dst_idx + 2] = 0;
                    self.atlas.data[dst_idx + 3] = 0;
                }
            }
        }
    }

    pub fn ascender(&self, font_size: f32, family: SFFontFamily) -> f32 {
        let face = self.face(family);
        let scale = font_size / face.units_per_em() as f32;
        face.ascender() as f32 * scale
    }

    pub fn descent(&self, font_size: f32, family: SFFontFamily) -> f32 {
        let face = self.face(family);
        let scale = font_size / face.units_per_em() as f32;

        (-(face.descender() as f32) * scale).max(0.0)
    }

    pub fn line_height(&self, font_size: f32, family: SFFontFamily) -> f32 {
        let face = self.face(family);
        let scale = font_size / face.units_per_em() as f32;
        let ascent = face.ascender() as f32 * scale;

        let descent = face.descender() as f32 * scale;
        let gap = face.line_gap() as f32 * scale;
        let height = ascent - descent + gap;
        if height > 0.0 {
            height
        } else {
            font_size * 1.2
        }
    }

    pub fn wrap(
        &mut self,
        text: &str,
        max_width: f32,
        font_size: f32,
        weight: f32,
        family: SFFontFamily,
        line_limit: usize,
    ) -> Vec<String> {
        let mut lines: Vec<String> = Vec::new();

        for paragraph in text.split('\n') {
            if max_width <= 0.0 {
                lines.push(paragraph.to_string());
                continue;
            }
            self.wrap_paragraph(paragraph, max_width, font_size, weight, family, &mut lines);
        }

        if line_limit > 0 && lines.len() > line_limit {
            lines.truncate(line_limit);
            if let Some(last) = lines.last_mut() {
                *last = self.ellipsize(last, max_width, font_size, weight, family);
            }
        }
        lines
    }

    fn wrap_paragraph(
        &mut self,
        paragraph: &str,
        max_width: f32,
        font_size: f32,
        weight: f32,
        family: SFFontFamily,
        out: &mut Vec<String>,
    ) {

        if paragraph.is_empty() {
            out.push(String::new());
            return;
        }

        let mut line = String::new();
        for word in space_runs(paragraph) {
            let candidate = format!("{line}{word}");

            if self.measure(&candidate, font_size, weight, family).0 <= max_width {
                line = candidate;
                continue;
            }

            if !line.is_empty() {
                out.push(std::mem::take(&mut line));
            }

            let word = word.trim_start_matches(' ');

            if self.measure(word, font_size, weight, family).0 > max_width {
                let mut chunk = String::new();
                for c in word.chars() {
                    let mut trial = chunk.clone();
                    trial.push(c);
                    if !chunk.is_empty() && self.measure(&trial, font_size, weight, family).0 > max_width {
                        out.push(std::mem::take(&mut chunk));
                    }
                    chunk.push(c);
                }
                line = chunk;
            } else {
                line = word.to_string();
            }
        }
        out.push(line);
    }

    fn ellipsize(
        &mut self,
        line: &str,
        max_width: f32,
        font_size: f32,
        weight: f32,
        family: SFFontFamily,
    ) -> String {
        const ELLIPSIS: char = '…';
        let mut chars: Vec<char> = line.trim_end().chars().collect();
        loop {
            let mut candidate: String = chars.iter().collect();
            candidate.push(ELLIPSIS);
            if chars.is_empty() || self.measure(&candidate, font_size, weight, family).0 <= max_width {
                return candidate;
            }
            chars.pop();

            while chars.last() == Some(&' ') {
                chars.pop();
            }
        }
    }

    pub fn measure_wrapped(
        &mut self,
        text: &str,
        max_width: f32,
        font_size: f32,
        weight: f32,
        family: SFFontFamily,
        line_limit: usize,
    ) -> (f32, f32) {
        let lines = self.wrap(text, max_width, font_size, weight, family, line_limit);
        let line_height = self.line_height(font_size, family);

        let mut width = 0.0_f32;

        let mut last_ink = 0.0_f32;
        for line in &lines {
            if line.is_empty() {
                continue;
            }
            let (w, h) = self.measure(line, font_size, weight, family);
            width = width.max(w);
            last_ink = h;
        }
        if last_ink == 0.0 {
            last_ink = line_height;
        }

        let gaps = lines.len().saturating_sub(1) as f32;
        (width, last_ink + line_height * gaps)
    }
}

use std::sync::Mutex;

static FONT_SYSTEM: Mutex<Option<FontSystem>> = Mutex::new(None);

pub fn init_font_system() {
    let mut lock = FONT_SYSTEM.lock().unwrap();
    if lock.is_none() {
        *lock = Some(FontSystem::new());
    }
}

pub fn with_font_system<F, R>(f: F) -> R
where
    F: FnOnce(&mut FontSystem) -> R,
{
    let mut lock = FONT_SYSTEM.lock().unwrap();
    f(lock
        .as_mut()
        .expect("Font system not initialized — call init_font_system() first"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refine_never_panics_when_atlas_overflows() {
        crate::glyph::init_glyph_loader();
        let mut fs = FontSystem::new();
        fs.cursor_y = ATLAS_SIZE - 4;
        fs.cursor_x = ATLAS_SIZE - 4;
        fs.row_height = 0;

        for weight in [100.0, 400.0, 700.0, 900.0] {
            for c in "Overflow Test 0123456789".chars() {
                let _ = fs.glyph(c, 40.0, weight, SFFontFamily::Sans);
            }
        }
        for frame in 0..20 {
            fs.refine(frame);
        }
        assert!(
            !fs.atlas.overflowed.is_empty(),
            "expected at least one overflowed glyph in this scenario"
        );
    }

    #[test]
    fn accumulator_remembers_exact_raster_size_not_the_bucket() {
        crate::glyph::init_glyph_loader();
        let mut fs = FontSystem::new();

        let raster_size = 51.0;
        let bucket = glyph_cache_key('C', raster_size, 700.0, SFFontFamily::Sans).size as f32;
        assert_ne!(
            bucket, raster_size,
            "test assumes this size actually falls into a different bucket"
        );

        for c in "Count: 0Tape".chars() {
            let _ = fs.glyph(c, raster_size, 700.0, SFFontFamily::Sans);
            let key = glyph_cache_key(c, raster_size, 700.0, SFFontFamily::Sans);
            if let Some(accum) = fs.atlas.accum.get(&key) {
                assert_eq!(
                    accum.raster_size, raster_size,
                    "'{c}' stored the bucketed size instead of the exact one"
                );
            }
        }

        for frame in 0..30 {
            fs.refine(frame);
        }
        for c in "Count: 0Tape".chars() {
            let key = glyph_cache_key(c, raster_size, 700.0, SFFontFamily::Sans);
            if let Some(accum) = fs.atlas.accum.get(&key) {
                assert_eq!(accum.raster_size, raster_size);
                assert_eq!(accum.coverage.len(), (accum.width * accum.height) as usize);
            }
        }
    }
}

fn space_runs(paragraph: &str) -> Vec<&str> {
    let bytes = paragraph.as_bytes();
    let mut chunks = Vec::new();
    let mut start = 0;
    let mut i = 0;
    while i < bytes.len() {
        while i < bytes.len() && bytes[i] == b' ' {
            i += 1;
        }
        while i < bytes.len() && bytes[i] != b' ' {
            i += 1;
        }
        chunks.push(&paragraph[start..i]);
        start = i;
    }
    chunks
}
