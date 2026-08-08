const RUBBER_BAND_RANGE: f32 = 120.0;
const OPEN_THRESHOLD: f32 = 0.5;
const PROJECTION_TIME: f32 = 0.12;
const FULL_SWIPE_THRESHOLD: f32 = 0.65;

fn rubber_band(excess: f32) -> f32 {
    if excess <= 0.0 {
        return 0.0;
    }
    (excess * RUBBER_BAND_RANGE) / (RUBBER_BAND_RANGE + excess)
}

fn offset(translation: f32, open_width: f32, sign: f32) -> f32 {
    let travel = translation * sign;
    if travel <= 0.0 {
        return 0.0;
    }
    if travel <= open_width {
        return travel * sign;
    }
    (open_width + rubber_band(travel - open_width)) * sign
}

fn is_full_swipe(offset: f32, container_width: f32, has_destructive: bool) -> bool {
    if !has_destructive || container_width <= 0.0 {
        return false;
    }
    offset.abs() >= container_width * FULL_SWIPE_THRESHOLD
}

fn resting_offset(offset: f32, velocity: f32, open_width: f32, sign: f32) -> f32 {
    if open_width <= 0.0 {
        return 0.0;
    }
    let projected = (offset + velocity * PROJECTION_TIME) * sign;
    if projected >= open_width * OPEN_THRESHOLD {
        open_width * sign
    } else {
        0.0
    }
}

const TRAILING: f32 = -1.0;
const LEADING: f32 = 1.0;
const OPEN_WIDTH: f32 = 160.0;

#[test]
fn the_content_tracks_the_finger_until_the_actions_are_revealed() {
    for &sign in &[TRAILING, LEADING] {
        for travel in [1.0f32, 40.0, 159.0, OPEN_WIDTH] {
            let o = offset(travel * sign, OPEN_WIDTH, sign);
            assert!(
                (o - travel * sign).abs() < 0.001,
                "at {travel} of travel the content sat at {o}, not {}",
                travel * sign
            );
        }
    }
}

#[test]
fn past_full_open_it_rubber_bands() {
    for &sign in &[TRAILING, LEADING] {
        let full = offset(OPEN_WIDTH * sign, OPEN_WIDTH, sign).abs();
        let mut previous = full;
        for excess in [1.0f32, 10.0, 60.0, 200.0, 1000.0] {
            let o = offset((OPEN_WIDTH + excess) * sign, OPEN_WIDTH, sign).abs();
            assert!(o > previous, "pulling {excess} further did not move it");
            assert!(
                o < OPEN_WIDTH + excess,
                "at {excess} past open it moved {o}, which is not resistance"
            );
            previous = o;
        }

        let first = offset((OPEN_WIDTH + 10.0) * sign, OPEN_WIDTH, sign).abs() - full;
        let later = offset((OPEN_WIDTH + 210.0) * sign, OPEN_WIDTH, sign).abs()
            - offset((OPEN_WIDTH + 200.0) * sign, OPEN_WIDTH, sign).abs();
        assert!(
            first > later,
            "the last ten points of pull moved it {later}, the first {first} — \
             resistance has to grow or there is a seam where it starts"
        );
    }
}

#[test]
fn dragging_away_from_the_actions_does_nothing() {
    for &sign in &[TRAILING, LEADING] {
        for wrong_way in [1.0f32, 50.0, 400.0] {
            assert_eq!(offset(wrong_way * -sign, OPEN_WIDTH, sign), 0.0);
        }
    }
}

#[test]
fn a_flick_opens_where_a_slow_drag_does_not() {
    for &sign in &[TRAILING, LEADING] {

        let flicked = resting_offset(OPEN_WIDTH * 0.3 * sign, 900.0 * sign, OPEN_WIDTH, sign);
        assert_eq!(
            flicked,
            OPEN_WIDTH * sign,
            "a hard flick a third of the way should open"
        );

        let crawled = resting_offset(OPEN_WIDTH * 0.45 * sign, 0.0, OPEN_WIDTH, sign);
        assert_eq!(crawled, 0.0, "a drag that stopped short should spring shut");
    }
}

#[test]
fn a_flick_back_shuts_an_open_row() {
    for &sign in &[TRAILING, LEADING] {
        let shut = resting_offset(OPEN_WIDTH * 0.9 * sign, 1200.0 * -sign, OPEN_WIDTH, sign);
        assert_eq!(shut, 0.0, "flicking back should close it");
    }
}

#[test]
fn it_only_ever_lands_open_or_shut() {
    for &sign in &[TRAILING, LEADING] {
        for step in 0..40 {
            let o = OPEN_WIDTH * (step as f32 / 39.0) * sign;
            for velocity in [-2000.0f32, -300.0, 0.0, 300.0, 2000.0] {
                let rest = resting_offset(o, velocity * sign, OPEN_WIDTH, sign);
                assert!(
                    rest == 0.0 || (rest - OPEN_WIDTH * sign).abs() < 0.001,
                    "landed at {rest}, which is neither shut nor open"
                );
            }
        }
    }
}

