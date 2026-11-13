struct InstanceInput {
    @location(0) rect          : vec4<f32>,
    @location(1) fill          : vec4<f32>,
    @location(2) border_color  : vec4<f32>,
    @location(3) params        : vec4<f32>,
    @location(4) shadow        : vec2<f32>,
    @location(5) _pad          : vec2<f32>,
    @location(6) clip_rect     : vec4<f32>,
    @location(7) clip_radii    : vec4<f32>,
};

struct VertexOutput {
    @builtin(position) position : vec4<f32>,
    @location(0)       uv       : vec2<f32>,

    @location(1)       size     : vec2<f32>,
    @location(2)       fill     : vec4<f32>,
    @location(3)       border_color : vec4<f32>,
    @location(4)       params   : vec4<f32>,

    @location(5)       shape    : vec2<f32>,
    @location(6)       shadow   : vec2<f32>,
    @location(7)       screen     : vec2<f32>,
    @location(8)       clip_rect  : vec4<f32>,
    @location(9)       clip_radii : vec4<f32>,
};

struct Globals {
    resolution : vec2<f32>,
    _pad       : vec2<f32>,
};

@group(0) @binding(0)
var<uniform> globals: Globals;

@vertex
fn vs_main(
    @builtin(vertex_index) vert_idx : u32,
    instance                        : InstanceInput,
) -> VertexOutput {

    var quad = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 1.0),
    );

    let local = quad[vert_idx];

    let rect_x = instance.rect.x;
    let rect_y = instance.rect.y;
    let rect_w = instance.rect.z;
    let rect_h = instance.rect.w;

    let grow = instance.shadow.x;

    let pixel = vec2<f32>(
        rect_x - grow + local.x * (rect_w + 2.0 * grow),
        rect_y - grow + local.y * (rect_h + 2.0 * grow),
    );
    let ndc = vec2<f32>(
         (pixel.x / globals.resolution.x) * 2.0 - 1.0,
        -((pixel.y / globals.resolution.y) * 2.0 - 1.0),
    );

    var out: VertexOutput;
    out.position     = vec4<f32>(ndc, 0.0, 1.0);
    out.uv           = local;
    out.size         = vec2<f32>(rect_w + 2.0 * grow, rect_h + 2.0 * grow);
    out.shape        = vec2<f32>(rect_w, rect_h);
    out.fill         = instance.fill;
    out.border_color = instance.border_color;
    out.params       = instance.params;
    out.shadow       = instance.shadow;
    out.screen       = pixel;
    out.clip_rect    = instance.clip_rect;
    out.clip_radii   = instance.clip_radii;
    return out;
}

const CORNER_SUPERELLIPSE_N: f32 = 2.0;

fn sd_rounded_rect(p: vec2<f32>, size: vec2<f32>, r: f32, n: f32) -> f32 {
    let q  = abs(p) - size + r;
    let qc = max(q, vec2<f32>(0.0));

    let corner_dist = pow(
        pow(qc.x, n) + pow(qc.y, n),
        1.0 / n
    );
    return corner_dist + min(max(q.x, q.y), 0.0) - r;
}

