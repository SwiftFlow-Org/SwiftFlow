use crate::draw::*;
use crate::node::*;
use crate::layout::alignment_fraction;
use crate::with_font_system;
use crate::types::*;

pub fn draw(node: &SFNode, list: &mut DrawList) {
    draw_inner(node, list, SFPoint { x: 0.0, y: 0.0 }, SFScale::DEFAULT);
}

fn draw_edge_effect(node: &SFNode, list: &mut DrawList, origin: SFPoint, w: f32, scale: SFScale) {
    if node.edge_effect_height <= 0.0 || node.blur_radius <= 0.0 {
        return;
    }
    list.material(
        SFRect::new(origin.x, origin.y, w, node.edge_effect_height * scale.y),
        0.0,
        SFBorder::NONE,
        node.fill,
        node.blur_radius,
        false,
        node.progressive_blur,
        node.progressive_start,
        node.glass_refraction,
        node.glass_interactive,
    );
}

fn clip_radii(corner_radius: f32, w: f32, h: f32) -> [f32; 4] {
    let r = if corner_radius < 0.0 {
        (w.min(h)) * 0.5
    } else {
        corner_radius.min(w.min(h) * 0.5)
    };
    [r; 4]
}

fn push_node_clip(node: &SFNode, list: &mut DrawList, origin: SFPoint, w: f32, h: f32) -> bool {
    if node.clip_content == 0.0 {
        return false;
    }
    list.push_clip_rounded(
        SFRect::new(origin.x, origin.y, w, h),
        clip_radii(node.corner_radius, w, h),
    );
    true
}

fn node_shadow(node: &SFNode, scale: SFScale) -> SFShadow {
    if node.shadow_radius <= 0.0 || node.shadow_opacity <= 0.0 {
        return SFShadow::NONE;
    }
    SFShadow {
        radius: node.shadow_radius * scale.x.max(scale.y),
        opacity: node.shadow_opacity,
    }
}

pub(crate) fn compose_scale(
    node: &SFNode,
    translate: SFPoint,
    scale: SFScale,
) -> (SFPoint, SFScale) {
    let center_x = node.frame.x + node.frame.width * 0.5;
    let center_y = node.frame.y + node.frame.height * 0.5;

    let new_scale = SFScale {
        x: scale.x * node.scale.x,
        y: scale.y * node.scale.y,
    };

    let new_translate = SFPoint {
        x: translate.x + scale.x * center_x * (1.0 - node.scale.x) + node.offset_x * scale.x,
        y: translate.y + scale.y * center_y * (1.0 - node.scale.y) + node.offset_y * scale.y,
    };

    (new_translate, new_scale)
}

