use swiftflow_core::layout::layout;
use swiftflow_core::node::*;
use swiftflow_core::types::*;

fn leak(children: Vec<SFNode>) -> (*mut SFNode, usize) {
    let len = children.len();
    let boxed = children.into_boxed_slice();
    (Box::into_raw(boxed) as *mut SFNode, len)
}

fn rect(w: f32, h: f32) -> SFNode {
    let mut n = SFNode::empty();
    n.kind = SFNodeKind::Rect;
    n.set_sizing(SFSizing::Fixed);
    n.fixed_width = w;
    n.fixed_height = h;
    n
}

fn weighted(weight: f32) -> SFNode {
    let mut n = rect(10.0, 10.0);
    n.weight = weight;
    n
}

fn spacer() -> SFNode {
    let mut n = SFNode::empty();
    n.kind = SFNodeKind::Spacer;
    n
}

fn stack(axis: SFAxis, children: Vec<SFNode>) -> SFNode {
    let mut n = SFNode::empty();
    n.kind = SFNodeKind::Stack;
    n.axis = axis;
    n.set_sizing(SFSizing::Fill);
    let (ptr, len) = leak(children);
    n.children = ptr;
    n.children_len = len;
    n
}

fn kids(node: &SFNode) -> &[SFNode] {
    unsafe { std::slice::from_raw_parts(node.children, node.children_len) }
}

const WIDTH: f32 = 300.0;

fn laid_out(node: &mut SFNode) {
    layout(node, SFRect::new(0.0, 0.0, WIDTH, 100.0));
}

#[test]
fn equal_weights_make_equal_columns() {
    let mut root = stack(
        SFAxis::Horizontal,
        vec![weighted(1.0), weighted(1.0), weighted(1.0)],
    );
    laid_out(&mut root);

    for child in kids(&root) {
        assert_eq!(child.frame.width, WIDTH / 3.0);
    }
}

#[test]
fn equal_weights_are_laid_end_to_end_without_overflowing() {

    let mut root = stack(
        SFAxis::Horizontal,
        vec![weighted(1.0), weighted(1.0), weighted(1.0)],
    );
    laid_out(&mut root);

    let children = kids(&root);
    assert_eq!(children[0].frame.x, 0.0);
    assert_eq!(children[1].frame.x, WIDTH / 3.0);
    assert_eq!(children[2].frame.x, WIDTH * 2.0 / 3.0);

    let far_edge = children[2].frame.x + children[2].frame.width;
    assert!(far_edge <= WIDTH, "content runs {far_edge} past {WIDTH}");
}

#[test]
fn weights_are_proportional() {
    let mut root = stack(SFAxis::Horizontal, vec![weighted(1.0), weighted(2.0)]);
    laid_out(&mut root);

    let children = kids(&root);
    assert_eq!(children[0].frame.width, WIDTH / 3.0);
    assert_eq!(children[1].frame.width, WIDTH * 2.0 / 3.0);
}

#[test]
fn inflexible_children_are_measured_first() {

    let mut root = stack(
        SFAxis::Horizontal,
        vec![rect(100.0, 10.0), weighted(1.0), weighted(1.0)],
    );
    laid_out(&mut root);

    let children = kids(&root);
    assert_eq!(children[0].frame.width, 100.0);
    assert_eq!(children[1].frame.width, 100.0);
    assert_eq!(children[2].frame.width, 100.0);
}

#[test]
fn a_weighted_child_fills_its_share_even_when_its_content_is_smaller() {

    let mut root = stack(SFAxis::Horizontal, vec![weighted(1.0), weighted(1.0)]);
    laid_out(&mut root);

    for child in kids(&root) {
        assert_eq!(child.frame.width, WIDTH / 2.0, "content was only 10 wide");
    }
}

#[test]
fn a_spacer_is_weight_one_in_the_same_pool() {

    let mut root = stack(SFAxis::Horizontal, vec![weighted(1.0), spacer()]);
    laid_out(&mut root);

    let children = kids(&root);
    assert_eq!(children[0].frame.width, WIDTH / 2.0);
    assert_eq!(children[1].frame.width, WIDTH / 2.0);
}

#[test]
fn spacers_alone_behave_exactly_as_they_did() {

    let mut root = stack(
        SFAxis::Horizontal,
        vec![spacer(), rect(100.0, 10.0), spacer()],
    );
    laid_out(&mut root);

    let children = kids(&root);
    assert_eq!(children[0].frame.width, 100.0);
    assert_eq!(children[1].frame.width, 100.0);
    assert_eq!(children[2].frame.width, 100.0);
}

#[test]
fn weight_works_down_a_vstack_too() {
    let mut root = stack(SFAxis::Vertical, vec![weighted(1.0), weighted(3.0)]);
    layout(&mut root, SFRect::new(0.0, 0.0, 100.0, 400.0));

    let children = kids(&root);
    assert_eq!(children[0].frame.height, 100.0);
    assert_eq!(children[1].frame.height, 300.0);
}

#[test]
fn spacing_comes_out_of_the_leftover_before_it_is_shared() {
    let mut root = stack(SFAxis::Horizontal, vec![weighted(1.0), weighted(1.0)]);
    root.spacing = 20.0;
    laid_out(&mut root);

    let children = kids(&root);
    assert_eq!(children[0].frame.width, 140.0);
    assert_eq!(children[1].frame.width, 140.0);

    assert_eq!(children[1].frame.x + children[1].frame.width, WIDTH);
}

#[test]
fn weight_is_ignored_in_a_hugging_stack() {

    let mut root = stack(SFAxis::Horizontal, vec![weighted(1.0), weighted(1.0)]);
    root.set_sizing(SFSizing::Hug);
    laid_out(&mut root);

    let children = kids(&root);
    assert_eq!(children[0].frame.width, 10.0);
    assert_eq!(children[1].frame.width, 10.0);
    assert_eq!(root.frame.width, 20.0);
}

#[test]
fn a_weighted_child_lays_its_own_children_out_inside_its_share() {

    let mut inner = SFNode::empty();
    inner.kind = SFNodeKind::Rect;
    inner.set_sizing(SFSizing::Fill);

    let mut column = stack(SFAxis::Horizontal, vec![inner]);
    column.weight = 1.0;

    let mut root = stack(SFAxis::Horizontal, vec![column, weighted(1.0)]);
    laid_out(&mut root);

    let column = &kids(&root)[0];
    assert_eq!(column.frame.width, WIDTH / 2.0);
    assert_eq!(
        kids(column)[0].frame.width,
        WIDTH / 2.0,
        "the nested Fill resolved against the stack, not its column"
    );
}
