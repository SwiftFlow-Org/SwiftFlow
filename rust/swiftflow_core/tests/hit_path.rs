use swiftflow_core::layout::layout;
use swiftflow_core::node::*;
use swiftflow_core::types::*;

extern "C" {
    fn sf_hit_test_path(
        root: *const SFNode,
        x: f32,
        y: f32,
        out: *mut u32,
        cap: usize,
    ) -> usize;
}

fn leak(children: Vec<SFNode>) -> (*mut SFNode, usize) {
    let len = children.len();
    let boxed = children.into_boxed_slice();
    (Box::into_raw(boxed) as *mut SFNode, len)
}

fn stack(axis: SFAxis, id: u32, children: Vec<SFNode>) -> SFNode {
    let mut n = SFNode::empty();
    n.kind = SFNodeKind::Stack;
    n.axis = axis;
    n.set_sizing(SFSizing::Hug);
    n.node_id = id;
    let (ptr, len) = leak(children);
    n.children = ptr;
    n.children_len = len;
    n
}

fn rect(id: u32, w: f32, h: f32) -> SFNode {
    let mut n = SFNode::empty();
    n.kind = SFNodeKind::Rect;
    n.set_sizing(SFSizing::Fixed);
    n.fixed_width = w;
    n.fixed_height = h;
    n.node_id = id;
    n
}

fn path_at(root: &SFNode, x: f32, y: f32) -> Vec<u32> {
    let mut buf = [0u32; 16];
    let n = unsafe { sf_hit_test_path(root, x, y, buf.as_mut_ptr(), buf.len()) };
    buf[..n].to_vec()
}

const VIEWPORT: SFRect = SFRect {
    x: 0.0,
    y: 0.0,
    width: 400.0,
    height: 800.0,
};

#[test]
fn the_chain_reaches_the_ancestor_holding_the_handler() {
    const BUTTON: u32 = 100;
    const CAPSULE: u32 = 101;
    const LABEL: u32 = 102;
    const ROOT: u32 = 1;

    let button = stack(
        SFAxis::Depth,
        BUTTON,
        vec![rect(CAPSULE, 200.0, 44.0), rect(LABEL, 120.0, 20.0)],
    );

    let mut root = stack(SFAxis::Vertical, ROOT, vec![button]);
    layout(&mut root, VIEWPORT);

    let path = path_at(&root, 100.0, 22.0);

    assert_eq!(
        path.first(),
        Some(&LABEL),
        "the deepest node still comes first: {path:?}"
    );
    assert!(
        path.contains(&BUTTON),
        "the button holding the action must be on the chain: {path:?}"
    );
    assert_eq!(path.last(), Some(&ROOT), "the chain ends at the root");

    let index = |id: u32| path.iter().position(|&p| p == id).unwrap();
    assert!(index(LABEL) < index(BUTTON));
    assert!(index(BUTTON) < index(ROOT));
}

#[test]
fn a_point_outside_everything_yields_nothing() {
    let mut root = stack(SFAxis::Vertical, 1, vec![rect(2, 100.0, 100.0)]);
    root.set_sizing(SFSizing::Hug);
    layout(&mut root, VIEWPORT);

    assert!(path_at(&root, -10.0, -10.0).is_empty());
}

#[test]
fn the_front_most_sibling_wins_an_overlap() {
    let under = rect(10, 200.0, 200.0);
    let over = rect(11, 200.0, 200.0);
    let mut root = stack(SFAxis::Depth, 1, vec![under, over]);
    root.set_sizing(SFSizing::Hug);
    layout(&mut root, VIEWPORT);

    let path = path_at(&root, 100.0, 100.0);
    assert_eq!(path, vec![11, 1], "the covered sibling must not appear");
}

#[test]
fn a_short_buffer_keeps_the_deepest_ids() {
    let inner = stack(SFAxis::Depth, 3, vec![rect(4, 50.0, 50.0)]);
    let middle = stack(SFAxis::Depth, 2, vec![inner]);
    let mut root = stack(SFAxis::Depth, 1, vec![middle]);
    root.set_sizing(SFSizing::Hug);
    layout(&mut root, VIEWPORT);

    let mut buf = [0u32; 2];
    let n = unsafe { sf_hit_test_path(&root, 25.0, 25.0, buf.as_mut_ptr(), buf.len()) };
    assert_eq!(n, 2);
    assert_eq!(buf, [4, 3]);
}
