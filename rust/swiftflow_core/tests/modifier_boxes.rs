use swiftflow_core::layout::layout;
use swiftflow_core::node::*;
use swiftflow_core::types::*;

fn leak(children: Vec<SFNode>) -> (*mut SFNode, usize) {
    let len = children.len();
    (Box::into_raw(children.into_boxed_slice()) as *mut SFNode, len)
}

fn kids(node: &SFNode) -> &[SFNode] {
    unsafe { std::slice::from_raw_parts(node.children, node.children_len) }
}

fn inset(v: f32) -> SFEdgeInsets {
    SFEdgeInsets {
        top: v,
        bottom: v,
        leading: v,
        trailing: v,
    }
}

fn shape(w: f32, h: f32) -> SFNode {
    let mut n = SFNode::empty();
    n.kind = SFNodeKind::Rect;
    n.set_sizing(SFSizing::Fixed);
    n.fixed_width = w;
    n.fixed_height = h;
    n
}

fn filling_shape() -> SFNode {
    let mut n = SFNode::empty();
    n.kind = SFNodeKind::Rect;
    n.set_sizing(SFSizing::Fill);
    n
}

fn inherited(sizing: SFSizing) -> SFSizing {
    if sizing == SFSizing::Fill {
        SFSizing::Fill
    } else {
        SFSizing::Hug
    }
}

fn padding_box(child: SFNode, amount: f32) -> SFNode {
    let mut n = SFNode::empty();
    n.kind = SFNodeKind::Stack;
    n.axis = SFAxis::Depth;
    n.padding = inset(amount);
    n.sizing_x = inherited(child.sizing_x);
    n.sizing_y = inherited(child.sizing_y);
    let (ptr, len) = leak(vec![child]);
    n.children = ptr;
    n.children_len = len;
    n
}

fn frame_box(child: SFNode, w: Option<f32>, h: Option<f32>) -> SFNode {
    let mut n = SFNode::empty();
    n.kind = SFNodeKind::Stack;
    n.axis = SFAxis::Depth;
    n.sizing_x = match w {
        Some(w) => {
            n.fixed_width = w;
            SFSizing::Fixed
        }
        None => inherited(child.sizing_x),
    };
    n.sizing_y = match h {
        Some(h) => {
            n.fixed_height = h;
            SFSizing::Fixed
        }
        None => inherited(child.sizing_y),
    };
    let (ptr, len) = leak(vec![child]);
    n.children = ptr;
    n.children_len = len;
    n
}

fn fill_box(child: SFNode) -> SFNode {
    let mut n = SFNode::empty();
    n.kind = SFNodeKind::Stack;
    n.axis = SFAxis::Depth;
    n.set_sizing(SFSizing::Fill);
    let (ptr, len) = leak(vec![child]);
    n.children = ptr;
    n.children_len = len;
    n
}

const SPACE: SFRect = SFRect {
    x: 0.0,
    y: 0.0,
    width: 400.0,
    height: 400.0,
};

#[test]
fn padding_leaves_a_gap_instead_of_inflating_the_shape() {

    let mut root = padding_box(shape(44.0, 44.0), 10.0);
    layout(&mut root, SPACE);

    assert_eq!(root.frame.width, 64.0, "the box is content plus padding");
    assert_eq!(root.frame.height, 64.0);

    let circle = &kids(&root)[0];
    assert_eq!(circle.frame.width, 44.0, "the shape kept its own size");
    assert_eq!(circle.frame.height, 44.0);
    assert_eq!((circle.frame.x, circle.frame.y), (10.0, 10.0));
}

#[test]
fn nested_frames_nest_instead_of_overwriting() {

    let mut root = frame_box(
        frame_box(shape(50.0, 50.0), Some(100.0), Some(100.0)),
        Some(200.0),
        Some(200.0),
    );
    layout(&mut root, SPACE);

    assert_eq!(root.frame.width, 200.0);
    let inner = &kids(&root)[0];
    assert_eq!(inner.frame.width, 100.0, "the inner frame survived");

    assert_eq!(inner.frame.x, 50.0);
}

#[test]
fn padding_composes_with_itself() {

    let mut root = padding_box(padding_box(shape(40.0, 40.0), 10.0), 20.0);
    layout(&mut root, SPACE);

    assert_eq!(root.frame.width, 100.0);
    assert_eq!(kids(&root)[0].frame.width, 60.0);
    assert_eq!(kids(&kids(&root)[0])[0].frame.width, 40.0);
}

#[test]
fn a_frame_inside_padding_is_not_the_same_as_padding_inside_a_frame() {

    let padded_then_framed = {
        let mut n = frame_box(padding_box(shape(40.0, 40.0), 20.0), Some(200.0), None);
        layout(&mut n, SPACE);
        n
    };
    let framed_then_padded = {
        let mut n = padding_box(frame_box(shape(40.0, 40.0), Some(200.0), None), 20.0);
        layout(&mut n, SPACE);
        n
    };

    assert_eq!(padded_then_framed.frame.width, 200.0);

    assert_eq!(framed_then_padded.frame.width, 240.0);
}

#[test]
fn a_fill_child_keeps_filling_through_padding() {

    let mut root = padding_box(fill_box(filling_shape()), 20.0);
    layout(&mut root, SPACE);

    assert_eq!(root.frame.width, 400.0, "the box still fills");
    assert_eq!(
        kids(&root)[0].frame.width,
        360.0,
        "the child fills what is left after the inset"
    );
}

#[test]
fn scroll_content_is_measured_through_wrapper_nodes() {

    let rows: Vec<SFNode> = (0..5).map(|_| shape(100.0, 200.0)).collect();
    let mut column = SFNode::empty();
    column.kind = SFNodeKind::Stack;
    column.axis = SFAxis::Vertical;
    column.set_sizing(SFSizing::Hug);
    let (ptr, len) = leak(rows);
    column.children = ptr;
    column.children_len = len;

    let mut scroll = SFNode::empty();
    scroll.kind = SFNodeKind::Scroll;
    scroll.axis = SFAxis::Vertical;
    scroll.set_sizing(SFSizing::Fill);
    let (ptr, len) = leak(vec![fill_box(column)]);
    scroll.children = ptr;
    scroll.children_len = len;

    layout(&mut scroll, SFRect::new(0.0, 0.0, 400.0, 300.0));

    assert_eq!(
        scroll.content_height, 1000.0,
        "five 200pt rows behind a wrapper should still measure 1000"
    );
}

#[test]
fn a_padded_scroll_child_reports_its_padding_too() {
    let mut column = SFNode::empty();
    column.kind = SFNodeKind::Stack;
    column.axis = SFAxis::Vertical;
    column.set_sizing(SFSizing::Hug);
    let (ptr, len) = leak(vec![shape(100.0, 200.0), shape(100.0, 200.0)]);
    column.children = ptr;
    column.children_len = len;

    let mut scroll = SFNode::empty();
    scroll.kind = SFNodeKind::Scroll;
    scroll.axis = SFAxis::Vertical;
    scroll.set_sizing(SFSizing::Fill);
    let (ptr, len) = leak(vec![padding_box(column, 20.0)]);
    scroll.children = ptr;
    scroll.children_len = len;

    layout(&mut scroll, SFRect::new(0.0, 0.0, 400.0, 300.0));

    assert_eq!(scroll.content_height, 440.0, "400 of rows plus 20 each end");
}
