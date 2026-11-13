struct Uniforms {
    resolution : vec2<f32>,
    time       : f32,
    _pad       : f32,
};
@group(0) @binding(0) var<uniform> u : Uniforms;

const CORNER_N : f32 = 2.0;

fn sd_rounded_rect(p: vec2<f32>, size: vec2<f32>, r: f32) -> f32 {
    let q  = abs(p) - size + r;
    let qc = max(q, vec2<f32>(0.0));
    let corner = pow(pow(qc.x, CORNER_N) + pow(qc.y, CORNER_N), 1.0 / CORNER_N);
    return corner + min(max(q.x, q.y), 0.0) - r;
}

fn sd_rounded_rect_normal(p: vec2<f32>, size: vec2<f32>, r: f32) -> vec2<f32> {
    let e = vec2<f32>(0.5, 0.0);
    let dx = sd_rounded_rect(p + e.xy, size, r) - sd_rounded_rect(p - e.xy, size, r);
    let dy = sd_rounded_rect(p + e.yx, size, r) - sd_rounded_rect(p - e.yx, size, r);
    let g   = vec2<f32>(dx, dy);
    let len = length(g);
    if (len < 1e-6) {
        return vec2<f32>(0.0, 0.0);
    }
    return g / len;
}

const RIM_WIDTH: f32       = 2.5;
const RIM_INTENSITY: f32   = 0.4;
const RIM_POWER: f32       = 2.0;
const RIM_COLOR: vec3<f32> = vec3<f32>(1.0, 1.0, 1.0);

// The lip: a quarter-round bevel running in from the border. Its width
// follows the refraction (both arrive in pixels, so it scales with the
// screen), clamped so a small pane is not all lip and a thin one does not
// fold over itself.
const GLASS_THICKNESS_PER_BEND: f32 = 1.5;
const GLASS_MIN_THICKNESS: f32 = 12.0;

const GLASS_RIM_SHARPNESS: f32 = 0.95;
const GLASS_BLUR_DISPLACE: f32 = 0.35;
const GLASS_ADAPTIVE: f32 = 0.25;
const GLASS_PRESS_REFRACTION: f32 = 0.25;
const GLASS_PRESS_RIM: f32 = 0.8;
// How much a held pane magnifies what is under it, about its own centre.
const GLASS_PRESS_ZOOM: f32 = 0.08;
const GLASS_IOR: f32 = 1.5;
// The sine of the lip's tilt at the border. 1 would stand it vertical, as
// LiquidGlassKit's does, but then the displacement jumps by a third of the
// bend inside the last pixel and the rim stair-steps. Just short of vertical
// keeps the same curve with a slope a pixel can sample.
const GLASS_LIP_SINE: f32 = 0.92;
// Red and blue get their own index either side of green's, so they bend by
// different amounts and fringe where the bend is steepest.
const GLASS_DISPERSION: f32 = 0.04;
const GLASS_VIBRANCY: f32 = 0.0;

// An even, faint brightening around the whole rim.
const FRESNEL_INTENSITY: f32 = 0.04;

// The glare is a pair of lobes on opposite sides of the rim, the brighter
// one facing GLARE_DIR. It lightens the backdrop rather than painting white
// over it, and pushes its chroma a little, so a highlight over a red photo
// reads as a brighter red.
const GLARE_DIR: vec2<f32> = vec2<f32>(-0.70710678, -0.70710678);
const GLARE_POWER: f32 = 1.5;
const GLARE_INTENSITY: f32 = 1.0;
const GLARE_BACK: f32 = 0.45;
// The width of the lit band, as a share of the lip.
const GLARE_BAND: f32 = 0.6;
const GLARE_LIGHTEN: f32 = 0.3;
const GLARE_CHROMA: f32 = 0.8;

const SHADOW_POWER: f32 = 3.0;
const SHADOW_DEPTH: f32 = 0.05;

fn refraction_active(refraction: f32) -> f32 {
    return step(0.001, refraction);
}

fn glass_thickness(refraction: f32, half: vec2<f32>) -> f32 {
    let wanted = max(refraction * GLASS_THICKNESS_PER_BEND, GLASS_MIN_THICKNESS);
    return max(min(wanted, min(half.x, half.y)), 1.0);
}

// How far a ray falling straight down is carried sideways by the lip, `depth`
// pixels in from the border, for glass of index `ior`.
//
// The lip's surface tilts from flat at the inner end to near vertical at
// the border; r² is the sine of that tilt (the profile LiquidGlassKit uses).
// The ray meets the surface at that angle, Snell's law bends it to a
// shallower one, and the difference is how far off straight down it
// travels. The result is scaled so green at the very border is 1: `bend` is
// then exactly the pixels of displacement asked for there, falling away to 0
// at the end of the lip.
fn lip_deviation(sine: f32, ior: f32) -> f32 {
    let incident = asin(sine);
    return tan(incident - asin(sine / ior));
}