fn draw_inner(node: &SFNode, list: &mut DrawList, translate: SFPoint, scale: SFScale) {
    let (translate, scale) = compose_scale(node, translate, scale);

    let node_origin = SFPoint {
        x: translate.x + node.frame.x * scale.x,
        y: translate.y + node.frame.y * scale.y,
    };
    let node_w = node.frame.width * scale.x;
    let node_h = node.frame.height * scale.y;

    match node.kind {

        SFNodeKind::Empty => {}

        SFNodeKind::Spacer => {}

        SFNodeKind::Rect => {
            let frame = SFRect::new(node_origin.x, node_origin.y, node_w, node_h);
            if node.blur_radius > 0.0 {

                list.material(
                    frame,
                    node.corner_radius,
                    node.border,
                    node.fill,
                    node.blur_radius,
                    node.specular != 0.0,
                    node.progressive_blur,
                    node.progressive_start,
                    node.glass_refraction,
                    node.glass_interactive,
                );
            } else {
                list.rect(
                    frame,
                    node.corner_radius,
                    node.fill,
                    node.border,
                    node.specular != 0.0,
                    node_shadow(node, scale),
                );
            }
        }

        SFNodeKind::Text => {
            if !node.text.is_null() && node.text_len > 0 {
                let bytes = unsafe { std::slice::from_raw_parts(node.text, node.text_len) };
                let content = String::from_utf8_lossy(bytes).to_string();

                let device_scale = *crate::ffi::SCALE.lock().unwrap();
                let physical_size = node.font_size * device_scale;

                let wrap_width =
                    node.frame.width - node.padding.leading - node.padding.trailing;

                let (lines, line_height) = with_font_system(|fs| {
                    (
                        fs.wrap(
                            &content,
                            wrap_width,
                            physical_size,
                            node.font_weight,
                            node.font_family,
                            node.line_limit as usize,
                        ),
                        fs.line_height(physical_size, node.font_family),
                    )
                });

                let origin_x = node_origin.x + node.padding.leading * scale.x;
                let origin_y = node_origin.y + node.padding.top * scale.y;

                let width = wrap_width * scale.x;

                for (index, line) in lines.iter().enumerate() {
                    if line.is_empty() {
                        continue;
                    }

                    let align_x = if node.text_align == SFAlignment::Leading {
                        0.0
                    } else {
                        let line_width = with_font_system(|fs| {
                            fs.measure(line, physical_size, node.font_weight, node.font_family)
                                .0
                        });

                        ((wrap_width - line_width).max(0.0)
                            * alignment_fraction(node.text_align))
                            * scale.x
                    };

                    list.text(
                        SFRect::new(
                            origin_x + align_x,
                            origin_y + line_height * index as f32 * scale.y,
                            width,
                            line_height * scale.y,
                        ),
                        line.clone(),

                        node.font_size,
                        scale.y,
                        node.font_weight,
                        node.font_family,
                        node.content_blur,
                        node.color,
                    );
                }
            }
        }

        SFNodeKind::Icon => {
            if !node.text.is_null() && node.text_len > 0 {
                let bytes = unsafe { std::slice::from_raw_parts(node.text, node.text_len) };
                let content = String::from_utf8_lossy(bytes).to_string();

                list.text(
                    SFRect::new(
                        node_origin.x + node.padding.leading * scale.x,
                        node_origin.y + node.padding.top * scale.y,
                        (node.frame.width - node.padding.leading - node.padding.trailing)
                            * scale.x,
                        (node.frame.height - node.padding.top - node.padding.bottom) * scale.y,
                    ),
                    content,
                    node.font_size,
                    scale.y,
                    node.font_weight,

                    SFFontFamily::Icon,
                    node.content_blur,
                    node.color,
                );
            }
        }

        SFNodeKind::Stack => {

            if node.fill.a > 0.0 && node.edge_effect_height <= 0.0 {
                list.rect(
                    SFRect::new(node_origin.x, node_origin.y, node_w, node_h),
                    node.corner_radius,
                    node.fill,
                    node.border,
                    node.specular != 0.0,
                    node_shadow(node, scale),
                );
            }

            if node.children.is_null() || node.children_len == 0 {
                return;
            }

            let clipped = push_node_clip(node, list, node_origin, node_w, node_h);

            let children = unsafe { std::slice::from_raw_parts(node.children, node.children_len) };
            draw_children_with_merge(children, list, node_origin, scale);
            draw_edge_effect(node, list, node_origin, node_w, scale);

            if clipped {
                list.pop_clip();
            }
        }

        SFNodeKind::Scroll => {

            if node.fill.a > 0.0 && node.edge_effect_height <= 0.0 {
                list.rect(
                    SFRect::new(node_origin.x, node_origin.y, node_w, node_h),
                    node.corner_radius,
                    node.fill,
                    node.border,
                    node.specular != 0.0,
                    node_shadow(node, scale),
                );
            }

            if node.children.is_null() || node.children_len == 0 {
                return;
            }

            list.push_clip_rounded(
                SFRect::new(node_origin.x, node_origin.y, node_w, node_h),
                clip_radii(node.corner_radius, node_w, node_h),
            );

            let children = unsafe { std::slice::from_raw_parts(node.children, node.children_len) };

            for child in children {
                draw_inner(child, list, node_origin, scale);
            }

            draw_edge_effect(node, list, node_origin, node_w, scale);

            list.pop_clip();
        }

        SFNodeKind::Image => {
            let frame = SFRect::new(node_origin.x, node_origin.y, node_w, node_h);
            if node.image_id != 0 {
                list.image(
                    frame,
                    node.corner_radius,
                    node.image_id,
                    node.image_content_mode,
                    node.color,
                );
            } else {

                list.rect(
                    frame,
                    node.corner_radius,
                    SFColor::SURFACE,
                    SFBorder::NONE,
                    node.specular != 0.0,
                    SFShadow::NONE,
                );
            }
        }
    }
}