#[test]
fn a_full_swipe_needs_much_more_than_opening_does() {
    let container = 393.0f32;

    assert!(
        !is_full_swipe(-OPEN_WIDTH, container, true),
        "opening the row armed the full swipe"
    );
    assert!(
        !is_full_swipe(-(OPEN_WIDTH + 60.0), container, true),
        "rubber-banding past open armed the full swipe"
    );
    assert!(
        is_full_swipe(-container * 0.7, container, true),
        "throwing the row two thirds across did not arm it"
    );
}

#[test]
fn nothing_arms_a_full_swipe_without_a_destructive_action() {
    let container = 393.0f32;
    for offset in [-100.0f32, -300.0, -1000.0, 300.0] {
        assert!(
            !is_full_swipe(offset, container, false),
            "a row of ordinary actions armed a full swipe at {offset}"
        );
    }
}

#[test]
fn the_full_swipe_works_from_either_edge() {
    let container = 393.0f32;
    assert!(is_full_swipe(-container * 0.7, container, true));
    assert!(is_full_swipe(container * 0.7, container, true));
}

#[test]
fn an_unmeasured_container_arms_nothing() {
    for offset in [0.0f32, -50.0, -5000.0] {
        assert!(!is_full_swipe(offset, 0.0, true));
    }
}

use swiftflow_core::layout::layout;
use swiftflow_core::node::*;
use swiftflow_core::types::*;

fn leak(children: Vec<SFNode>) -> (*mut SFNode, usize) {
    let len = children.len();
    (
        Box::into_raw(children.into_boxed_slice()) as *mut SFNode,
        len,
    )
}

const ROW_HEIGHT: f32 = 44.0;
const VIEWPORT_HEIGHT: f32 = 800.0;

fn row_content() -> SFNode {
    let mut n = SFNode::empty();
    n.kind = SFNodeKind::Rect;
    n.set_sizing(SFSizing::Fixed);
    n.fixed_width = 300.0;
    n.fixed_height = ROW_HEIGHT;
    n
}

fn action_button() -> SFNode {
    let mut n = SFNode::empty();
    n.kind = SFNodeKind::Stack;
    n.axis = SFAxis::Depth;
    n.sizing_x = SFSizing::Fixed;
    n.sizing_y = SFSizing::Hug;
    n.fixed_width = 74.0;
    let (p, l) = leak(vec![SFNode::empty()]);
    n.children = p;
    n.children_len = l;
    n
}

fn buttons_row(sizing: SFSizing) -> SFNode {
    let mut n = SFNode::empty();
    n.kind = SFNodeKind::Stack;
    n.axis = SFAxis::Horizontal;
    n.set_sizing(sizing);
    n.alignment = SFAlignment::Center;
    n.vertical_alignment = SFAlignment::Center;
    let (p, l) = leak(vec![action_button(), action_button()]);
    n.children = p;
    n.children_len = l;
    n
}

fn swipe_row(buttons_sizing: SFSizing) -> SFNode {
    let mut n = SFNode::empty();
    n.kind = SFNodeKind::Stack;
    n.axis = SFAxis::Depth;
    n.set_sizing(SFSizing::Hug);
    n.alignment = SFAlignment::Trailing;
    n.vertical_alignment = SFAlignment::Center;
    let (p, l) = leak(vec![buttons_row(buttons_sizing), row_content()]);
    n.children = p;
    n.children_len = l;
    n
}

#[test]
fn a_width_only_frame_leaves_the_cross_axis_alone() {
    let mut root = swipe_row(SFSizing::Hug);
    layout(&mut root, SFRect::from_size(400.0, VIEWPORT_HEIGHT));
    assert!(
        (root.frame.height - ROW_HEIGHT).abs() < 0.01,
        "a row of width-only buttons should still be its content's height \
         ({ROW_HEIGHT}), came out {}",
        root.frame.height
    );
}

#[test]
fn filling_buttons_leave_the_row_its_own_height() {
    let mut root = swipe_row(SFSizing::Fill);
    layout(&mut root, SFRect::from_size(400.0, VIEWPORT_HEIGHT));
    assert!(
        (root.frame.height - ROW_HEIGHT).abs() < 0.01,
        "a swiped row should be as tall as its content ({ROW_HEIGHT}), came \
         out {}",
        root.frame.height
    );
}

#[test]
fn filling_buttons_match_the_rows_height() {
    let mut root = swipe_row(SFSizing::Fill);
    layout(&mut root, SFRect::from_size(400.0, VIEWPORT_HEIGHT));
    let buttons = unsafe { &std::slice::from_raw_parts(root.children, root.children_len)[0] };
    assert!(
        (buttons.frame.height - ROW_HEIGHT).abs() < 0.01,
        "the buttons came out {} tall against a {ROW_HEIGHT} row",
        buttons.frame.height
    );
}