fn lip_shift(depth: f32, thickness: f32, ior: f32) -> f32 {
    let r = 1.0 - clamp(depth / thickness, 0.0, 1.0);
    return lip_deviation(r * r * GLASS_LIP_SINE, ior)
         / lip_deviation(GLASS_LIP_SINE, GLASS_IOR);
}

// The lit band: full strength at the border, gone a little over halfway
// across the lip, with most of it in the outer few pixels.
fn glass_band(depth: f32, thickness: f32) -> f32 {
    let t = 1.0 - clamp(depth / (thickness * GLARE_BAND), 0.0, 1.0);
    return t * t * t * t * t;
}

// Two lobes along GLARE_DIR: cos² of the angle between the rim's normal and
// the light, narrowed by GLARE_POWER, with the far side dimmed.
fn glare_lobe(outward: vec2<f32>) -> f32 {
    let along = dot(outward, GLARE_DIR);
    let side = select(GLARE_BACK, 1.0, along > 0.0);
    return pow(along * along, GLARE_POWER) * side;
}

// Oklab, for lightening a colour without washing it out. The textures are
// plain Unorm, so what comes in is sRGB-encoded; a 2.2 power is close enough
// for a highlight.
fn glass_to_oklab(srgb: vec3<f32>) -> vec3<f32> {
    let c = pow(max(srgb, vec3<f32>(0.0)), vec3<f32>(2.2));
    let l = 0.4122214708 * c.r + 0.5363325363 * c.g + 0.0514459929 * c.b;
    let m = 0.2119034982 * c.r + 0.6806995451 * c.g + 0.1073969566 * c.b;
    let s = 0.0883024619 * c.r + 0.2817188376 * c.g + 0.6299787005 * c.b;
    let lms = pow(vec3<f32>(l, m, s), vec3<f32>(1.0 / 3.0));
    return vec3<f32>(
        0.2104542553 * lms.x + 0.7936177850 * lms.y - 0.0040720468 * lms.z,
        1.9779984951 * lms.x - 2.4285922050 * lms.y + 0.4505937099 * lms.z,
        0.0259040371 * lms.x + 0.7827717662 * lms.y - 0.8086757660 * lms.z,
    );
}

fn glass_from_oklab(lab: vec3<f32>) -> vec3<f32> {
    let l_ = lab.x + 0.3963377774 * lab.y + 0.2158037573 * lab.z;
    let m_ = lab.x - 0.1055613458 * lab.y - 0.0638541728 * lab.z;
    let s_ = lab.x - 0.0894841775 * lab.y - 1.2914855480 * lab.z;
    let l = l_ * l_ * l_;
    let m = m_ * m_ * m_;
    let s = s_ * s_ * s_;
    let c = vec3<f32>(
         4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
        -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
        -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s,
    );
    return pow(clamp(c, vec3<f32>(0.0), vec3<f32>(1.0)), vec3<f32>(1.0 / 2.2));
}

// Lightens `rgb` by `amount` of a glare, keeping and slightly raising its
// chroma. 0 returns the colour as it was.
fn glare_tone(rgb: vec3<f32>, amount: f32) -> vec3<f32> {
    if (amount <= 0.0) {
        return rgb;
    }
    let lab = glass_to_oklab(rgb);
    let lit = vec3<f32>(
        min(lab.x + GLARE_LIGHTEN * amount, 1.0),
        lab.yz * (1.0 + GLARE_CHROMA * amount),
    );
    return glass_from_oklab(lit);
}

fn hash(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453);
}

fn backdrop(uv: vec2<f32>) -> vec3<f32> {
    let px = uv * u.resolution;
    let scrolled = px + vec2<f32>(0.0, u.time * 40.0);

    var col = vec3<f32>(0.086, 0.078, 0.059);

    let row = floor(scrolled.y / 22.0);
    let in_row = fract(scrolled.y / 22.0);
    if (in_row < 0.55) {
        let indent = floor(hash(vec2<f32>(row, 1.0)) * 4.0) * 28.0;
        let width  = 120.0 + hash(vec2<f32>(row, 2.0)) * 420.0;
        let x = scrolled.x - 40.0 - indent;
        if (x > 0.0 && x < width) {

            let seg = floor(x / 46.0);
            let h   = hash(vec2<f32>(row, seg));
            var tok = vec3<f32>(0.91, 0.88, 0.84);
            if (h < 0.25) {
                tok = vec3<f32>(0.757, 0.357, 0.229);
            } else if (h < 0.45) {
                tok = vec3<f32>(0.878, 0.643, 0.361);
            } else if (h < 0.62) {
                tok = vec3<f32>(0.608, 0.690, 0.420);
            } else if (h < 0.72) {
                tok = vec3<f32>(0.365, 0.337, 0.282);
            }
            col = tok;
        }
    }

    return col * (0.75 + 0.5 * uv.x);
}

