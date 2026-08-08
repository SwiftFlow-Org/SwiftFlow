use swiftflow_core::node::SFFontFamily;
use swiftflow_core::{init_font_system, init_glyph_loader, with_font_system};

const SIZE: f32 = 16.0;
const WEIGHT: f32 = 400.0;

fn setup() {

    init_glyph_loader();
    init_font_system();
}

fn wrap(text: &str, max_width: f32, limit: usize) -> Vec<String> {
    setup();
    with_font_system(|fs| fs.wrap(text, max_width, SIZE, WEIGHT, SFFontFamily::Sans, limit))
}

fn width_of(line: &str) -> f32 {
    setup();
    with_font_system(|fs| fs.measure(line, SIZE, WEIGHT, SFFontFamily::Sans).0)
}

#[test]
fn short_text_stays_on_one_line() {
    let lines = wrap("Hello", 500.0, 0);
    assert_eq!(lines, vec!["Hello"]);
}

#[test]
fn long_text_breaks_at_spaces_and_respects_the_width() {
    let text = "The quick brown fox jumps over the lazy dog again and again";
    let max = 120.0;
    let lines = wrap(text, max, 0);

    assert!(lines.len() > 1, "should have wrapped, got {lines:?}");
    for line in &lines {
        assert!(
            width_of(line) <= max,
            "line {line:?} is {} wide, over the {max} limit",
            width_of(line)
        );
    }

    assert_eq!(
        lines.join(" ").split_whitespace().collect::<Vec<_>>(),
        text.split_whitespace().collect::<Vec<_>>()
    );
}

#[test]
fn explicit_newlines_always_break() {

    let lines = wrap("one\n\ntwo", 500.0, 0);
    assert_eq!(lines, vec!["one", "", "two"]);
}

#[test]
fn a_word_longer_than_the_line_is_split_rather_than_overflowing() {

    let max = 60.0;
    let lines = wrap("supercalifragilisticexpialidocious", max, 0);

    assert!(lines.len() > 1, "should have broken mid-word");
    for line in &lines {
        assert!(width_of(line) <= max, "{line:?} overflows");
    }
    assert_eq!(lines.concat(), "supercalifragilisticexpialidocious");
}

#[test]
fn line_limit_truncates_and_ellipsises() {
    let text = "The quick brown fox jumps over the lazy dog again and again and again";
    let max = 120.0;
    let lines = wrap(text, max, 2);

    assert_eq!(lines.len(), 2);
    assert!(
        lines[1].ends_with('…'),
        "the last kept line should show it was cut: {:?}",
        lines[1]
    );

    assert!(width_of(&lines[1]) <= max, "ellipsised line overflows");
}

#[test]
fn a_line_limit_that_fits_adds_no_ellipsis() {
    let lines = wrap("short", 500.0, 3);
    assert_eq!(lines, vec!["short"]);
}

#[test]
fn unbounded_width_does_not_wrap() {

    let text = "The quick brown fox jumps over the lazy dog";
    assert_eq!(wrap(text, 0.0, 0), vec![text]);
}

#[test]
fn measured_height_grows_by_one_line_box_per_extra_line() {
    setup();
    let text = "The quick brown fox jumps over the lazy dog again and again";
    let lines = wrap(text, 120.0, 0);
    let (_, tall) =
        with_font_system(|fs| fs.measure_wrapped(text, 120.0, SIZE, WEIGHT, SFFontFamily::Sans, 0));
    let (_, short) = with_font_system(|fs| {
        fs.measure_wrapped(text, 5000.0, SIZE, WEIGHT, SFFontFamily::Sans, 0)
    });
    let line_height = with_font_system(|fs| fs.line_height(SIZE, SFFontFamily::Sans));

    assert_eq!(
        wrap(text, 5000.0, 0).len(),
        1,
        "the short case must be one line"
    );
    assert!(lines.len() > 1, "the tall case must have wrapped");
    assert!(tall > short, "wrapped text must measure taller");

    let last = with_font_system(|fs| {
        fs.measure(lines.last().unwrap(), SIZE, WEIGHT, SFFontFamily::Sans)
            .1
    });
    let gaps = (lines.len() - 1) as f32;
    assert!(
        (tall - (last + gaps * line_height)).abs() < 0.01,
        "{} lines should measure {last} + {gaps} × {line_height}, got {tall}",
        lines.len()
    );
    assert!(
        short < line_height,
        "a single line measures cap-to-descender ({short}), not a full \
         baseline-to-baseline step ({line_height})"
    );
}

