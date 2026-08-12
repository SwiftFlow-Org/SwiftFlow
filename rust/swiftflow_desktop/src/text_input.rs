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
pub const SF_KEY_SPACE: u32 = 14;

pub const SF_MOD_SHIFT: u32 = 1 << 0;
pub const SF_MOD_CONTROL: u32 = 1 << 1;
pub const SF_MOD_ALT: u32 = 1 << 2;
pub const SF_MOD_SUPER: u32 = 1 << 3;

pub fn map_key(key: &Key) -> u32 {
    println!("[rust] Key: {:?}", key);
    match key {
        Key::Named(named) => {
            return match named {
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
                NamedKey::Space => SF_KEY_SPACE,

                _ => SF_KEY_OTHER,
            }
        }
        Key::Character(smol_str) => {
            println!("Char");
            if let Some(first_char) = smol_str.chars().next() {
                println!("[rust] Code: {:?}", first_char as u32);
                return first_char as u32;
            }
            return SF_KEY_OTHER;
        }
        _ => {
            return SF_KEY_OTHER;
        }
    };
    return SF_KEY_OTHER;
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

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Mutex;

static IME_ALLOWED: AtomicBool = AtomicBool::new(false);
static IME_DIRTY: AtomicBool = AtomicBool::new(false);
static CURSOR_AREA: Mutex<Option<(f32, f32, f32, f32)>> = Mutex::new(None);

static CURSOR_GENERATION: AtomicU32 = AtomicU32::new(0);

pub fn request_ime_allowed(allowed: bool) {
    IME_ALLOWED.store(allowed, Ordering::Relaxed);
    IME_DIRTY.store(true, Ordering::Release);
}

pub fn request_cursor_area(x: f32, y: f32, width: f32, height: f32) {
    if let Ok(mut area) = CURSOR_AREA.lock() {
        *area = Some((x, y, width, height));
    }
    CURSOR_GENERATION.fetch_add(1, Ordering::Release);
}

pub fn take_ime_allowed() -> Option<bool> {
    if IME_DIRTY.swap(false, Ordering::Acquire) {
        Some(IME_ALLOWED.load(Ordering::Relaxed))
    } else {
        None
    }
}

pub fn take_cursor_area(last_seen: &mut u32) -> Option<(f32, f32, f32, f32)> {
    let generation = CURSOR_GENERATION.load(Ordering::Acquire);
    if generation == *last_seen {
        return None;
    }
    *last_seen = generation;
    CURSOR_AREA.lock().ok().and_then(|area| *area)
}

#[cfg(test)]
mod tests {
    use super::*;
    use winit::keyboard::SmolStr;

    #[test]
    fn editing_keys_map_to_their_codes() {
        assert_eq!(map_key(&Key::Named(NamedKey::Backspace)), SF_KEY_BACKSPACE);
        assert_eq!(map_key(&Key::Named(NamedKey::ArrowLeft)), SF_KEY_LEFT);
        assert_eq!(map_key(&Key::Named(NamedKey::PageDown)), SF_KEY_PAGE_DOWN);
        assert_eq!(map_key(&Key::Named(NamedKey::Escape)), SF_KEY_ESCAPE);
    }

    #[test]
    fn printable_keys_are_not_reported() {
        assert_eq!(map_key(&Key::Character(SmolStr::new("a"))), SF_KEY_OTHER);
        assert_eq!(map_key(&Key::Character(SmolStr::new("é"))), SF_KEY_OTHER);
        assert_eq!(map_key(&Key::Named(NamedKey::Space)), SF_KEY_OTHER);

        assert_eq!(map_key(&Key::Named(NamedKey::F1)), SF_KEY_OTHER);
        assert_eq!(map_key(&Key::Named(NamedKey::Shift)), SF_KEY_OTHER);
    }

    #[test]
    fn modifiers_combine_as_a_bitmask() {
        assert_eq!(modifier_mask(ModifiersState::empty()), 0);
        assert_eq!(modifier_mask(ModifiersState::SHIFT), SF_MOD_SHIFT);
        assert_eq!(
            modifier_mask(ModifiersState::SHIFT | ModifiersState::CONTROL),
            SF_MOD_SHIFT | SF_MOD_CONTROL
        );
        assert_eq!(
            modifier_mask(ModifiersState::ALT | ModifiersState::SUPER),
            SF_MOD_ALT | SF_MOD_SUPER
        );
    }

    #[test]
    fn ime_allowed_is_reported_once_per_change() {
        let _ = take_ime_allowed();
        assert_eq!(take_ime_allowed(), None);
        request_ime_allowed(true);
        assert_eq!(take_ime_allowed(), Some(true));
        assert_eq!(take_ime_allowed(), None);
    }

    #[test]
    fn the_caret_area_is_only_read_when_it_moves() {
        let mut seen = CURSOR_GENERATION.load(Ordering::Acquire);
        assert_eq!(take_cursor_area(&mut seen), None);
        request_cursor_area(4.0, 8.0, 2.0, 16.0);
        assert_eq!(take_cursor_area(&mut seen), Some((4.0, 8.0, 2.0, 16.0)));
        assert_eq!(take_cursor_area(&mut seen), None);
    }
}
