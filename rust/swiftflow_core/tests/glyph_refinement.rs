use swiftflow_core::{init_font_system, init_glyph_loader, with_font_system};
use swiftflow_core::node::SFFontFamily;

const BUDGET: usize = 8;

fn setup() {

    init_glyph_loader();
    init_font_system();
}

#[test]
fn refinement_is_capped_per_frame() {
    setup();

    with_font_system(|fs| {
        for (i, c) in "abcdefghijklmnopqrst".chars().enumerate() {
            fs.glyph(c, 16.0 + (i as f32) * 2.0, 400.0, SFFontFamily::Sans);
        }
    });

    let refined = with_font_system(|fs| fs.refine(1));
    assert!(
        refined <= BUDGET,
        "refined {refined} glyphs in one frame; the budget is {BUDGET}. \
         Uncapped this is every unconverged glyph on screen, every frame \
         — the launch stall."
    );
}

#[test]
fn refinement_converges_and_the_upload_stays_tight() {
    setup();

    with_font_system(|fs| {
        fs.atlas_dirty_rows();
        fs.glyph('\u{2318}', 19.0, 500.0, SFFontFamily::Sans);

        let Some((y0, y1)) = fs.atlas_dirty_rows() else {
            panic!("rasterizing a glyph should mark rows for upload");
        };
        assert!(y1 > y0, "the range must be non-empty");
        assert!(
            y1 - y0 < 256,
            "one glyph marked {} rows of the 2048-row atlas — the upload \
             is meant to be a tight bound on what changed, not the whole \
             16 MB texture",
            y1 - y0
        );

        assert!(
            fs.atlas_dirty_rows().is_none(),
            "taking the range must clear it, or every later frame \
             re-uploads rows nothing touched"
        );
    });

    let c = '\u{00A7}';
    let size = 21.0;
    with_font_system(|fs| {
        fs.glyph(c, size, 600.0, SFFontFamily::Sans);
    });
    let initial = with_font_system(|fs| fs.sample_count(c, size, 600.0, SFFontFamily::Sans))
        .expect("the glyph should be in the atlas");

    for frame in 0..400 {
        with_font_system(|fs| fs.refine(frame));
    }

    let settled = with_font_system(|fs| fs.sample_count(c, size, 600.0, SFFontFamily::Sans)).unwrap();
    assert!(
        settled > initial,
        "after 400 frames the glyph still has {settled} samples (started \
         at {initial}) — the round-robin cursor isn't advancing, so the \
         same batch is served every frame and the rest never converge"
    );

    let after = with_font_system(|fs| fs.sample_count(c, size, 600.0, SFFontFamily::Sans)).unwrap();
    assert!(
        after >= settled && after <= 128,
        "refinement should stop at the target, not run forever (got {after})"
    );
}
