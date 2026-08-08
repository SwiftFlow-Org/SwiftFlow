use std::collections::HashMap;
use ttf_parser::{Face, GlyphId, OutlineBuilder, Tag};

use crate::node::SFFontFamily;

const WEIGHT_AXIS: Tag = Tag::from_bytes(b"wght");

#[derive(Clone, Debug)]
pub struct BezierCurve {
    pub p0: [f32; 2],
    pub p1: [f32; 2],
    pub p2: [f32; 2],
}

struct GlyphOutlineBuilder {
    curves: Vec<BezierCurve>,
    current: [f32; 2],
    start: [f32; 2],
}

impl GlyphOutlineBuilder {
    fn new() -> Self {
        Self {
            curves: Vec::new(),
            current: [0.0; 2],
            start: [0.0; 2],
        }
    }
}

impl OutlineBuilder for GlyphOutlineBuilder {
    fn move_to(&mut self, x: f32, y: f32) {
        self.current = [x, y];
        self.start = [x, y];
    }

    fn line_to(&mut self, x: f32, y: f32) {

        let mid = [(self.current[0] + x) * 0.5, (self.current[1] + y) * 0.5];
        self.curves.push(BezierCurve {
            p0: self.current,
            p1: mid,
            p2: [x, y],
        });
        self.current = [x, y];
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {

        self.curves.push(BezierCurve {
            p0: self.current,
            p1: [x1, y1],
            p2: [x, y],
        });
        self.current = [x, y];
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {

        let p0 = self.current;
        let p1 = [x1, y1];
        let p2 = [x2, y2];
        let p3 = [x, y];

        let c0 = lerp2(p0, p1, 0.75);
        let c1 = lerp2(p3, p2, 0.75);
        let m = lerp2(c0, c1, 0.5);

        self.curves.push(BezierCurve { p0, p1: c0, p2: m });
        self.curves.push(BezierCurve {
            p0: m,
            p1: c1,
            p2: p3,
        });

        self.current = p3;
    }

    fn close(&mut self) {

        if self.current != self.start {
            self.line_to(self.start[0], self.start[1]);
        }
    }
}

fn lerp2(a: [f32; 2], b: [f32; 2], t: f32) -> [f32; 2] {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]
}

#[derive(Clone, Debug)]
pub struct GlyphData {
    pub curves: Vec<BezierCurve>,
    pub bbox_min: [f32; 2],
    pub bbox_max: [f32; 2],
    pub advance: f32,
    pub units_per_em: f32,
}

impl GlyphData {

    pub fn em_to_pixel(&self, point: [f32; 2], font_size: f32) -> [f32; 2] {
        let scale = font_size / self.units_per_em;
        [
            (point[0] - self.bbox_min[0]) * scale,

            (self.bbox_max[1] - point[1]) * scale,
        ]
    }

    pub fn width_px(&self, font_size: f32) -> f32 {
        let scale = font_size / self.units_per_em;
        (self.bbox_max[0] - self.bbox_min[0]) * scale
    }

    pub fn height_px(&self, font_size: f32) -> f32 {
        let scale = font_size / self.units_per_em;
        (self.bbox_max[1] - self.bbox_min[1]) * scale
    }
}

pub struct GlyphLoader {
    sans: Vec<u8>,
    monospaced: Vec<u8>,
    cache: HashMap<(char, u32, SFFontFamily), GlyphData>,
}

static SANS_DATA: &[u8] = include_bytes!("../../fonts/Inter.ttf");
static MONO_DATA: &[u8] = include_bytes!("../../fonts/JetBrainsMono.ttf");

impl GlyphLoader {
    pub fn new() -> Self {
        let sans = SANS_DATA.to_vec();
        let monospaced = MONO_DATA.to_vec();

        println!(
            "GlyphLoader: Inter upem={}, JetBrainsMono upem={}",
            Face::parse(&sans, 0).map(|f| f.units_per_em()).unwrap_or(0),
            Face::parse(&monospaced, 0)
                .map(|f| f.units_per_em())
                .unwrap_or(0),
        );

        Self {
            sans,
            monospaced,
            cache: HashMap::new(),
        }
    }

    fn family_data(&self, family: SFFontFamily) -> &[u8] {
        match family {
            SFFontFamily::Monospaced => &self.monospaced,
            _ => &self.sans,
        }
    }

    fn candidates(&self, c: char, weight: f32, family: SFFontFamily) -> [Option<&[u8]>; 2] {
        let icon = crate::icon::face_data_for(c, weight);
        match family {
            SFFontFamily::Icon => [icon, Some(&self.sans)],
            _ => [Some(self.family_data(family)), icon],
        }
    }

    pub fn load(&mut self, c: char, weight: f32, family: SFFontFamily) -> Option<&GlyphData> {
        let key = (c, weight.round() as u32, family);
        if !self.cache.contains_key(&key) {
            let data = self.load_uncached(c, weight, family)?;
            self.cache.insert(key, data);
        }
        self.cache.get(&key)
    }

    pub fn load_uncached(
        &self,
        c: char,
        weight: f32,
        family: SFFontFamily,
    ) -> Option<GlyphData> {
        for data in self.candidates(c, weight, family).into_iter().flatten() {
            if let Some(glyph) = self.build(data, c, weight) {
                return Some(glyph);
            }
        }
        None
    }

    fn build(&self, data: &[u8], c: char, weight: f32) -> Option<GlyphData> {
        let mut face = Face::parse(data, 0).ok()?;

        face.set_variation(WEIGHT_AXIS, weight);

        let glyph_id = face.glyph_index(c)?;

        let bbox = face.glyph_bounding_box(glyph_id)?;

        let advance = face.glyph_hor_advance(glyph_id).unwrap_or(0) as f32;

        let mut builder = GlyphOutlineBuilder::new();
        face.outline_glyph(glyph_id, &mut builder);

        println!(
            "loaded {:?}: {} curves bbox=({},{})→({},{}) advance={}",
            c,
            builder.curves.len(),
            bbox.x_min,
            bbox.y_min,
            bbox.x_max,
            bbox.y_max,
            advance
        );

        Some(GlyphData {
            curves: builder.curves,
            bbox_min: [bbox.x_min as f32, bbox.y_min as f32],
            bbox_max: [bbox.x_max as f32, bbox.y_max as f32],
            advance,

            units_per_em: face.units_per_em() as f32,
        })
    }
}

use std::sync::Mutex;
static GLYPH_LOADER: Mutex<Option<GlyphLoader>> = Mutex::new(None);

pub fn init_glyph_loader() {
    let mut lock = GLYPH_LOADER.lock().unwrap();
    if lock.is_none() {
        *lock = Some(GlyphLoader::new());
    }
}

pub fn with_glyph_loader<F, R>(f: F) -> R
where
    F: FnOnce(&mut GlyphLoader) -> R,
{
    let mut lock = GLYPH_LOADER.lock().unwrap();
    f(lock.as_mut().expect("GlyphLoader not initialized"))
}
