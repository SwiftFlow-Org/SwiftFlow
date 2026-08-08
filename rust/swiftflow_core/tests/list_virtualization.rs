use swiftflow_core::layout::layout;
use swiftflow_core::node::*;
use swiftflow_core::types::*;

const OVERSCAN: f32 = 180.0;
const FIRST_FRAME_ROWS: i32 = 40;

#[derive(Debug, PartialEq)]
struct ListWindow {
    first: i32,
    count: i32,
    leading_pad: f32,
    trailing_pad: f32,
}

impl ListWindow {
    fn new(heights: &[f32], offset: f32, viewport_length: f32) -> Self {
        let row_count = heights.len() as i32;
        let h = |i: i32| heights[i as usize].max(0.0);

        if row_count <= 0 {
            return ListWindow {
                first: 0,
                count: 0,
                leading_pad: 0.0,
                trailing_pad: 0.0,
            };
        }

        if viewport_length <= 0.0 {
            let count = row_count.min(FIRST_FRAME_ROWS);
            let trailing: f32 = (count..row_count).map(h).sum();
            return ListWindow {
                first: 0,
                count,
                leading_pad: 0.0,
                trailing_pad: trailing,
            };
        }

        let content: f32 = (0..row_count).map(h).sum();
        let max_offset = (content - viewport_length).max(0.0);
        let visible_top = offset.max(0.0).min(max_offset);
        let top = (visible_top - OVERSCAN).max(0.0);
        let bottom = content.min(visible_top + viewport_length + OVERSCAN);

        let mut first_index = 0;
        let mut leading = 0.0f32;
        let mut cursor = 0.0f32;
        let mut index = 0;

        while index < row_count {
            let height = h(index);
            if cursor + height > top {
                break;
            }
            cursor += height;
            leading = cursor;
            index += 1;
            first_index = index;
        }

        let mut built = 0;
        while index < row_count && cursor < bottom {
            cursor += h(index);
            index += 1;
            built += 1;
        }

        let trailing: f32 = (index..row_count).map(h).sum();

        ListWindow {
            first: first_index,
            count: built,
            leading_pad: leading,
            trailing_pad: trailing,
        }
    }

    fn last(&self) -> i32 {
        self.first + self.count
    }
}

fn heights(row_count: usize, pattern: &[f32]) -> Vec<f32> {
    (0..row_count).map(|i| pattern[i % pattern.len()]).collect()
}

fn total(heights: &[f32]) -> f32 {
    heights.iter().sum()
}

fn row_top(heights: &[f32], i: i32) -> f32 {
    heights[..i as usize].iter().sum()
}

fn covers_viewport(w: &ListWindow, hs: &[f32], offset: f32, viewport: f32) {
    let content = total(hs);
    let max_offset = (content - viewport).max(0.0);
    let top = offset.clamp(0.0, max_offset);
    let bottom = (top + viewport).min(content);

    for i in 0..hs.len() as i32 {
        let row_top = row_top(hs, i);
        let row_bottom = row_top + hs[i as usize];

        let visible = row_bottom > top + 0.01 && row_top < bottom - 0.01;
        if visible {
            assert!(
                i >= w.first && i < w.last(),
                "row {i} (spanning {row_top}..{row_bottom}) is on screen but the \
                 window is {}..{}: offset {offset}, viewport {viewport}",
                w.first,
                w.last()
            );
        }
    }
}

#[test]
fn the_window_always_covers_what_is_on_screen() {
    for pattern in [
        &[44.0f32][..],
        &[44.0, 44.0, 120.0][..],
        &[20.0, 200.0][..],
        &[1.0, 300.0, 55.5, 88.0][..],
    ] {
        for &row_count in &[1usize, 2, 7, 40, 41, 500, 3000] {
            let hs = heights(row_count, pattern);
            let content = total(&hs);
            for &viewport in &[100.0f32, 400.0, 852.0] {
                let mut offsets: Vec<f32> = (0..30).map(|i| content * (i as f32 / 29.0)).collect();
                offsets.extend_from_slice(&[
                    -viewport,
                    -1.0,
                    0.0,
                    content,
                    content + viewport,
                    content * 2.0,
                ]);
                for offset in offsets {
                    let w = ListWindow::new(&hs, offset, viewport);
                    assert!(w.first >= 0 && w.count >= 0, "{w:?} at offset {offset}");
                    assert!(
                        w.last() <= row_count as i32,
                        "{w:?} runs past {row_count} rows at offset {offset}"
                    );
                    covers_viewport(&w, &hs, offset, viewport);
                }
            }
        }
    }
}

