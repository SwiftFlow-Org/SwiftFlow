use swiftflow_core::draw::{DrawCommand, DrawItem};
use swiftflow_core::draw_pass::draw;
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

fn plate(red: f32) -> SFNode {
    let mut n = SFNode::empty();
    n.kind = SFNodeKind::Rect;
    n.set_sizing(SFSizing::Fill);
    n.no_merge = 1.0;
    n.fill = SFColor {
        r: red,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    n
}

fn buried(child: SFNode, depth: usize) -> SFNode {
    let mut node = child;
    for _ in 0..depth {
        let mut wrapper = SFNode::empty();
        wrapper.kind = SFNodeKind::Stack;
        wrapper.axis = SFAxis::Depth;
        wrapper.set_sizing(SFSizing::Fill);
        wrapper.alignment = SFAlignment::Center;
        wrapper.vertical_alignment = SFAlignment::Center;
        let (p, l) = leak(vec![node]);
        wrapper.children = p;
        wrapper.children_len = l;
        node = wrapper;
    }
    node
}

fn depth_stack(children: Vec<SFNode>) -> SFNode {
    let mut n = SFNode::empty();
    n.kind = SFNodeKind::Stack;
    n.axis = SFAxis::Depth;
    n.set_sizing(SFSizing::Fill);
    n.alignment = SFAlignment::Center;
    n.vertical_alignment = SFAlignment::Center;
    let (p, l) = leak(children);
    n.children = p;
    n.children_len = l;
    n
}

fn commands(mut root: SFNode) -> Vec<DrawItem> {
    layout(&mut root, SFRect::new(0.0, 0.0, 400.0, 800.0));
    let mut list = DrawList::new();
    draw(&mut root, &mut list);
    list.commands
}

fn index_of(items: &[DrawItem], red: f32) -> usize {
    items
        .iter()
        .position(|item| match &item.command {
            DrawCommand::Rect { fill, .. } => (fill.r - red).abs() < 1e-4,
            _ => false,
        })
        .unwrap_or_else(|| panic!("no rect was filled with the marker {red}"))
}

const CONTENT: f32 = 0.1;
const BAR: f32 = 0.2;
const SHEET: f32 = 0.3;

#[test]
fn a_sheet_nested_in_the_tab_content_draws_under_the_bar() {
    let content = buried(depth_stack(vec![plate(CONTENT), plate(SHEET)]), 3);
    let items = commands(depth_stack(vec![content, plate(BAR)]));

    assert!(
        index_of(&items, SHEET) < index_of(&items, BAR),
        "the bar no longer draws over a nested sheet, so the layer no longer \
         needs hoisting and PresentationStore has stopped earning its keep"
    );
}

#[test]
fn a_hoisted_sheet_draws_over_the_bar() {
    let content = buried(plate(CONTENT), 3);
    let items = commands(depth_stack(vec![content, plate(BAR), plate(SHEET)]));

    let (content_at, bar_at, sheet_at) = (
        index_of(&items, CONTENT),
        index_of(&items, BAR),
        index_of(&items, SHEET),
    );
    assert!(
        content_at < bar_at && bar_at < sheet_at,
        "expected content ({content_at}) then bar ({bar_at}) then sheet \
         ({sheet_at}) — a sheet that does not come last is still under the bar"
    );
}

#[test]
fn the_scrim_covers_the_bar_as_well() {
    let scrim = 0.4_f32;
    let content = buried(plate(CONTENT), 2);

    let layer = depth_stack(vec![plate(scrim), plate(SHEET)]);
    let items = commands(depth_stack(vec![content, plate(BAR), layer]));

    assert!(
        index_of(&items, BAR) < index_of(&items, scrim),
        "the scrim draws before the bar, so the bar stays bright behind it"
    );
    assert!(
        index_of(&items, scrim) < index_of(&items, SHEET),
        "the sheet must sit on top of its own scrim"
    );
}