fn backdrop_blurred(uv: vec2<f32>, radius: f32) -> vec3<f32> {
    let r = radius / u.resolution.y;
    var sum = backdrop(uv);
    sum = sum + backdrop(uv + vec2<f32>(r, 0.0));
    sum = sum + backdrop(uv + vec2<f32>(-r, 0.0));
    sum = sum + backdrop(uv + vec2<f32>(0.0, r));
    sum = sum + backdrop(uv + vec2<f32>(0.0, -r));
    sum = sum + backdrop(uv + vec2<f32>(r, r) * 0.7);
    sum = sum + backdrop(uv + vec2<f32>(-r, r) * 0.7);
    sum = sum + backdrop(uv + vec2<f32>(r, -r) * 0.7);
    sum = sum + backdrop(uv + vec2<f32>(-r, -r) * 0.7);
    return sum / 9.0;
}

struct VSOut {
    @builtin(position) position : vec4<f32>,
    @location(0)       uv       : vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VSOut {

    let uv  = vec2<f32>(f32((vi << 1u) & 2u), f32(vi & 2u));
    let pos = uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0);
    var out: VSOut;
    out.position = vec4<f32>(pos, 0.0, 1.0);
    out.uv = uv;
    return out;
}

@fragment
fn fs_main(in: VSOut) -> @location(0) vec4<f32> {
    let px = in.uv * u.resolution;

    let centre = u.resolution * vec2<f32>(0.5, 0.5)
               + vec2<f32>(sin(u.time * 0.6) * u.resolution.x * 0.08, 0.0);
    let half   = u.resolution * vec2<f32>(0.30, 0.16);
    let radius = 40.0;

    let refraction  = 18.0 * 2.0;
    let blur_radius = 24.0 * 2.0;
    let tint        = vec4<f32>(1.0, 1.0, 1.0, 0.10);
    let specular    = 1.0;
    let clarity     = 1.0;

    let interactive = 0.5 + 0.5 * sin(u.time * 2.0);

    let p    = px - centre;
    let dist = sd_rounded_rect(p, half, radius);
    let aa   = length(vec2<f32>(dpdx(dist), dpdy(dist)));
    let fill = smoothstep(aa, -aa, dist);

    if (fill <= 0.001) {
        return vec4<f32>(backdrop(in.uv), 1.0);
    }

    let normal = sd_rounded_rect_normal(p, half, radius);

    let depth = max(-dist, 0.0);
    let thickness = glass_thickness(refraction, half);
    let edge = 1.0 - smoothstep(0.0, 1.0, depth / thickness);
    let bend = refraction * (1.0 + interactive * GLASS_PRESS_REFRACTION);

    let zoom = 1.0 / (1.0 + interactive * GLASS_PRESS_ZOOM);
    let lens_uv = (centre + (px - centre) * zoom) / u.resolution;

    let inward = -normal * bend / u.resolution;
    let off_g = inward * lip_shift(depth, thickness, GLASS_IOR);
    let off_r = inward * lip_shift(depth, thickness, GLASS_IOR * (1.0 - GLASS_DISPERSION));
    let off_b = inward * lip_shift(depth, thickness, GLASS_IOR * (1.0 + GLASS_DISPERSION));

    let sharp_uv = clamp(lens_uv + off_g, vec2<f32>(0.0), vec2<f32>(1.0));
    let soft_uv  = clamp(lens_uv + off_g * GLASS_BLUR_DISPLACE, vec2<f32>(0.0), vec2<f32>(1.0));
    let uv_r     = clamp(lens_uv + off_r, vec2<f32>(0.0), vec2<f32>(1.0));
    let uv_b     = clamp(lens_uv + off_b, vec2<f32>(0.0), vec2<f32>(1.0));

    let sharp = vec3<f32>(
        backdrop(uv_r).r,
        backdrop(sharp_uv).g,
        backdrop(uv_b).b,
    );
    let soft = backdrop_blurred(soft_uv, blur_radius);

    let backdrop_rgb = mix(soft, sharp, max(clarity, edge * GLASS_RIM_SHARPNESS));

    let bd_luma = dot(backdrop_rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
    let vibrant = mix(vec3<f32>(bd_luma), backdrop_rgb, 1.0 + GLASS_VIBRANCY);

    let luma     = dot(soft, vec3<f32>(0.2126, 0.7152, 0.0722));
    let adaptive = mix(tint.rgb, vec3<f32>(luma), GLASS_ADAPTIVE);
    let tinted   = mix(vibrant, adaptive, tint.a);

    let rim_gain = 1.0 + interactive * GLASS_PRESS_RIM;
    let band     = glass_band(depth, thickness) * specular * rim_gain;
    let glare    = glare_lobe(normal) * GLARE_INTENSITY * band;
    let fresnel  = FRESNEL_INTENSITY * band;
    let away     = max(-dot(normal, GLARE_DIR), 0.0);
    let shade    = pow(away, SHADOW_POWER) * SHADOW_DEPTH * band / rim_gain;

    let col = max(glare_tone(tinted, glare) + RIM_COLOR * fresnel - shade, vec3<f32>(0.0));

    return vec4<f32>(mix(backdrop(in.uv), col, fill), 1.0);
}