fn sd_rounded_rect_normal(p: vec2<f32>, size: vec2<f32>, r: f32, n: f32) -> vec2<f32> {
    let e = vec2<f32>(0.5, 0.0);
    let dx = sd_rounded_rect(p + e.xy, size, r, n) - sd_rounded_rect(p - e.xy, size, r, n);
    let dy = sd_rounded_rect(p + e.yx, size, r, n) - sd_rounded_rect(p - e.yx, size, r, n);
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

fn sd_clip_rect(p: vec2<f32>, half: vec2<f32>, radii: vec4<f32>) -> f32 {
    var r = radii.z;
    if (p.y < 0.0 && p.x < 0.0)        { r = radii.x; }
    else if (p.y < 0.0 && p.x >= 0.0)  { r = radii.y; }
    else if (p.y >= 0.0 && p.x < 0.0)  { r = radii.w; }
    let q = abs(p) - half + vec2<f32>(r, r);
    return min(max(q.x, q.y), 0.0) + length(max(q, vec2<f32>(0.0, 0.0))) - r;
}

fn clip_coverage(screen: vec2<f32>, clip_rect: vec4<f32>, clip_radii: vec4<f32>) -> f32 {
    if (clip_rect.z < 0.0) {
        return 1.0;
    }
    let half   = clip_rect.zw * 0.5;
    let centre = clip_rect.xy + half;
    let d = sd_clip_rect(screen - centre, half, clip_radii);

    return 1.0 - smoothstep(-0.5, 0.5, d);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {

    let half = in.shape * 0.5;
    var corner_radius = in.params.x;
    var corner_n = CORNER_SUPERELLIPSE_N;

    if (corner_radius < 0.0) {
        corner_radius = min(half.x, half.y);
        corner_n = 2.0;
    }
    let border_width  = in.params.y;

    let p    = (in.uv - vec2<f32>(0.5)) * in.size;

    let dist = sd_rounded_rect(p, half, corner_radius, corner_n);

    let aa = length(vec2<f32>(dpdx(dist), dpdy(dist)));

    let fill   = smoothstep( aa, -aa, dist);
    // The band between the outer edge and border_width inside it: what is
    // inside the outer edge less what is inside the inner one.
    let border = smoothstep( aa, -aa, dist)
               - smoothstep( aa, -aa, dist + border_width);

    var color = vec4<f32>(0.0);
    color = mix(color, in.fill,         fill);
    color = mix(color, in.border_color, border);

    let specular = in.params.z;
    let normal   = sd_rounded_rect_normal(p, half, corner_radius, corner_n);
    let rim_axis = normalize(vec2<f32>(-1.0, -1.0));
    let facing   = abs(dot(normal, rim_axis));
    let rim_mask = 1.0 - smoothstep(0.0, RIM_WIDTH, abs(dist));
    let rim      = pow(facing, RIM_POWER) * rim_mask * RIM_INTENSITY * specular;
    color = vec4<f32>(color.rgb + RIM_COLOR * rim * color.a, color.a);

    let clip = clip_coverage(in.screen, in.clip_rect, in.clip_radii);

    let shadow_radius = in.shadow.x;
    let shadow_op = in.shadow.y;
    if (shadow_radius > 0.0 && shadow_op > 0.0) {
        let falloff = clamp(1.0 - dist / shadow_radius, 0.0, 1.0);
        let shadow_a = shadow_op * falloff * falloff * (1.0 - color.a);
        let out_a = color.a + shadow_a;
        if (out_a > 0.0) {

            return vec4<f32>(color.rgb * (color.a / out_a), out_a * clip);
        }
    }

    return vec4<f32>(color.rgb, color.a * clip);
}

struct MergedInstance {
    bounds              : vec4<f32>,
    group_meta          : vec4<f32>,
    member_rect         : array<vec4<f32>, 4>,
    member_fill         : array<vec4<f32>, 4>,
    member_border_color : array<vec4<f32>, 4>,
    member_params       : array<vec4<f32>, 4>,
    clip_rect           : vec4<f32>,
    clip_radii          : vec4<f32>,
};

struct MergedInstances {
    data: array<MergedInstance>,
};

@group(1) @binding(0)
var<storage, read> merged: MergedInstances;

struct MergedVertexOutput {
    @builtin(position) position : vec4<f32>,
    @location(0)       uv       : vec2<f32>,
    @location(1) @interpolate(flat) instance_index : u32,
};

@vertex
fn vs_merged(
    @builtin(vertex_index)   vert_idx : u32,
    @builtin(instance_index) inst_idx : u32,
) -> MergedVertexOutput {
    var quad = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 1.0),
    );

    let local = quad[vert_idx];
    let inst  = merged.data[inst_idx];

    let pixel = vec2<f32>(
        inst.bounds.x + local.x * inst.bounds.z,
        inst.bounds.y + local.y * inst.bounds.w,
    );
    let ndc = vec2<f32>(
         (pixel.x / globals.resolution.x) * 2.0 - 1.0,
        -((pixel.y / globals.resolution.y) * 2.0 - 1.0),
    );

    var out: MergedVertexOutput;
    out.position       = vec4<f32>(ndc, 0.0, 1.0);
    out.uv             = local;
    out.instance_index = inst_idx;
    return out;
}

