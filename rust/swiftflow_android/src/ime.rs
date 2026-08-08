#[derive(Debug, Default, Clone, PartialEq)]
pub struct ImeState {
    pub text: String,

    pub compose: Option<(usize, usize)>,
}

#[derive(Default)]
pub struct ImeBridge {
    last_preedit: String,

    enabled: bool,
}

#[derive(Debug, Default, PartialEq)]
pub struct ImeUpdate {

    pub commit: Option<String>,

    pub preedit: Option<String>,

    pub reset_buffer: bool,
}

impl ImeBridge {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_enabled(&mut self, enabled: bool) -> ImeUpdate {
        self.enabled = enabled;
        if enabled {
            return ImeUpdate::default();
        }

        let had = !self.last_preedit.is_empty();
        self.last_preedit.clear();
        ImeUpdate {
            commit: None,
            preedit: had.then(String::new),
            reset_buffer: true,
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn poll(&mut self, state: &ImeState) -> ImeUpdate {
        if !self.enabled {
            return ImeUpdate::default();
        }

        match state.compose {
            Some((start, end)) => {

                let composing = slice_on_boundaries(&state.text, start, end);
                if composing == self.last_preedit {
                    return ImeUpdate::default();
                }
                self.last_preedit = composing.to_string();
                ImeUpdate {
                    commit: None,
                    preedit: Some(composing.to_string()),
                    reset_buffer: false,
                }
            }
            None if !state.text.is_empty() => {

                let had_preedit = !self.last_preedit.is_empty();
                self.last_preedit.clear();
                ImeUpdate {
                    commit: Some(state.text.clone()),

                    preedit: had_preedit.then(String::new),
                    reset_buffer: true,
                }
            }
            None => {

                if self.last_preedit.is_empty() {
                    return ImeUpdate::default();
                }
                self.last_preedit.clear();
                ImeUpdate {
                    commit: None,
                    preedit: Some(String::new()),
                    reset_buffer: false,
                }
            }
        }
    }
}

fn slice_on_boundaries(text: &str, start: usize, end: usize) -> &str {
    let start = floor_boundary(text, start.min(text.len()));
    let end = ceil_boundary(text, end.min(text.len()));
    if start >= end {
        return "";
    }
    &text[start..end]
}

fn floor_boundary(text: &str, mut index: usize) -> usize {
    while index > 0 && !text.is_char_boundary(index) {
        index -= 1;
    }
    index
}

fn ceil_boundary(text: &str, mut index: usize) -> usize {
    while index < text.len() && !text.is_char_boundary(index) {
        index += 1;
    }
    index
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(text: &str, compose: Option<(usize, usize)>) -> ImeState {
        ImeState { text: text.to_string(), compose }
    }

    fn enabled() -> ImeBridge {
        let mut bridge = ImeBridge::new();
        bridge.set_enabled(true);
        bridge
    }

    #[test]
    fn nothing_focused_means_nothing_happens() {
        let mut bridge = ImeBridge::new();
        assert_eq!(bridge.poll(&state("hello", None)), ImeUpdate::default());
    }

    #[test]
    fn a_settled_buffer_commits_and_resets() {
        let mut bridge = enabled();
        let update = bridge.poll(&state("hi", None));
        assert_eq!(update.commit.as_deref(), Some("hi"));
        assert!(update.reset_buffer, "the buffer must be emptied or it commits again");
    }

    #[test]
    fn a_composition_is_a_preedit_and_never_resets() {
        let mut bridge = enabled();
        let update = bridge.poll(&state("にほん", Some((0, 9))));
        assert_eq!(update.preedit.as_deref(), Some("にほん"));
        assert_eq!(update.commit, None);
        assert!(
            !update.reset_buffer,
            "resetting mid-composition cancels the composition"
        );
    }

    #[test]
    fn an_unchanged_composition_says_nothing() {
        let mut bridge = enabled();
        bridge.poll(&state("にほ", Some((0, 6))));
        assert_eq!(bridge.poll(&state("にほ", Some((0, 6)))), ImeUpdate::default());
    }

    #[test]
    fn committing_a_composition_clears_the_preedit_too() {
        let mut bridge = enabled();
        bridge.poll(&state("にほん", Some((0, 9))));
        let update = bridge.poll(&state("日本", None));
        assert_eq!(update.commit.as_deref(), Some("日本"));
        assert_eq!(
            update.preedit.as_deref(),
            Some(""),
            "the composition must stop being drawn once it is real text"
        );
        assert!(update.reset_buffer);
    }

    #[test]
    fn a_cancelled_composition_clears_without_committing() {
        let mut bridge = enabled();
        bridge.poll(&state("にほ", Some((0, 6))));
        let update = bridge.poll(&state("", None));
        assert_eq!(update.commit, None);
        assert_eq!(update.preedit.as_deref(), Some(""));
    }

    #[test]
    fn losing_focus_discards_the_composition() {
        let mut bridge = enabled();
        bridge.poll(&state("にほ", Some((0, 6))));
        let update = bridge.set_enabled(false);
        assert_eq!(update.commit, None, "an abandoned composition is not text the user asked for");
        assert_eq!(update.preedit.as_deref(), Some(""));
        assert!(update.reset_buffer);
    }

    #[test]
    fn the_resting_state_is_free() {
        let mut bridge = enabled();
        for _ in 0..10 {
            assert_eq!(bridge.poll(&state("", None)), ImeUpdate::default());
        }
    }

    #[test]
    fn a_region_splitting_a_character_does_not_panic() {

        let text = "にほん";
        assert_eq!(slice_on_boundaries(text, 1, 8), "にほん");
        assert_eq!(slice_on_boundaries(text, 0, 4), "にほ");
        assert_eq!(slice_on_boundaries(text, 2, 3), "に");
    }

    #[test]
    fn out_of_range_indices_are_clamped_rather_than_trapping() {

        let text = "ab";
        assert_eq!(slice_on_boundaries(text, 0, 99), "ab");
        assert_eq!(slice_on_boundaries(text, 99, 99), "");
        assert_eq!(slice_on_boundaries(text, 2, 1), "");
    }

    #[test]
    fn an_emoji_region_survives_a_mid_character_bound() {

        let text = "a🙂b";
        assert_eq!(slice_on_boundaries(text, 1, 5), "🙂");
        assert_eq!(slice_on_boundaries(text, 2, 4), "🙂");
    }

    #[test]
    fn a_composition_polled_repeatedly_grows_without_committing() {

        let mut bridge = enabled();
        let mut commits = 0;
        for text in ["に", "にほ", "にほん"] {
            let update = bridge.poll(&state(text, Some((0, text.len()))));
            assert_eq!(update.preedit.as_deref(), Some(text));
            if update.commit.is_some() {
                commits += 1;
            }
        }
        assert_eq!(commits, 0);
        assert_eq!(bridge.poll(&state("日本", None)).commit.as_deref(), Some("日本"));
    }
}
