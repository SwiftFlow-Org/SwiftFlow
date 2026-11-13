struct GlyphInstance {
    @location(0) rect       : vec4<f32>,
    @location(1) uv         : vec4<f32>,
    @location(2) color      : vec4<f32>,
    @location(3) clip_rect  : vec4<f32>,
    @location(4) clip_radii : vec4<f32>,
    @location(5) blur       : vec4<f32>,
};

struct VertexOutput {
    @builtin(position) position : vec4<f32>,

    @location(0)       glyph_uv   : vec2<f32>,
    @location(1)       color      : vec4<f32>,
    @location(2)       screen     : vec2<f32>,
    @location(3)       clip_rect  : vec4<f32>,
    @location(4)       clip_radii : vec4<f32>,
    @location(5)       atlas_rect : vec4<f32>,
    @location(6)       blur       : vec4<f32>,
};

struct Globals {
    resolution : vec2<f32>,
    _pad       : vec2<f32>,
};

@group(0) @binding(0) var<uniform> globals : Globals;
@group(1) @binding(0) var atlas_texture    : texture_2d<f32>;
@group(1) @binding(1) var atlas_sampler    : sampler;

@vertex
fn vs_main(
    @builtin(vertex_index) vert_idx : u32,
    instance                        : GlyphInstance,
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

    let grow = instance.blur.x;
    let size = instance.rect.zw;
    let outer = size + 2.0 * grow;

    let pixel = instance.rect.xy - vec2<f32>(grow, grow) + local * outer;

    let ndc = vec2<f32>(
         (pixel.x / globals.resolution.x) * 2.0 - 1.0,
        -((pixel.y / globals.resolution.y) * 2.0 - 1.0),
    );

    var out: VertexOutput;
    out.position   = vec4<f32>(ndc, 0.0, 1.0);
    out.glyph_uv   = (local * outer - vec2<f32>(grow, grow)) / max(size, vec2<f32>(1.0, 1.0));
    out.color      = instance.color;
    out.screen     = pixel;
    out.clip_rect  = instance.clip_rect;
    out.clip_radii = instance.clip_radii;
    out.atlas_rect = instance.uv;
    out.blur       = instance.blur;
    return out;
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

fn glyph_blur_weight(i: i32) -> f32 {
    let a = abs(i);
    if (a == 0) { return 20.0 / 64.0; }
    if (a == 1) { return 15.0 / 64.0; }
    if (a == 2) { return  6.0 / 64.0; }
    return 1.0 / 64.0;
}

fn glyph_alpha(g: vec2<f32>, atlas_rect: vec4<f32>) -> f32 {
    let inside = f32(g.x >= 0.0 && g.x <= 1.0 && g.y >= 0.0 && g.y <= 1.0);
    let uv = atlas_rect.xy + clamp(g, vec2<f32>(0.0), vec2<f32>(1.0)) * atlas_rect.zw;
    return textureSampleLevel(atlas_texture, atlas_sampler, uv, 0.0).a * inside;
}

fn srgb_decode(c: f32) -> f32 {
    return select(pow((c + 0.055) / 1.055, 2.4), c / 12.92, c <= 0.04045);
}

fn srgb_encode(c: f32) -> f32 {
    let x = max(c, 0.0);
    return select(1.055 * pow(x, 1.0 / 2.4) - 0.055, x * 12.92, x <= 0.0031308);
}

// Glyph coverage for text that should look blended in linear light, as on
// a framebuffer with an sRGB format, while this one blends the encoded
// values. The page behind isn't known here, so it is taken to be dark under
// light text and light under dark text; the coverage is then chosen so the
// encoded blend lands where the linear one would.
fn linear_light_coverage(c: f32, fg: vec3<f32>) -> f32 {
    let lum = dot(fg, vec3<f32>(0.2126, 0.7152, 0.0722));
    let bg = select(0.94, 0.11, lum >= 0.5);
    let denom = lum - bg;
    if (abs(denom) < 0.02) {
        return c;
    }
    let wanted = srgb_encode(c * srgb_decode(lum) + (1.0 - c) * srgb_decode(bg));
    return clamp((wanted - bg) / denom, 0.0, 1.0);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let clip = clip_coverage(in.screen, in.clip_rect, in.clip_radii);
    let radius = in.blur.x;

    if (radius <= 0.0) {
        var a = glyph_alpha(in.glyph_uv, in.atlas_rect);
        if (globals._pad.x > 0.5) {
            a = linear_light_coverage(a, in.color.rgb);
        }
        return vec4<f32>(in.color.rgb, in.color.a * a * clip);
    }

    let step = (radius / 3.0) / max(in.blur.yz, vec2<f32>(1.0, 1.0));

    var sum = 0.0;
    for (var j = -3; j <= 3; j = j + 1) {
        let wj = glyph_blur_weight(j);
        for (var i = -3; i <= 3; i = i + 1) {
            let offset = vec2<f32>(f32(i), f32(j)) * step;
            sum += glyph_alpha(in.glyph_uv + offset, in.atlas_rect)
                 * wj * glyph_blur_weight(i);
        }
    }

    return vec4<f32>(in.color.rgb, in.color.a * sum * clip);
}