struct MergedFold {
    dist          : f32,
    fill          : vec4<f32>,
    border_color  : vec4<f32>,
    border_width  : f32,
    specular      : f32,
};

fn fold_members(p_screen: vec2<f32>, inst_idx: u32, count: u32, blend_k: f32, color_blend_k: f32) -> MergedFold {
    var result: MergedFold;

    for (var i: u32 = 0u; i < count; i = i + 1u) {
        let m_rect   = merged.data[inst_idx].member_rect[i];
        let m_fill   = merged.data[inst_idx].member_fill[i];
        let m_border = merged.data[inst_idx].member_border_color[i];
        let m_params = merged.data[inst_idx].member_params[i];

        let half   = m_rect.zw * 0.5;
        let center = m_rect.xy + half;
        let p      = p_screen - center;

        var corner_radius = m_params.x;
        var corner_n      = CORNER_SUPERELLIPSE_N;
        if (corner_radius < 0.0) {
            corner_radius = min(half.x, half.y);
            corner_n = 2.0;
        }
        let border_width = m_params.y;
        let specular      = m_params.z;
        let d = sd_rounded_rect(p, half, corner_radius, corner_n);

        if (i == 0u) {
            result.dist         = d;
            result.fill         = m_fill;
            result.border_color = m_border;
            result.border_width = border_width;
            result.specular     = specular;
        } else {
            let dist_diff = d - result.dist;
            let h = clamp(0.5 + 0.5 * dist_diff / blend_k, 0.0, 1.0);

            let h_color = smoothstep(0.0, 1.0, clamp(0.5 + 0.5 * dist_diff / color_blend_k, 0.0, 1.0));
            result.dist         = mix(d, result.dist, h) - blend_k * h * (1.0 - h);
            result.fill         = mix(m_fill, result.fill, h_color);
            result.border_color = mix(m_border, result.border_color, h_color);
            result.border_width = mix(border_width, result.border_width, h_color);

            result.specular     = mix(specular, result.specular, h_color);
        }
    }

    return result;
}

fn sd_merged(p_screen: vec2<f32>, inst_idx: u32, count: u32, blend_k: f32, color_blend_k: f32) -> f32 {
    return fold_members(p_screen, inst_idx, count, blend_k, color_blend_k).dist;
}

fn sd_merged_normal(p_screen: vec2<f32>, inst_idx: u32, count: u32, blend_k: f32, color_blend_k: f32) -> vec2<f32> {
    let e  = vec2<f32>(0.5, 0.0);
    let dx = sd_merged(p_screen + e.xy, inst_idx, count, blend_k, color_blend_k) - sd_merged(p_screen - e.xy, inst_idx, count, blend_k, color_blend_k);
    let dy = sd_merged(p_screen + e.yx, inst_idx, count, blend_k, color_blend_k) - sd_merged(p_screen - e.yx, inst_idx, count, blend_k, color_blend_k);
    let g   = vec2<f32>(dx, dy);
    let len = length(g);
    if (len < 1e-6) {
        return vec2<f32>(0.0, 0.0);
    }
    return g / len;
}

