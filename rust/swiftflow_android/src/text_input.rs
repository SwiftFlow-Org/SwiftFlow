use winit::keyboard::{Key, ModifiersState, NamedKey};

pub const SF_KEY_OTHER: u32 = 0;
pub const SF_KEY_BACKSPACE: u32 = 1;
pub const SF_KEY_DELETE: u32 = 2;
pub const SF_KEY_ENTER: u32 = 3;
pub const SF_KEY_TAB: u32 = 4;
pub const SF_KEY_ESCAPE: u32 = 5;
pub const SF_KEY_LEFT: u32 = 6;
pub const SF_KEY_RIGHT: u32 = 7;
pub const SF_KEY_UP: u32 = 8;
pub const SF_KEY_DOWN: u32 = 9;
pub const SF_KEY_HOME: u32 = 10;
pub const SF_KEY_END: u32 = 11;
pub const SF_KEY_PAGE_UP: u32 = 12;
pub const SF_KEY_PAGE_DOWN: u32 = 13;

pub const SF_MOD_SHIFT: u32 = 1 << 0;
pub const SF_MOD_CONTROL: u32 = 1 << 1;
pub const SF_MOD_ALT: u32 = 1 << 2;
pub const SF_MOD_SUPER: u32 = 1 << 3;

pub fn map_key(key: &Key) -> u32 {
    let Key::Named(named) = key else {
        return SF_KEY_OTHER;
    };
    match named {
        NamedKey::Backspace => SF_KEY_BACKSPACE,
        NamedKey::Delete => SF_KEY_DELETE,
        NamedKey::Enter => SF_KEY_ENTER,
        NamedKey::Tab => SF_KEY_TAB,
        NamedKey::Escape => SF_KEY_ESCAPE,
        NamedKey::ArrowLeft => SF_KEY_LEFT,
        NamedKey::ArrowRight => SF_KEY_RIGHT,
        NamedKey::ArrowUp => SF_KEY_UP,
        NamedKey::ArrowDown => SF_KEY_DOWN,
        NamedKey::Home => SF_KEY_HOME,
        NamedKey::End => SF_KEY_END,
        NamedKey::PageUp => SF_KEY_PAGE_UP,
        NamedKey::PageDown => SF_KEY_PAGE_DOWN,
        _ => SF_KEY_OTHER,
    }
}

pub fn printable_text<'a>(code: u32, text: Option<&'a str>) -> Option<&'a str> {
    if code != SF_KEY_OTHER {
        return None;
    }
    let text = text?;
    if text.is_empty() || text.chars().any(|c| c.is_control()) {
        return None;
    }
    Some(text)
}

pub fn modifier_mask(state: ModifiersState) -> u32 {
    let mut mask = 0;
    if state.shift_key() {
        mask |= SF_MOD_SHIFT;
    }
    if state.control_key() {
        mask |= SF_MOD_CONTROL;
    }
    if state.alt_key() {
        mask |= SF_MOD_ALT;
    }
    if state.super_key() {
        mask |= SF_MOD_SUPER;
    }
    mask
}

use std::sync::atomic::{AtomicBool, Ordering};

static IME_ALLOWED: AtomicBool = AtomicBool::new(false);
static IME_DIRTY: AtomicBool = AtomicBool::new(false);

pub fn request_ime_allowed(allowed: bool) {
    IME_ALLOWED.store(allowed, Ordering::Relaxed);
    IME_DIRTY.store(true, Ordering::Release);
}

pub fn take_ime_allowed() -> Option<bool> {
    if IME_DIRTY.swap(false, Ordering::Acquire) {
        Some(IME_ALLOWED.load(Ordering::Relaxed))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use winit::keyboard::SmolStr;

    #[test]
    fn editing_keys_map_to_their_codes() {
        assert_eq!(map_key(&Key::Named(NamedKey::Backspace)), SF_KEY_BACKSPACE);
        assert_eq!(map_key(&Key::Named(NamedKey::ArrowUp)), SF_KEY_UP);
        assert_eq!(map_key(&Key::Named(NamedKey::Enter)), SF_KEY_ENTER);
    }

    #[test]
    fn printable_keys_are_not_reported() {
        assert_eq!(map_key(&Key::Character(SmolStr::new("z"))), SF_KEY_OTHER);
        assert_eq!(map_key(&Key::Named(NamedKey::Space)), SF_KEY_OTHER);
    }

    #[test]
    fn the_two_hosts_agree_on_every_code() {

        let table = [
            (SF_KEY_OTHER, 0),
            (SF_KEY_BACKSPACE, 1),
            (SF_KEY_DELETE, 2),
            (SF_KEY_ENTER, 3),
            (SF_KEY_TAB, 4),
            (SF_KEY_ESCAPE, 5),
            (SF_KEY_LEFT, 6),
            (SF_KEY_RIGHT, 7),
            (SF_KEY_UP, 8),
            (SF_KEY_DOWN, 9),
            (SF_KEY_HOME, 10),
            (SF_KEY_END, 11),
            (SF_KEY_PAGE_UP, 12),
            (SF_KEY_PAGE_DOWN, 13),
        ];
        for (actual, expected) in table {
            assert_eq!(actual, expected);
        }
        assert_eq!(
            (SF_MOD_SHIFT, SF_MOD_CONTROL, SF_MOD_ALT, SF_MOD_SUPER),
            (1, 2, 4, 8)
        );
    }

    #[test]
    fn a_printable_key_carries_its_character() {
        assert_eq!(printable_text(SF_KEY_OTHER, Some("z")), Some("z"));
        assert_eq!(printable_text(SF_KEY_OTHER, Some(" ")), Some(" "));

        assert_eq!(printable_text(SF_KEY_OTHER, Some("é")), Some("é"));
    }

    #[test]
    fn a_command_key_never_inserts_its_text() {

        assert_eq!(printable_text(SF_KEY_ENTER, Some("\r")), None);
        assert_eq!(printable_text(SF_KEY_TAB, Some("\t")), None);
        assert_eq!(printable_text(SF_KEY_BACKSPACE, Some("\u{8}")), None);
    }

    #[test]
    fn control_characters_are_not_text() {
        assert_eq!(printable_text(SF_KEY_OTHER, Some("\u{1b}")), None);
        assert_eq!(printable_text(SF_KEY_OTHER, Some("\u{7f}")), None);
        assert_eq!(printable_text(SF_KEY_OTHER, Some("")), None);
        assert_eq!(printable_text(SF_KEY_OTHER, None), None);
    }

    #[test]
    fn ime_allowed_is_reported_once_per_change() {
        let _ = take_ime_allowed();
        request_ime_allowed(true);
        assert_eq!(take_ime_allowed(), Some(true));
        assert_eq!(take_ime_allowed(), None);
    }
}
