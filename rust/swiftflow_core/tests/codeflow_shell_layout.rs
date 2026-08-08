use swiftflow_core::layout::layout;
use swiftflow_core::node::*;
use swiftflow_core::types::*;

fn leak(children: Vec<SFNode>) -> (*mut SFNode, usize) {
    let len = children.len();
    let boxed = children.into_boxed_slice();
    (Box::into_raw(boxed) as *mut SFNode, len)
}

fn kids(node: &SFNode) -> &[SFNode] {
    unsafe { std::slice::from_raw_parts(node.children, node.children_len) }
}

fn fixed(w: f32, h: f32) -> SFNode {
    let mut n = SFNode::empty();
    n.kind = SFNodeKind::Rect;
    n.set_sizing(SFSizing::Fixed);
    n.fixed_width = w;
    n.fixed_height = h;
    n
}

fn stack(axis: SFAxis, sizing: SFSizing, children: Vec<SFNode>) -> SFNode {
    let mut n = SFNode::empty();
    n.kind = SFNodeKind::Stack;
    n.axis = axis;
    n.set_sizing(sizing);
    let (ptr, len) = leak(children);
    n.children = ptr;
    n.children_len = len;
    n
}

fn weighted(mut node: SFNode, weight: f32) -> SFNode {
    node.weight = weight;
    node
}

const W: f32 = 1200.0;
const H: f32 = 800.0;
const SIDEBAR: f32 = 220.0;
const STATUS: f32 = 24.0;

fn frame_fill(child: SFNode) -> SFNode {
    stack(SFAxis::Depth, SFSizing::Fill, vec![child])
}

#[test]
fn a_hug_stack_drops_its_childrens_weights() {

    let mut root = frame_fill(stack(
        SFAxis::Vertical,
        SFSizing::Hug,
        vec![weighted(fixed(100.0, 10.0), 1.0), fixed(100.0, STATUS)],
    ));
    layout(&mut root, SFRect::new(0.0, 0.0, W, H));

    let column = &kids(&root)[0];
    let editor = &kids(column)[0];
    assert_eq!(
        editor.frame.height, 10.0,
        "a Hug container ignores weight — the pane keeps its natural height"
    );
    assert!(
        column.frame.height < H,
        "so the column hugs its content rather than filling the window"
    );
}

#[test]
fn a_fill_column_gives_the_editor_everything_but_the_status_bar() {
    let mut root = stack(
        SFAxis::Vertical,
        SFSizing::Fill,
        vec![weighted(fixed(100.0, 10.0), 1.0), fixed(100.0, STATUS)],
    );
    layout(&mut root, SFRect::new(0.0, 0.0, W, H));

    let editor = &kids(&root)[0];
    let status = &kids(&root)[1];
    assert_eq!(editor.frame.height, H - STATUS);
    assert_eq!(status.frame.height, STATUS);
    assert_eq!(status.frame.y, H - STATUS);
}

#[test]
fn the_sidebar_keeps_its_width_and_the_editor_takes_the_rest() {

    let mut root = stack(
        SFAxis::Horizontal,
        SFSizing::Fill,
        vec![
            fixed(SIDEBAR, 10.0),
            fixed(1.0, 10.0),
            weighted(fixed(10.0, 10.0), 1.0),
        ],
    );
    layout(&mut root, SFRect::new(0.0, 0.0, W, H));

    let sidebar = &kids(&root)[0];
    let rule = &kids(&root)[1];
    let editor = &kids(&root)[2];

    assert_eq!(sidebar.frame.width, SIDEBAR);
    assert_eq!(rule.frame.width, 1.0);
    assert_eq!(editor.frame.width, W - SIDEBAR - 1.0);
    assert_eq!(editor.frame.x, SIDEBAR + 1.0);
}

#[test]
fn the_whole_shell_lays_out_when_every_container_fills() {

    let editor_column = weighted(
        stack(
            SFAxis::Vertical,
            SFSizing::Fill,
            vec![fixed(10.0, 36.0), weighted(fixed(10.0, 10.0), 1.0)],
        ),
        1.0,
    );
    let body = weighted(
        stack(
            SFAxis::Horizontal,
            SFSizing::Fill,
            vec![fixed(SIDEBAR, 10.0), editor_column],
        ),
        1.0,
    );
    let mut root = stack(
        SFAxis::Vertical,
        SFSizing::Fill,
        vec![body, fixed(10.0, STATUS)],
    );
    layout(&mut root, SFRect::new(0.0, 0.0, W, H));

    let body = &kids(&root)[0];
    let status = &kids(&root)[1];
    assert_eq!(body.frame.height, H - STATUS);
    assert_eq!(status.frame.y, H - STATUS);

    let column = &kids(body)[1];
    assert_eq!(column.frame.width, W - SIDEBAR);

    let tabs = &kids(column)[0];
    let editor = &kids(column)[1];
    assert_eq!(tabs.frame.height, 36.0);
    assert_eq!(
        editor.frame.height,
        H - STATUS - 36.0,
        "the code pane is everything the tab strip and status bar leave"
    );
}

#[test]
fn weight_wins_over_fill_on_the_same_node() {

    let mut pane = stack(
        SFAxis::Vertical,
        SFSizing::Fill,
        vec![fixed(10.0, 10.0)],
    );
    pane.weight = 1.0;

    let mut root = stack(
        SFAxis::Vertical,
        SFSizing::Fill,
        vec![pane, fixed(10.0, STATUS)],
    );
    layout(&mut root, SFRect::new(0.0, 0.0, W, H));

    assert_eq!(kids(&root)[0].frame.height, H - STATUS);
    assert_eq!(kids(&root)[1].frame.y, H - STATUS);
}

#[test]
fn the_sidebar_gets_full_height_from_a_width_only_frame() {

    let mut sidebar = fixed(SIDEBAR, 0.0);
    sidebar.sizing_y = SFSizing::Fill;

    let mut root = stack(
        SFAxis::Horizontal,
        SFSizing::Fill,
        vec![sidebar, weighted(fixed(10.0, 10.0), 1.0)],
    );
    layout(&mut root, SFRect::new(0.0, 0.0, W, H));

    let sidebar = &kids(&root)[0];
    assert_eq!(sidebar.frame.width, SIDEBAR);
    assert_eq!(sidebar.frame.height, H);
}
