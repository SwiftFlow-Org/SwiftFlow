use swiftflow_core::layout::layout;
use swiftflow_core::node::*;
use swiftflow_core::types::*;

fn leak(children: Vec<SFNode>) -> (*mut SFNode, usize) {
    let len = children.len();
    let boxed = children.into_boxed_slice();
    (Box::into_raw(boxed) as *mut SFNode, len)
}

fn stack(axis: SFAxis, spacing: f32, children: Vec<SFNode>) -> SFNode {
    let mut n = SFNode::empty();
    n.kind = SFNodeKind::Stack;
    n.axis = axis;
    n.spacing = spacing;
    n.set_sizing(SFSizing::Hug);
    n.alignment = SFAlignment::Center;
    n.vertical_alignment = SFAlignment::Center;
    let (ptr, len) = leak(children);
    n.children = ptr;
    n.children_len = len;
    n
}

fn fixed_rect(w: f32, h: f32) -> SFNode {
    let mut n = SFNode::empty();
    n.kind = SFNodeKind::Rect;
    n.set_sizing(SFSizing::Fixed);
    n.fixed_width = w;
    n.fixed_height = h;
    n
}

fn fill_rect(h: f32) -> SFNode {
    let mut n = SFNode::empty();
    n.kind = SFNodeKind::Rect;
    n.set_sizing(SFSizing::Fixed);
    n.fixed_height = h;
    n
}

fn insets(top: f32, bottom: f32, leading: f32, trailing: f32) -> SFEdgeInsets {
    SFEdgeInsets {
        top,
        bottom,
        leading,
        trailing,
    }
}

#[test]
fn scroll_content_height_exceeds_viewport() {
    let s = 3.0_f32;
    let viewport_w = 393.0 * s;
    let viewport_h = 852.0 * s;

    let nav_inset = (59.0 + 44.0 + 52.0) * s;
    let vstack_pad = 20.0 * s;
    let vstack_spacing = 16.0 * s;

    let text_h = 20.0 * s;
    let row_h = 44.0 * s;
    let card_h = 200.0 * s;

    let mut button_row = stack(
        SFAxis::Horizontal,
        10.0 * s,
        vec![
            fixed_rect(row_h, row_h),
            fixed_rect(120.0 * s, 40.0 * s),
            fixed_rect(row_h, row_h),
        ],
    );
    button_row.set_sizing(SFSizing::Hug);

    let mut content = stack(
        SFAxis::Vertical,
        vstack_spacing,
        vec![
            fixed_rect(200.0 * s, text_h),
            button_row,
            fill_rect(card_h),
            fill_rect(card_h),
            fill_rect(card_h),
            fill_rect(card_h),
        ],
    );

    content.padding = insets(vstack_pad, vstack_pad, vstack_pad, vstack_pad);
    content.set_sizing(SFSizing::Fill);

    let mut scroll = SFNode::empty();
    scroll.kind = SFNodeKind::Scroll;
    scroll.set_sizing(SFSizing::Fill);
    scroll.scroll_id = 42;
    scroll.padding = insets(nav_inset, 0.0, 0.0, 0.0);
    let (ptr, len) = leak(vec![content]);
    scroll.children = ptr;
    scroll.children_len = len;

    let mut root = stack(SFAxis::Depth, 0.0, vec![scroll]);
    root.set_sizing(SFSizing::Fill);
    root.vertical_alignment = SFAlignment::Leading;

    layout(&mut root, SFRect::new(0.0, 0.0, viewport_w, viewport_h));

    let scroll = unsafe { &*root.children };
    let vstack = unsafe { &*scroll.children };

    let natural = text_h + row_h + card_h * 4.0 + vstack_spacing * 5.0 + vstack_pad * 2.0;

    println!("viewport_h        = {viewport_h}");
    println!("nav_inset         = {nav_inset}");
    println!("vstack.frame.h    = {} (natural {natural})", vstack.frame.height);
    println!("scroll.frame.h    = {}", scroll.frame.height);
    println!("scroll.content_h  = {}", scroll.content_height);
    println!(
        "maxOffset         = {}",
        (scroll.content_height - scroll.frame.height).max(0.0)
    );

    assert_eq!(
        vstack.frame.height, natural,
        "the scroll's content stack must report its natural height, not the \
         viewport height it was proposed — otherwise content_height collapses \
         to the viewport and nothing is scrollable"
    );

    assert_eq!(
        scroll.content_height,
        natural + nav_inset,
        "content_height is the content plus the scroll node's own padding \
         (NavigationStack insets by the bar height)"
    );

    let max_offset = scroll.content_height - scroll.frame.height;
    assert!(
        max_offset > 0.0,
        "maxOffset must be positive or every release springs the list back \
         to the top; got content_height {} vs viewport {}",
        scroll.content_height,
        scroll.frame.height
    );
}

#[test]
fn scroll_metrics_ffi_finds_the_node() {
    let s = 3.0_f32;
    let viewport_h = 852.0 * s;

    let mut content = stack(SFAxis::Vertical, 16.0 * s, (0..6).map(|_| fill_rect(200.0 * s)).collect::<Vec<_>>());
    content.set_sizing(SFSizing::Fill);

    let mut scroll = SFNode::empty();
    scroll.kind = SFNodeKind::Scroll;
    scroll.set_sizing(SFSizing::Fill);
    scroll.scroll_id = 7;
    let (ptr, len) = leak(vec![content]);
    scroll.children = ptr;
    scroll.children_len = len;

    let mut root = stack(SFAxis::Depth, 0.0, vec![scroll]);
    root.set_sizing(SFSizing::Fill);

    layout(&mut root, SFRect::new(0.0, 0.0, 393.0 * s, viewport_h));

    let m = swiftflow_core::ffi::sf_get_scroll_metrics(&root, 7);
    println!("ffi viewport={} content={}", m.viewport_height, m.content_height);
    assert!(m.viewport_height > 0.0, "metrics lookup missed the scroll node");
    assert!(
        m.content_height > m.viewport_height,
        "content {} must exceed viewport {}",
        m.content_height,
        m.viewport_height
    );
}