const MERGE_DISTANCE: f32 = 24.0;

const MERGE_BLEND_K: f32 = MERGE_DISTANCE * 1.7;

const MERGE_COLOR_BLEND_K: f32 = 12.0;

const MAX_MERGE_GROUP: usize = 4;

struct RectCandidate {
    frame: SFRect,
    corner_radius: f32,
    fill: SFColor,
    border: SFBorder,
    specular: bool,
}

struct Dsu {
    parent: Vec<usize>,
}

impl Dsu {
    fn new(n: usize) -> Self {
        Self {
            parent: (0..n).collect(),
        }
    }

    fn find(&mut self, x: usize) -> usize {
        if self.parent[x] != x {
            self.parent[x] = self.find(self.parent[x]);
        }
        self.parent[x]
    }

    fn union(&mut self, a: usize, b: usize) {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra != rb {
            self.parent[ra] = rb;
        }
    }
}

fn aabb_gap(a: SFRect, b: SFRect) -> f32 {
    let dx = (a.x - (b.x + b.width)).max(b.x - (a.x + a.width)).max(0.0);
    let dy = (a.y - (b.y + b.height))
        .max(b.y - (a.y + a.height))
        .max(0.0);
    (dx * dx + dy * dy).sqrt()
}

fn node_screen_rect(
    node: &SFNode,
    translate: SFPoint,
    scale: SFScale,
) -> (SFPoint, SFScale, SFRect) {
    let (translate, scale) = compose_scale(node, translate, scale);
    let origin = SFPoint {
        x: translate.x + node.frame.x * scale.x,
        y: translate.y + node.frame.y * scale.y,
    };
    let rect = SFRect::new(
        origin.x,
        origin.y,
        node.frame.width * scale.x,
        node.frame.height * scale.y,
    );

    (origin, scale, rect)
}

enum DeferredDraw<'a> {

    Node(&'a SFNode, SFPoint, SFScale),

    Rect(SFRect, f32, SFColor, SFBorder, bool),
}

fn draw_children_with_merge(
    children: &[SFNode],
    list: &mut DrawList,
    node_origin: SFPoint,
    scale: SFScale,
) {
    let mut run: Vec<RectCandidate> = Vec::new();
    let mut deferred: Vec<DeferredDraw> = Vec::new();
    collect_merge_run(children, &mut run, &mut deferred, list, node_origin, scale, false);
    flush_run_and_deferred(&mut run, &mut deferred, list);
}

fn flush_run_and_deferred(run: &mut Vec<RectCandidate>, deferred: &mut Vec<DeferredDraw>, list: &mut DrawList) {
    flush_run(run, list);
    for d in deferred.drain(..) {
        match d {
            DeferredDraw::Node(node, translate, scale) => draw_inner(node, list, translate, scale),
            DeferredDraw::Rect(rect, corner_radius, fill, border, specular) => {
                list.rect(rect, corner_radius, fill, border, specular, SFShadow::NONE)
            }
        }
    }
}

