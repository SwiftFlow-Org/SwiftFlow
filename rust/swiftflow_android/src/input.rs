pub const LIFECYCLE_FOREGROUND: u32 = 0;
pub const LIFECYCLE_BACKGROUND: u32 = 1;
pub const LIFECYCLE_TERMINATE: u32 = 2;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum PointerAction {
    Down,
    Moved,
    Up,

    Ignore,
}

#[derive(Default, Debug)]
pub struct TouchTracker {
    active: Option<u64>,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Phase {
    Started,
    Moved,
    Ended,
    Cancelled,
}

impl TouchTracker {

    pub fn update(&mut self, id: u64, phase: Phase) -> PointerAction {
        match phase {
            Phase::Started => {
                if self.active.is_some() {
                    return PointerAction::Ignore;
                }
                self.active = Some(id);
                PointerAction::Down
            }
            Phase::Moved => {
                if self.active == Some(id) {
                    PointerAction::Moved
                } else {
                    PointerAction::Ignore
                }
            }

            Phase::Ended | Phase::Cancelled => {
                if self.active == Some(id) {
                    self.active = None;
                    PointerAction::Up
                } else {
                    PointerAction::Ignore
                }
            }
        }
    }

    pub fn reset(&mut self) {
        self.active = None;
    }
}

pub fn frame_delta_seconds(previous_ns: Option<i64>, now_ns: i64, fallback: f32) -> f32 {
    let Some(previous) = previous_ns else {
        return fallback;
    };
    let delta = now_ns - previous;
    if delta <= 0 {
        return fallback;
    }
    let seconds = delta as f64 / 1_000_000_000.0;

    seconds.min(MAX_FRAME_SECONDS as f64) as f32
}

pub const MAX_FRAME_SECONDS: f32 = 1.0 / 15.0;

pub fn frame_interval_seconds(hz: f32) -> f32 {
    if hz.is_finite() && hz >= 20.0 && hz <= 480.0 {
        1.0 / hz
    } else {
        1.0 / 60.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_finger_produces_the_ordinary_sequence() {
        let mut t = TouchTracker::default();
        assert_eq!(t.update(7, Phase::Started), PointerAction::Down);
        assert_eq!(t.update(7, Phase::Moved), PointerAction::Moved);
        assert_eq!(t.update(7, Phase::Ended), PointerAction::Up);

        assert_eq!(t.update(9, Phase::Started), PointerAction::Down);
    }

    #[test]
    fn a_second_finger_is_ignored_entirely() {

        let mut t = TouchTracker::default();
        assert_eq!(t.update(1, Phase::Started), PointerAction::Down);
        assert_eq!(t.update(2, Phase::Started), PointerAction::Ignore);
        assert_eq!(t.update(2, Phase::Moved), PointerAction::Ignore);
        assert_eq!(t.update(2, Phase::Ended), PointerAction::Ignore);

        assert_eq!(t.update(1, Phase::Moved), PointerAction::Moved);
    }

    #[test]
    fn a_cancelled_touch_still_releases_the_gesture() {

        let mut t = TouchTracker::default();
        t.update(3, Phase::Started);
        assert_eq!(t.update(3, Phase::Cancelled), PointerAction::Up);
        assert_eq!(t.update(4, Phase::Started), PointerAction::Down);
    }

    #[test]
    fn a_move_without_a_press_is_not_a_drag() {
        let mut t = TouchTracker::default();
        assert_eq!(t.update(1, Phase::Moved), PointerAction::Ignore);
        assert_eq!(t.update(1, Phase::Ended), PointerAction::Ignore);
    }

    #[test]
    fn losing_the_surface_frees_the_slot() {
        let mut t = TouchTracker::default();
        t.update(1, Phase::Started);
        t.reset();

        assert_eq!(t.update(2, Phase::Started), PointerAction::Down);
    }

    #[test]
    fn frame_delta_is_the_gap_between_timestamps() {

        let dt = frame_delta_seconds(Some(1_000_000_000), 1_008_333_333, 1.0 / 60.0);
        assert!((dt - 0.008_333_333).abs() < 1e-6, "got {dt}");
    }

    #[test]
    fn the_first_frame_uses_the_fallback() {
        assert_eq!(frame_delta_seconds(None, 5_000, 1.0 / 90.0), 1.0 / 90.0);
    }

    #[test]
    fn a_stalled_or_backwards_clock_never_yields_zero() {

        assert_eq!(frame_delta_seconds(Some(100), 100, 0.5), 0.5);
        assert_eq!(frame_delta_seconds(Some(100), 50, 0.5), 0.5);
    }

    #[test]
    fn a_long_pause_is_clamped_rather_than_replayed() {

        let dt = frame_delta_seconds(Some(0), 10_000_000_000, 1.0 / 60.0);
        assert_eq!(dt, MAX_FRAME_SECONDS);
    }

    #[test]
    fn refresh_rates_that_make_no_sense_fall_back_to_sixty() {
        assert_eq!(frame_interval_seconds(120.0), 1.0 / 120.0);
        assert_eq!(frame_interval_seconds(90.0), 1.0 / 90.0);

        assert_eq!(frame_interval_seconds(0.0), 1.0 / 60.0);
        assert_eq!(frame_interval_seconds(f32::NAN), 1.0 / 60.0);
        assert_eq!(frame_interval_seconds(-60.0), 1.0 / 60.0);
    }
}
