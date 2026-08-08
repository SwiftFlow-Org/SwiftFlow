use swiftflow_core::draw::{DrawCommand, DrawItem};
use swiftflow_core::draw_pass::draw;
use swiftflow_core::layout::layout;
use swiftflow_core::node::*;
use swiftflow_core::types::*;
use swiftflow_core::DrawList;

fn image_node(image_id: u32) -> SFNode {
    let mut n = SFNode::empty();
    n.kind = SFNodeKind::Image;
    n.set_sizing(SFSizing::Fixed);
    n.fixed_width = 120.0;
    n.fixed_height = 80.0;
    n.corner_radius = 12.0;
    n.image_id = image_id;
    n.image_content_mode = SFContentMode::Fill;
    n.color = SFColor::WHITE;
    n
}

fn draw_one(mut node: SFNode) -> Vec<DrawItem> {
    layout(&mut node, SFRect::new(0.0, 0.0, 400.0, 800.0));
    let mut list = DrawList::new();
    draw(&mut node, &mut list);
    list.commands
}

#[test]
fn registered_image_emits_a_textured_quad() {
    let commands = draw_one(image_node(7));

    let image = commands
        .iter()
        .find_map(|i| match &i.command {
            DrawCommand::Image {
                frame,
                corner_radius,
                image_id,
                content_mode,
                ..
            } => Some((*frame, *corner_radius, *image_id, *content_mode)),
            _ => None,
        })
        .expect("an Image node with an id should emit DrawCommand::Image");

    let (frame, corner_radius, image_id, content_mode) = image;
    assert_eq!(image_id, 7);
    assert_eq!(content_mode, SFContentMode::Fill);
    assert_eq!(corner_radius, 12.0);
    assert_eq!((frame.width, frame.height), (120.0, 80.0));
}

#[test]
fn unloaded_image_still_occupies_its_frame() {
    let commands = draw_one(image_node(0));

    assert!(
        !commands
            .iter()
            .any(|i| matches!(i.command, DrawCommand::Image { .. })),
        "id 0 means nothing is registered — there is no texture to sample"
    );

    let rect = commands
        .iter()
        .find_map(|i| match &i.command {
            DrawCommand::Rect { frame, .. } => Some(*frame),
            _ => None,
        })
        .expect("the placeholder rect should hold the space instead");
    assert_eq!((rect.width, rect.height), (120.0, 80.0));
}
