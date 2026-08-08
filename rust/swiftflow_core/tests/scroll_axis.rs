use swiftflow_core::ffi::SFScrollHit;
use swiftflow_core::layout::layout;
use swiftflow_core::node::*;
use swiftflow_core::types::*;

extern "C" {
    fn sf_hit_test_scroll_path(
        root: *const SFNode,
        x: f32,
        y: f32,
        out: *mut SFScrollHit,
        cap: usize,
    ) -> usize;
}

fn leak(children: Vec<SFNode>) -> (*mut SFNode, usize) {
    let len = children.len();
    let boxed = children.into_boxed_slice();
    (Box::into_raw(boxed) as *mut SFNode, len)
}

fn stack(axis: SFAxis, spacing: f32, children: Vec<SFNode>) -> SFNode {
    let mut n = SFNode::empty();
    n.kind = SFNodeKind::Stack;
    n.axis = axis;
    n.spacing = spacing;
    n.set_sizing(SFSizing::Hug);
    let (ptr, len) = leak(children);
    n.children = ptr;
    n.children_len = len;
    n
}

fn scroll(axis: SFAxis, id: u32, children: Vec<SFNode>) -> SFNode {
    let mut n = SFNode::empty();
    n.kind = SFNodeKind::Scroll;
    n.axis = axis;
    n.set_sizing(SFSizing::Fill);
    n.scroll_id = id;
    let (ptr, len) = leak(children);
    n.children = ptr;
    n.children_len = len;
    n
}

fn rect(w: f32, h: f32) -> SFNode {
    let mut n = SFNode::empty();
    n.kind = SFNodeKind::Rect;
    n.set_sizing(SFSizing::Fixed);
    n.fixed_width = w;
    n.fixed_height = h;
    n
}

fn child(node: &SFNode, i: usize) -> &SFNode {
    assert!(!node.children.is_null() && i < node.children_len);
    unsafe { &std::slice::from_raw_parts(node.children, node.children_len)[i] }
}

const VIEWPORT: SFRect = SFRect {
    x: 0.0,
    y: 0.0,
    width: 400.0,
    height: 800.0,
};

#[test]
fn a_horizontal_scroll_lays_its_children_out_across() {
    let cards: Vec<SFNode> = (0..5).map(|_| rect(120.0, 150.0)).collect();
    let row = stack(SFAxis::Horizontal, 10.0, cards);
    let mut view = scroll(SFAxis::Horizontal, 1, vec![row]);

    layout(&mut view, VIEWPORT);

    let row = child(&view, 0);
    assert_eq!(row.frame.width, 5.0 * 120.0 + 4.0 * 10.0);

    let first = child(row, 0);
    let second = child(row, 1);
    assert_eq!(second.frame.x, first.frame.x + 120.0 + 10.0);
    assert_eq!(second.frame.y, first.frame.y, "no vertical stacking");

    assert_eq!(view.content_width, 640.0, "extent runs along the scroll axis");
}

#[test]
fn a_horizontal_scroll_shifts_its_content_sideways_only() {
    let build = |offset: f32| -> (f32, f32) {
        let cards: Vec<SFNode> = (0..5).map(|_| rect(120.0, 150.0)).collect();
        let row = stack(SFAxis::Horizontal, 10.0, cards);
        let mut view = scroll(SFAxis::Horizontal, 1, vec![row]);
        view.content_offset_x = offset;
        layout(&mut view, VIEWPORT);
        let row = child(&view, 0);
        (row.frame.x, row.frame.y)
    };

    let (rest_x, rest_y) = build(0.0);
    let (scrolled_x, scrolled_y) = build(100.0);

    assert_eq!(scrolled_x, rest_x - 100.0, "scrolled left by the offset");
    assert_eq!(scrolled_y, rest_y, "and not at all vertically");
}

#[test]
fn a_vertical_scroll_shifts_its_content_downward_only() {
    let build = |offset: f32| -> (f32, f32) {
        let rows: Vec<SFNode> = (0..5).map(|_| rect(300.0, 100.0)).collect();
        let column = stack(SFAxis::Vertical, 10.0, rows);
        let mut view = scroll(SFAxis::Vertical, 1, vec![column]);
        view.content_offset_y = offset;
        layout(&mut view, VIEWPORT);
        let column = child(&view, 0);
        (column.frame.x, column.frame.y)
    };

    let (rest_x, rest_y) = build(0.0);
    let (scrolled_x, scrolled_y) = build(50.0);

    assert_eq!(scrolled_y, rest_y - 50.0);
    assert_eq!(scrolled_x, rest_x);
}

