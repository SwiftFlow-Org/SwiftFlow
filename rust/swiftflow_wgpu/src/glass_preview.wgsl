struct Uniforms {
    resolution : vec2<f32>,
    time       : f32,
    _pad       : f32,
};
@group(0) @binding(0) var<uniform> u : Uniforms;

const GLASS_MIN_THICKNESS: f32 = 12.0;
const GLASS_RIM_SHARPNESS: f32 = 0.85;
const GLASS_BLUR_DISPLACE: f32 = 0.35;
const GLASS_ADAPTIVE: f32 = 0.25;
const GLASS_PRESS_REFRACTION: f32 = 0.6;
const GLASS_PRESS_RIM: f32 = 0.8;

const GLASS_IOR: f32 = 1.5;
const GLASS_ETA: f32 = 1.0 / GLASS_IOR;
const GLASS_DISPERSION: f32 = 0.02;

const FRESNEL_POWER: f32 = 5.0;
const FRESNEL_INTENSITY: f32 = 0.35;
const GLARE_DIR: vec2<f32> = vec2<f32>(-0.70710678, -0.70710678);
const GLARE_POWER: f32 = 6.0;
const GLARE_INTENSITY: f32 = 0.85;
const GLARE_BACK: f32 = 0.3;
const SHADOW_POWER: f32 = 3.0;
const SHADOW_DEPTH: f32 = 0.3;

const GLASS_VIBRANCY: f32 = 0.35;

const RIM_WIDTH: f32       = 3.5;
const RIM_INTENSITY: f32   = 0.4;
const RIM_POWER: f32       = 2.0;
const RIM_COLOR: vec3<f32> = vec3<f32>(1.0, 1.0, 1.0);

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

fn bevel_normal(outward: vec2<f32>, t: f32) -> vec3<f32> {
    let u_ = 1.0 - clamp(t, 0.0, 1.0);
    let h = sqrt(max(1.0 - u_ * u_, 0.0));
    let slope = u_ / max(h, 1e-3);
    return normalize(vec3<f32>(-outward * slope, 1.0));
}

fn refract_offset(n: vec3<f32>, eta: f32) -> vec2<f32> {
    let r = refract(vec3<f32>(0.0, 0.0, -1.0), n, eta);
    return r.xy / sqrt(max(1.0 - GLASS_ETA * GLASS_ETA, 1e-6));
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

    let interactive = 0.5 + 0.5 * sin(u.time * 2.0);

    let p    = px - centre;
    let dist = sd_rounded_rect(p, half, radius);
    let aa   = length(vec2<f32>(dpdx(dist), dpdy(dist)));
    let fill = smoothstep(aa, -aa, dist);

    if (fill <= 0.001) {
        return vec4<f32>(backdrop(in.uv), 1.0);
    }

    let normal = sd_rounded_rect_normal(p, half, radius);
    let bevel  = max(radius, GLASS_MIN_THICKNESS);

    let depth = clamp(-dist / bevel, 0.0, 1.0);
    let edge  = 1.0 - smoothstep(0.0, 1.0, depth);
    let bend  = refraction * (1.0 + interactive * GLASS_PRESS_REFRACTION);

    let n3 = bevel_normal(normal, depth);
    let offset = refract_offset(n3, GLASS_ETA) * bend / u.resolution;

    let sharp_uv = clamp(in.uv + offset, vec2<f32>(0.0), vec2<f32>(1.0));
    let soft_uv  = clamp(in.uv + offset * GLASS_BLUR_DISPLACE, vec2<f32>(0.0), vec2<f32>(1.0));

    let uv_r = clamp(
        in.uv + refract_offset(n3, GLASS_ETA * (1.0 + GLASS_DISPERSION)) * bend / u.resolution,
        vec2<f32>(0.0), vec2<f32>(1.0)
    );
    let uv_b = clamp(
        in.uv + refract_offset(n3, GLASS_ETA * (1.0 - GLASS_DISPERSION)) * bend / u.resolution,
        vec2<f32>(0.0), vec2<f32>(1.0)
    );
    let sharp = vec3<f32>(
        backdrop(uv_r).r,
        backdrop(sharp_uv).g,
        backdrop(uv_b).b,
    );
    let soft = backdrop_blurred(soft_uv, blur_radius);

    let backdrop_rgb = mix(soft, sharp, edge * GLASS_RIM_SHARPNESS);

    let bd_luma = dot(backdrop_rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
    let vibrant = mix(vec3<f32>(bd_luma), backdrop_rgb, 1.0 + GLASS_VIBRANCY);

    let luma     = dot(soft, vec3<f32>(0.2126, 0.7152, 0.0722));
    let adaptive = mix(tint.rgb, vec3<f32>(luma), GLASS_ADAPTIVE);
    var col      = mix(vibrant, adaptive, tint.a);

    let rim_gain = 1.0 + interactive * GLASS_PRESS_RIM;
    let fresnel  = pow(1.0 - n3.z, FRESNEL_POWER);
    let toward   = max(dot(normal, GLARE_DIR), 0.0);
    let away     = max(dot(normal, -GLARE_DIR), 0.0);
    let glare    = pow(toward, GLARE_POWER) + pow(away, GLARE_POWER) * GLARE_BACK;
    let lit      = fresnel * (FRESNEL_INTENSITY + glare * GLARE_INTENSITY);
    col = col + RIM_COLOR * lit * specular * rim_gain;

    let shade = fresnel * pow(away, SHADOW_POWER) * SHADOW_DEPTH * specular;
    col = max(col - shade, vec3<f32>(0.0));

    return vec4<f32>(mix(backdrop(in.uv), col, fill), 1.0);
}
