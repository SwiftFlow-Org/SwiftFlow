// A CPU copy of the glass maths in `shader.wgsl`'s `fs_material`, so the
// optics can be checked without a GPU. The constants are the shipped ones;
// `shaders_validate.rs` keeps the preview shader on the same values.

fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn mix(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn length(v: [f32; 2]) -> f32 {
    (v[0] * v[0] + v[1] * v[1]).sqrt()
}

const RIM_WIDTH: f32 = 2.5;
const RIM_INTENSITY: f32 = 0.4;
const RIM_POWER: f32 = 2.0;
const GLASS_THICKNESS_PER_BEND: f32 = 1.5;
const GLASS_MIN_THICKNESS: f32 = 12.0;
const GLASS_RIM_SHARPNESS: f32 = 0.95;
const GLASS_ADAPTIVE: f32 = 0.25;
const GLASS_PRESS_REFRACTION: f32 = 0.25;
const GLASS_PRESS_RIM: f32 = 0.8;
const GLASS_IOR: f32 = 1.5;
const GLASS_LIP_SINE: f32 = 0.92;
const GLASS_DISPERSION: f32 = 0.04;
const GLASS_VIBRANCY: f32 = 0.0;
const FRESNEL_INTENSITY: f32 = 0.04;
const GLARE_DIR: [f32; 2] = [-0.70710678, -0.70710678];
const GLARE_POWER: f32 = 1.5;
const GLARE_INTENSITY: f32 = 1.0;
const GLARE_BACK: f32 = 0.45;
const GLARE_BAND: f32 = 0.6;
const GLARE_LIGHTEN: f32 = 0.3;
const GLARE_CHROMA: f32 = 0.8;
const SHADOW_POWER: f32 = 3.0;
const SHADOW_DEPTH: f32 = 0.05;

fn refraction_active(refraction: f32) -> f32 {
    if refraction < 0.001 {
        0.0
    } else {
        1.0
    }
}

fn glass_thickness(refraction: f32, half: [f32; 2]) -> f32 {
    let wanted = (refraction * GLASS_THICKNESS_PER_BEND).max(GLASS_MIN_THICKNESS);
    wanted.min(half[0].min(half[1])).max(1.0)
}

fn lip_deviation(sine: f32, ior: f32) -> f32 {
    let incident = sine.asin();
    (incident - (sine / ior).asin()).tan()
}

fn lip_shift(depth: f32, thickness: f32, ior: f32) -> f32 {
    let r = 1.0 - (depth / thickness).clamp(0.0, 1.0);
    lip_deviation(r * r * GLASS_LIP_SINE, ior) / lip_deviation(GLASS_LIP_SINE, GLASS_IOR)
}

fn glass_band(depth: f32, thickness: f32) -> f32 {
    let t = 1.0 - (depth / (thickness * GLARE_BAND)).clamp(0.0, 1.0);
    t.powi(5)
}

fn glare_lobe(outward: [f32; 2]) -> f32 {
    let along = outward[0] * GLARE_DIR[0] + outward[1] * GLARE_DIR[1];
    let side = if along > 0.0 { 1.0 } else { GLARE_BACK };
    (along * along).powf(GLARE_POWER) * side
}

fn to_oklab(srgb: [f32; 3]) -> [f32; 3] {
    let c = srgb.map(|v| v.max(0.0).powf(2.2));
    let l = 0.4122214708 * c[0] + 0.5363325363 * c[1] + 0.0514459929 * c[2];
    let m = 0.2119034982 * c[0] + 0.6806995451 * c[1] + 0.1073969566 * c[2];
    let s = 0.0883024619 * c[0] + 0.2817188376 * c[1] + 0.6299787005 * c[2];
    let [l, m, s] = [l, m, s].map(|v| v.powf(1.0 / 3.0));
    [
        0.2104542553 * l + 0.7936177850 * m - 0.0040720468 * s,
        1.9779984951 * l - 2.4285922050 * m + 0.4505937099 * s,
        0.0259040371 * l + 0.7827717662 * m - 0.8086757660 * s,
    ]
}

fn from_oklab(lab: [f32; 3]) -> [f32; 3] {
    let l_ = lab[0] + 0.3963377774 * lab[1] + 0.2158037573 * lab[2];
    let m_ = lab[0] - 0.1055613458 * lab[1] - 0.0638541728 * lab[2];
    let s_ = lab[0] - 0.0894841775 * lab[1] - 1.2914855480 * lab[2];
    let (l, m, s) = (l_.powi(3), m_.powi(3), s_.powi(3));
    [
        4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
        -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
        -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s,
    ]
    .map(|v| v.clamp(0.0, 1.0).powf(1.0 / 2.2))
}

fn glare_tone(rgb: [f32; 3], amount: f32) -> [f32; 3] {
    if amount <= 0.0 {
        return rgb;
    }
    let lab = to_oklab(rgb);
    from_oklab([
        (lab[0] + GLARE_LIGHTEN * amount).min(1.0),
        lab[1] * (1.0 + GLARE_CHROMA * amount),
        lab[2] * (1.0 + GLARE_CHROMA * amount),
    ])
}

struct Sample {
    /// Where green is read from, in pixels from the fragment.
    offset: [f32; 2],
    offset_r: [f32; 2],
    offset_b: [f32; 2],
    /// The pane's colour before the rim light and shade go on.
    tinted: [f32; 3],
    /// The pane's colour after them.
    lit: [f32; 3],
    /// The trim a plain material gets instead.
    trim: f32,
    shade: f32,
}

const HALF: [f32; 2] = [120.0, 40.0];

#[allow(clippy::too_many_arguments)]
fn glass(
    refraction: f32,
    interactive: f32,
    specular: f32,
    dist: f32,
    normal: [f32; 2],
    soft: [f32; 3],
    sharp: [f32; 3],
    tint_rgb: [f32; 3],
    tint_a: f32,
) -> Sample {
    let g = refraction_active(refraction);

    let depth = (-dist).max(0.0);
    let thickness = glass_thickness(refraction, HALF);
    let edge = 1.0 - smoothstep(0.0, 1.0, depth / thickness);
    let bend = refraction * (1.0 + interactive * GLASS_PRESS_REFRACTION) * g;

    let off = |ior: f32| {
        let s = lip_shift(depth, thickness, ior) * bend;
        [-normal[0] * s, -normal[1] * s]
    };

    let clarity: f32 = 0.0;
    let mut backdrop = [0.0f32; 3];
    for i in 0..3 {
        backdrop[i] = mix(soft[i], sharp[i], clarity.max(edge * GLASS_RIM_SHARPNESS) * g);
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
    let band = glass_band(depth, thickness) * specular * rim_gain * g;
    let glare = glare_lobe(normal) * GLARE_INTENSITY * band;
    let fresnel = FRESNEL_INTENSITY * band;
    let along = normal[0] * GLARE_DIR[0] + normal[1] * GLARE_DIR[1];
    let away = (-along).max(0.0);
    let shade = away.powf(SHADOW_POWER) * SHADOW_DEPTH * band / rim_gain;

    let toned = glare_tone(tinted, glare);
    let lit = toned.map(|c| (c + fresnel - shade).max(0.0));

    let rim_mask = 1.0 - smoothstep(0.0, RIM_WIDTH, dist.abs());
    let trim = along.abs().powf(RIM_POWER) * rim_mask * RIM_INTENSITY * specular * rim_gain * (1.0 - g);

    Sample {
        offset: off(GLASS_IOR),
        offset_r: off(GLASS_IOR * (1.0 - GLASS_DISPERSION)),
        offset_b: off(GLASS_IOR * (1.0 + GLASS_DISPERSION)),
        tinted,
        lit,
        trim,
        shade,
    }
}

fn brightness(c: [f32; 3]) -> f32 {
    c.iter().sum()
}

const SOFT: [f32; 3] = [0.2, 0.3, 0.4];
const SHARP: [f32; 3] = [0.9, 0.1, 0.5];
const WHITE: [f32; 3] = [1.0, 1.0, 1.0];
const UP: [f32; 2] = [0.0, -1.0];

const TOWARD_LIGHT: [f32; 2] = [-0.70710678, -0.70710678];
const AWAY_FROM_LIGHT: [f32; 2] = [0.70710678, 0.70710678];
const ACROSS_LIGHT: [f32; 2] = [0.70710678, -0.70710678];

#[test]
fn a_material_without_refraction_is_untouched() {
    for dist in [-60.0, -30.0, -12.0, -6.0, -1.0, 0.0] {
        for normal in [UP, TOWARD_LIGHT, AWAY_FROM_LIGHT, [-1.0, 0.0]] {
            let s = glass(0.0, 0.0, 1.0, dist, normal, SOFT, SHARP, WHITE, 0.10);
            assert_eq!(s.offset, [0.0, 0.0], "a plain material must not displace its backdrop");
            assert_eq!(s.shade, 0.0, "a plain material must not be darkened");

            for i in 0..3 {
                let plain = mix(SOFT[i], WHITE[i], 0.10);
                assert!(
                    (s.lit[i] - plain).abs() < 1e-6,
                    "channel {i} drifted from a plain material: {} vs {plain}",
                    s.lit[i]
                );
            }

            let axis = normal[0] * GLARE_DIR[0] + normal[1] * GLARE_DIR[1];
            let mask = 1.0 - smoothstep(0.0, RIM_WIDTH, dist.abs());
            let trim = axis.abs().powf(RIM_POWER) * mask * RIM_INTENSITY;
            assert!(
                (s.trim - trim).abs() < 1e-6,
                "the specular trim changed: {} vs {trim}",
                s.trim
            );
        }
    }
}

#[test]
fn glass_gets_no_plain_trim() {
    let s = glass(18.0, 0.0, 1.0, 0.0, TOWARD_LIGHT, SOFT, SHARP, WHITE, 0.1);
    assert_eq!(s.trim, 0.0);
}

#[test]
fn refraction_is_still_pixels_of_bend_at_the_rim() {
    for refraction in [6.0, 18.0, 26.0, 54.0] {
        let s = glass(refraction, 0.0, 1.0, 0.0, UP, SOFT, SHARP, WHITE, 0.1);
        let got = length(s.offset);
        assert!(
            (got - refraction).abs() / refraction < 0.002,
            "asked for {refraction}px of bend at the rim, got {got}"
        );
    }
}

#[test]
fn the_convex_lip_reads_from_further_in() {
    // The outward normal points up, so further in is down: +y.
    let s = glass(18.0, 0.0, 1.0, -3.0, UP, SOFT, SHARP, WHITE, 0.1);
    assert!(s.offset[1] > 0.0, "the lip should sample inward, got {:?}", s.offset);
    assert!(s.offset[0].abs() < 1e-6);
}

#[test]
fn displacement_falls_away_across_the_lip_and_is_gone_past_it() {
    let at = |dist: f32| length(glass(18.0, 0.0, 1.0, dist, UP, SOFT, SHARP, WHITE, 0.1).offset);
    let thickness = glass_thickness(18.0, HALF);

    let mut previous = at(0.0);
    for step in 1..=(thickness as i32) {
        let next = at(-(step as f32));
        assert!(next <= previous, "the lip is not monotone at {step}px in: {next} after {previous}");
        previous = next;
    }
    assert!(at(-thickness) < 1e-4, "the end of the lip should be undisplaced");
    assert!(at(-60.0) < 1e-4, "the interior should be undisplaced");
}

#[test]
fn no_pixel_of_the_lip_jumps_by_more_than_a_sixth_of_the_bend() {
    let thickness = glass_thickness(36.0, HALF);
    let mut previous = lip_shift(0.0, thickness, GLASS_IOR);
    let mut depth = 1.0;
    while depth <= thickness {
        let next = lip_shift(depth, thickness, GLASS_IOR);
        assert!(
            previous - next < 1.0 / 6.0,
            "the lip drops {} of the bend between {}px and {depth}px",
            previous - next,
            depth - 1.0
        );
        previous = next;
        depth += 1.0;
    }
}

#[test]
fn a_stronger_refraction_has_a_wider_lip() {
    let at = |refraction: f32| {
        length(glass(refraction, 0.0, 1.0, -20.0, UP, SOFT, SHARP, WHITE, 0.1).offset)
    };
    assert!(at(30.0) > 0.0, "a 30px bend should still be bending 20px in");
    assert_eq!(at(8.0), 0.0, "an 8px bend has a 12px lip and should be done by 20px in");
}

#[test]
fn the_lip_never_runs_past_the_middle() {
    let thin = glass_thickness(200.0, [80.0, 10.0]);
    assert!(thin <= 10.0, "a 20px-tall pane got a {thin}px lip");
}

#[test]
fn a_press_bends_further() {
    let at = |held: f32| length(glass(18.0, held, 1.0, -2.0, UP, SOFT, SHARP, WHITE, 0.1).offset);
    assert!(at(1.0) > at(0.0), "interactive glass should deepen on press");
}

#[test]
fn blue_bends_most_and_red_least_and_they_split_furthest_at_the_rim() {
    let split = |dist: f32| {
        let s = glass(18.0, 0.0, 1.0, dist, UP, SOFT, SHARP, WHITE, 0.1);
        let (r, g, b) = (length(s.offset_r), length(s.offset), length(s.offset_b));
        assert!(r < g && g < b, "the channels must stay ordered red < green < blue, got {r} {g} {b}");
        b - r
    };
    assert!(split(-0.5) > split(-12.0), "the fringe should be a rim effect, not a whole-pane one");
}

#[test]
fn the_glare_burns_toward_the_light_and_the_shadow_falls_away_from_it() {
    let at = |normal: [f32; 2]| {
        let s = glass(18.0, 0.0, 1.0, 0.0, normal, SOFT, SOFT, WHITE, 0.1);
        (brightness(s.lit), s.shade)
    };
    let (lit_h, lit_s) = at(TOWARD_LIGHT);
    let (dark_h, dark_s) = at(AWAY_FROM_LIGHT);
    let (side_h, _) = at(ACROSS_LIGHT);

    assert!(lit_h > dark_h, "the lit side should be brighter: {lit_h} vs {dark_h}");
    assert!(dark_h > side_h, "the far side keeps a dimmer second lobe: {dark_h} vs {side_h}");
    assert!(dark_s > lit_s, "the shadow should fall on the far side: {dark_s} vs {lit_s}");
    assert_eq!(lit_s, 0.0, "the lit side must not also be shadowed");
}

#[test]
fn the_glare_keeps_the_colour_of_what_is_behind_it() {
    let red = [0.76, 0.36, 0.23];
    let s = glass(18.0, 0.0, 1.0, 0.0, TOWARD_LIGHT, red, red, WHITE, 0.0);
    assert!(brightness(s.lit) > brightness(s.tinted), "the glare should lighten");
    let spread = |c: [f32; 3]| c[0] - c[2];
    assert!(
        spread(s.lit) >= spread(red) * 0.9,
        "the glare washed the red out: {:?} from {red:?}",
        s.lit
    );
}

#[test]
fn nothing_is_lit_or_shadowed_away_from_the_lip() {
    let s = glass(18.0, 0.0, 1.0, -60.0, TOWARD_LIGHT, SOFT, SHARP, WHITE, 0.1);
    for i in 0..3 {
        assert!((s.lit[i] - s.tinted[i]).abs() < 1e-4, "the interior is lit: {:?} vs {:?}", s.lit, s.tinted);
    }
    assert!(s.shade < 1e-4, "the interior is shadowed: {}", s.shade);
}

#[test]
fn lighting_is_off_without_specular() {
    let s = glass(18.0, 1.0, 0.0, 0.0, TOWARD_LIGHT, SOFT, SHARP, WHITE, 0.1);
    assert_eq!(s.lit, s.tinted);
    assert_eq!(s.shade, 0.0);
}

#[test]
fn the_tint_follows_the_backdrop() {
    let sum = |backdrop: [f32; 3]| {
        let s = glass(18.0, 0.0, 1.0, -60.0, UP, backdrop, SHARP, WHITE, 0.6);
        brightness(s.tinted)
    };
    assert!(
        sum([0.02; 3]) < sum([0.95; 3]),
        "glass over dark content should not come out lighter than glass over light"
    );
}