#[test]
fn the_measured_height_does_not_depend_on_the_characters() {
    setup();
    for family in [SFFontFamily::Sans, SFFontFamily::Monospaced] {
        for size in [11.0f32, SIZE, 28.0, 34.0] {
            let heights: Vec<f32> = ["Hello", "Happy", "HELL", "mn", "Og", "jgpqy", "Settings"]
                .iter()
                .map(|s| with_font_system(|fs| fs.measure(s, size, WEIGHT, family).1))
                .collect();
            let first = heights[0];
            for (text, h) in ["Hello", "Happy", "HELL", "mn", "Og", "jgpqy", "Settings"]
                .iter()
                .zip(&heights)
            {
                assert!(
                    (h - first).abs() < 0.01,
                    "{text:?} measured {h} at {size}pt where \"Hello\" measured {first} \
                     — a parent centring the two would put one lower than the other"
                );
            }
        }
    }
}

#[test]
fn the_box_starts_at_the_cap_line() {
    setup();
    let cap = with_font_system(|fs| {
        fs.glyph('H', SIZE, WEIGHT, SFFontFamily::Sans)
            .map(|g| g.height)
    })
    .expect("the bundled font must have an H");
    let measured = with_font_system(|fs| fs.measure("HELL", SIZE, WEIGHT, SFFontFamily::Sans).1);
    assert!(
        measured >= cap - 0.01,
        "a line measured {measured}, shorter than the {cap} cap height the \
         renderer draws it at — the text would be clipped"
    );
}

#[test]
fn a_descender_fits_inside_the_box() {
    setup();
    let cap = with_font_system(|fs| {
        fs.glyph('H', SIZE, WEIGHT, SFFontFamily::Sans)
            .map(|g| g.height)
    })
    .expect("the bundled font must have an H");

    let ink_below = with_font_system(|fs| {
        fs.glyph('g', SIZE, WEIGHT, SFFontFamily::Sans)
            .map(|g| -g.offset_y)
            .unwrap_or(0.0)
    });
    assert!(ink_below > 0.0, "the bundled font's g must descend");

    let measured = with_font_system(|fs| fs.measure("Og", SIZE, WEIGHT, SFFontFamily::Sans).1);
    assert!(
        measured >= cap + ink_below - 0.01,
        "\"Og\" measured {measured}, but its ink runs from the cap line to \
         {ink_below} below the baseline ({}) — the g would hang out of its own frame",
        cap + ink_below
    );
}

#[test]
fn measured_width_never_exceeds_the_wrap_width() {
    setup();
    let text = "The quick brown fox jumps over the lazy dog again and again";
    let max = 120.0;
    let (width, _) =
        with_font_system(|fs| fs.measure_wrapped(text, max, SIZE, WEIGHT, SFFontFamily::Sans, 0));
    assert!(width <= max, "measured {width} against a {max} limit");
}

#[test]
fn re_wrapping_at_the_measured_width_reproduces_the_same_lines() {
    setup();
    let text = "The quick brown fox jumps over the lazy dog again and again \
                and once more for good measure";

    for max in [80.0_f32, 120.0, 200.0, 340.0] {
        let layout_lines = wrap(text, max, 0);
        let measured_width = with_font_system(|fs| {
            fs.measure_wrapped(text, max, SIZE, WEIGHT, SFFontFamily::Sans, 0)
                .0
        });
        let draw_lines = wrap(text, measured_width, 0);

        assert_eq!(
            layout_lines, draw_lines,
            "re-wrap at measured width {measured_width} (from limit {max}) diverged"
        );
    }
}

#[test]
fn a_leading_indent_survives() {
    assert_eq!(wrap("    foo", 500.0, 0), vec!["    foo"]);
    assert!(width_of_wrapped("    foo", 500.0) > width_of("foo") + 1.0);
}

#[test]
fn a_whitespace_only_token_has_width() {
    let four = width_of_wrapped("    ", 500.0);
    assert!(four > 0.0, "four spaces measured {four} wide");
    let one = width_of_wrapped(" ", 500.0);
    assert!(
        (four - one * 4.0).abs() < 0.5,
        "four spaces came to {four}, one space to {one}"
    );
}

#[test]
fn runs_of_spaces_between_words_are_not_collapsed() {
    assert_eq!(wrap("a    b", 500.0, 0), vec!["a    b"]);
}

#[test]
fn a_wrapped_line_does_not_start_with_the_gap_it_broke_at() {

    let lines = wrap("aaaaaaaa    bbbbbbbb", width_of("aaaaaaaa") + 4.0, 0);
    assert!(lines.len() > 1, "{lines:?}");
    assert!(
        lines[1..].iter().all(|l| !l.starts_with(' ')),
        "{lines:?}"
    );
}

fn width_of_wrapped(text: &str, max: f32) -> f32 {
    setup();
    with_font_system(|fs| {
        fs.measure_wrapped(text, max, SIZE, WEIGHT, SFFontFamily::Sans, 0).0
    })
}
