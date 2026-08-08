use swiftflow_core::layout::layout;
use swiftflow_core::node::*;
use swiftflow_core::types::*;

fn leak(children: Vec<SFNode>) -> (*mut SFNode, usize) {
    let len = children.len();
    (
        Box::into_raw(children.into_boxed_slice()) as *mut SFNode,
        len,
    )
}

fn kids(node: &SFNode) -> &[SFNode] {
    unsafe { std::slice::from_raw_parts(node.children, node.children_len) }
}

fn inherited(sizing: SFSizing) -> SFSizing {
    if sizing == SFSizing::Fill {
        SFSizing::Fill
    } else {
        SFSizing::Hug
    }
}

#[derive(Copy, Clone)]
enum Ask {

    Size(f32),

    Infinity,

    Nothing,
}

fn frame(child: SFNode, w: Ask, h: Ask) -> SFNode {
    let mut n = SFNode::empty();
    n.kind = SFNodeKind::Stack;
    n.axis = SFAxis::Depth;
    n.alignment = SFAlignment::Center;
    n.vertical_alignment = SFAlignment::Center;

    n.sizing_x = match w {
        Ask::Size(w) => {
            n.fixed_width = w;
            SFSizing::Fixed
        }
        Ask::Infinity => SFSizing::Fill,
        Ask::Nothing => inherited(child.sizing_x),
    };
    n.sizing_y = match h {
        Ask::Size(h) => {
            n.fixed_height = h;
            SFSizing::Fixed
        }
        Ask::Infinity => SFSizing::Fill,
        Ask::Nothing => inherited(child.sizing_y),
    };

    let (p, l) = leak(vec![child]);
    n.children = p;
    n.children_len = l;
    n
}

fn padding(child: SFNode, insets: SFEdgeInsets) -> SFNode {
    let mut n = SFNode::empty();
    n.kind = SFNodeKind::Stack;
    n.axis = SFAxis::Depth;
    n.alignment = SFAlignment::Center;
    n.vertical_alignment = SFAlignment::Center;
    n.padding = insets;
    n.sizing_x = inherited(child.sizing_x);
    n.sizing_y = inherited(child.sizing_y);
    let (p, l) = leak(vec![child]);
    n.children = p;
    n.children_len = l;
    n
}

fn content(w: f32, h: f32) -> SFNode {
    let mut n = SFNode::empty();
    n.kind = SFNodeKind::Rect;
    n.set_sizing(SFSizing::Fixed);
    n.fixed_width = w;
    n.fixed_height = h;
    n
}

fn vstack(children: Vec<SFNode>) -> SFNode {
    let mut n = SFNode::empty();
    n.kind = SFNodeKind::Stack;
    n.axis = SFAxis::Vertical;
    n.set_sizing(SFSizing::Hug);
    n.alignment = SFAlignment::Leading;
    let (p, l) = leak(children);
    n.children = p;
    n.children_len = l;
    n
}

const VIEWPORT: SFRect = SFRect {
    x: 0.0,
    y: 0.0,
    width: 400.0,
    height: 800.0,
};

#[test]
fn max_width_infinity_does_not_claim_the_height() {

    let mut row = frame(
        vstack(vec![content(120.0, 44.0)]),
        Ask::Infinity,
        Ask::Nothing,
    );
    layout(&mut row, VIEWPORT);

    assert_eq!(row.frame.width, 400.0, "it should fill the width");
    assert_eq!(
        row.frame.height, 44.0,
        "and hug the height — it came out the viewport's before per-axis sizing"
    );
}

#[test]
fn max_height_infinity_does_not_claim_the_width() {

    let mut column = frame(
        vstack(vec![content(120.0, 44.0)]),
        Ask::Nothing,
        Ask::Infinity,
    );
    layout(&mut column, VIEWPORT);

    assert_eq!(column.frame.width, 120.0);
    assert_eq!(column.frame.height, 800.0);
}

