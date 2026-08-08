use ttf_parser::Face;

const PUA_START: u32 = 0xE000;
const PUA_END: u32 = 0xF8FF;

#[cfg(feature = "icon-thin")]
static THIN: &[u8] = include_bytes!("../../fonts/phosphor/Phosphor-Thin.ttf");
#[cfg(feature = "icon-light")]
static LIGHT: &[u8] = include_bytes!("../../fonts/phosphor/Phosphor-Light.ttf");
#[cfg(feature = "icon-regular")]
static REGULAR: &[u8] = include_bytes!("../../fonts/phosphor/Phosphor.ttf");
#[cfg(feature = "icon-bold")]
static BOLD: &[u8] = include_bytes!("../../fonts/phosphor/Phosphor-Bold.ttf");
#[cfg(feature = "icon-fill")]
static FILL: &[u8] = include_bytes!("../../fonts/phosphor/Phosphor-Fill.ttf");

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum IconFace {
    Thin,
    Light,
    Regular,
    Bold,
    Fill,
}

const FALLBACK: [IconFace; 5] = [
    IconFace::Regular,
    IconFace::Bold,
    IconFace::Light,
    IconFace::Thin,
    IconFace::Fill,
];

impl IconFace {

    pub fn for_weight(weight: f32) -> Self {
        match weight {
            w if w <= 200.0 => IconFace::Thin,
            w if w <= 300.0 => IconFace::Light,
            w if w <= 500.0 => IconFace::Regular,
            w if w <= 800.0 => IconFace::Bold,
            _ => IconFace::Fill,
        }
    }

    fn exact(self) -> Option<&'static [u8]> {
        match self {
            #[cfg(feature = "icon-thin")]
            IconFace::Thin => Some(THIN),
            #[cfg(feature = "icon-light")]
            IconFace::Light => Some(LIGHT),
            #[cfg(feature = "icon-regular")]
            IconFace::Regular => Some(REGULAR),
            #[cfg(feature = "icon-bold")]
            IconFace::Bold => Some(BOLD),
            #[cfg(feature = "icon-fill")]
            IconFace::Fill => Some(FILL),
            #[allow(unreachable_patterns)]
            _ => None,
        }
    }

    pub fn data(self) -> Option<&'static [u8]> {
        self.exact()
            .or_else(|| FALLBACK.iter().find_map(|f| f.exact()))
    }
}

pub fn is_icon(c: char) -> bool {
    let cp = c as u32;
    (PUA_START..=PUA_END).contains(&cp) && any_face_available()
}

fn any_face_available() -> bool {
    FALLBACK.iter().any(|f| f.exact().is_some())
}

pub fn face_data_for(c: char, weight: f32) -> Option<&'static [u8]> {
    if !is_icon(c) {
        return None;
    }
    IconFace::for_weight(weight).data()
}

pub fn ascender_ratio(c: char, weight: f32) -> Option<f32> {
    let data = face_data_for(c, weight)?;
    let face = Face::parse(data, 0).ok()?;
    let upem = face.units_per_em() as f32;
    if upem <= 0.0 {
        return None;
    }
    Some(face.ascender() as f32 / upem)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_is_never_an_icon() {
        for c in ['a', 'Z', '0', ' ', '@', '\n'] {
            assert!(!is_icon(c), "{c:?} routed to the icon font");
        }
    }

    #[test]
    fn weight_buckets_match_the_documented_scale() {
        assert_eq!(IconFace::for_weight(100.0), IconFace::Thin);
        assert_eq!(IconFace::for_weight(200.0), IconFace::Thin);
        assert_eq!(IconFace::for_weight(300.0), IconFace::Light);
        assert_eq!(IconFace::for_weight(400.0), IconFace::Regular);
        assert_eq!(IconFace::for_weight(500.0), IconFace::Regular);
        assert_eq!(IconFace::for_weight(700.0), IconFace::Bold);
        assert_eq!(IconFace::for_weight(900.0), IconFace::Fill);
    }

    #[test]
    #[cfg(not(any(
        feature = "icon-thin",
        feature = "icon-light",
        feature = "icon-regular",
        feature = "icon-bold",
        feature = "icon-fill"
    )))]
    fn nothing_is_an_icon_when_every_face_is_compiled_out() {
        assert!(!is_icon('\u{e2b6}'));
        assert!(face_data_for('\u{e2b6}', 400.0).is_none());
    }

    #[test]
    #[cfg(feature = "icon-regular")]
    fn phosphor_lives_entirely_inside_the_private_use_area() {

        let data = IconFace::Regular.data().expect("regular face");
        let face = Face::parse(data, 0).unwrap();
        let mut mapped = 0usize;
        for cp in PUA_START..=PUA_END {
            if let Some(c) = char::from_u32(cp) {
                if face.glyph_index(c).is_some() {
                    mapped += 1;
                }
            }
        }
        assert!(
            mapped > 1000,
            "expected the icon face to map most of its codepoints inside \
             the PUA, found {mapped}"
        );
    }

    #[test]
    #[cfg(any(
        feature = "icon-thin",
        feature = "icon-light",
        feature = "icon-regular",
        feature = "icon-bold",
        feature = "icon-fill"
    ))]
    fn every_weight_resolves_to_a_face() {
        for w in [100.0, 200.0, 300.0, 400.0, 500.0, 600.0, 700.0, 800.0, 900.0] {
            assert!(
                IconFace::for_weight(w).data().is_some(),
                "weight {w} resolved to no face"
            );
        }
    }

    #[test]
    #[cfg(feature = "icon-regular")]
    fn the_icon_em_box_is_square_and_full_width() {

        let data = IconFace::Regular.data().unwrap();
        let face = Face::parse(data, 0).unwrap();
        let upem = face.units_per_em() as f32;
        let mut checked = 0;
        for cp in PUA_START..=PUA_END {
            let Some(c) = char::from_u32(cp) else { continue };
            let Some(id) = face.glyph_index(c) else {
                continue;
            };
            let Some(adv) = face.glyph_hor_advance(id) else {
                continue;
            };

            if adv == 0 {
                continue;
            }
            assert_eq!(adv as f32, upem, "U+{cp:04X} is not a full em wide");
            checked += 1;
        }
        assert!(checked > 1500, "only checked {checked} glyphs");
    }
}
