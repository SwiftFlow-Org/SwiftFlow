#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct SFScale {
    pub x: f32,
    pub y: f32,
}

impl SFScale {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };
    pub const DEFAULT: Self = Self { x: 1.0, y: 1.0 };
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct SFColor {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl SFColor {
    pub const CLEAR: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.0,
    };
    pub const BLACK: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    pub const WHITE: Self = Self {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };

    pub const SURFACE: Self = Self {
        r: 0.090,
        g: 0.090,
        b: 0.090,
        a: 1.0,
    };
    pub const BORDER: Self = Self {
        r: 0.231,
        g: 0.231,
        b: 0.231,
        a: 1.0,
    };

    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub fn from_hex(hex: u32) -> Self {
        Self {
            r: ((hex >> 16) & 0xFF) as f32 / 255.0,
            g: ((hex >> 8) & 0xFF) as f32 / 255.0,
            b: ((hex) & 0xFF) as f32 / 255.0,
            a: 1.0,
        }
    }

    pub fn with_alpha(mut self, a: f32) -> Self {
        self.a = a;
        self
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct SFPoint {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct SFRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl SFRect {
    pub const ZERO: Self = Self {
        x: 0.0,
        y: 0.0,
        width: 0.0,
        height: 0.0,
    };

    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn from_size(width: f32, height: f32) -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width,
            height,
        }
    }

    pub fn center(&self) -> (f32, f32) {
        (self.x + self.width * 0.5, self.y + self.height * 0.5)
    }

    pub fn inset(&self, insets: SFEdgeInsets) -> Self {
        Self {
            x: self.x + insets.leading,
            y: self.y + insets.top,
            width: self.width - insets.leading - insets.trailing,
            height: self.height - insets.top - insets.bottom,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct SFEdgeInsets {
    pub top: f32,
    pub bottom: f32,
    pub leading: f32,
    pub trailing: f32,
}

impl SFEdgeInsets {
    pub const ZERO: Self = Self {
        top: 0.0,
        bottom: 0.0,
        leading: 0.0,
        trailing: 0.0,
    };

    pub fn all(value: f32) -> Self {
        Self {
            top: value,
            bottom: value,
            leading: value,
            trailing: value,
        }
    }

    pub fn horizontal(value: f32) -> Self {
        Self {
            top: 0.0,
            bottom: 0.0,
            leading: value,
            trailing: value,
        }
    }

    pub fn vertical(value: f32) -> Self {
        Self {
            top: value,
            bottom: value,
            leading: 0.0,
            trailing: 0.0,
        }
    }

    pub fn new(top: f32, bottom: f32, leading: f32, trailing: f32) -> Self {
        Self {
            top,
            bottom,
            leading,
            trailing,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct SFBorder {
    pub color: SFColor,
    pub width: f32,
    _pad: [f32; 3],
}

impl SFBorder {
    pub const NONE: Self = Self {
        color: SFColor::CLEAR,
        width: 0.0,
        _pad: [0.0; 3],
    };

    pub fn new(color: SFColor, width: f32) -> Self {
        Self {
            color,
            width,
            _pad: [0.0; 3],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct SFSize {
    pub width: f32,
    pub height: f32,
}

impl SFSize {
    pub const ZERO: Self = Self {
        width: 0.0,
        height: 0.0,
    };

    pub fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
}