#[test]
fn a_width_only_frame_leaves_the_height_to_the_content() {

    let mut button = frame(content(20.0, 30.0), Ask::Size(74.0), Ask::Nothing);
    layout(&mut button, VIEWPORT);

    assert_eq!(button.frame.width, 74.0);
    assert_eq!(
        button.frame.height, 30.0,
        "a width-only frame used to resolve the height to the whole \
         proposal, which is how a 74pt button reached the bottom of the screen"
    );
}

#[test]
fn a_height_only_frame_leaves_the_width_to_the_content() {
    let mut bar = frame(content(20.0, 30.0), Ask::Nothing, Ask::Size(44.0));
    layout(&mut bar, VIEWPORT);

    assert_eq!(bar.frame.width, 20.0);
    assert_eq!(bar.frame.height, 44.0);
}

#[test]
fn a_width_and_an_infinite_height_both_take_effect() {

    let mut sidebar = frame(content(20.0, 30.0), Ask::Size(100.0), Ask::Infinity);
    layout(&mut sidebar, VIEWPORT);

    assert_eq!(sidebar.frame.width, 100.0);
    assert_eq!(sidebar.frame.height, 800.0);
}

#[test]
fn a_zero_height_frame_is_zero_and_not_unstated() {

    let mut pad = frame(content(0.0, 0.0), Ask::Infinity, Ask::Size(0.0));
    layout(&mut pad, VIEWPORT);

    assert_eq!(
        pad.frame.height, 0.0,
        "`fixed_height > 0` read this as 'no height was asked for' and \
         resolved it to the full 800 — a viewport of phantom content at \
         the one scroll position every list starts in"
    );
}

#[test]
fn a_zero_width_frame_is_zero_too() {
    let mut pad = frame(content(0.0, 0.0), Ask::Size(0.0), Ask::Infinity);
    layout(&mut pad, VIEWPORT);
    assert_eq!(pad.frame.width, 0.0);
}

const INSET: SFEdgeInsets = SFEdgeInsets {
    top: 10.0,
    bottom: 10.0,
    leading: 20.0,
    trailing: 20.0,
};

#[test]
fn padding_around_a_one_sided_fill_keeps_that_one_side() {

    let mut padded = padding(
        frame(content(120.0, 44.0), Ask::Infinity, Ask::Nothing),
        INSET,
    );
    layout(&mut padded, VIEWPORT);

    assert_eq!(padded.frame.width, 400.0, "still filling across");
    assert_eq!(padded.frame.height, 64.0, "44 of content plus 10 each end");

    let inner = &kids(&padded)[0];
    assert_eq!(
        inner.frame.width, 360.0,
        "the child fills what is left after the inset"
    );
    assert_eq!((inner.frame.x, inner.frame.y), (20.0, 10.0));
}

#[test]
fn padding_does_not_inherit_fixed() {

    let mut padded = padding(content(120.0, 44.0), INSET);
    layout(&mut padded, VIEWPORT);

    assert_eq!(padded.frame.width, 160.0);
    assert_eq!(padded.frame.height, 64.0);
}

#[test]
fn a_child_that_fills_a_hugging_axis_waits_for_the_others() {

    let mut backdrop = SFNode::empty();
    backdrop.kind = SFNodeKind::Rect;
    backdrop.set_sizing(SFSizing::Fill);

    let mut wrapper = SFNode::empty();
    wrapper.kind = SFNodeKind::Stack;
    wrapper.axis = SFAxis::Depth;
    wrapper.alignment = SFAlignment::Center;
    wrapper.vertical_alignment = SFAlignment::Center;

    wrapper.sizing_x = SFSizing::Fill;
    wrapper.sizing_y = SFSizing::Hug;
    let (p, l) = leak(vec![backdrop, content(120.0, 44.0)]);
    wrapper.children = p;
    wrapper.children_len = l;

    layout(&mut wrapper, VIEWPORT);

    assert_eq!(wrapper.frame.width, 400.0);
    assert_eq!(wrapper.frame.height, 44.0, "the content set the height");

    let backdrop = &kids(&wrapper)[0];
    assert_eq!(
        (backdrop.frame.width, backdrop.frame.height),
        (400.0, 44.0),
        "and the backdrop filled the box that came out of it, not the viewport"
    );
}

