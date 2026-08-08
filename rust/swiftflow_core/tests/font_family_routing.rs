use swiftflow_core::node::SFFontFamily;
use swiftflow_core::{init_glyph_loader, with_glyph_loader};

const HOUSE: char = '\u{e2b6}';

const POWERLINE: char = '\u{e0a0}';

fn setup() {
    init_glyph_loader();
}

fn advance_em(c: char, family: SFFontFamily) -> f32 {
    with_glyph_loader(|l| {
        let g = l.load(c, 400.0, family).expect("glyph should load");
        g.advance / g.units_per_em
    })
}

#[test]
fn a_shared_codepoint_resolves_differently_per_family() {
    setup();
    let as_icon = advance_em(HOUSE, SFFontFamily::Icon);
    let as_text = advance_em(HOUSE, SFFontFamily::Sans);

    assert!(
        (as_icon - 1.0).abs() < 0.01,
        "icon family should advance a full em, got {as_icon}"
    );
    assert!(
        as_text < 0.5,
        "sans family should use Inter's own glyph, got {as_text}"
    );
    assert_ne!(
        as_icon, as_text,
        "the codepoint alone cannot decide which face this is"
    );
}

#[test]
fn monospaced_keeps_its_own_powerline_glyph() {
    setup();

    let mono = advance_em(POWERLINE, SFFontFamily::Monospaced);
    let icon = advance_em(POWERLINE, SFFontFamily::Icon);
    assert!(
        (mono - 0.6).abs() < 0.01,
        "monospaced should advance its uniform 0.6em, got {mono}"
    );
    assert!((icon - 1.0).abs() < 0.01, "icon family advances a full em");
}

#[test]
fn every_ascii_glyph_in_the_monospaced_face_shares_one_advance() {
    setup();

    let reference = advance_em('M', SFFontFamily::Monospaced);

    for c in '!'..='~' {
        let a = advance_em(c, SFFontFamily::Monospaced);
        assert!(
            (a - reference).abs() < 1e-6,
            "{c:?} advances {a}, not {reference} — the face is not monospaced"
        );
    }
    assert!((reference - 0.6).abs() < 0.01);
}

#[test]
fn a_text_family_still_falls_back_to_icons_for_glyphs_it_lacks() {
    setup();

    let unmapped = with_glyph_loader(|l| {
        ('\u{e000}'..='\u{ee82}')
            .find(|c| l.load_uncached(*c, 400.0, SFFontFamily::Sans).is_some_and(|g| {

                (g.advance / g.units_per_em - 1.0).abs() < 0.01
            }))
    });
    assert!(
        unmapped.is_some(),
        "expected at least one PUA codepoint to fall through Inter to the icon face"
    );
}

#[test]
fn an_unbundled_family_renders_as_sans_rather_than_failing() {
    setup();

    assert_eq!(
        advance_em('a', SFFontFamily::Serif),
        advance_em('a', SFFontFamily::Sans)
    );
    assert!(!SFFontFamily::Serif.is_bundled());
    assert!(SFFontFamily::Monospaced.is_bundled());
}