@fragment
fn fs_merged(in: MergedVertexOutput) -> @location(0) vec4<f32> {
    let inst_idx      = in.instance_index;
    let count         = u32(merged.data[inst_idx].group_meta.x);
    let blend_k       = merged.data[inst_idx].group_meta.y;
    let color_blend_k = merged.data[inst_idx].group_meta.z;
    let p_screen      = merged.data[inst_idx].bounds.xy + in.uv * merged.data[inst_idx].bounds.zw;

    let folded = fold_members(p_screen, inst_idx, count, blend_k, color_blend_k);
    let dist   = folded.dist;

    let aa = length(vec2<f32>(dpdx(dist), dpdy(dist)));

    let fill_mask   = smoothstep(aa, -aa, dist);
    let border_mask = smoothstep(aa, -aa, dist)
                    - smoothstep(aa, -aa, dist + folded.border_width);

    var color = vec4<f32>(0.0);
    color = mix(color, folded.fill,         fill_mask);
    color = mix(color, folded.border_color, border_mask);

    let normal     = sd_merged_normal(p_screen, inst_idx, count, blend_k, color_blend_k);
    let rim_axis   = normalize(vec2<f32>(-1.0, -1.0));
    let facing     = abs(dot(normal, rim_axis));
    let rim_mask   = 1.0 - smoothstep(0.0, RIM_WIDTH, abs(dist));
    let rim        = pow(facing, RIM_POWER) * rim_mask * RIM_INTENSITY * folded.specular;
    color = vec4<f32>(color.rgb + RIM_COLOR * rim * color.a, color.a);

    let clip = clip_coverage(
        p_screen, merged.data[inst_idx].clip_rect, merged.data[inst_idx].clip_radii);
    return vec4<f32>(color.rgb, color.a * clip);
}

struct FullscreenOut {
    @builtin(position) position : vec4<f32>,
    @location(0)       uv       : vec2<f32>,
    @location(1) @interpolate(flat) instance_index : u32,
};

@vertex
fn vs_fullscreen(
    @builtin(vertex_index)   vert_idx : u32,
    @builtin(instance_index) inst_idx : u32,
) -> FullscreenOut {
    let uv  = vec2<f32>(f32((vert_idx << 1u) & 2u), f32(vert_idx & 2u));
    let pos = uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0);
    var out: FullscreenOut;
    out.position       = vec4<f32>(pos, 0.0, 1.0);
    out.uv             = uv;
    out.instance_index = inst_idx;
    return out;
}

@group(0) @binding(0) var src_tex     : texture_2d<f32>;
@group(0) @binding(1) var src_sampler : sampler;

const DOWNSAMPLE_RATIO: f32 = 8.0;

const BLUR_ITERATIONS: f32 = 4.0;

@fragment
fn fs_halve(in: FullscreenOut) -> @location(0) vec4<f32> {
    let texel = 1.0 / vec2<f32>(textureDimensions(src_tex));

    var c = textureSample(src_tex, src_sampler, in.uv) * 4.0;

    c += textureSample(src_tex, src_sampler, in.uv + vec2<f32>(-texel.x, -texel.y));
    c += textureSample(src_tex, src_sampler, in.uv + vec2<f32>( texel.x, -texel.y));
    c += textureSample(src_tex, src_sampler, in.uv + vec2<f32>(-texel.x,  texel.y));
    c += textureSample(src_tex, src_sampler, in.uv + vec2<f32>( texel.x,  texel.y));

    return c * (1.0 / 8.0);
}

struct MaterialGpu {
    frame        : vec4<f32>,
    tint         : vec4<f32>,
    border_color : vec4<f32>,
    params       : vec4<f32>,
    params2      : vec4<f32>,
    params3      : vec4<f32>,
    clip_rect    : vec4<f32>,
    clip_radii   : vec4<f32>,
};
struct MaterialBuf {
    data: array<MaterialGpu>,
};

@group(1) @binding(0) var<storage, read> materials: MaterialBuf;

@group(2) @binding(0) var blur_tex     : texture_2d<f32>;
@group(2) @binding(1) var blur_sampler : sampler;

@group(2) @binding(2) var scene_tex    : texture_2d<f32>;

// The scene at full resolution, untouched. Only a progressive material reads
// it, and only at the end of its ramp, where the point is to leave the screen
// exactly as it found it.
@group(2) @binding(3) var sharp_tex   : texture_2d<f32>;

