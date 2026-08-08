use crate::types::*;

#[repr(u32)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum SFNodeKind {
    Empty = 0,
    Rect = 1,
    Text = 2,
    Stack = 3,
    Spacer = 4,
    Image = 5,
    Scroll = 6,

    Icon = 7,
}

#[repr(u32)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum SFAxis {
    Vertical = 0,
    Horizontal = 1,
    Depth = 2,
}

#[repr(u32)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum SFAlignment {
    Leading = 0,
    Center = 1,
    Trailing = 2,
}

#[repr(u32)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum SFContentMode {

    Fit = 0,

    Fill = 1,

    Stretch = 2,
}

#[repr(u32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub enum SFFontFamily {

    #[default]
    Sans = 0,

    Monospaced = 1,

    Serif = 2,

    Rounded = 3,

    Icon = 4,
}

impl SFFontFamily {
    pub fn from_raw(raw: u32) -> Self {
        match raw {
            1 => SFFontFamily::Monospaced,
            2 => SFFontFamily::Serif,
            3 => SFFontFamily::Rounded,
            4 => SFFontFamily::Icon,
            _ => SFFontFamily::Sans,
        }
    }

    pub fn is_bundled(self) -> bool {
        matches!(
            self,
            SFFontFamily::Sans | SFFontFamily::Monospaced | SFFontFamily::Icon
        )
    }
}

#[repr(u32)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum SFSizing {

    Hug = 0,

    Fill = 1,

    Fixed = 2,
}

#[repr(C)]
pub struct SFNode {
    pub node_id: u32,
    pub kind: SFNodeKind,
    pub frame: SFRect,
    pub padding: SFEdgeInsets,

    pub sizing_x: SFSizing,
    pub sizing_y: SFSizing,

    pub scale: SFScale,

    pub offset_x: f32,
    pub offset_y: f32,

    pub fill: SFColor,
    pub border: SFBorder,
    pub corner_radius: f32,

    pub blur_radius: f32,

    pub specular: f32,

    pub clip_content: f32,

    pub edge_effect_height: f32,

    pub shadow_radius: f32,

    pub shadow_opacity: f32,

    pub no_merge: f32,

    pub progressive_blur: f32,
    pub progressive_start: f32,

    pub glass_refraction: f32,

    pub glass_interactive: f32,

    pub text: *const u8,
    pub text_len: usize,
    pub font_size: f32,

    pub font_weight: f32,

    pub font_family: SFFontFamily,

    pub content_blur: f32,

    pub line_limit: u32,

    pub text_align: SFAlignment,
    pub color: SFColor,

    pub image_id: u32,
    pub image_content_mode: SFContentMode,

    pub axis: SFAxis,

    pub alignment: SFAlignment,
    pub vertical_alignment: SFAlignment,

    pub main_axis_alignment: SFAlignment,
    pub spacing: f32,
    pub children: *mut SFNode,
    pub children_len: usize,

    pub min_length: f32,

    pub fixed_width: f32,
    pub fixed_height: f32,

    pub scroll_id: u32,
    pub content_offset_x: f32,
    pub content_offset_y: f32,
    pub content_width: f32,
    pub content_height: f32,

    pub weight: f32,
}

impl SFNode {

    pub fn set_sizing(&mut self, sizing: SFSizing) {
        self.sizing_x = sizing;
        self.sizing_y = sizing;
    }

    pub fn sizing_on(&self, axis: SFAxis) -> SFSizing {
        match axis {
            SFAxis::Horizontal => self.sizing_x,
            _ => self.sizing_y,
        }
    }

    pub fn empty() -> Self {
        Self {
            node_id: 0,
            kind: SFNodeKind::Empty,
            frame: SFRect::ZERO,
            padding: SFEdgeInsets::ZERO,
            sizing_x: SFSizing::Hug,
            sizing_y: SFSizing::Hug,
            scale: SFScale::DEFAULT,
            offset_x: 0.0,
            offset_y: 0.0,
            fill: SFColor::CLEAR,
            border: SFBorder::NONE,
            corner_radius: 0.0,
            blur_radius: 0.0,
            specular: 0.0,
            clip_content: 0.0,
            edge_effect_height: 0.0,
            shadow_radius: 0.0,
            shadow_opacity: 0.0,
            no_merge: 0.0,
            progressive_blur: 0.0,
            progressive_start: 0.5,
            glass_refraction: 0.0,
            glass_interactive: 0.0,
            text: std::ptr::null(),
            text_len: 0,
            font_size: 16.0,
            font_weight: 400.0,
            font_family: SFFontFamily::Sans,
            content_blur: 0.0,
            line_limit: 0,
            text_align: SFAlignment::Leading,
            color: SFColor::WHITE,
            image_id: 0,
            image_content_mode: SFContentMode::Fit,
            axis: SFAxis::Vertical,
            alignment: SFAlignment::Center,
            vertical_alignment: SFAlignment::Center,
            main_axis_alignment: SFAlignment::Center,
            spacing: 0.0,
            children: std::ptr::null_mut(),
            children_len: 0,
            min_length: 0.0,
            fixed_width: 0.0,
            fixed_height: 0.0,
            scroll_id: 0,
            content_offset_x: 0.0,
            content_offset_y: 0.0,
            content_width: 0.0,
            content_height: 0.0,
            weight: 0.0,
        }
    }

    pub fn rect(fill: SFColor, corner_radius: f32) -> Self {
        let mut n = Self::empty();
        n.kind = SFNodeKind::Rect;
        n.fill = fill;
        n.corner_radius = corner_radius;
        n.set_sizing(SFSizing::Fill);
        n
    }

    pub fn stack(axis: SFAxis, spacing: f32) -> Self {
        let mut n = Self::empty();
        n.kind = SFNodeKind::Stack;
        n.axis = axis;
        n.spacing = spacing;
        n.set_sizing(SFSizing::Hug);
        n
    }

    pub fn spacer(min_length: f32) -> Self {
        let mut n = Self::empty();
        n.kind = SFNodeKind::Spacer;
        n.min_length = min_length;
        n.set_sizing(SFSizing::Fill);
        n
    }

    pub fn text(ptr: *const u8, len: usize, font_size: f32, color: SFColor) -> Self {
        let mut n = Self::empty();
        n.kind = SFNodeKind::Text;
        n.text = ptr;
        n.text_len = len;
        n.font_size = font_size;
        n.color = color;
        n.set_sizing(SFSizing::Hug);
        n
    }
}

unsafe impl Send for SFNode {}
unsafe impl Sync for SFNode {}

#[no_mangle]
pub extern "C" fn sf_create_empty_root() -> *mut SFNode {
    Box::into_raw(Box::new(SFNode::empty()))
}
