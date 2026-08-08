use swiftflow_core::layout::layout;
use swiftflow_core::node::*;
use swiftflow_core::types::*;

fn leak(children: Vec<SFNode>) -> (*mut SFNode, usize) {
    let len = children.len();
    let boxed = children.into_boxed_slice();
    (Box::into_raw(boxed) as *mut SFNode, len)
}

fn row(height: f32) -> SFNode {
    let mut n = SFNode::empty();
    n.kind = SFNodeKind::Rect;
    n.set_sizing(SFSizing::Fixed);
    n.fixed_width = 100.0;
    n.fixed_height = height;
    n
}

fn filling_vstack(main_axis: SFAlignment) -> SFNode {
    let mut n = SFNode::empty();
    n.kind = SFNodeKind::Stack;
    n.axis = SFAxis::Vertical;

    n.set_sizing(SFSizing::Fill);
    n.main_axis_alignment = main_axis;
    let (ptr, len) = leak(vec![row(100.0), row(100.0)]);
    n.children = ptr;
    n.children_len = len;
    n
}

fn scroll_containing(child: SFNode) -> SFNode {
    let mut n = SFNode::empty();
    n.kind = SFNodeKind::Scroll;
    n.set_sizing(SFSizing::Fill);
    n.scroll_id = 1;
    let (ptr, len) = leak(vec![child]);
    n.children = ptr;
    n.children_len = len;
    n
}

fn first_row_y(stack: &SFNode) -> f32 {
    let children = unsafe { std::slice::from_raw_parts(stack.children, stack.children_len) };
    children[0].frame.y
}

#[test]
fn a_filling_stack_still_centres_by_default() {
    let mut stack = filling_vstack(SFAlignment::Center);
    layout(&mut stack, SFRect::new(0.0, 0.0, 400.0, 600.0));

    assert_eq!(first_row_y(&stack), 200.0);
}

#[test]
fn leading_alignment_pins_to_the_top() {
    let mut stack = filling_vstack(SFAlignment::Leading);
    layout(&mut stack, SFRect::new(0.0, 0.0, 400.0, 600.0));

    assert_eq!(first_row_y(&stack), 0.0);
}

#[test]
fn trailing_alignment_pushes_to_the_bottom() {
    let mut stack = filling_vstack(SFAlignment::Trailing);
    layout(&mut stack, SFRect::new(0.0, 0.0, 400.0, 600.0));

    assert_eq!(first_row_y(&stack), 400.0);
}

#[test]
fn scroll_content_pins_to_the_top_even_when_it_asks_to_fill() {

    let mut scroll = scroll_containing(filling_vstack(SFAlignment::Center));
    layout(&mut scroll, SFRect::new(0.0, 0.0, 400.0, 600.0));

    let children = unsafe { std::slice::from_raw_parts(scroll.children, scroll.children_len) };
    assert_eq!(
        first_row_y(&children[0]),
        0.0,
        "content must start at the top of the scroll view, not floating in it"
    );
}

#[test]
fn a_hugging_stack_has_no_slack_to_align() {
    let mut stack = filling_vstack(SFAlignment::Center);
    stack.set_sizing(SFSizing::Hug);
    layout(&mut stack, SFRect::new(0.0, 0.0, 400.0, 600.0));

    assert_eq!(first_row_y(&stack), 0.0);
    assert_eq!(stack.frame.height, 200.0);
}

#[test]
fn horizontal_stacks_align_along_their_own_main_axis() {
    let mut stack = filling_vstack(SFAlignment::Trailing);
    stack.axis = SFAxis::Horizontal;
    layout(&mut stack, SFRect::new(0.0, 0.0, 400.0, 600.0));

    let children = unsafe { std::slice::from_raw_parts(stack.children, stack.children_len) };
    assert_eq!(children[0].frame.x, 200.0);
}
