use crate::node::{SFContentMode, SFFontFamily};
use crate::types::*;

#[derive(Copy, Clone, Debug, PartialEq, Default)]
pub struct SFShadow {

    pub radius: f32,

    pub opacity: f32,
}

impl SFShadow {
    pub const NONE: Self = Self {
        radius: 0.0,
        opacity: 0.0,
    };

    pub fn is_visible(&self) -> bool {
        self.radius > 0.0 && self.opacity > 0.0
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct SFClip {
    pub rect: SFRect,
    pub radii: [f32; 4],
}

impl SFClip {

    pub const NONE: Self = Self {
        rect: SFRect {
            x: 0.0,
            y: 0.0,
            width: -1.0,
            height: -1.0,
        },
        radii: [0.0; 4],
    };

    pub fn is_active(&self) -> bool {
        self.rect.width >= 0.0 && self.rect.height >= 0.0
    }

    pub fn intersect(self, outer: Self) -> Self {
        if !outer.is_active() {
            return self;
        }
        if !self.is_active() {
            return outer;
        }

        let x0 = self.rect.x.max(outer.rect.x);
        let y0 = self.rect.y.max(outer.rect.y);
        let x1 = (self.rect.x + self.rect.width).min(outer.rect.x + outer.rect.width);
        let y1 = (self.rect.y + self.rect.height).min(outer.rect.y + outer.rect.height);
        let rect = SFRect::new(x0, y0, (x1 - x0).max(0.0), (y1 - y0).max(0.0));

        const EPS: f32 = 0.5;
        let corners = [(x0, y0), (x1, y0), (x1, y1), (x0, y1)];
        let mut radii = [0.0f32; 4];
        for (i, (cx, cy)) in corners.iter().enumerate() {
            let from = |c: &Self| -> f32 {
                let (ox, oy) = match i {
                    0 => (c.rect.x, c.rect.y),
                    1 => (c.rect.x + c.rect.width, c.rect.y),
                    2 => (c.rect.x + c.rect.width, c.rect.y + c.rect.height),
                    _ => (c.rect.x, c.rect.y + c.rect.height),
                };
                if (ox - cx).abs() <= EPS && (oy - cy).abs() <= EPS {
                    c.radii[i]
                } else {
                    0.0
                }
            };
            radii[i] = from(&self).max(from(&outer));
        }

        Self { rect, radii }
    }
}

#[derive(Debug, Clone)]
pub struct DrawItem {
    pub command: DrawCommand,
    pub clip: SFClip,
}

#[derive(Debug, Clone)]
pub enum DrawCommand {

    Rect {
        frame: SFRect,
        corner_radius: f32,
        fill: SFColor,
        border: SFBorder,

        specular: bool,
        shadow: SFShadow,
    },

    Text {
        frame: SFRect,
        content: String,
        font_size: f32,
        render_scale: f32,

        weight: f32,

        family: SFFontFamily,

        blur: f32,
        color: SFColor,
    },

    Fill { frame: SFRect, color: SFColor },

    MergedRect {
        bounds: SFRect,
        blend_k: f32,
        color_blend_k: f32,
        members: [MergedMember; 4],
        count: u8,
    },

    Material {
        frame: SFRect,
        corner_radius: f32,
        border: SFBorder,
        tint: SFColor,
        blur_radius: f32,
        specular: bool,

        progressive: f32,
        progressive_start: f32,

        glass_refraction: f32,

        glass_interactive: f32,
    },

    Image {
        frame: SFRect,
        corner_radius: f32,
        image_id: u32,
        content_mode: SFContentMode,
        tint: SFColor,
    },
}

#[derive(Copy, Clone, Debug)]
pub struct MergedMember {
    pub frame: SFRect,
    pub corner_radius: f32,
    pub fill: SFColor,
    pub border: SFBorder,
    pub specular: bool,
}

impl Default for MergedMember {
    fn default() -> Self {
        Self {
            frame: SFRect::ZERO,
            corner_radius: 0.0,
            fill: SFColor::CLEAR,
            border: SFBorder::NONE,
            specular: false,
        }
    }
}

#[derive(Debug)]
pub struct DrawList {
    pub commands: Vec<DrawItem>,

    clips: Vec<SFClip>,
}

impl DrawList {
    pub fn new() -> Self {
        Self {
            commands: Vec::with_capacity(256),
            clips: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.commands.clear();
        self.clips.clear();
    }

    pub fn current_clip(&self) -> SFClip {
        *self.clips.last().unwrap_or(&SFClip::NONE)
    }

    pub fn push(&mut self, command: DrawCommand) {
        let clip = self.current_clip();
        self.commands.push(DrawItem { command, clip });
    }

    pub fn rect(
        &mut self,
        frame: SFRect,
        corner_radius: f32,
        fill: SFColor,
        border: SFBorder,
        specular: bool,
        shadow: SFShadow,
    ) {
        self.push(DrawCommand::Rect {
            frame,
            corner_radius,
            fill,
            border,
            specular,
            shadow,
        });
    }

    pub fn text(
        &mut self,
        frame: SFRect,
        content: String,
        font_size: f32,
        render_scale: f32,
        weight: f32,
        family: SFFontFamily,
        blur: f32,
        color: SFColor,
    ) {
        self.push(DrawCommand::Text {
            frame,
            content,
            font_size,
            render_scale,
            weight,
            family,
            blur,
            color,
        });
    }

    pub fn fill(&mut self, frame: SFRect, color: SFColor) {
        self.push(DrawCommand::Fill { frame, color });
    }

    pub fn merged_rect(
        &mut self,
        bounds: SFRect,
        blend_k: f32,
        color_blend_k: f32,
        members: [MergedMember; 4],
        count: u8,
    ) {
        self.push(DrawCommand::MergedRect {
            bounds,
            blend_k,
            color_blend_k,
            members,
            count,
        });
    }

    pub fn material(
        &mut self,
        frame: SFRect,
        corner_radius: f32,
        border: SFBorder,
        tint: SFColor,
        blur_radius: f32,
        specular: bool,
        progressive: f32,
        progressive_start: f32,
        glass_refraction: f32,
        glass_interactive: f32,
    ) {
        self.push(DrawCommand::Material {
            frame,
            corner_radius,
            border,
            tint,
            blur_radius,
            specular,
            progressive,
            progressive_start,
            glass_refraction,
            glass_interactive,
        });
    }

    pub fn image(
        &mut self,
        frame: SFRect,
        corner_radius: f32,
        image_id: u32,
        content_mode: SFContentMode,
        tint: SFColor,
    ) {
        self.push(DrawCommand::Image {
            frame,
            corner_radius,
            image_id,
            content_mode,
            tint,
        });
    }

    pub fn push_clip(&mut self, frame: SFRect) {
        self.push_clip_rounded(frame, [0.0; 4]);
    }

    pub fn push_clip_rounded(&mut self, frame: SFRect, radii: [f32; 4]) {
        let clip = SFClip { rect: frame, radii }.intersect(self.current_clip());
        self.clips.push(clip);
    }

    pub fn pop_clip(&mut self) {
        self.clips.pop();
    }
}
