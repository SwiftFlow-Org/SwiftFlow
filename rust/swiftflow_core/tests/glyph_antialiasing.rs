use swiftflow_core::{init_glyph_loader, rasterize_glyph, with_glyph_loader, GlyphData};
use swiftflow_core::node::SFFontFamily;

const SIZE: f32 = 102.0;
const BOLD: f32 = 700.0;

fn glyph(c: char, weight: f32) -> GlyphData {
    init_glyph_loader();
    with_glyph_loader(|l| l.load(c, weight, SFFontFamily::Sans).cloned()).expect("glyph should load")
}

fn reference(g: &GlyphData) -> (Vec<u8>, u32, u32) {
    rasterize_glyph(g, SIZE, 512, 0)
}

fn edge_coherence(bitmap: &[u8], reference: &[u8], w: u32, h: u32) -> f32 {
    let edge = |i: usize| reference[i] > 5 && reference[i] < 250;
    let err = |i: usize| bitmap[i] as f32 - reference[i] as f32;

    let mut pairs = Vec::new();
    for py in 0..h.saturating_sub(1) {
        for px in 0..w {
            let (i, j) = ((py * w + px) as usize, ((py + 1) * w + px) as usize);
            if edge(i) && edge(j) {
                pairs.push((err(i), err(j)));
            }
        }
    }
    if pairs.len() < 16 {
        return 0.0;
    }
    let n = pairs.len() as f32;
    let (ma, mb) = (
        pairs.iter().map(|p| p.0).sum::<f32>() / n,
        pairs.iter().map(|p| p.1).sum::<f32>() / n,
    );
    let cov = pairs.iter().map(|p| (p.0 - ma) * (p.1 - mb)).sum::<f32>() / n;
    let va = pairs.iter().map(|p| (p.0 - ma).powi(2)).sum::<f32>() / n;
    let vb = pairs.iter().map(|p| (p.1 - mb).powi(2)).sum::<f32>() / n;
    if va <= 0.0 || vb <= 0.0 {
        0.0
    } else {
        cov / (va.sqrt() * vb.sqrt())
    }
}

#[test]
fn diagonal_glyphs_have_incoherent_residuals() {

    for c in ['y', 'v', 'w', 'x', 'k'] {
        let g = glyph(c, BOLD);
        let (reference, w, h) = reference(&g);
        let (bitmap, bw, bh) = rasterize_glyph(&g, SIZE, 32, 0);
        assert_eq!((w, h), (bw, bh));

        let coherence = edge_coherence(&bitmap, &reference, w, h);
        assert!(
            coherence.abs() < 0.35,
            "{c:?} bold: edge error is banded rather than noisy \
             (coherence {coherence:.3}) — this is what a jagged diagonal is"
        );
    }
}

#[test]
fn the_pattern_still_covers_each_pixel_well() {

    for c in ['y', 'n', 'o'] {
        let g = glyph(c, BOLD);
        let (reference, w, h) = reference(&g);
        let (bitmap, _, _) = rasterize_glyph(&g, SIZE, 32, 0);

        let mut worst = 0.0f32;
        let mut sum = 0.0f32;
        let mut count = 0u32;
        for i in 0..(w * h) as usize {
            if reference[i] > 5 && reference[i] < 250 {
                let e = (bitmap[i] as f32 - reference[i] as f32).abs() / 255.0;
                worst = worst.max(e);
                sum += e;
                count += 1;
            }
        }
        let mean = sum / count.max(1) as f32;
        assert!(mean < 0.05, "{c:?}: mean edge error {mean:.4} at 32 samples");
        assert!(worst < 0.20, "{c:?}: worst edge error {worst:.4} at 32 samples");
    }
}

#[test]
fn refinement_converges() {

    let g = glyph('y', BOLD);
    let (reference, w, h) = reference(&g);

    let (first, _, _) = rasterize_glyph(&g, SIZE, 32, 0);
    let mut accumulated: Vec<f32> = first.iter().map(|&v| v as f32).collect();
    let mut total = 32u32;
    for frame in 1..=12u32 {
        let (next, _, _) = rasterize_glyph(&g, SIZE, 8, frame);
        for i in 0..accumulated.len() {
            accumulated[i] =
                (accumulated[i] * total as f32 + next[i] as f32 * 8.0) / (total + 8) as f32;
        }
        total += 8;
    }

    let mut sum = 0.0f32;
    let mut count = 0u32;
    for i in 0..(w * h) as usize {
        if reference[i] > 5 && reference[i] < 250 {
            sum += (accumulated[i] - reference[i] as f32).abs() / 255.0;
            count += 1;
        }
    }
    let mean = sum / count.max(1) as f32;
    assert!(
        mean < 0.02,
        "refined to {total} samples and still {mean:.4} off"
    );
}
