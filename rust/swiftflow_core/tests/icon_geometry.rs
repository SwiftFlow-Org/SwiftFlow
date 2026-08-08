#![cfg(feature = "icons")]

use swiftflow_core::layout::layout;
use swiftflow_core::node::SFFontFamily;
use swiftflow_core::node::*;
use swiftflow_core::types::*;
use swiftflow_core::{init_font_system, init_glyph_loader, is_icon, with_font_system};

const HOUSE: char = '\u{e2b6}';

const GEAR: char = '\u{e270}';

fn icon_node(c: char, size: f32) -> SFNode {
    let mut n = SFNode::empty();
    n.kind = SFNodeKind::Icon;
    n.font_size = size;
    n.font_weight = 400.0;

    let bytes = c.to_string().into_bytes().into_boxed_slice();
    n.text_len = bytes.len();
    n.text = Box::into_raw(bytes) as *const u8;
    n
}

fn laid_out(c: char, size: f32) -> SFRect {
    let mut node = icon_node(c, size);
    layout(&mut node, SFRect::new(0.0, 0.0, 1000.0, 1000.0));
    node.frame
}

#[test]
fn an_icon_is_a_square_the_size_of_its_font() {
    let frame = laid_out(HOUSE, 24.0);
    assert_eq!(frame.width, 24.0, "width should be the em box");
    assert_eq!(frame.height, 24.0, "height should be the em box");
}

#[test]
fn every_icon_at_one_size_lays_out_identically() {

    let house = laid_out(HOUSE, 24.0);
    let gear = laid_out(GEAR, 24.0);
    assert_eq!((house.width, house.height), (gear.width, gear.height));
}

#[test]
fn padding_grows_the_box_around_the_em_square() {
    let mut node = icon_node(HOUSE, 24.0);
    node.padding = SFEdgeInsets {
        top: 2.0,
        bottom: 3.0,
        leading: 4.0,
        trailing: 5.0,
    };
    layout(&mut node, SFRect::new(0.0, 0.0, 1000.0, 1000.0));
    assert_eq!(node.frame.width, 24.0 + 4.0 + 5.0);
    assert_eq!(node.frame.height, 24.0 + 2.0 + 3.0);
    assert_eq!(node.frame.x, 4.0);
    assert_eq!(node.frame.y, 2.0);
}

#[test]
fn the_ink_sits_centred_in_the_box() {
    init_glyph_loader();
    init_font_system();

    const SIZE: f32 = 64.0;
    for c in [HOUSE, GEAR] {
        assert!(is_icon(c), "{c:?} did not route to an icon face");

        let (ascender, glyph) = with_font_system(|fs| {
            (
                fs.ascender_for(c, SIZE, 400.0, SFFontFamily::Icon),
                fs.glyph(c, SIZE, 400.0, SFFontFamily::Icon).expect("icon should rasterize"),
            )
        });

        let ink_top = ascender - glyph.ascender;
        let ink_bottom = ascender - glyph.offset_y;

        assert!(ink_top >= 0.0, "{c:?} ink starts above its box: {ink_top}");
        assert!(
            ink_bottom <= SIZE,
            "{c:?} ink runs past the bottom of its box: {ink_bottom} > {SIZE}"
        );

        let above = ink_top;
        let below = SIZE - ink_bottom;
        assert!(
            (above - below).abs() <= 1.0,
            "{c:?} is off centre: {above:.2}px above, {below:.2}px below"
        );
    }
}

#[test]
fn an_icon_advances_the_full_box_width() {
    init_glyph_loader();
    init_font_system();

    const SIZE: f32 = 64.0;

    let (house, gear) = with_font_system(|fs| {
        (
            fs.glyph(HOUSE, SIZE, 400.0, SFFontFamily::Icon).unwrap(),
            fs.glyph(GEAR, SIZE, 400.0, SFFontFamily::Icon).unwrap(),
        )
    });
    for (name, g) in [("house", house), ("gear", gear)] {
        assert!(
            (g.advance - SIZE).abs() <= 0.5,
            "{name} advances {} at size {SIZE}",
            g.advance
        );
        let left = g.offset_x;
        let right = SIZE - (g.offset_x + g.width);
        assert!(
            (left - right).abs() <= 1.0,
            "{name} is off centre horizontally: {left:.2} left, {right:.2} right"
        );
    }
}

#[test]
fn weight_selects_a_different_face_rather_than_a_variation() {
    init_glyph_loader();
    init_font_system();

    const SIZE: f32 = 64.0;
    let thin = with_font_system(|fs| fs.glyph(HOUSE, SIZE, 100.0, SFFontFamily::Icon)).expect("thin");
    let fill = with_font_system(|fs| fs.glyph(HOUSE, SIZE, 900.0, SFFontFamily::Icon)).expect("fill");

    assert!(
        (thin.width - fill.width).abs() > 0.5 || (thin.height - fill.height).abs() > 0.5,
        "thin and fill rasterized to the same ink box \
         ({}x{} vs {}x{}) — the weight never reached the face",
        thin.width,
        thin.height,
        fill.width,
        fill.height
    );
}
