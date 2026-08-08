use swiftflow_core::layout::layout;
use swiftflow_core::node::*;
use swiftflow_core::types::*;

fn leak(children: Vec<SFNode>) -> (*mut SFNode, usize) {
    let len = children.len();
    let boxed = children.into_boxed_slice();
    (Box::into_raw(boxed) as *mut SFNode, len)
}

fn stack(axis: SFAxis, children: Vec<SFNode>) -> SFNode {
    let mut n = SFNode::empty();
    n.kind = SFNodeKind::Stack;
    n.axis = axis;
    n.set_sizing(SFSizing::Hug);
    n.alignment = SFAlignment::Center;
    n.vertical_alignment = SFAlignment::Center;
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

fn spacer() -> SFNode {
    let mut n = SFNode::empty();
    n.kind = SFNodeKind::Spacer;
    n.set_sizing(SFSizing::Fill);
    n
}

fn dump(label: &str, node: &SFNode, depth: usize, abs_x: f32, abs_y: f32) {
    let x = abs_x + node.frame.x;
    let y = abs_y + node.frame.y;
    println!(
        "{:indent$}{label:<22} rel=({:7.1},{:7.1}) size=({:7.1},{:7.1})  ABS_Y={:7.1}..{:7.1}",
        "",
        node.frame.x,
        node.frame.y,
        node.frame.width,
        node.frame.height,
        y,
        y + node.frame.height,
        indent = depth * 2
    );
    if !node.children.is_null() && node.children_len > 0 {
        let kids = unsafe { std::slice::from_raw_parts(node.children, node.children_len) };

        let inner_x = x;
        let inner_y = y;
        for (i, k) in kids.iter().enumerate() {
            dump(&format!("child{i}"), k, depth + 1, inner_x, inner_y);
        }
    }
}

#[test]
fn nav_bar_item_row_position() {
    let safe_top = 59.0_f32;
    let bar_h = 44.0_f32;
    let large = 52.0_f32;
    let collapsed = safe_top + bar_h;
    let total = collapsed + large;

    let mut item_row = stack(SFAxis::Horizontal, vec![spacer(), rect(44.0, 44.0)]);
    item_row.set_sizing(SFSizing::Fixed);
    item_row.fixed_height = bar_h;
    item_row.padding = SFEdgeInsets {
        top: 0.0,
        bottom: 0.0,
        leading: 20.0,
        trailing: 20.0,
    };

    let mut bar_row = stack(SFAxis::Depth, vec![item_row]);
    bar_row.set_sizing(SFSizing::Fixed);
    bar_row.fixed_height = collapsed;
    bar_row.padding = SFEdgeInsets {
        top: safe_top,
        bottom: 0.0,
        leading: 0.0,
        trailing: 0.0,
    };

    let mut large_title = stack(SFAxis::Horizontal, vec![rect(200.0, 40.0), spacer()]);
    large_title.set_sizing(SFSizing::Fixed);
    large_title.fixed_height = large;
    large_title.padding = SFEdgeInsets {
        top: 0.0,
        bottom: 0.0,
        leading: 20.0,
        trailing: 20.0,
    };

    let bar_stack = stack(SFAxis::Vertical, vec![bar_row, large_title]);

    let mut material = SFNode::empty();
    material.kind = SFNodeKind::Rect;
    material.set_sizing(SFSizing::Fill);
    material.blur_radius = 9.0;

    let mut bg_wrap = stack(SFAxis::Depth, vec![material, bar_stack]);
    bg_wrap.set_sizing(SFSizing::Fixed);
    bg_wrap.fixed_height = total;
    bg_wrap.alignment = SFAlignment::Leading;
    bg_wrap.vertical_alignment = SFAlignment::Leading;

    let mut content = SFNode::empty();
    content.kind = SFNodeKind::Rect;
    content.set_sizing(SFSizing::Fill);

    let mut root = stack(SFAxis::Depth, vec![content, bg_wrap]);
    root.set_sizing(SFSizing::Fill);
    root.alignment = SFAlignment::Center;
    root.vertical_alignment = SFAlignment::Leading;

    layout(&mut root, SFRect::new(0.0, 0.0, 393.0, 852.0));

    dump("root", &root, 0, 0.0, 0.0);

    let bg_wrap = child(&root, 1);
    let bar_stack = child(bg_wrap, 1);
    let bar_row = child(bar_stack, 0);
    let item_row = child(bar_row, 0);
    let item = child(item_row, 1);

    let item_y = root.frame.y
        + bg_wrap.frame.y
        + bar_stack.frame.y
        + bar_row.frame.y
        + item_row.frame.y
        + item.frame.y;

    assert_eq!(
        item_y, safe_top,
        "toolbar item should sit at the top of the bar row (just below the \
         safe area), not offset by it twice"
    );
    assert_eq!(item.frame.height, 44.0);
    assert!(
        item_y + item.frame.height <= collapsed,
        "toolbar item must stay inside the bar row, not spill into the \
         large-title band"
    );

    let large = child(bar_stack, 1);
    let large_y = root.frame.y + bg_wrap.frame.y + bar_stack.frame.y + large.frame.y;
    assert_eq!(large_y, collapsed);
}

fn child(node: &SFNode, i: usize) -> &SFNode {
    assert!(!node.children.is_null() && i < node.children_len);
    unsafe { &std::slice::from_raw_parts(node.children, node.children_len)[i] }
}
