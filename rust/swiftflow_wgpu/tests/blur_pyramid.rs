use swiftflow_wgpu::pyramid_dims;

fn bilinear(p: (f32, f32)) -> Vec<((i32, i32), f32)> {
    let axis = |v: f32| {
        let base = (v - 0.5).floor();
        let frac = (v - 0.5) - base;
        [(base as i32, 1.0 - frac), (base as i32 + 1, frac)]
    };
    let mut out = Vec::new();
    for (x, wx) in axis(p.0) {
        for (y, wy) in axis(p.1) {
            if wx * wy > 0.0 {
                out.push(((x, y), wx * wy));
            }
        }
    }
    out
}

fn accumulate(into: &mut Vec<((i32, i32), f32)>, texel: (i32, i32), weight: f32) {
    match into.iter_mut().find(|(t, _)| *t == texel) {
        Some((_, w)) => *w += weight,
        None => into.push((texel, weight)),
    }
}

fn halve_kernel(dst: (i32, i32)) -> Vec<((i32, i32), f32)> {
    let centre = ((2 * dst.0) as f32 + 1.0, (2 * dst.1) as f32 + 1.0);
    let taps = [
        (centre, 4.0),
        ((centre.0 - 1.0, centre.1 - 1.0), 1.0),
        ((centre.0 + 1.0, centre.1 - 1.0), 1.0),
        ((centre.0 - 1.0, centre.1 + 1.0), 1.0),
        ((centre.0 + 1.0, centre.1 + 1.0), 1.0),
    ];
    let mut out = Vec::new();
    for (p, tap_weight) in taps {
        for (texel, w) in bilinear(p) {
            accumulate(&mut out, texel, w * tap_weight / 8.0);
        }
    }
    out
}

fn old_downsample_kernel(dst: (i32, i32)) -> Vec<((i32, i32), f32)> {
    let centre = ((8 * dst.0) as f32 + 4.0, (8 * dst.1) as f32 + 4.0);
    let o = 8.0 / 3.0;
    let taps = [
        ((0.0, 0.0), 4.0),
        ((-o, 0.0), 2.0),
        ((o, 0.0), 2.0),
        ((0.0, -o), 2.0),
        ((0.0, o), 2.0),
        ((-o, -o), 1.0),
        ((o, -o), 1.0),
        ((-o, o), 1.0),
        ((o, o), 1.0),
    ];
    let mut out = Vec::new();
    for ((dx, dy), tap_weight) in taps {
        for (texel, w) in bilinear((centre.0 + dx, centre.1 + dy)) {
            accumulate(&mut out, texel, w * tap_weight / 16.0);
        }
    }
    out
}

fn through_chain(dst: (i32, i32), levels: usize) -> Vec<((i32, i32), f32)> {
    let mut current = vec![(dst, 1.0f32)];
    for _ in 0..levels {
        let mut next = Vec::new();
        for (texel, weight) in current {
            for (src, w) in halve_kernel(texel) {
                accumulate(&mut next, src, w * weight);
            }
        }
        current = next;
    }
    current
}

fn total(weights: &[((i32, i32), f32)]) -> f32 {
    weights.iter().map(|(_, w)| w).sum()
}

#[test]
fn every_level_is_half_the_one_above_it() {
    for (w, h) in [(1170u32, 2532u32), (2048, 1536), (393, 852), (800, 600)] {
        let levels = pyramid_dims(w, h);
        let mut prev = (w, h);
        for (i, level) in levels.iter().enumerate() {
            assert_eq!(
                *level,
                ((prev.0 / 2).max(1), (prev.1 / 2).max(1)),
                "level {i} of {w}x{h} is not a halving of {prev:?}"
            );
            prev = *level;
        }
    }
}

#[test]
fn no_level_ever_collapses_to_zero() {
    for (w, h) in [(1u32, 1u32), (3, 7), (0, 0), (5, 1)] {
        for (lw, lh) in pyramid_dims(w, h) {
            assert!(lw >= 1 && lh >= 1, "{w}x{h} produced a {lw}x{lh} level");
        }
    }
}

#[test]
fn the_centre_tap_is_an_exact_box_average() {
    let taps = bilinear((2.0 * 3.0 + 1.0, 2.0 * 5.0 + 1.0));
    assert_eq!(taps.len(), 4, "a corner fetch should touch four texels");
    for (texel, w) in &taps {
        assert!(
            (w - 0.25).abs() < 1e-6,
            "texel {texel:?} got weight {w}, not a quarter"
        );
    }
}

#[test]
fn a_halving_reads_every_texel_it_collapses() {
    let weights = halve_kernel((3, 5));
    assert!(
        (total(&weights) - 1.0).abs() < 1e-5,
        "the kernel does not sum to 1: {}",
        total(&weights)
    );
    for x in 6..8 {
        for y in 10..12 {
            let w = weights
                .iter()
                .find(|(t, _)| *t == (x, y))
                .map(|(_, w)| *w)
                .unwrap_or(0.0);
            assert!(w > 0.0, "texel ({x},{y}) is discarded unread");
        }
    }
}

#[test]
fn the_chain_reads_all_sixty_four_texels_behind_a_blur_texel() {
    let dst = (4, 4);
    let weights = through_chain(dst, 3);
    assert!(
        (total(&weights) - 1.0).abs() < 1e-4,
        "the chain does not preserve energy: {}",
        total(&weights)
    );

    let mut unread = Vec::new();
    for x in (dst.0 * 8)..(dst.0 * 8 + 8) {
        for y in (dst.1 * 8)..(dst.1 * 8 + 8) {
            let seen = weights.iter().any(|(t, w)| *t == (x, y) && *w > 0.0);
            if !seen {
                unread.push((x, y));
            }
        }
    }
    assert!(
        unread.is_empty(),
        "{} of 64 source texels never reach the blur: {unread:?}",
        unread.len()
    );
}

#[test]
fn the_chain_weights_evenly_where_the_single_jump_did_not() {
    let dst = (4, 4);
    let profile = |weights: Vec<((i32, i32), f32)>| {
        let inside: Vec<f32> = (0..8)
            .flat_map(|x| (0..8).map(move |y| (dst.0 * 8 + x, dst.1 * 8 + y)))
            .map(|texel| {
                weights
                    .iter()
                    .find(|(t, _)| *t == texel)
                    .map(|(_, w)| *w)
                    .unwrap_or(0.0)
            })
            .collect();
        let unread = inside.iter().filter(|w| **w <= 0.0).count();
        let heaviest = inside.iter().copied().fold(0.0f32, f32::max);
        let lightest = inside
            .iter()
            .copied()
            .filter(|w| *w > 0.0)
            .fold(f32::MAX, f32::min);
        (unread, heaviest / lightest)
    };

    let (old_unread, old_spread) = profile(old_downsample_kernel(dst));
    assert!(
        old_unread >= 25 && old_spread > 20.0,
        "the single jump now leaves {old_unread} unread at a {old_spread:.1}:1 \
         spread — this test has stopped describing the bug it documents"
    );

    let (new_unread, new_spread) = profile(through_chain(dst, 3));
    assert_eq!(new_unread, 0, "the chain left texels unread");
    assert!(
        new_spread < 2.0,
        "the chain weights the footprint {new_spread:.1}:1, which is no longer \
         close enough to a box filter to be trusted not to alias"
    );
}
