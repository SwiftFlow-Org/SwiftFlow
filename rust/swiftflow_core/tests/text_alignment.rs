use swiftflow_core::draw::{DrawCommand, DrawList};
use swiftflow_core::draw_pass::draw;
use swiftflow_core::layout::layout;
use swiftflow_core::node::*;
use swiftflow_core::types::*;
use swiftflow_core::{init_font_system, init_glyph_loader};

const SIZE: f32 = 16.0;
const BOX: f32 = 240.0;

fn setup() {
    init_glyph_loader();
    init_font_system();
}

fn text_node(content: &'static str, align: SFAlignment) -> SFNode {
    let mut n = SFNode::empty();
    n.kind = SFNodeKind::Text;
    n.text = content.as_ptr();
    n.text_len = content.len();
    n.font_size = SIZE;
    n.font_weight = 400.0;
    n.font_family = SFFontFamily::Sans;
    n.color = SFColor {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };
    n.text_align = align;
    n
}

fn line_origins(content: &'static str, align: SFAlignment) -> Vec<f32> {
    setup();
    let mut node = text_node(content, align);

    layout(&mut node, SFRect::from_size(BOX, 400.0));
    node.frame.width = BOX;

    let mut list = DrawList::new();
    draw(&node, &mut list);

    list.commands
        .iter()
        .filter_map(|item| match &item.command {
            DrawCommand::Text { frame, .. } => Some(frame.x),
            _ => None,
        })
        .collect()
}

fn line_widths(content: &'static str) -> Vec<f32> {
    setup();
    let mut node = text_node(content, SFAlignment::Leading);
    layout(&mut node, SFRect::from_size(BOX, 400.0));
    let lines = swiftflow_core::with_font_system(|fs| {
        fs.wrap(content, node.frame.width, SIZE, 400.0, SFFontFamily::Sans, 0)
    });
    swiftflow_core::with_font_system(|fs| {
        lines
            .iter()
            .filter(|l| !l.is_empty())
            .map(|l| fs.measure(l, SIZE, 400.0, SFFontFamily::Sans).0)
            .collect()
    })
}

const PARAGRAPH: &str = "a short one\nand a considerably longer second line";

#[test]
fn leading_starts_every_line_at_the_box_edge() {
    let origins = line_origins(PARAGRAPH, SFAlignment::Leading);
    assert!(origins.len() >= 2, "{origins:?}");
    assert!(
        origins.iter().all(|x| *x == 0.0),
        "leading is the default and must not move anything: {origins:?}"
    );
}

#[test]
fn centre_centres_each_line_not_the_block() {
    let origins = line_origins(PARAGRAPH, SFAlignment::Center);
    let widths = line_widths(PARAGRAPH);
    assert_eq!(origins.len(), widths.len());

    for (x, w) in origins.iter().zip(&widths) {
        assert!(
            (x - (BOX - w) / 2.0).abs() < 0.5,
            "a line {w} wide in a {BOX} box should start at {}, got {x}",
            (BOX - w) / 2.0
        );
    }

    assert!(
        origins[0] > origins[1] + 1.0,
        "the shorter line should be pushed in further: {origins:?} for widths {widths:?}"
    );
}

#[test]
fn trailing_ends_every_line_at_the_box_edge() {
    let origins = line_origins(PARAGRAPH, SFAlignment::Trailing);
    let widths = line_widths(PARAGRAPH);

    for (x, w) in origins.iter().zip(&widths) {
        assert!(
            (x + w - BOX).abs() < 0.5,
            "a line {w} wide should end at {BOX}, started at {x}"
        );
    }
}

#[test]
fn a_line_wider_than_its_box_is_not_pushed_off_the_leading_edge() {

    setup();
    let long = "an unbroken run of text far wider than the box it is given";
    let mut node = text_node(long, SFAlignment::Center);
    node.padding = SFEdgeInsets {
        top: 0.0,
        bottom: 0.0,
        leading: 40.0,
        trailing: 40.0,
    };
    layout(&mut node, SFRect::from_size(BOX, 400.0));
    node.frame.width = 20.0;

    let mut list = DrawList::new();
    draw(&node, &mut list);

    let mut seen = 0;
    for item in &list.commands {
        if let DrawCommand::Text { frame, .. } = &item.command {
            seen += 1;
            assert!(
                frame.x >= node.padding.leading,
                "a line drawn at x = {}, left of its own padded box",
                frame.x
            );
        }
    }
    assert!(seen > 0, "nothing was drawn, so nothing was checked");
}