#[test]
fn a_vertical_scroll_is_unchanged() {
    let rows: Vec<SFNode> = (0..5).map(|_| rect(300.0, 100.0)).collect();
    let column = stack(SFAxis::Vertical, 10.0, rows);
    let mut view = scroll(SFAxis::Vertical, 1, vec![column]);
    view.content_offset_y = 50.0;

    layout(&mut view, VIEWPORT);

    let column = child(&view, 0);
    assert_eq!(column.frame.height, 5.0 * 100.0 + 4.0 * 10.0);
    assert_eq!(column.frame.y, -50.0);
    assert_eq!(view.content_height, 540.0);
}

#[test]
fn both_extents_are_reported_either_way() {
    let cards: Vec<SFNode> = (0..3).map(|_| rect(120.0, 150.0)).collect();
    let row = stack(SFAxis::Horizontal, 10.0, cards);
    let mut view = scroll(SFAxis::Horizontal, 1, vec![row]);

    layout(&mut view, VIEWPORT);

    assert_eq!(view.content_width, 3.0 * 120.0 + 2.0 * 10.0);
    assert_eq!(view.content_height, 150.0, "across the axis, the tallest");
}

#[test]
fn a_horizontal_scroll_nests_inside_a_vertical_one() {
    let cards: Vec<SFNode> = (0..6).map(|_| rect(120.0, 150.0)).collect();
    let mut strip = scroll(SFAxis::Horizontal, 2, vec![stack(SFAxis::Horizontal, 10.0, cards)]);

    strip.sizing_y = SFSizing::Fixed;
    strip.fixed_height = 150.0;

    let rows = stack(
        SFAxis::Vertical,
        20.0,
        vec![rect(300.0, 200.0), strip, rect(300.0, 200.0)],
    );
    let mut outer = scroll(SFAxis::Vertical, 1, vec![rows]);

    layout(&mut outer, VIEWPORT);

    let rows = child(&outer, 0);
    let strip = child(rows, 1);
    assert_eq!(strip.frame.height, 150.0);
    assert_eq!(strip.content_width, 6.0 * 120.0 + 5.0 * 10.0);

    assert_eq!(outer.content_height, 200.0 + 150.0 + 200.0 + 2.0 * 20.0);
}

#[test]
fn the_scroll_chain_is_innermost_first_and_carries_each_axis() {
    let cards: Vec<SFNode> = (0..6).map(|_| rect(120.0, 150.0)).collect();
    let mut strip = scroll(SFAxis::Horizontal, 2, vec![stack(SFAxis::Horizontal, 10.0, cards)]);
    strip.sizing_y = SFSizing::Fixed;
    strip.fixed_height = 150.0;

    let rows = stack(SFAxis::Vertical, 0.0, vec![strip]);
    let mut outer = scroll(SFAxis::Vertical, 1, vec![rows]);
    layout(&mut outer, VIEWPORT);

    let mut buf = [SFScrollHit { scroll_id: 0, axis: 0 }; 8];
    let n = unsafe { sf_hit_test_scroll_path(&outer, 60.0, 60.0, buf.as_mut_ptr(), buf.len()) };

    assert_eq!(n, 2, "both scroll views contain the point");
    assert_eq!(buf[0].scroll_id, 2, "the strip is innermost");
    assert_eq!(buf[0].axis, SFAxis::Horizontal as u32);
    assert_eq!(buf[1].scroll_id, 1, "the list is outermost");
    assert_eq!(buf[1].axis, SFAxis::Vertical as u32);
}

#[test]
fn a_point_outside_every_scroll_view_yields_an_empty_chain() {
    let mut view = scroll(SFAxis::Vertical, 1, vec![rect(100.0, 100.0)]);
    view.set_sizing(SFSizing::Fixed);
    view.fixed_width = 100.0;
    view.fixed_height = 100.0;
    layout(&mut view, VIEWPORT);

    let mut buf = [SFScrollHit { scroll_id: 0, axis: 0 }; 4];
    let n = unsafe { sf_hit_test_scroll_path(&view, 300.0, 300.0, buf.as_mut_ptr(), buf.len()) };
    assert_eq!(n, 0);
}
