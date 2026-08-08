use swiftflow_core::layout::layout;
use swiftflow_core::node::*;
use swiftflow_core::types::*;

const VIEWPORT_W: f32 = 400.0;
const VIEWPORT_H: f32 = 800.0;
const ROW_H: f32 = 100.0;
const ROWS: usize = 20;
const CONTENT_H: f32 = ROW_H * ROWS as f32;
const MAX_OFFSET: f32 = CONTENT_H - VIEWPORT_H;

fn leak(children: Vec<SFNode>) -> (*mut SFNode, usize) {
    let len = children.len();
    let boxed = children.into_boxed_slice();
    (Box::into_raw(boxed) as *mut SFNode, len)
}

fn kids(node: &SFNode) -> &[SFNode] {
    unsafe { std::slice::from_raw_parts(node.children, node.children_len) }
}

fn row(h: f32) -> SFNode {
    let mut n = SFNode::empty();
    n.kind = SFNodeKind::Rect;
    n.set_sizing(SFSizing::Fixed);
    n.fixed_width = 300.0;
    n.fixed_height = h;
    n
}

fn rows() -> Vec<SFNode> {
    (0..ROWS).map(|_| row(ROW_H)).collect()
}

fn vstack(children: Vec<SFNode>, sizing: SFSizing) -> SFNode {
    let mut n = SFNode::empty();
    n.kind = SFNodeKind::Stack;
    n.axis = SFAxis::Vertical;
    n.set_sizing(sizing);
    n.alignment = SFAlignment::Center;
    n.vertical_alignment = SFAlignment::Center;
    n.main_axis_alignment = SFAlignment::Center;
    let (p, l) = leak(children);
    n.children = p;
    n.children_len = l;
    n
}

fn box_wrapper(child: SFNode, sizing: SFSizing, pad: f32) -> SFNode {
    let mut n = SFNode::empty();
    n.kind = SFNodeKind::Stack;
    n.axis = SFAxis::Depth;
    n.set_sizing(sizing);
    n.alignment = SFAlignment::Center;
    n.vertical_alignment = SFAlignment::Center;
    n.padding = SFEdgeInsets {
        top: pad,
        bottom: pad,
        leading: pad,
        trailing: pad,
    };
    let (p, l) = leak(vec![child]);
    n.children = p;
    n.children_len = l;
    n
}

fn scroll(children: Vec<SFNode>, offset: f32) -> SFNode {
    let mut n = SFNode::empty();
    n.kind = SFNodeKind::Scroll;
    n.axis = SFAxis::Vertical;
    n.set_sizing(SFSizing::Fill);
    n.content_offset_y = offset;
    let (p, l) = leak(children);
    n.children = p;
    n.children_len = l;
    n
}

fn row_y(scroll: &SFNode, index: usize) -> f32 {
    fn walk(node: &SFNode, acc: f32, index: usize) -> Option<f32> {
        let here = acc + node.frame.y;
        if node.children.is_null() || node.children_len == 0 {
            return None;
        }
        let children = kids(node);
        if children.len() == ROWS && children.iter().all(|c| c.kind == SFNodeKind::Rect) {
            return Some(here + children[index].frame.y);
        }
        children.iter().find_map(|c| walk(c, here, index))
    }
    walk(scroll, 0.0, index).expect("the row list should be somewhere under the scroll view")
}

fn laid_out(mut node: SFNode) -> SFNode {
    layout(&mut node, SFRect::from_size(VIEWPORT_W, VIEWPORT_H));
    node
}

fn shapes(offset: f32) -> Vec<(&'static str, SFNode)> {
    vec![
        ("VStack { … }", scroll(vec![vstack(rows(), SFSizing::Hug)], offset)),
        (
            "VStack { … }.frame(maxWidth: .infinity)",
            scroll(
                vec![box_wrapper(vstack(rows(), SFSizing::Hug), SFSizing::Fill, 0.0)],
                offset,
            ),
        ),
        (
            "VStack { … }.expands()",
            scroll(vec![vstack(rows(), SFSizing::Fill)], offset),
        ),
        ("the rows directly", scroll(rows(), offset)),
    ]
}

#[test]
fn content_starts_at_the_top_of_the_scroll_view() {
    for (name, tree) in shapes(0.0) {
        let tree = laid_out(tree);
        assert_eq!(
            row_y(&tree, 0),
            0.0,
            "ScrollView {{ {name} }} does not begin at its own top — it opens \
             already scrolled while the scroll state still reads 0"
        );
    }
}

#[test]
fn the_content_moves_by_exactly_the_offset() {
    for offset in [1.0, 250.0, MAX_OFFSET] {
        for (name, tree) in shapes(offset) {
            let tree = laid_out(tree);
            assert_eq!(
                row_y(&tree, 0),
                -offset,
                "ScrollView {{ {name} }} at offset {offset} moved its content \
                 somewhere other than {} — a scroll that does not track its \
                 own offset cannot be corrected by the physics either",
                -offset
            );
        }
    }
}

#[test]
fn the_end_of_the_range_lands_the_last_row_at_the_bottom() {
    for (name, tree) in shapes(MAX_OFFSET) {
        let tree = laid_out(tree);
        let bottom = row_y(&tree, ROWS - 1) + ROW_H;
        assert_eq!(
            bottom, VIEWPORT_H,
            "ScrollView {{ {name} }} scrolled to its own maximum leaves the \
             last row's bottom at {bottom} rather than at the viewport edge"
        );
    }
}

#[test]
fn a_child_smaller_than_its_frame_is_still_centred() {
    let framed = box_wrapper(row(200.0), SFSizing::Fill, 0.0);
    let tree = laid_out(framed);
    let child = &kids(&tree)[0];
    assert_eq!(child.frame.y, (VIEWPORT_H - 200.0) / 2.0);
    assert_eq!(child.frame.x, (VIEWPORT_W - 300.0) / 2.0);
}

#[test]
fn a_child_larger_than_its_frame_starts_flush_with_it() {
    let framed = box_wrapper(row(VIEWPORT_H + 400.0), SFSizing::Fill, 0.0);
    let tree = laid_out(framed);
    let child = &kids(&tree)[0];
    assert_eq!(
        child.frame.y, 0.0,
        "an overflowing child has no slack to distribute, so it starts at \
         the box's own edge rather than hanging above it"
    );
}

#[test]
fn an_overflowing_child_still_sits_inside_its_padding() {
    let padded = box_wrapper(row(VIEWPORT_H + 400.0), SFSizing::Fill, 16.0);
    let tree = laid_out(padded);
    assert_eq!(kids(&tree)[0].frame.y, 16.0);
}
