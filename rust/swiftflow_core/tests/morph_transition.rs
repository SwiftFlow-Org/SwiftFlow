#[derive(Debug, Clone, Copy)]
struct Morph {
    scale_x: f32,
    scale_y: f32,
    offset_x: f32,
    offset_y: f32,
    corner_delta: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Rect {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

impl Rect {
    fn mid_x(&self) -> f32 {
        self.x + self.w / 2.0
    }
    fn mid_y(&self) -> f32 {
        self.y + self.h / 2.0
    }
}

const SHEET_CORNER_RADIUS: f32 = 28.0;

fn morph(source: Rect, destination: Rect) -> Morph {
    Morph {
        scale_x: source.w / destination.w,
        scale_y: source.h / destination.h,
        corner_delta: source.w.min(source.h) / 2.0 - SHEET_CORNER_RADIUS,
        offset_x: source.mid_x() - destination.mid_x(),
        offset_y: source.mid_y() - destination.mid_y(),
    }
}

fn drawn(destination: Rect, m: Morph) -> Rect {
    let center_x = destination.mid_x();
    let center_y = destination.mid_y();

    let tx = center_x * (1.0 - m.scale_x) + m.offset_x;
    let ty = center_y * (1.0 - m.scale_y) + m.offset_y;
    Rect {
        x: tx + destination.x * m.scale_x,
        y: ty + destination.y * m.scale_y,
        w: destination.w * m.scale_x,
        h: destination.h * m.scale_y,
    }
}

fn close(a: Rect, b: Rect) -> bool {
    (a.x - b.x).abs() < 0.01
        && (a.y - b.y).abs() < 0.01
        && (a.w - b.w).abs() < 0.01
        && (a.h - b.h).abs() < 0.01
}

#[test]
fn the_far_side_lands_exactly_on_the_source() {

    let sheet = Rect {
        x: 0.0,
        y: 99.0,
        w: 393.0,
        h: 753.0,
    };
    let sources = [
        Rect {
            x: 337.0,
            y: 62.0,
            w: 44.0,
            h: 44.0,
        },
        Rect {
            x: 176.0,
            y: 800.0,
            w: 40.0,
            h: 40.0,
        },
        Rect {
            x: 16.0,
            y: 700.0,
            w: 200.0,
            h: 52.0,
        },
        Rect {
            x: 0.0,
            y: 0.0,
            w: 28.0,
            h: 28.0,
        },
        Rect {
            x: 365.0,
            y: 824.0,
            w: 28.0,
            h: 28.0,
        },
    ];

    for source in sources {
        let landed = drawn(sheet, morph(source, sheet));
        assert!(
            close(landed, source),
            "a morph from {source:?} put the sheet at {landed:?}"
        );
    }
}

#[test]
fn it_holds_for_any_destination() {
    let source = Rect {
        x: 337.0,
        y: 62.0,
        w: 44.0,
        h: 44.0,
    };
    for destination in [
        Rect {
            x: 0.0,
            y: 99.0,
            w: 393.0,
            h: 753.0,
        },
        Rect {
            x: 40.0,
            y: 200.0,
            w: 300.0,
            h: 400.0,
        },
        Rect {
            x: 200.0,
            y: 40.0,
            w: 180.0,
            h: 180.0,
        },
        Rect {
            x: 0.0,
            y: 0.0,
            w: 1024.0,
            h: 768.0,
        },
    ] {
        let landed = drawn(destination, morph(source, destination));
        assert!(
            close(landed, source),
            "into {destination:?} the morph landed at {landed:?}, not {source:?}"
        );
    }
}

#[test]
fn the_far_side_carries_the_sources_own_radius() {
    let sheet = Rect {
        x: 0.0,
        y: 99.0,
        w: 393.0,
        h: 753.0,
    };
    for (source, expected) in [

        (
            Rect {
                x: 0.0,
                y: 0.0,
                w: 44.0,
                h: 44.0,
            },
            22.0,
        ),

        (
            Rect {
                x: 0.0,
                y: 0.0,
                w: 200.0,
                h: 52.0,
            },
            26.0,
        ),
    ] {
        let radius = SHEET_CORNER_RADIUS + morph(source, sheet).corner_delta;
        assert!(
            (radius - expected).abs() < 0.01,
            "a {source:?} source morphs from a {radius} radius, expected {expected}"
        );
    }
}

#[test]
fn the_midpoint_stays_between_the_button_and_the_sheet() {
    let sheet = Rect {
        x: 0.0,
        y: 99.0,
        w: 393.0,
        h: 753.0,
    };
    let source = Rect {
        x: 337.0,
        y: 62.0,
        w: 44.0,
        h: 44.0,
    };
    let full = morph(source, sheet);
    let half = Morph {
        scale_x: 1.0 + (full.scale_x - 1.0) * 0.5,
        scale_y: 1.0 + (full.scale_y - 1.0) * 0.5,
        offset_x: full.offset_x * 0.5,
        offset_y: full.offset_y * 0.5,
        corner_delta: full.corner_delta * 0.5,
    };
    let mid = drawn(sheet, half);

    let between = |v: f32, a: f32, b: f32| v >= a.min(b) - 0.01 && v <= a.max(b) + 0.01;
    assert!(
        between(mid.w, source.w, sheet.w) && between(mid.h, source.h, sheet.h),
        "midpoint size {:?}x{:?} is outside the two ends",
        mid.w,
        mid.h
    );
    assert!(
        between(mid.mid_x(), source.mid_x(), sheet.mid_x())
            && between(mid.mid_y(), source.mid_y(), sheet.mid_y()),
        "midpoint centre ({}, {}) swings outside the path between the two",
        mid.mid_x(),
        mid.mid_y()
    );
}

#[test]
fn a_zero_sized_destination_would_produce_infinities() {
    let source = Rect {
        x: 0.0,
        y: 0.0,
        w: 44.0,
        h: 44.0,
    };
    let degenerate = Rect {
        x: 0.0,
        y: 0.0,
        w: 0.0,
        h: 0.0,
    };
    let m = morph(source, degenerate);
    assert!(
        !m.scale_x.is_finite() || !m.scale_y.is_finite(),
        "the guard in Transition.morph is what keeps this out of the \
         interpolator — if the arithmetic became safe on its own, say so \
         there rather than leaving a guard nobody can justify"
    );
}
