use crate::color::Color;

pub const BG_WIDTH: usize = 1280;
pub const BG_HEIGHT: usize = 720;

/// Raw ARGB pixels of the Dolomiti mountain background (resized & darkened)
static BG_DATA: &[u8] = include_bytes!("bg.bin");

/// Blurred variant — used for frosted glass panel backgrounds
static BG_BLUR_DATA: &[u8] = include_bytes!("bg_blur.bin");

#[inline]
pub fn bg_pixel(x: usize, y: usize) -> Color {
    let idx = (y * BG_WIDTH + x) * 4;
    Color {
        a: BG_DATA[idx],
        r: BG_DATA[idx + 1],
        g: BG_DATA[idx + 2],
        b: BG_DATA[idx + 3],
    }
}

#[inline]
pub fn bg_blur_pixel(x: usize, y: usize) -> Color {
    let idx = (y * BG_WIDTH + x) * 4;
    Color {
        a: BG_BLUR_DATA[idx],
        r: BG_BLUR_DATA[idx + 1],
        g: BG_BLUR_DATA[idx + 2],
        b: BG_BLUR_DATA[idx + 3],
    }
}
