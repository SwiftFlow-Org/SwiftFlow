use swiftflow_core::draw::{DrawCommand, DrawItem};
use swiftflow_core::draw_pass::draw;
use swiftflow_core::layout::layout;
use swiftflow_core::node::*;
use swiftflow_core::types::*;
use swiftflow_core::DrawList;

fn overlapping_pair(configure: fn(&mut SFNode)) -> Vec<DrawItem> {
    let mut children = vec![leaf(0.0), leaf(8.0)];
    configure(&mut children[1]);

    let mut root = SFNode::empty();
    root.kind = SFNodeKind::Stack;
    root.axis = SFAxis::Depth;
    root.set_sizing(SFSizing::Fill);
    root.children = children.as_mut_ptr();
    root.children_len = children.len();

    layout(&mut root, SFRect::new(0.0, 0.0, 400.0, 800.0));
    let mut list = DrawList::new();
    draw(&mut root, &mut list);

    drop(children);
    list.commands
}

fn leaf(offset_x: f32) -> SFNode {
    let mut n = SFNode::empty();
    n.kind = SFNodeKind::Rect;
    n.set_sizing(SFSizing::Fixed);
    n.fixed_width = 100.0;
    n.fixed_height = 100.0;
    n.corner_radius = 20.0;
    n.offset_x = offset_x;
    n.fill = SFColor::WHITE;
    n
}

fn counts(commands: &[DrawItem]) -> (usize, usize) {
    let rects = commands
        .iter()
        .filter(|i| matches!(i.command, DrawCommand::Rect { .. }))
        .count();
    let merged = commands
        .iter()
        .filter(|i| matches!(i.command, DrawCommand::MergedRect { .. }))
        .count();
    (rects, merged)
}

#[test]
fn neighbouring_rects_merge_by_default() {
    let (rects, merged) = counts(&overlapping_pair(|_| {}));
    assert_eq!(
        (rects, merged),
        (0, 1),
        "the baseline this file exists to contrast with — if this ever \
         changes, the two tests below stop proving anything"
    );
}

#[test]
fn no_merge_keeps_rects_separate() {
    let (rects, merged) = counts(&overlapping_pair(|n| n.no_merge = 1.0));
    assert_eq!(merged, 0, "an opted-out rect must not be folded into a group");
    assert_eq!(rects, 2, "both rects should still be drawn, individually");
}

#[test]
fn a_shadow_caster_keeps_its_own_edge() {
    let (rects, merged) = counts(&overlapping_pair(|n| {
        n.shadow_radius = 18.0;
        n.shadow_opacity = 0.3;
    }));
    assert_eq!(
        merged, 0,
        "the merged pipeline folds distances across members and has \
         nowhere to put a per-member falloff"
    );
    assert_eq!(rects, 2);
}

#[test]
fn an_invisible_shadow_does_not_block_merging() {

    let (_, merged) = counts(&overlapping_pair(|n| n.shadow_radius = 18.0));
    assert_eq!(merged, 1);

    let (_, merged) = counts(&overlapping_pair(|n| n.shadow_opacity = 0.3));
    assert_eq!(merged, 1);
}

#[test]
fn shadow_reaches_the_draw_command() {
    let commands = overlapping_pair(|n| {
        n.no_merge = 1.0;
        n.shadow_radius = 18.0;
        n.shadow_opacity = 0.3;
    });

    let shadows: Vec<_> = commands
        .iter()
        .filter_map(|i| match &i.command {
            DrawCommand::Rect { shadow, .. } if shadow.is_visible() => Some(*shadow),
            _ => None,
        })
        .collect();

    assert_eq!(shadows.len(), 1, "only the configured rect casts one");
    assert_eq!(shadows[0].radius, 18.0);
    assert_eq!(shadows[0].opacity, 0.3);
}