const BLUR_W0: f32 = 0.2270270270;
const BLUR_W1: f32 = 0.1945945946;
const BLUR_W2: f32 = 0.1216216216;
const BLUR_W3: f32 = 0.0540540541;
const BLUR_W4: f32 = 0.0162162162;

const MAX_TAP_SPACING: f32 = 1.5;

fn blur_tap_step(blur_radius: f32, texel: f32) -> f32 {
    let radius_texels = blur_radius / DOWNSAMPLE_RATIO;
    let per_pass = radius_texels / sqrt(BLUR_ITERATIONS);
    return min(per_pass * 0.5, MAX_TAP_SPACING) * texel;
}

@fragment
fn fs_blur_h(in: FullscreenOut) -> @location(0) vec4<f32> {
    let blur_radius = materials.data[in.instance_index].params.z;
    let texel = 1.0 / vec2<f32>(textureDimensions(src_tex));
    let step = blur_tap_step(blur_radius, texel.x);

    var sum = textureSample(src_tex, src_sampler, in.uv) * BLUR_W0;
    sum += textureSample(src_tex, src_sampler, in.uv + vec2<f32>(1.0 * step, 0.0)) * BLUR_W1;
    sum += textureSample(src_tex, src_sampler, in.uv - vec2<f32>(1.0 * step, 0.0)) * BLUR_W1;
    sum += textureSample(src_tex, src_sampler, in.uv + vec2<f32>(2.0 * step, 0.0)) * BLUR_W2;
    sum += textureSample(src_tex, src_sampler, in.uv - vec2<f32>(2.0 * step, 0.0)) * BLUR_W2;
    sum += textureSample(src_tex, src_sampler, in.uv + vec2<f32>(3.0 * step, 0.0)) * BLUR_W3;
    sum += textureSample(src_tex, src_sampler, in.uv - vec2<f32>(3.0 * step, 0.0)) * BLUR_W3;
    sum += textureSample(src_tex, src_sampler, in.uv + vec2<f32>(4.0 * step, 0.0)) * BLUR_W4;
    sum += textureSample(src_tex, src_sampler, in.uv - vec2<f32>(4.0 * step, 0.0)) * BLUR_W4;
    return sum;
}

@fragment
fn fs_blur_v(in: FullscreenOut) -> @location(0) vec4<f32> {
    let blur_radius = materials.data[in.instance_index].params.z;
    let texel = 1.0 / vec2<f32>(textureDimensions(src_tex));
    let step = blur_tap_step(blur_radius, texel.y);

    var sum = textureSample(src_tex, src_sampler, in.uv) * BLUR_W0;
    sum += textureSample(src_tex, src_sampler, in.uv + vec2<f32>(0.0, 1.0 * step)) * BLUR_W1;
    sum += textureSample(src_tex, src_sampler, in.uv - vec2<f32>(0.0, 1.0 * step)) * BLUR_W1;
    sum += textureSample(src_tex, src_sampler, in.uv + vec2<f32>(0.0, 2.0 * step)) * BLUR_W2;
    sum += textureSample(src_tex, src_sampler, in.uv - vec2<f32>(0.0, 2.0 * step)) * BLUR_W2;
    sum += textureSample(src_tex, src_sampler, in.uv + vec2<f32>(0.0, 3.0 * step)) * BLUR_W3;
    sum += textureSample(src_tex, src_sampler, in.uv - vec2<f32>(0.0, 3.0 * step)) * BLUR_W3;
    sum += textureSample(src_tex, src_sampler, in.uv + vec2<f32>(0.0, 4.0 * step)) * BLUR_W4;
    sum += textureSample(src_tex, src_sampler, in.uv - vec2<f32>(0.0, 4.0 * step)) * BLUR_W4;
    return sum;
}

@fragment
fn fs_present(in: FullscreenOut) -> @location(0) vec4<f32> {
    return textureSample(src_tex, src_sampler, in.uv);
}

struct MaterialVSOut {
    @builtin(position) position : vec4<f32>,
    @location(0)       uv       : vec2<f32>,
    @location(1)       size     : vec2<f32>,
    @location(2) @interpolate(flat) instance_index : u32,
};