fn collect_merge_run<'a>(
    children: &'a [SFNode],
    run: &mut Vec<RectCandidate>,
    deferred: &mut Vec<DeferredDraw<'a>>,
    list: &mut DrawList,
    translate: SFPoint,
    scale: SFScale,
    is_nested: bool,
) {
    for child in children {

        let is_transparent_zstack = child.kind == SFNodeKind::Stack
            && child.axis == SFAxis::Depth
            && child.clip_content == 0.0
            && child.padding.top == 0.0
            && child.padding.bottom == 0.0
            && child.padding.leading == 0.0
            && child.padding.trailing == 0.0;

        let mergeable = child.blur_radius <= 0.0
            && child.no_merge == 0.0
            && !node_shadow(child, scale).is_visible();
        if child.kind == SFNodeKind::Rect && mergeable {
            let (_, _, rect) = node_screen_rect(child, translate, scale);
            run.push(RectCandidate {
                frame: rect,
                corner_radius: child.corner_radius,
                fill: child.fill,
                border: child.border,
                specular: child.specular != 0.0,
            });
        } else if is_transparent_zstack {
            let (child_translate, child_scale, rect) = node_screen_rect(child, translate, scale);
            if child.fill.a > 0.0 {
                deferred.push(DeferredDraw::Rect(
                    rect,
                    child.corner_radius,
                    child.fill,
                    child.border,
                    child.specular != 0.0,
                ));
            }
            if !child.children.is_null() && child.children_len > 0 {
                let grandchildren =
                    unsafe { std::slice::from_raw_parts(child.children, child.children_len) };
                collect_merge_run(grandchildren, run, deferred, list, child_translate, child_scale, true);
            }
        } else if is_nested {
            deferred.push(DeferredDraw::Node(child, translate, scale));
        } else {
            flush_run_and_deferred(run, deferred, list);
            draw_inner(child, list, translate, scale);
        }
    }
}

fn flush_run(run: &mut Vec<RectCandidate>, list: &mut DrawList) {
    if run.is_empty() {
        return;
    }

    let n = run.len();
    let mut dsu = Dsu::new(n);
    for i in 0..n {
        for j in (i + 1)..n {
            if aabb_gap(run[i].frame, run[j].frame) <= MERGE_DISTANCE {
                dsu.union(i, j);
            }
        }
    }

    let mut clusters: Vec<Vec<usize>> = Vec::new();
    let mut root_to_cluster: std::collections::HashMap<usize, usize> =
        std::collections::HashMap::new();
    for i in 0..n {
        let root = dsu.find(i);
        let idx = *root_to_cluster.entry(root).or_insert_with(|| {
            clusters.push(Vec::new());
            clusters.len() - 1
        });
        clusters[idx].push(i);
    }

    for cluster in clusters {
        if cluster.len() == 1 || cluster.len() > MAX_MERGE_GROUP {
            for i in cluster {
                let c = &run[i];
                list.rect(c.frame, c.corner_radius, c.fill, c.border, c.specular, SFShadow::NONE);
            }
        } else {
            let members: Vec<&RectCandidate> = cluster.iter().map(|&i| &run[i]).collect();
            emit_merged(list, &members);
        }
    }

    run.clear();
}

fn emit_merged(list: &mut DrawList, members: &[&RectCandidate]) {
    let (mut minx, mut miny, mut maxx, mut maxy) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    let mut arr = [MergedMember::default(); 4];

    for (i, m) in members.iter().enumerate() {
        minx = minx.min(m.frame.x);
        miny = miny.min(m.frame.y);
        maxx = maxx.max(m.frame.x + m.frame.width);
        maxy = maxy.max(m.frame.y + m.frame.height);
        arr[i] = MergedMember {
            frame: m.frame,
            corner_radius: m.corner_radius,
            fill: m.fill,
            border: m.border,
            specular: m.specular,
        };
    }

    let expand = MERGE_BLEND_K.max(MERGE_COLOR_BLEND_K);
    let bounds = SFRect::new(
        minx - expand,
        miny - expand,
        (maxx - minx) + 2.0 * expand,
        (maxy - miny) + 2.0 * expand,
    );

    list.merged_rect(
        bounds,
        MERGE_BLEND_K,
        MERGE_COLOR_BLEND_K,
        arr,
        members.len() as u8,
    );
}