#[test]
fn the_pads_and_the_built_rows_add_up_to_the_whole_list() {
    for pattern in [&[44.0f32][..], &[20.0, 200.0][..], &[1.0, 300.0, 55.5][..]] {
        for &row_count in &[1usize, 9, 60, 3000] {
            let hs = heights(row_count, pattern);
            let content = total(&hs);
            for step in 0..40 {
                let offset = content * (step as f32 / 39.0);
                let w = ListWindow::new(&hs, offset, 800.0);
                let built: f32 = (w.first..w.last()).map(|i| hs[i as usize]).sum();
                let sum = w.leading_pad + built + w.trailing_pad;
                assert!(
                    (sum - content).abs() < 0.5,
                    "{w:?} accounts for {sum} of {content} points"
                );
                assert!(
                    (w.leading_pad - row_top(&hs, w.first)).abs() < 0.5,
                    "the leading pad {} does not reach row {}, which starts at {}",
                    w.leading_pad,
                    w.first,
                    row_top(&hs, w.first)
                );
            }
        }
    }
}

#[test]
fn the_first_frame_builds_rows_before_anything_is_measured() {
    let hs = heights(1000, &[44.0]);
    let w = ListWindow::new(&hs, 0.0, 0.0);
    assert_eq!(w.first, 0);
    assert_eq!(w.count, FIRST_FRAME_ROWS);
    assert!((w.trailing_pad - (1000 - FIRST_FRAME_ROWS) as f32 * 44.0).abs() < 0.01);
}

#[test]
fn nothing_degenerate_traps() {
    for hs in [vec![], vec![0.0f32; 10], vec![44.0], vec![0.0, 44.0, 0.0]] {
        for &offset in &[-500.0f32, 0.0, 500.0] {
            let w = ListWindow::new(&hs, offset, 800.0);
            assert!(w.first >= 0 && w.count >= 0, "{w:?} for {hs:?}");
            assert!(w.last() <= hs.len() as i32, "{w:?} for {hs:?}");
            assert!(w.leading_pad >= 0.0 && w.trailing_pad >= 0.0, "{w:?}");
        }
    }
}

#[test]
fn a_long_list_builds_a_screenful() {
    let hs = heights(3000, &[44.0]);
    let w = ListWindow::new(&hs, 5_000.0, 852.0);
    assert!(
        w.count < 40,
        "built {} rows to fill an 852pt viewport of 44pt rows",
        w.count
    );
}

#[test]
fn a_list_of_mixed_heights_still_lands() {
    let hs = heights(400, &[18.0, 240.0, 44.0, 96.0]);
    let content = total(&hs);
    for step in 0..60 {
        let offset = content * (step as f32 / 59.0);
        let w = ListWindow::new(&hs, offset, 852.0);
        covers_viewport(&w, &hs, offset, 852.0);
        let built: f32 = (w.first..w.last()).map(|i| hs[i as usize]).sum();
        assert!(
            (w.leading_pad + built + w.trailing_pad - content).abs() < 0.5,
            "{w:?} does not account for the whole list at offset {offset}"
        );
    }
}

fn leak(children: Vec<SFNode>) -> (*mut SFNode, usize) {
    let len = children.len();
    (
        Box::into_raw(children.into_boxed_slice()) as *mut SFNode,
        len,
    )
}

fn kids(node: &SFNode) -> &[SFNode] {
    unsafe { std::slice::from_raw_parts(node.children, node.children_len) }
}

fn inherited(sizing: SFSizing) -> SFSizing {
    if sizing == SFSizing::Fill {
        SFSizing::Fill
    } else {
        SFSizing::Hug
    }
}

fn padding(child: SFNode, insets: SFEdgeInsets) -> SFNode {
    let mut n = SFNode::empty();
    n.kind = SFNodeKind::Stack;
    n.axis = SFAxis::Depth;
    n.sizing_x = inherited(child.sizing_x);
    n.sizing_y = inherited(child.sizing_y);
    n.alignment = SFAlignment::Center;
    n.vertical_alignment = SFAlignment::Center;
    n.padding = insets;
    let (p, l) = leak(vec![child]);
    n.children = p;
    n.children_len = l;
    n
}

fn frame(child: SFNode, height: Option<f32>, leading: bool) -> SFNode {
    let mut n = SFNode::empty();
    n.kind = SFNodeKind::Stack;
    n.axis = SFAxis::Depth;

    n.sizing_x = SFSizing::Fill;
    n.sizing_y = if height.is_some() {
        SFSizing::Fixed
    } else {
        inherited(child.sizing_y)
    };
    n.fixed_height = height.unwrap_or(0.0);
    n.alignment = if leading {
        SFAlignment::Leading
    } else {
        SFAlignment::Center
    };
    n.vertical_alignment = SFAlignment::Center;
    let (p, l) = leak(vec![child]);
    n.children = p;
    n.children_len = l;
    n
}

fn vstack(children: Vec<SFNode>) -> SFNode {
    let mut n = SFNode::empty();
    n.kind = SFNodeKind::Stack;
    n.axis = SFAxis::Vertical;
    n.set_sizing(SFSizing::Hug);
    n.alignment = SFAlignment::Leading;
    n.vertical_alignment = SFAlignment::Center;
    n.main_axis_alignment = SFAlignment::Center;
    let (p, l) = leak(children);
    n.children = p;
    n.children_len = l;
    n
}

