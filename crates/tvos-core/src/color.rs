#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub const fn transparent() -> Self {
        Self { r: 0, g: 0, b: 0, a: 0 }
    }

    pub const fn white() -> Self {
        Self { r: 255, g: 255, b: 255, a: 255 }
    }

    pub const fn black() -> Self {
        Self { r: 0, g: 0, b: 0, a: 255 }
    }

    pub const fn from_hex(hex: u32) -> Self {
        Self {
            r: ((hex >> 16) & 0xFF) as u8,
            g: ((hex >> 8) & 0xFF) as u8,
            b: (hex & 0xFF) as u8,
            a: 255,
        }
    }

    pub const fn from_hexa(hex: u32, a: u8) -> Self {
        Self {
            r: ((hex >> 16) & 0xFF) as u8,
            g: ((hex >> 8) & 0xFF) as u8,
            b: (hex & 0xFF) as u8,
            a,
        }
    }

    #[inline(always)]
    pub fn to_u32_argb(&self) -> u32 {
        ((self.a as u32) << 24) | ((self.r as u32) << 16) | ((self.g as u32) << 8) | (self.b as u32)
    }

    #[inline(always)]
    pub fn to_u32_rgba(&self) -> u32 {
        ((self.r as u32) << 24) | ((self.g as u32) << 16) | ((self.b as u32) << 8) | (self.a as u32)
    }

    #[inline(always)]
    pub fn from_u32_argb(val: u32) -> Self {
        Self {
            a: ((val >> 24) & 0xFF) as u8,
            r: ((val >> 16) & 0xFF) as u8,
            g: ((val >> 8) & 0xFF) as u8,
            b: (val & 0xFF) as u8,
        }
    }

    /// Linear interpolation between two colors
    pub fn lerp(self, other: Color, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        let inv = 1.0 - t;
        Self {
            r: (self.r as f32 * inv + other.r as f32 * t) as u8,
            g: (self.g as f32 * inv + other.g as f32 * t) as u8,
            b: (self.b as f32 * inv + other.b as f32 * t) as u8,
            a: (self.a as f32 * inv + other.a as f32 * t) as u8,
        }
    }

    /// Alpha blend source over destination
    #[inline(always)]
    pub fn blend_over(src: Color, dst: Color) -> Color {
        if src.a == 255 {
            return src;
        }
        if src.a == 0 {
            return dst;
        }
        let alpha = src.a as u32;
        let inv_alpha = 255 - alpha;

        let r = ((src.r as u32 * alpha + dst.r as u32 * inv_alpha) / 255) as u8;
        let g = ((src.g as u32 * alpha + dst.g as u32 * inv_alpha) / 255) as u8;
        let b = ((src.b as u32 * alpha + dst.b as u32 * inv_alpha) / 255) as u8;
        let a = (alpha + (dst.a as u32 * inv_alpha) / 255).min(255) as u8;

        Color { r, g, b, a }
    }
}