fn fills_width_states_height() -> SFNode {
    let mut n = SFNode::empty();
    n.kind = SFNodeKind::Rect;
    n.sizing_x = SFSizing::Fill;
    n.sizing_y = SFSizing::Fixed;
    n.fixed_height = 44.0;
    n
}

#[test]
fn a_stack_that_hugs_only_the_height_measures_a_width_filling_child() {

    let mut stack = SFNode::empty();
    stack.kind = SFNodeKind::Stack;
    stack.axis = SFAxis::Depth;
    stack.sizing_x = SFSizing::Fill;
    stack.sizing_y = SFSizing::Hug;
    let (p, l) = leak(vec![fills_width_states_height()]);
    stack.children = p;
    stack.children_len = l;

    layout(&mut stack, VIEWPORT);

    assert_eq!(stack.frame.width, 400.0);
    assert_eq!(stack.frame.height, 44.0);
}

#[test]
fn a_stack_that_hugs_both_axes_cannot_measure_a_width_filling_child() {
    let mut stack = SFNode::empty();
    stack.kind = SFNodeKind::Stack;
    stack.axis = SFAxis::Depth;
    stack.set_sizing(SFSizing::Hug);
    let (p, l) = leak(vec![fills_width_states_height()]);
    stack.children = p;
    stack.children_len = l;

    layout(&mut stack, VIEWPORT);

    assert_eq!(
        (stack.frame.width, stack.frame.height),
        (0.0, 0.0),
        "the only child fills an axis this stack hugs, so it waits for a \
         hug that has nothing else to measure and both axes come out empty"
    );
}

#[test]
fn scroll_content_stops_at_a_declared_zero_height() {

    let pad = frame(content(120.0, 44.0), Ask::Infinity, Ask::Size(0.0));

    let mut scroll = SFNode::empty();
    scroll.kind = SFNodeKind::Scroll;
    scroll.axis = SFAxis::Vertical;
    scroll.set_sizing(SFSizing::Fill);
    let (p, l) = leak(vec![vstack(vec![pad, content(120.0, 44.0)])]);
    scroll.children = p;
    scroll.children_len = l;

    layout(&mut scroll, VIEWPORT);

    assert_eq!(
        scroll.content_height, 44.0,
        "the zero-height pad contributes nothing, so only the real row counts"
    );
}

#[test]
fn weight_is_read_from_the_main_axis_sizing() {

    let mut row = content(120.0, 44.0);
    row.weight = 1.0;

    let mut column = vstack(vec![row]);
    column.sizing_x = SFSizing::Fill;
    column.sizing_y = SFSizing::Hug;
    layout(&mut column, VIEWPORT);

    assert_eq!(kids(&column)[0].frame.height, 44.0);
    assert_eq!(column.frame.height, 44.0);
}

#[test]
fn cross_axis_alignment_is_read_from_the_cross_axis_sizing() {

    let mut n = SFNode::empty();
    n.kind = SFNodeKind::Stack;
    n.axis = SFAxis::Horizontal;
    n.sizing_x = SFSizing::Hug;
    n.sizing_y = SFSizing::Fill;
    n.alignment = SFAlignment::Center;
    let (p, l) = leak(vec![content(120.0, 44.0)]);
    n.children = p;
    n.children_len = l;

    layout(&mut n, VIEWPORT);

    assert_eq!(n.frame.width, 120.0, "hugging across");
    assert_eq!(n.frame.height, 800.0, "filling down");
    assert_eq!(
        kids(&n)[0].frame.y,
        (800.0 - 44.0) / 2.0,
        "and the child centred in the height it actually claimed"
    );
}
