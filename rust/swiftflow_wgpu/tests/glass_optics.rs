fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn mix(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn normalize3(v: [f32; 3]) -> [f32; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    [v[0] / len, v[1] / len, v[2] / len]
}

fn length(v: [f32; 2]) -> f32 {
    (v[0] * v[0] + v[1] * v[1]).sqrt()
}

fn refract(i: [f32; 3], n: [f32; 3], eta: f32) -> [f32; 3] {
    let d = n[0] * i[0] + n[1] * i[1] + n[2] * i[2];
    let k = 1.0 - eta * eta * (1.0 - d * d);
    if k < 0.0 {
        return [0.0; 3];
    }
    let s = eta * d + k.sqrt();
    [
        eta * i[0] - s * n[0],
        eta * i[1] - s * n[1],
        eta * i[2] - s * n[2],
    ]
}

const GLASS_MIN_THICKNESS: f32 = 12.0;
const GLASS_RIM_SHARPNESS: f32 = 0.85;
const GLASS_ADAPTIVE: f32 = 0.25;
const GLASS_PRESS_REFRACTION: f32 = 0.6;
const GLASS_PRESS_RIM: f32 = 0.8;
const GLASS_IOR: f32 = 1.5;
const GLASS_ETA: f32 = 1.0 / GLASS_IOR;
const GLASS_DISPERSION: f32 = 0.02;
const GLASS_VIBRANCY: f32 = 0.35;
const FRESNEL_POWER: f32 = 5.0;
const FRESNEL_INTENSITY: f32 = 0.35;
const GLARE_DIR: [f32; 2] = [-0.70710678, -0.70710678];
const GLARE_POWER: f32 = 6.0;
const GLARE_INTENSITY: f32 = 0.85;
const GLARE_BACK: f32 = 0.3;
const SHADOW_POWER: f32 = 3.0;
const SHADOW_DEPTH: f32 = 0.3;
const RIM_WIDTH: f32 = 3.5;
const RIM_INTENSITY: f32 = 0.4;
const RIM_POWER: f32 = 2.0;

fn refraction_active(refraction: f32) -> f32 {
    if refraction < 0.001 {
        0.0
    } else {
        1.0
    }
}

fn bevel_normal(outward: [f32; 2], t: f32) -> [f32; 3] {
    let u = 1.0 - t.clamp(0.0, 1.0);
    let h = (1.0 - u * u).max(0.0).sqrt();
    let slope = u / h.max(1e-3);
    normalize3([-outward[0] * slope, -outward[1] * slope, 1.0])
}

fn refract_offset(n: [f32; 3], eta: f32) -> [f32; 2] {
    let r = refract([0.0, 0.0, -1.0], n, eta);
    let norm = (1.0 - GLASS_ETA * GLASS_ETA).max(1e-6).sqrt();
    [r[0] / norm, r[1] / norm]
}

struct Sample {

    offset: [f32; 2],

    offset_r: [f32; 2],
    offset_b: [f32; 2],

    normal_z: f32,

    tinted: [f32; 3],

    highlight: f32,
    shade: f32,
}

#[allow(clippy::too_many_arguments)]
fn glass(
    refraction: f32,
    interactive: f32,
    specular: f32,
    dist: f32,
    corner_radius: f32,
    normal: [f32; 2],
    soft: [f32; 3],
    sharp: [f32; 3],
    tint_rgb: [f32; 3],
    tint_a: f32,
) -> Sample {
    let g = refraction_active(refraction);

    let bevel = corner_radius.max(GLASS_MIN_THICKNESS);
    let depth = (-dist / bevel).clamp(0.0, 1.0);
    let edge = 1.0 - smoothstep(0.0, 1.0, depth);
    let bend = refraction * (1.0 + interactive * GLASS_PRESS_REFRACTION);

    let n3 = bevel_normal(normal, depth);
    let scaled = |eta: f32| {
        let o = refract_offset(n3, eta);
        [o[0] * bend, o[1] * bend]
    };

    let mut backdrop = [0.0f32; 3];
    for i in 0..3 {
        backdrop[i] = mix(soft[i], sharp[i], edge * GLASS_RIM_SHARPNESS * g);
    }
    let bd_luma = 0.2126 * backdrop[0] + 0.7152 * backdrop[1] + 0.0722 * backdrop[2];
    let luma = 0.2126 * soft[0] + 0.7152 * soft[1] + 0.0722 * soft[2];
    let mut tinted = [0.0f32; 3];
    for i in 0..3 {
        let vibrant = mix(bd_luma, backdrop[i], 1.0 + GLASS_VIBRANCY * g);
        let adaptive = mix(tint_rgb[i], luma, GLASS_ADAPTIVE * g);
        tinted[i] = mix(vibrant, adaptive, tint_a);
    }

    let rim_gain = 1.0 + interactive * GLASS_PRESS_RIM;
    let axis = normal[0] * GLARE_DIR[0] + normal[1] * GLARE_DIR[1];

    let rim_mask = 1.0 - smoothstep(0.0, RIM_WIDTH, dist.abs());
    let trim = axis.abs().powf(RIM_POWER) * rim_mask * RIM_INTENSITY;

    let fresnel = (1.0 - n3[2]).powf(FRESNEL_POWER);
    let toward = axis.max(0.0);
    let away = (-axis).max(0.0);
    let glare = toward.powf(GLARE_POWER) + away.powf(GLARE_POWER) * GLARE_BACK;
    let lit = fresnel * (FRESNEL_INTENSITY + glare * GLARE_INTENSITY);

    Sample {
        offset: scaled(GLASS_ETA),
        offset_r: scaled(GLASS_ETA * (1.0 + GLASS_DISPERSION)),
        offset_b: scaled(GLASS_ETA * (1.0 - GLASS_DISPERSION)),
        normal_z: n3[2],
        tinted,
        highlight: mix(trim, lit, g) * specular * rim_gain,
        shade: fresnel * away.powf(SHADOW_POWER) * SHADOW_DEPTH * specular * g,
    }
}

const SOFT: [f32; 3] = [0.2, 0.3, 0.4];
const SHARP: [f32; 3] = [0.9, 0.1, 0.5];
const WHITE: [f32; 3] = [1.0, 1.0, 1.0];
const UP: [f32; 2] = [0.0, -1.0];

const TOWARD_LIGHT: [f32; 2] = [-0.70710678, -0.70710678];
const AWAY_FROM_LIGHT: [f32; 2] = [0.70710678, 0.70710678];

#[test]
fn a_material_without_refraction_is_untouched() {
    for dist in [-60.0, -30.0, -12.0, -6.0, -1.0, 0.0] {
        for radius in [0.0, 24.0, -1.0] {
            for normal in [UP, TOWARD_LIGHT, AWAY_FROM_LIGHT, [-1.0, 0.0]] {
                let s = glass(
                    0.0, 0.0, 1.0, dist, radius, normal, SOFT, SHARP, WHITE, 0.10,
                );
                assert_eq!(
                    s.offset,
                    [0.0, 0.0],
                    "a plain material must not displace its backdrop"
                );
                assert_eq!(s.shade, 0.0, "a plain material must not be darkened");

                for i in 0..3 {
                    let plain = mix(SOFT[i], WHITE[i], 0.10);
                    assert!(
                        (s.tinted[i] - plain).abs() < 1e-6,
                        "channel {i} drifted from a plain material: {} vs {plain}",
                        s.tinted[i]
                    );
                }

                let axis = normal[0] * GLARE_DIR[0] + normal[1] * GLARE_DIR[1];
                let mask = 1.0 - smoothstep(0.0, RIM_WIDTH, dist.abs());
                let trim = axis.abs().powf(RIM_POWER) * mask * RIM_INTENSITY;
                assert!(
                    (s.highlight - trim).abs() < 1e-6,
                    "the specular trim changed: {} vs {trim}",
                    s.highlight
                );
            }
        }
    }
}

#[test]
fn the_bevel_stands_vertical_at_the_border_and_lies_flat_inside() {
    let at = |dist: f32| glass(18.0, 0.0, 1.0, dist, 24.0, UP, SOFT, SHARP, WHITE, 0.1).normal_z;
    assert!(
        at(0.0) < 0.01,
        "the border should be vertical, got {}",
        at(0.0)
    );
    assert!(
        at(-24.0) > 0.999,
        "the interior should be flat, got {}",
        at(-24.0)
    );

    let mut previous = at(0.0);
    for step in 1..=24 {
        let next = at(-(step as f32));
        assert!(
            next >= previous,
            "the bevel is not monotone at {step}px in: {next} after {previous}"
        );
        previous = next;
    }
}

#[test]
fn refraction_is_still_pixels_of_bend_at_the_rim() {
    for refraction in [6.0, 18.0, 26.0, 54.0] {
        let s = glass(refraction, 0.0, 1.0, 0.0, 24.0, UP, SOFT, SHARP, WHITE, 0.1);
        let got = length(s.offset);
        assert!(
            (got - refraction).abs() / refraction < 0.002,
            "asked for {refraction}px of bend at the rim, got {got}"
        );
    }
}

#[test]
fn displacement_peaks_at_the_rim_and_vanishes_inside() {
    let at =
        |dist: f32| length(glass(18.0, 0.0, 1.0, dist, 24.0, UP, SOFT, SHARP, WHITE, 0.1).offset);
    let (edge, mid, deep) = (at(-0.5), at(-12.0), at(-60.0));
    assert!(edge > mid, "rim {edge} should bend more than mid {mid}");
    assert!(mid > deep, "mid {mid} should bend more than deep {deep}");
    assert!(
        deep < 1e-3,
        "the interior should be undisplaced, got {deep}"
    );
}

#[test]
fn a_larger_radius_lenses_further_in() {
    let at = |radius: f32| {
        length(glass(18.0, 0.0, 1.0, -20.0, radius, UP, SOFT, SHARP, WHITE, 0.1).offset)
    };
    assert!(
        at(40.0) > at(8.0),
        "a rounder shape should still be bending 20px in"
    );
}

#[test]
fn a_press_bends_further() {
    let at =
        |held: f32| length(glass(18.0, held, 1.0, -2.0, 24.0, UP, SOFT, SHARP, WHITE, 0.1).offset);
    assert!(
        at(1.0) > at(0.0),
        "interactive glass should deepen on press"
    );
}

#[test]
fn red_and_blue_straddle_green_and_split_furthest_at_the_rim() {
    let split = |dist: f32| {
        let s = glass(18.0, 0.0, 1.0, dist, 24.0, UP, SOFT, SHARP, WHITE, 0.1);
        let (r, g, b) = (length(s.offset_r), length(s.offset), length(s.offset_b));
        assert!(
            r < g && g < b,
            "the channels must stay ordered red < green < blue, got {r} {g} {b}"
        );
        b - r
    };
    assert!(
        split(-0.5) > split(-12.0),
        "the fringe should be a rim effect, not a whole-pane one"
    );
}

#[test]
fn the_fresnel_ring_is_even_all_the_way_round() {
    let tilt = |normal: [f32; 2]| {
        glass(18.0, 0.0, 1.0, 0.0, 24.0, normal, SOFT, SHARP, WHITE, 0.1).normal_z
    };
    for normal in [UP, TOWARD_LIGHT, AWAY_FROM_LIGHT, [1.0, 0.0], [-1.0, 0.0]] {
        assert!(
            (tilt(normal) - tilt(UP)).abs() < 1e-6,
            "the bevel's tilt varies with direction, so the ring cannot be even"
        );
    }
}

#[test]
fn the_glare_burns_toward_the_light_and_the_shadow_falls_away_from_it() {
    let at = |normal: [f32; 2]| {
        let s = glass(18.0, 0.0, 1.0, 0.0, 24.0, normal, SOFT, SHARP, WHITE, 0.1);
        (s.highlight, s.shade)
    };
    let (lit_h, lit_s) = at(TOWARD_LIGHT);
    let (dark_h, dark_s) = at(AWAY_FROM_LIGHT);

    assert!(
        lit_h > dark_h,
        "the lit side should be brighter: {lit_h} vs {dark_h}"
    );
    assert!(
        dark_s > lit_s,
        "the shadow should fall on the far side: {dark_s} vs {lit_s}"
    );
    assert_eq!(lit_s, 0.0, "the lit side must not also be shadowed");
}

#[test]
fn nothing_is_lit_or_shadowed_away_from_the_bevel() {
    let s = glass(
        18.0,
        0.0,
        1.0,
        -60.0,
        24.0,
        TOWARD_LIGHT,
        SOFT,
        SHARP,
        WHITE,
        0.1,
    );
    assert!(s.highlight < 1e-4, "the interior is lit: {}", s.highlight);
    assert!(s.shade < 1e-4, "the interior is shadowed: {}", s.shade);
}

#[test]
fn lighting_is_off_without_specular() {
    let s = glass(
        18.0,
        1.0,
        0.0,
        0.0,
        24.0,
        TOWARD_LIGHT,
        SOFT,
        SHARP,
        WHITE,
        0.1,
    );
    assert_eq!(s.highlight, 0.0);
    assert_eq!(s.shade, 0.0);
}

#[test]
fn the_tint_follows_the_backdrop() {
    let sum = |backdrop: [f32; 3]| {
        let s = glass(18.0, 0.0, 1.0, -60.0, 24.0, UP, backdrop, SHARP, WHITE, 0.6);
        s.tinted.iter().sum::<f32>()
    };
    assert!(
        sum([0.02; 3]) < sum([0.95; 3]),
        "glass over dark content should not come out lighter than glass over light"
    );
}

#[test]
fn glass_leaves_the_backdrop_more_saturated_than_a_plain_material_does() {
    let saturation = |refraction: f32| {
        let coloured = [0.75, 0.25, 0.15];
        let s = glass(
            refraction, 0.0, 1.0, -60.0, 24.0, UP, coloured, coloured, WHITE, 0.05,
        );
        let hi = s.tinted.iter().copied().fold(f32::MIN, f32::max);
        let lo = s.tinted.iter().copied().fold(f32::MAX, f32::min);
        hi - lo
    };
    assert!(
        saturation(18.0) > saturation(0.0),
        "glass ({}) should hold more colour than a plain material ({})",
        saturation(18.0),
        saturation(0.0)
    );
}
