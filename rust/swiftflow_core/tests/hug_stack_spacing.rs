use swiftflow_core::layout::layout;
use swiftflow_core::node::*;
use swiftflow_core::types::*;

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
    width: 393.0,
    height: 852.0,
};

#[test]
fn a_hug_vstack_reports_the_gaps_between_its_rows() {
    let rows: Vec<SFNode> = (0..5).map(|_| rect(100.0, 20.0)).collect();
    let mut v = stack(SFAxis::Vertical, 10.0, rows);

    layout(&mut v, VIEWPORT);

    assert_eq!(
        v.frame.height,
        5.0 * 20.0 + 4.0 * 10.0,
        "five 20pt rows at spacing 10 occupy 140pt, not 100"
    );
}

#[test]
fn a_hug_hstack_reports_the_gaps_between_its_columns() {
    let cols = vec![rect(44.0, 44.0), rect(200.0, 44.0), rect(44.0, 44.0)];
    let mut h = stack(SFAxis::Horizontal, 10.0, cols);

    layout(&mut h, VIEWPORT);

    assert_eq!(h.frame.width, 44.0 + 200.0 + 44.0 + 2.0 * 10.0);
}

#[test]
fn a_sibling_after_a_spaced_hug_stack_clears_it() {
    let hero = stack(
        SFAxis::Vertical,
        10.0,
        vec![
            rect(200.0, 254.0),
            rect(150.0, 24.0),
            rect(120.0, 18.0),
            rect(180.0, 20.0),
            rect(300.0, 56.0),
        ],
    );
    let paragraph = rect(350.0, 60.0);

    let mut outer = stack(SFAxis::Vertical, 16.0, vec![hero, paragraph]);
    layout(&mut outer, VIEWPORT);

    let hero = child(&outer, 0);
    let paragraph = child(&outer, 1);

    let hero_content = 254.0 + 24.0 + 18.0 + 20.0 + 56.0 + 4.0 * 10.0;
    assert_eq!(hero.frame.height, hero_content);
    assert_eq!(
        paragraph.frame.y,
        hero.frame.y + hero_content + 16.0,
        "the paragraph must start below the hero's last row, not inside it"
    );
}

#[test]
fn a_spaced_hug_hstack_centres_on_its_true_width() {
    let row = stack(
        SFAxis::Horizontal,
        10.0,
        vec![rect(44.0, 44.0), rect(200.0, 44.0), rect(44.0, 44.0)],
    );
    let mut outer = stack(SFAxis::Vertical, 0.0, vec![row]);
    outer.set_sizing(SFSizing::Fill);

    layout(&mut outer, VIEWPORT);

    let row = child(&outer, 0);
    let row_centre = row.frame.x + row.frame.width * 0.5;
    assert!(
        (row_centre - VIEWPORT.width * 0.5).abs() < 0.01,
        "row centred at {row_centre}, viewport centre is {}",
        VIEWPORT.width * 0.5
    );
}

#[test]
fn a_padded_fixed_container_hands_its_child_the_size_inside_the_padding() {

    let mut cover = SFNode::empty();
    cover.kind = SFNodeKind::Image;
    cover.sizing_x = SFSizing::Fixed;
    cover.sizing_y = SFSizing::Fill;
    cover.fixed_width = 200.0;

    let mut hero = stack(SFAxis::Depth, 0.0, vec![cover]);
    hero.set_sizing(SFSizing::Fixed);
    hero.fixed_width = 200.0;
    hero.fixed_height = 254.0 + 180.0 + 10.0;
    hero.padding = SFEdgeInsets {
        top: 180.0,
        bottom: 10.0,
        leading: 0.0,
        trailing: 0.0,
    };

    layout(&mut hero, VIEWPORT);

    let cover = child(&hero, 0);
    assert_eq!(cover.frame.width, 200.0);
    assert_eq!(cover.frame.height, 254.0);
    assert_eq!(cover.frame.y, 180.0, "the child sits below the top padding");
}