@vertex
fn vs_material(
    @builtin(vertex_index)   vert_idx : u32,
    @builtin(instance_index) inst_idx : u32,
) -> MaterialVSOut {
    var quad = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0), vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0), vec2<f32>(1.0, 1.0), vec2<f32>(0.0, 1.0),
    );
    let local = quad[vert_idx];
    let inst  = materials.data[inst_idx];

    let pixel = inst.frame.xy + local * inst.frame.zw;
    let ndc = vec2<f32>(
         (pixel.x / globals.resolution.x) * 2.0 - 1.0,
        -((pixel.y / globals.resolution.y) * 2.0 - 1.0),
    );

    var out: MaterialVSOut;
    out.position       = vec4<f32>(ndc, 0.0, 1.0);
    out.uv             = local;
    out.size           = inst.frame.zw;
    out.instance_index = inst_idx;
    return out;
}

@fragment
fn fs_material(in: MaterialVSOut) -> @location(0) vec4<f32> {
    let inst = materials.data[in.instance_index];
    let half = in.size * 0.5;

    var corner_radius = inst.params.x;
    var corner_n = CORNER_SUPERELLIPSE_N;
    if (corner_radius < 0.0) {
        corner_radius = min(half.x, half.y);
        corner_n = 2.0;
    }
    // A pane squashed by a drag can come out shorter than its radius was
    // meant for. Past half the short side the rounded-rect distance folds
    // over itself, so hold the radius there.
    corner_radius = min(corner_radius, min(half.x, half.y));
    let border_width = inst.params.y;
    let specular      = inst.params.w;

    let p    = (in.uv - vec2<f32>(0.5)) * in.size;
    let dist = sd_rounded_rect(p, half, corner_radius, corner_n);
    let aa   = length(vec2<f32>(dpdx(dist), dpdy(dist)));

    let fill_mask   = smoothstep(aa, -aa, dist);
    let border_mask = smoothstep(aa, -aa, dist) - smoothstep(aa, -aa, dist + border_width);

    let normal = sd_rounded_rect_normal(p, half, corner_radius, corner_n);
    let refraction = inst.params2.z;
    let interactive = inst.params2.w;

    let glass = refraction_active(refraction);

    let depth = max(-dist, 0.0);
    let thickness = glass_thickness(refraction, half);
    let edge = 1.0 - smoothstep(0.0, 1.0, depth / thickness);
    let bend = refraction * (1.0 + interactive * GLASS_PRESS_REFRACTION) * glass;

    let screen_uv = in.position.xy / globals.resolution;

    // A held pane lifts and magnifies what is under it. The zoom is about the
    // pane's own centre, so the content under its middle stays put.
    let centre = inst.frame.xy + inst.frame.zw * 0.5;
    let zoom = 1.0 / (1.0 + interactive * GLASS_PRESS_ZOOM * glass);
    let lens_uv = (centre + (in.position.xy - centre) * zoom) / globals.resolution;

    // The lip is convex, so a ray through it lands further in than it
    // entered: each fragment shows what is under a point closer to the
    // middle, which stretches the content outward across the lip. Blue bends
    // most, red least, so the fringe falls the way a prism's does.
    let inward = -normal * bend / globals.resolution;
    let off_g = inward * lip_shift(depth, thickness, GLASS_IOR);
    let off_r = inward * lip_shift(depth, thickness, GLASS_IOR * (1.0 - GLASS_DISPERSION));
    let off_b = inward * lip_shift(depth, thickness, GLASS_IOR * (1.0 + GLASS_DISPERSION));

    let sharp_uv = clamp(lens_uv + off_g, vec2<f32>(0.0), vec2<f32>(1.0));
    let soft_uv  = clamp(lens_uv + off_g * GLASS_BLUR_DISPLACE, vec2<f32>(0.0), vec2<f32>(1.0));
    let uv_r     = clamp(lens_uv + off_r, vec2<f32>(0.0), vec2<f32>(1.0));
    let uv_b     = clamp(lens_uv + off_b, vec2<f32>(0.0), vec2<f32>(1.0));

    let sharp = vec3<f32>(
        textureSample(scene_tex, blur_sampler, uv_r).r,
        textureSample(scene_tex, blur_sampler, sharp_uv).g,
        textureSample(scene_tex, blur_sampler, uv_b).b,
    );
    let soft = textureSample(blur_tex, blur_sampler, soft_uv).rgb;

    // The two rungs below `soft`: the half-resolution pyramid level, and the
    // screen exactly as it stands. A progressive material walks down these.
    let mild  = textureSample(scene_tex, blur_sampler, soft_uv).rgb;
    let crisp = textureSample(sharp_tex, blur_sampler, screen_uv).rgb;

    // How much of this fragment's blur survives. 1 everywhere on a plain
    // material; on a progressive one it thins out down the shape.
    let progressive = inst.params2.x;
    let p_start = clamp(inst.params2.y, 0.0, 0.999);

    // A negative amount runs the ramp the other way up: strongest at the
    // bottom edge, gone by the top, which is what sits under a tab bar. The
    // direction rides on the sign because `SFNode` is a C struct the renderer
    // and all three hosts agree on, and a new slot in it is not free.
    let p_y = select(in.uv.y, 1.0 - in.uv.y, progressive < 0.0);
    let ramp = mix(1.0, 1.0 - smoothstep(p_start, 1.0, p_y), abs(progressive));

    // Walk down the pyramid rather than fade a full blur out. Fading opacity
    // alone leaves a heavy blur half-dissolved over the sharp content it came
    // from, which reads as a doubled image; stepping the radius down through
    // the levels reads as what it is, less blur. At the end of the ramp the
    // backdrop is the screen itself, so nothing has to fade to hide a seam.
    let level = ramp * 2.0;
    let soft_progressive = mix(
        mix(crisp, mild, clamp(level, 0.0, 1.0)),
        soft,
        clamp(level - 1.0, 0.0, 1.0)
    );

    // Glass is sharp across its lip and, with clarity, all the way in. A
    // material has no lip, so `glass` keeps this a plain blur for it.
    let clarity = inst.params3.x;
    let backdrop_rgb = mix(
        soft_progressive, sharp, max(clarity, edge * GLASS_RIM_SHARPNESS) * glass);

    let bd_luma = dot(backdrop_rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
    let vibrant = mix(vec3<f32>(bd_luma), backdrop_rgb, 1.0 + GLASS_VIBRANCY * glass);

    let luma = dot(soft, vec3<f32>(0.2126, 0.7152, 0.0722));
    let adaptive = mix(inst.tint.rgb, vec3<f32>(luma), GLASS_ADAPTIVE * glass);
    // The tint goes with the blur. Coverage does not: at the end of a ramp the
    // backdrop is already the screen, so the pane can paint it straight back
    // at full alpha and leave no edge where it stops.
    let tinted = mix(vibrant, adaptive, inst.tint.a * ramp);

    // Light on the lip. All of it is gated on `glass`; a plain material only
    // gets the thin trim further down.
    let rim_gain = 1.0 + interactive * GLASS_PRESS_RIM;
    let band = glass_band(depth, thickness) * specular * rim_gain * glass;
    let glare = glare_lobe(normal) * GLARE_INTENSITY * band;
    let fresnel = FRESNEL_INTENSITY * band;
    let away = max(-dot(normal, GLARE_DIR), 0.0);
    let shade = pow(away, SHADOW_POWER) * SHADOW_DEPTH * band / rim_gain;

    let lit = max(glare_tone(tinted, glare) + RIM_COLOR * fresnel - shade, vec3<f32>(0.0));

    var color = vec4<f32>(0.0);
    color = mix(color, vec4<f32>(lit, 1.0), fill_mask);
    color = mix(color, inst.border_color,   border_mask);

    let rim_mask = 1.0 - smoothstep(0.0, RIM_WIDTH, abs(dist));
    let rim_axis = normalize(vec2<f32>(-1.0, -1.0));
    let facing   = abs(dot(normal, rim_axis));
    let trim     = pow(facing, RIM_POWER) * rim_mask * RIM_INTENSITY * specular * rim_gain * (1.0 - glass);
    color = vec4<f32>(color.rgb + RIM_COLOR * trim * fill_mask, color.a);

    let screen = inst.frame.xy + in.uv * inst.frame.zw;
    let clip = clip_coverage(screen, inst.clip_rect, inst.clip_radii);

    // Straight alpha on the way out. Everything above is premultiplied by
    // coverage, and the pipeline blends with ALPHA_BLENDING, which multiplies
    // by it again — that second multiply is what darkens a half-covered edge
    // pixel into a line. Invisible while the pane also faded out there;
    // plainly a black hairline once it stopped.
    let coverage = max(color.a, 0.0001);
    return vec4<f32>(color.rgb / coverage, color.a * clip);
}

struct ImageGpu {
    frame     : vec4<f32>,
    uv        : vec4<f32>,
    tint      : vec4<f32>,
    params    : vec4<f32>,
    clip_rect : vec4<f32>,
    clip_radii: vec4<f32>,
};
struct ImageBuf {
    data: array<ImageGpu>,
};

@group(1) @binding(2) var<storage, read> images: ImageBuf;
@group(1) @binding(3) var image_tex     : texture_2d<f32>;
@group(1) @binding(4) var image_sampler : sampler;

struct ImageVSOut {
    @builtin(position) position : vec4<f32>,
    @location(0)       uv       : vec2<f32>,
    @location(1)       size     : vec2<f32>,
    @location(2) @interpolate(flat) instance_index : u32,
};

@vertex
fn vs_image(
    @builtin(vertex_index)   vert_idx : u32,
    @builtin(instance_index) inst_idx : u32,
) -> ImageVSOut {
    var quad = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0), vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0), vec2<f32>(1.0, 1.0), vec2<f32>(0.0, 1.0),
    );
    let local = quad[vert_idx];
    let inst  = images.data[inst_idx];

    let pixel = inst.frame.xy + local * inst.frame.zw;
    let ndc = vec2<f32>(
         (pixel.x / globals.resolution.x) * 2.0 - 1.0,
        -((pixel.y / globals.resolution.y) * 2.0 - 1.0),
    );

    var out: ImageVSOut;
    out.position       = vec4<f32>(ndc, 0.0, 1.0);
    out.uv             = local;
    out.size           = inst.frame.zw;
    out.instance_index = inst_idx;
    return out;
}

