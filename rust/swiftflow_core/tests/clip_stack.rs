use swiftflow_core::draw::{DrawCommand, DrawList, SFClip};
use swiftflow_core::types::*;

fn clip_of(list: &DrawList, index: usize) -> SFClip {
    list.commands[index].clip
}

fn dummy(list: &mut DrawList) {
    list.fill(SFRect::new(0.0, 0.0, 1.0, 1.0), SFColor::WHITE);
}

#[test]
fn commands_outside_any_clip_are_unclipped() {
    let mut list = DrawList::new();
    dummy(&mut list);
    assert!(!clip_of(&list, 0).is_active());
}

#[test]
fn a_command_carries_the_clip_in_force_when_it_was_emitted() {
    let mut list = DrawList::new();
    dummy(&mut list);
    list.push_clip(SFRect::new(10.0, 20.0, 100.0, 200.0));
    dummy(&mut list);
    list.pop_clip();
    dummy(&mut list);

    assert!(!clip_of(&list, 0).is_active());
    assert_eq!(clip_of(&list, 1).rect, SFRect::new(10.0, 20.0, 100.0, 200.0));
    assert!(
        !clip_of(&list, 2).is_active(),
        "popping has to restore the outer clip, not leave the inner one \
         stamped on everything after it"
    );
}

#[test]
fn nested_clips_intersect() {
    let mut list = DrawList::new();
    list.push_clip(SFRect::new(0.0, 0.0, 100.0, 100.0));
    list.push_clip(SFRect::new(50.0, 50.0, 100.0, 100.0));
    dummy(&mut list);

    assert_eq!(clip_of(&list, 0).rect, SFRect::new(50.0, 50.0, 50.0, 50.0));
}

#[test]
fn a_corner_keeps_its_radius_only_where_it_survives() {

    let mut list = DrawList::new();
    list.push_clip_rounded(SFRect::new(0.0, 0.0, 400.0, 800.0), [44.0; 4]);
    list.push_clip(SFRect::new(0.0, 100.0, 400.0, 700.0));
    dummy(&mut list);

    let clip = clip_of(&list, 0);
    assert_eq!(clip.rect, SFRect::new(0.0, 100.0, 400.0, 700.0));
    assert_eq!(
        clip.radii,
        [0.0, 0.0, 44.0, 44.0],
        "the top corners belong to the scroll view and are square; the \
         bottom two are still the screen's own and stay round"
    );
}

#[test]
fn an_inner_clip_that_reaches_no_corner_rounds_nothing() {
    let mut list = DrawList::new();
    list.push_clip_rounded(SFRect::new(0.0, 0.0, 400.0, 800.0), [44.0; 4]);
    list.push_clip(SFRect::new(50.0, 50.0, 100.0, 100.0));
    dummy(&mut list);

    assert_eq!(clip_of(&list, 0).radii, [0.0; 4]);
}

#[test]
fn clearing_drops_the_stack_too() {

    let mut list = DrawList::new();
    list.push_clip(SFRect::new(0.0, 0.0, 10.0, 10.0));
    list.clear();
    dummy(&mut list);
    assert!(!clip_of(&list, 0).is_active());
}

#[test]
fn a_scroll_view_masks_its_children() {
    use swiftflow_core::draw_pass::draw;
    use swiftflow_core::layout::layout;
    use swiftflow_core::node::*;

    let mut child = SFNode::empty();
    child.kind = SFNodeKind::Rect;
    child.set_sizing(SFSizing::Fixed);
    child.fixed_width = 200.0;
    child.fixed_height = 2000.0;
    child.fill = SFColor::WHITE;

    let mut children = vec![child];
    let mut root = SFNode::empty();
    root.kind = SFNodeKind::Scroll;
    root.axis = SFAxis::Vertical;
    root.set_sizing(SFSizing::Fill);
    root.children = children.as_mut_ptr();
    root.children_len = children.len();

    layout(&mut root, SFRect::new(0.0, 0.0, 400.0, 800.0));
    let mut list = DrawList::new();
    draw(&mut root, &mut list);
    drop(children);

    let clipped = list
        .commands
        .iter()
        .find(|i| matches!(i.command, DrawCommand::Rect { .. }))
        .expect("the child rect should be drawn");
    assert_eq!(clipped.clip.rect, SFRect::new(0.0, 0.0, 400.0, 800.0));
}

#[test]
fn an_edge_effect_band_is_masked_by_its_own_layer() {

    use swiftflow_core::draw::DrawCommand;
    use swiftflow_core::draw_pass::draw;
    use swiftflow_core::layout::layout;
    use swiftflow_core::node::*;

    let mut child = SFNode::empty();
    child.kind = SFNodeKind::Rect;
    child.set_sizing(SFSizing::Fill);
    child.fill = SFColor::WHITE;

    let mut children = vec![child];
    let mut root = SFNode::empty();
    root.kind = SFNodeKind::Stack;
    root.axis = SFAxis::Vertical;
    root.set_sizing(SFSizing::Fill);
    root.clip_content = 1.0;
    root.corner_radius = 44.0;

    root.offset_x = 130.0;
    root.edge_effect_height = 96.0;
    root.blur_radius = 50.0;
    root.fill = SFColor::new(0.1, 0.1, 0.1, 1.0);
    root.children = children.as_mut_ptr();
    root.children_len = children.len();

    layout(&mut root, SFRect::new(0.0, 0.0, 400.0, 800.0));
    let mut list = DrawList::new();
    draw(&mut root, &mut list);
    drop(children);

    let band = list
        .commands
        .iter()
        .find(|i| matches!(i.command, DrawCommand::Material { .. }))
        .expect("a node with an edge effect should emit one");

    let DrawCommand::Material { frame, .. } = band.command else {
        unreachable!()
    };
    assert_eq!(
        frame,
        SFRect::new(130.0, 0.0, 400.0, 96.0),
        "the band takes the layer's offset, so it slides with the screen"
    );
    assert_eq!(
        band.clip.radii[0], 44.0,
        "and the layer's top-left corner, so the corner reads during a pop"
    );

    assert!(
        !list
            .commands
            .iter()
            .any(|i| matches!(i.command, DrawCommand::Rect { .. })
                && matches!(i.command, DrawCommand::Rect { fill, .. } if fill.a > 0.0 && fill.r < 0.5)),
        "`fill` is the band's tint here, not a background to flood the layer with"
    );
}
