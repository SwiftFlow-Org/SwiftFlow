use crate::glyph::{BezierCurve, GlyphData};

fn intersect_bezier(curve: &BezierCurve, px: f32, py: f32) -> i32 {
    let p0 = curve.p0;
    let p1 = curve.p1;
    let p2 = curve.p2;

    let min_y = p0[1].min(p1[1]).min(p2[1]);
    let max_y = p0[1].max(p1[1]).max(p2[1]);

    if py < min_y || py >= max_y {
        return 0;
    }

    let a = p0[1] - 2.0 * p1[1] + p2[1];
    let b = 2.0 * (p1[1] - p0[1]);
    let c = p0[1] - py;

    let mut winding = 0i32;

    let discriminant = b * b - 4.0 * a * c;
    if discriminant < 0.0 {
        return 0;
    }
    let sqrt_d = discriminant.sqrt();
    let q = -0.5 * (b + if b >= 0.0 { sqrt_d } else { -sqrt_d });

    let mut roots = [f32::NAN, f32::NAN];
    if a != 0.0 {
        roots[0] = q / a;
    }
    if q != 0.0 {
        roots[1] = c / q;
    }

    for (index, &t) in roots.iter().enumerate() {
        if t.is_nan() || t < 0.0 || t > 1.0 {
            continue;
        }

        if index == 1 && (roots[0] - t).abs() < 1e-6 {
            continue;
        }
        let x = bezier_x(p0[0], p1[0], p2[0], t);
        if x <= px {
            let dy = bezier_dy(p0[1], p1[1], p2[1], t);

            winding += if dy > 0.0 || (dy == 0.0 && p2[1] > p0[1]) {
                1
            } else {
                -1
            };
        }
    }

    winding
}

fn bezier_x(x0: f32, x1: f32, x2: f32, t: f32) -> f32 {
    let mt = 1.0 - t;
    mt * mt * x0 + 2.0 * mt * t * x1 + t * t * x2
}

fn bezier_dy(y0: f32, y1: f32, y2: f32, t: f32) -> f32 {

    2.0 * (1.0 - t) * (y1 - y0) + 2.0 * t * (y2 - y1)
}

pub fn coverage(
    curves: &[BezierCurve],
    px: f32,
    py: f32,
    pixel_size_in_em: f32,
    samples: u32,
    frame: u32,

    rotation: (f32, f32),
) -> f32 {
    let mut total = 0.0f32;

    for i in 0..samples {
        let (u, v) = quasirandom(frame * samples + i);

        let sx = px + ((u + rotation.0).fract() - 0.5) * pixel_size_in_em;
        let sy = py + ((v + rotation.1).fract() - 0.5) * pixel_size_in_em;

        let mut winding = 0i32;
        for curve in curves {
            winding += intersect_bezier(curve, sx, sy);
        }

        total += if winding != 0 { 1.0 } else { 0.0 };
    }

    total / samples as f32
}

fn pixel_rotation(px: u32, py: u32) -> (f32, f32) {
    let mut h = px
        .wrapping_mul(0x9E37_79B9)
        .wrapping_add(py.wrapping_mul(0x85EB_CA6B));
    h ^= h >> 15;
    h = h.wrapping_mul(0x2545_F491);
    h ^= h >> 13;
    (
        (h & 0xFFFF) as f32 / 65536.0,
        ((h >> 16) & 0xFFFF) as f32 / 65536.0,
    )
}

fn quasirandom(n: u32) -> (f32, f32) {
    const G: f64 = 1.32471795724474602596;
    const A1: f64 = 1.0 / G;
    const A2: f64 = 1.0 / (G * G);

    let x = (0.5 + A1 * n as f64).fract() as f32;
    let y = (0.5 + A2 * n as f64).fract() as f32;

    (x, y)
}

pub fn rasterize_glyph(
    glyph: &GlyphData,
    font_size: f32,
    samples: u32,
    frame: u32,
) -> (Vec<u8>, u32, u32) {
    let scale = font_size / glyph.units_per_em;

    let width = (glyph.width_px(font_size).ceil() as u32).max(1);
    let height = (glyph.height_px(font_size).ceil() as u32).max(1);

    let mut bitmap = vec![0u8; (width * height) as usize];

    for py in 0..height {
        for px in 0..width {

            let em_x = glyph.bbox_min[0] + px as f32 / scale;
            let em_y = glyph.bbox_max[1] - py as f32 / scale;

            let pixel_size_em = 1.0 / scale;

            let cov = coverage(
                &glyph.curves,
                em_x,
                em_y,
                pixel_size_em,
                samples,
                frame,
                pixel_rotation(px, py),
            );

            bitmap[(py * width + px) as usize] = (cov * 255.0) as u8;
        }
    }

    (bitmap, width, height)
}