fn divider() -> SFNode {
    let mut n = SFNode::empty();
    n.kind = SFNodeKind::Rect;
    n.set_sizing(SFSizing::Fill);
    n.fixed_height = 1.0;
    n
}

fn row_content(height: f32) -> SFNode {
    let mut n = SFNode::empty();
    n.kind = SFNodeKind::Rect;
    n.set_sizing(SFSizing::Fixed);
    n.fixed_width = 120.0;
    n.fixed_height = height;
    n
}

const INSETS: SFEdgeInsets = SFEdgeInsets {
    top: 11.0,
    bottom: 11.0,
    leading: 16.0,
    trailing: 16.0,
};

fn list_row(_row_height: f32, content_height: f32) -> SFNode {
    let content = padding(row_content(content_height), INSETS);
    let separator = frame(
        padding(
            divider(),
            SFEdgeInsets {
                top: 0.0,
                bottom: 0.0,
                leading: 16.0,
                trailing: 0.0,
            },
        ),
        Some(1.0),
        true,
    );
    vstack(vec![content, separator])
}

fn list_tree(w: &ListWindow, row_height: f32, content_height: f32) -> SFNode {
    let column = frame(
        vstack(
            (0..w.count)
                .map(|_| list_row(row_height, content_height))
                .collect(),
        ),
        None,
        true,
    );
    let padded = padding(
        column,
        SFEdgeInsets {
            top: w.leading_pad,
            bottom: w.trailing_pad,
            leading: 0.0,
            trailing: 0.0,
        },
    );

    let mut scroll = SFNode::empty();
    scroll.kind = SFNodeKind::Scroll;
    scroll.axis = SFAxis::Vertical;
    scroll.set_sizing(SFSizing::Fill);
    let (p, l) = leak(vec![padded]);
    scroll.children = p;
    scroll.children_len = l;
    scroll
}

#[test]
fn a_row_is_as_tall_as_its_content() {
    const SEPARATOR: f32 = 1.0;
    let vertical_insets = INSETS.top + INSETS.bottom;
    for &content_height in &[4.0f32, 20.0, 43.0, 300.0] {
        let mut root = list_row(0.0, content_height);
        layout(&mut root, SFRect::from_size(400.0, 800.0));
        let expected = content_height + vertical_insets + SEPARATOR;
        assert!(
            (root.frame.height - expected).abs() < 0.01,
            "a row holding {content_height} of content came out {} tall, \
             expected {expected}",
            root.frame.height
        );
    }
}

#[test]
fn the_pads_are_exact_at_both_ends_of_the_scroll() {

    let row_height = 20.0 + INSETS.top + INSETS.bottom + 1.0;
    let row_count = 100;
    let content = row_count as f32 * row_height;

    let hs = heights(row_count as usize, &[row_height]);
    for &offset in &[0.0f32, 44.0, 1000.0, content] {
        let w = ListWindow::new(&hs, offset, 800.0);
        let mut tree = list_tree(&w, row_height, 20.0);
        layout(&mut tree, SFRect::from_size(400.0, 800.0));

        assert!(
            (tree.content_height - content).abs() < 0.01,
            "at offset {offset} the list reports {} points of content, not {content} \
             — the scrollable range is wrong, so it will stop scrolling early or run past the end",
            tree.content_height
        );

        if w.count > 0 {

            let pad = &kids(&tree)[0];
            let framed = &kids(pad)[0];
            let column = &kids(framed)[0];
            let first_row = &kids(column)[0];
            let absolute = pad.frame.y + framed.frame.y + column.frame.y + first_row.frame.y;
            assert!(
                (absolute - w.leading_pad).abs() < 0.01,
                "at offset {offset} the first built row sits at {absolute}, not {}",
                w.leading_pad
            );
        }
    }
}

#[test]
fn a_blank_view_of_zero_height_measures_zero() {
    let mut column = SFNode::empty();
    column.kind = SFNodeKind::Stack;
    column.axis = SFAxis::Vertical;
    column.set_sizing(SFSizing::Hug);
    column.alignment = SFAlignment::Leading;
    column.main_axis_alignment = SFAlignment::Leading;

    let (p, l) = leak(vec![
        row_content(0.0),
        row_content(44.0),
        row_content(44.0),
        row_content(0.0),
    ]);
    column.children = p;
    column.children_len = l;

    let mut scroll = SFNode::empty();
    scroll.kind = SFNodeKind::Scroll;
    scroll.axis = SFAxis::Vertical;
    scroll.set_sizing(SFSizing::Fill);
    let (p, l) = leak(vec![column]);
    scroll.children = p;
    scroll.children_len = l;
    layout(&mut scroll, SFRect::from_size(400.0, 800.0));

    assert_eq!(
        scroll.content_height, 88.0,
        "two 44pt rows between two zero-height pads is 88 points of \
         content; before per-axis sizing this measured 1688, a whole \
         viewport of nothing at the top of every list"
    );
}
