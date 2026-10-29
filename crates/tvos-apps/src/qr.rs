//! QR codes on screen: the phone opens the remote or the app installer by pointing its camera at the TV.

use qrcodegen::{QrCode, QrCodeEcc};
use tvos_core::color::Color;
use tvos_core::surface::Surface;

/// Draws `text` as a QR code on a white rounded card `size` pixels wide, top-left at (x, y).
pub fn draw(s: &mut Surface, x: i32, y: i32, size: i32, text: &str) {
    s.fill_rrect(x, y, size as u32, size as u32, size / 28, Color::white());
    let Ok(qr) = QrCode::encode_text(text, QrCodeEcc::Medium) else { return };
    let n = qr.size();
    // Whole pixels per module keep the code sharp; at least 2 modules of white border
    let cell = (size / (n + 4)).max(1);
    let off = (size - cell * n) / 2;
    let dark = Color::rgb(18, 18, 24);
    for my in 0..n {
        for mx in 0..n {
            if qr.get_module(mx, my) {
                s.fill_rect(x + off + mx * cell, y + off + my * cell, cell as u32, cell as u32, dark);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn encodes_remote_url() {
        let qr = qrcodegen::QrCode::encode_text("http://192.168.1.23:8080", qrcodegen::QrCodeEcc::Medium).unwrap();
        assert!(qr.size() >= 21);
    }
}