@fragment
fn fs_image(in: ImageVSOut) -> @location(0) vec4<f32> {
    let inst = images.data[in.instance_index];
    let half = in.size * 0.5;

    var corner_radius = inst.params.x;
    var corner_n = CORNER_SUPERELLIPSE_N;
    if (corner_radius < 0.0) {
        corner_radius = min(half.x, half.y);
        corner_n = 2.0;
    }

    let p    = (in.uv - vec2<f32>(0.5)) * in.size;
    let dist = sd_rounded_rect(p, half, corner_radius, corner_n);
    let aa   = length(vec2<f32>(dpdx(dist), dpdy(dist)));
    let mask = smoothstep(aa, -aa, dist);

    let tex_uv = in.uv * inst.uv.xy + inst.uv.zw;
    let inside = f32(
        tex_uv.x >= 0.0 && tex_uv.x <= 1.0 &&
        tex_uv.y >= 0.0 && tex_uv.y <= 1.0
    );

    let texel = textureSample(image_tex, image_sampler, tex_uv);
    let screen = inst.frame.xy + in.uv * inst.frame.zw;
    let clip = clip_coverage(screen, inst.clip_rect, inst.clip_radii);
    return vec4<f32>(
        texel.rgb * inst.tint.rgb,
        texel.a * inst.tint.a * mask * inside * clip
    );
}
