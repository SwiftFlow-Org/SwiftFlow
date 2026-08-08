use swiftflow_core::draw::DrawCommand;
use swiftflow_core::draw_pass::draw;
use swiftflow_core::ffi::sf_get_node_frame;
use swiftflow_core::layout::layout;
use swiftflow_core::node::*;
use swiftflow_core::types::*;
use swiftflow_core::DrawList;

fn leak(children: Vec<SFNode>) -> (*mut SFNode, usize) {
    let len = children.len();
    (
        Box::into_raw(children.into_boxed_slice()) as *mut SFNode,
        len,
    )
}

const TARGET: u32 = 77;

fn button(id: u32, w: f32, h: f32) -> SFNode {
    let mut n = SFNode::empty();
    n.kind = SFNodeKind::Rect;
    n.set_sizing(SFSizing::Fixed);
    n.fixed_width = w;
    n.fixed_height = h;
    n.no_merge = 1.0;
    n.node_id = id;

    n.fill = SFColor {
        r: id as f32 / 255.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    n
}

fn vstack(children: Vec<SFNode>, padding: SFEdgeInsets) -> SFNode {
    let mut n = SFNode::empty();
    n.kind = SFNodeKind::Stack;
    n.axis = SFAxis::Vertical;
    n.set_sizing(SFSizing::Hug);
    n.alignment = SFAlignment::Leading;
    n.main_axis_alignment = SFAlignment::Leading;
    n.padding = padding;
    let (p, l) = leak(children);
    n.children = p;
    n.children_len = l;
    n
}

fn insets(top: f32, leading: f32) -> SFEdgeInsets {
    SFEdgeInsets {
        top,
        bottom: 0.0,
        leading,
        trailing: 0.0,
    }
}

fn laid_out(mut root: SFNode) -> SFNode {
    layout(&mut root, SFRect::new(0.0, 0.0, 400.0, 800.0));
    root
}

fn drawn_frame(root: &mut SFNode) -> SFRect {
    let mut list = DrawList::new();
    draw(root, &mut list);
    list.commands
        .iter()
        .find_map(|item| match &item.command {
            DrawCommand::Rect { frame, fill, .. }
                if (fill.r - TARGET as f32 / 255.0).abs() < 1e-4 =>
            {
                Some(*frame)
            }
            _ => None,
        })
        .expect("the target rect was never drawn")
}

#[test]
fn a_nested_node_reports_its_world_space_frame() {
    let mut root = laid_out(vstack(
        vec![
            button(1, 40.0, 20.0),
            vstack(vec![button(TARGET, 44.0, 44.0)], insets(12.0, 8.0)),
        ],
        insets(30.0, 16.0),
    ));

    let reported = sf_get_node_frame(&root, TARGET);
    let drawn = drawn_frame(&mut root);
    assert_eq!(
        reported, drawn,
        "the readback disagrees with the draw pass about where the node is"
    );
    assert_eq!(reported.width, 44.0);
    assert_eq!(reported.height, 44.0);

    assert_eq!(reported.x, 16.0 + 8.0);
    assert_eq!(reported.y, 30.0 + 20.0 + 12.0);
}

#[test]
fn an_ancestors_scale_and_offset_are_folded_in() {
    let mut inner = vstack(vec![button(TARGET, 44.0, 44.0)], insets(0.0, 0.0));
    inner.scale = SFScale { x: 0.5, y: 0.5 };
    inner.offset_x = 10.0;
    inner.offset_y = -6.0;

    let mut root = laid_out(vstack(vec![inner], insets(30.0, 16.0)));

    let reported = sf_get_node_frame(&root, TARGET);
    let drawn = drawn_frame(&mut root);
    assert_eq!(
        reported, drawn,
        "an ancestor's transform reached the draw pass but not the readback"
    );
    assert_eq!(
        (reported.width, reported.height),
        (22.0, 22.0),
        "a halved ancestor should report a halved button"
    );
}

#[test]
fn a_missing_id_reports_nothing_rather_than_failing() {
    let root = laid_out(vstack(vec![button(TARGET, 44.0, 44.0)], insets(0.0, 0.0)));
    assert_eq!(sf_get_node_frame(&root, 9999), SFRect::ZERO);
}

#[test]
fn the_zero_id_never_matches() {
    let mut plain = SFNode::empty();
    plain.kind = SFNodeKind::Rect;
    plain.set_sizing(SFSizing::Fixed);
    plain.fixed_width = 10.0;
    plain.fixed_height = 10.0;
    let root = laid_out(vstack(vec![plain], insets(0.0, 0.0)));
    assert_eq!(sf_get_node_frame(&root, 0), SFRect::ZERO);
}

#[test]
fn a_null_root_is_survivable() {
    assert_eq!(
        sf_get_node_frame(std::ptr::null(), TARGET),
        SFRect::ZERO,
        "a host that asks before its first frame must not crash"
    );
}
