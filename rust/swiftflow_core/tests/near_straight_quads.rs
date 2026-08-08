use swiftflow_core::{rasterize_glyph, BezierCurve, GlyphData};

const P0: [f32; 2] = [349.81598, 1118.0];
const P1: [f32; 2] = [443.707, 827.80835];
const P2: [f32; 2] = [537.598, 537.61664];

fn line(from: [f32; 2], to: [f32; 2]) -> BezierCurve {
    BezierCurve {
        p0: from,
        p1: [(from[0] + to[0]) * 0.5, (from[1] + to[1]) * 0.5],
        p2: to,
    }
}

fn wedge() -> GlyphData {
    let bottom_left = [P2[0] - 160.0, P2[1]];
    let top_left = [P0[0] - 160.0, P0[1]];
    GlyphData {
        curves: vec![
            BezierCurve { p0: P0, p1: P1, p2: P2 },
            line(P2, bottom_left),
            line(bottom_left, top_left),
            line(top_left, P0),
        ],
        bbox_min: [bottom_left[0], P2[1]],
        bbox_max: [P2[0], P0[1]],
        advance: 600.0,
        units_per_em: 2048.0,
    }
}

fn right_edge(bitmap: &[u8], w: u32, row: u32) -> Option<f32> {
    let at = |x: u32| bitmap[(row * w + x) as usize] as f32 / 255.0;
    let last_full = (0..w).rev().find(|&x| at(x) > 0.99)?;
    let partial = if last_full + 1 < w { at(last_full + 1) } else { 0.0 };
    Some(last_full as f32 + 0.5 + partial)
}

#[test]
fn a_straight_edge_stored_as_a_quadratic_rasterizes_straight() {
    let g = wedge();
    let (bitmap, w, h) = rasterize_glyph(&g, 102.0, 256, 0);

    let edges: Vec<f32> = (2..h - 2).filter_map(|y| right_edge(&bitmap, w, y)).collect();

    assert!(edges.len() > 15, "only measured {} rows", edges.len());

    let expected = (P2[0] - P0[0]) / (P0[1] - P2[1]);
    for pair in edges.windows(2) {
        let step = pair[1] - pair[0];
        assert!(
            (step - expected).abs() < 0.25,
            "edge moved {step:.3}px in one row, expected about {expected:.3} — \
             this is the staircase: it used to sit still for twenty rows and \
             then jump five pixels"
        );
    }
}

#[test]
fn the_control_point_is_a_hair_off_the_midpoint() {

    let a = P0[1] - 2.0 * P1[1] + P2[1];
    assert_ne!(a, 0.0, "the segment is exactly straight; nothing to test");
    assert!(
        a.abs() < 1e-3,
        "a = {a}, far enough from zero that the naive formula is stable"
    );

    assert!(a.abs() > 1e-6, "the old `a.abs() < 1e-6` guard would have caught this");
}

#[test]
fn exact_straight_lines_are_unaffected() {

    let a = {
        let from = [100.0f32, 1000.0];
        let to = [400.0f32, 200.0];
        let c = line(from, to);
        c.p0[1] - 2.0 * c.p1[1] + c.p2[1]
    };
    assert_eq!(a, 0.0, "a midpoint control should be exact in binary");
}
