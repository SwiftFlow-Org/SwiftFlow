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

const RIM_WIDTH: f32       = 3.5;
const RIM_INTENSITY: f32   = 0.4;
const RIM_POWER: f32       = 2.0;
const RIM_COLOR: vec3<f32> = vec3<f32>(1.0, 1.0, 1.0);

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

fn bevel_normal(outward: vec2<f32>, t: f32) -> vec3<f32> {
    let u = 1.0 - clamp(t, 0.0, 1.0);
    let h = sqrt(max(1.0 - u * u, 0.0));

    let slope = u / max(h, 1e-3);
    return normalize(vec3<f32>(-outward * slope, 1.0));
}

fn refract_offset(n: vec3<f32>, eta: f32) -> vec2<f32> {
    let r = refract(vec3<f32>(0.0, 0.0, -1.0), n, eta);
    return r.xy / sqrt(max(1.0 - GLASS_ETA * GLASS_ETA, 1e-6));
}

fn refraction_active(refraction: f32) -> f32 {
    return step(0.001, refraction);
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
    let border = smoothstep( aa, -aa, dist + border_width)
               - smoothstep( aa, -aa, dist);

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
    let border_mask = smoothstep(aa, -aa, dist + folded.border_width)
                    - smoothstep(aa, -aa, dist);

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
    let border_width = inst.params.y;
    let specular      = inst.params.w;

    let p    = (in.uv - vec2<f32>(0.5)) * in.size;
    let dist = sd_rounded_rect(p, half, corner_radius, corner_n);
    let aa   = length(vec2<f32>(dpdx(dist), dpdy(dist)));

    let fill_mask   = smoothstep(aa, -aa, dist);
    let border_mask = smoothstep(aa, -aa, dist + border_width) - smoothstep(aa, -aa, dist);

    let normal = sd_rounded_rect_normal(p, half, corner_radius, corner_n);
    let refraction = inst.params2.z;
    let interactive = inst.params2.w;

    let glass = refraction_active(refraction);

    let bevel = max(corner_radius, GLASS_MIN_THICKNESS);

    let depth = clamp(-dist / bevel, 0.0, 1.0);

    let edge = 1.0 - smoothstep(0.0, 1.0, depth);

    let bend = refraction * (1.0 + interactive * GLASS_PRESS_REFRACTION);

    let n3 = bevel_normal(normal, depth);
    let screen_uv = in.position.xy / globals.resolution;
    let offset = refract_offset(n3, GLASS_ETA) * bend / globals.resolution;

    let sharp_uv = clamp(screen_uv + offset, vec2<f32>(0.0), vec2<f32>(1.0));
    let soft_uv  = clamp(screen_uv + offset * GLASS_BLUR_DISPLACE, vec2<f32>(0.0), vec2<f32>(1.0));

    let uv_r = clamp(
        screen_uv + refract_offset(n3, GLASS_ETA * (1.0 + GLASS_DISPERSION)) * bend / globals.resolution,
        vec2<f32>(0.0), vec2<f32>(1.0)
    );
    let uv_b = clamp(
        screen_uv + refract_offset(n3, GLASS_ETA * (1.0 - GLASS_DISPERSION)) * bend / globals.resolution,
        vec2<f32>(0.0), vec2<f32>(1.0)
    );
    let sharp = vec3<f32>(
        textureSample(scene_tex, blur_sampler, uv_r).r,
        textureSample(scene_tex, blur_sampler, sharp_uv).g,
        textureSample(scene_tex, blur_sampler, uv_b).b,
    );
    let soft = textureSample(blur_tex, blur_sampler, soft_uv).rgb;

    let backdrop_rgb = mix(soft, sharp, edge * GLASS_RIM_SHARPNESS * glass);

    let bd_luma = dot(backdrop_rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
    let vibrant = mix(vec3<f32>(bd_luma), backdrop_rgb, 1.0 + GLASS_VIBRANCY * glass);

    let luma = dot(soft, vec3<f32>(0.2126, 0.7152, 0.0722));
    let adaptive = mix(inst.tint.rgb, vec3<f32>(luma), GLASS_ADAPTIVE * glass);
    let tinted = mix(vibrant, adaptive, inst.tint.a);

    var color = vec4<f32>(0.0);
    color = mix(color, vec4<f32>(tinted, 1.0), fill_mask);
    color = mix(color, inst.border_color,      border_mask);

    let rim_gain = 1.0 + interactive * GLASS_PRESS_RIM;
    let rim_mask = 1.0 - smoothstep(0.0, RIM_WIDTH, abs(dist));

    let rim_axis = normalize(vec2<f32>(-1.0, -1.0));
    let facing   = abs(dot(normal, rim_axis));
    let trim     = pow(facing, RIM_POWER) * rim_mask * RIM_INTENSITY;

    let fresnel = pow(1.0 - n3.z, FRESNEL_POWER);

    let toward = max(dot(normal, GLARE_DIR), 0.0);
    let away   = max(dot(normal, -GLARE_DIR), 0.0);
    let glare  = pow(toward, GLARE_POWER) + pow(away, GLARE_POWER) * GLARE_BACK;
    let lit    = fresnel * (FRESNEL_INTENSITY + glare * GLARE_INTENSITY);

    let highlight = mix(trim, lit, glass) * specular * rim_gain;
    color = vec4<f32>(color.rgb + RIM_COLOR * highlight * fill_mask, color.a);

    let shade = fresnel * pow(away, SHADOW_POWER) * SHADOW_DEPTH * specular * glass;
    color = vec4<f32>(max(color.rgb - shade * fill_mask, vec3<f32>(0.0)), color.a);

    let progressive = inst.params2.x;
    if (progressive > 0.0) {

        let start = clamp(inst.params2.y, 0.0, 0.999);
        let ramp  = 1.0 - smoothstep(start, 1.0, in.uv.y);
        color = vec4<f32>(color.rgb, color.a * mix(1.0, ramp, progressive));
    }

    let screen = inst.frame.xy + in.uv * inst.frame.zw;
    let clip = clip_coverage(screen, inst.clip_rect, inst.clip_radii);
    return vec4<f32>(color.rgb, color.a * clip);
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
