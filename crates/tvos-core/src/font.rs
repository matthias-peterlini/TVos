//! Embedded 8x16 font table and scalable text rendering for embedded & bare-metal UI.

pub const FONT_WIDTH: usize = 8;
pub const FONT_HEIGHT: usize = 16;

pub static FONT_8X16: &[u8; 95 * 16] = include_bytes!("font8x16.bin");

#[inline]
pub fn get_glyph_bitmap(c: char) -> &'static [u8] {
    let ascii = c as usize;
    if (32..127).contains(&ascii) {
        let idx = (ascii - 32) * 16;
        &FONT_8X16[idx..idx + 16]
    } else {
        // Fallback placeholder (a filled box or bullet)
        static PLACEHOLDER: [u8; 16] = [
            0x00, 0x00, 0x3C, 0x7E, 0x7E, 0x7E, 0x7E, 0x7E, 0x7E, 0x7E, 0x7E, 0x7E, 0x3C, 0x00, 0x00, 0x00
        ];
        &PLACEHOLDER
    }
}
