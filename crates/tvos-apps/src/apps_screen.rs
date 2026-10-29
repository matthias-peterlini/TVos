//! "Installa app" screen: QR code + address + PIN for the phone, and the list of installed apps.

use crate::qr;
use tvos_core::color::Color;
use tvos_core::input::RemoteKey;
use tvos_core::surface::Surface;
use tvos_core::text::{text_width, Weight};

#[derive(Default)]
pub struct AppsScreen {
    pub ip: String,
    pub port: u16,
    pub pin: String,
    /// Names of the installed apps
    pub installed: Vec<String>,
}

impl AppsScreen {
    pub fn handle_key(&mut self, _key: RemoteKey) -> bool {
        false
    }

    pub fn render(&self, s: &mut Surface) {
        let (w, h) = (s.width as i32, s.height as i32);
        let k = h as f32 / 720.0;
        let px = |v: f32| (v * k).round() as i32;
        s.draw_background();
        s.fill_rect(0, 0, w as u32, h as u32, Color::rgba(0, 0, 0, 60));

        let x = (w as f32 * 0.075) as i32;
        s.draw_text_ex(x, (h as f32 * 0.11) as i32, "Installa app", Color::white(), 60.0 * k, Weight::Display);
        s.draw_text_ex(x, (h as f32 * 0.235) as i32, "Dal telefono, collegato alla stessa rete della TV.", Color::rgba(255, 255, 255, 220), 22.0 * k, Weight::Regular);

        let has_ip = !self.ip.is_empty() && self.ip != "0.0.0.0";
        let url = format!("http://{}:{}/apps", self.ip, self.port);
        let steps = [
            "Inquadra il codice QR con la fotocamera",
            "Inserisci il PIN mostrato qui sotto",
            "Scegli un servizio: compare subito nella Home",
        ];
        let (cw, rh) = ((w as f32 * 0.46) as i32, px(60.0));
        let y0 = (h as f32 * 0.33) as i32;
        s.draw_frosted_card(x, y0, cw as u32, (rh * 3 + px(20.0)) as u32, px(24.0), Color::rgba(30, 30, 40, 70), Color::rgba(255, 255, 255, 80));
        for (i, step) in steps.iter().enumerate() {
            let y = y0 + px(10.0) + i as i32 * rh;
            let n = px(34.0);
            s.fill_rrect(x + px(22.0), y + (rh - n) / 2, n as u32, n as u32, n / 2, Color::rgba(255, 255, 255, 230));
            let num = (i + 1).to_string();
            let ns = 18.0 * k;
            s.draw_text_ex(x + px(22.0) + (n - text_width(&num, ns, Weight::Bold)) / 2, y + rh / 2 - (ns * 0.68) as i32, &num, Color::rgb(25, 25, 32), ns, Weight::Bold);
            s.draw_text_ex(x + px(72.0), y + rh / 2 - (19.0 * k * 0.68) as i32, step, Color::white(), 19.0 * k, Weight::SemiBold);
        }

        // PIN
        let py = y0 + rh * 3 + px(40.0);
        s.draw_text_ex(x, py, "PIN", Color::rgba(255, 255, 255, 200), 18.0 * k, Weight::SemiBold);
        let pin = if self.pin.is_empty() { "----" } else { &self.pin };
        s.draw_text_ex(x, py + px(26.0), pin, Color::white(), 52.0 * k, Weight::Display);

        // Installed apps as chips
        let iy = py + px(110.0);
        let label = if self.installed.is_empty() { "Nessuna app installata".to_string() } else { format!("Installate ({})", self.installed.len()) };
        s.draw_text_ex(x, iy, &label, Color::rgba(255, 255, 255, 200), 16.0 * k, Weight::SemiBold);
        let (mut cx, mut cy) = (x, iy + px(30.0));
        let cs = 15.0 * k;
        for name in &self.installed {
            let tw = text_width(name, cs, Weight::SemiBold) + px(28.0);
            if cx + tw > x + cw {
                cx = x;
                cy += px(42.0);
                if cy + px(34.0) > h - px(20.0) {
                    break;
                }
            }
            s.draw_frosted_card(cx, cy, tw as u32, px(34.0) as u32, px(17.0), Color::rgba(255, 255, 255, 25), Color::rgba(255, 255, 255, 70));
            s.draw_text_ex(cx + px(14.0), cy + px(17.0) - (cs * 0.68) as i32, name, Color::white(), cs, Weight::SemiBold);
            cx += tw + px(8.0);
        }

        // QR card
        let (qcx, qcw) = ((w as f32 * 0.615) as i32, (w as f32 * 0.31) as i32);
        let (qcy, qch) = ((h as f32 * 0.14) as i32, (h as f32 * 0.72) as i32);
        s.draw_frosted_card(qcx, qcy, qcw as u32, qch as u32, px(30.0), Color::rgba(30, 30, 40, 70), Color::rgba(255, 255, 255, 90));
        let pad = px(30.0);
        let qs = qcw - 2 * pad;
        let (qx, qy) = (qcx + pad, qcy + pad);
        if has_ip {
            qr::draw(s, qx, qy, qs, &url);
            let us = 18.0 * k;
            s.draw_text_ex(qcx + (qcw - text_width(&url, us, Weight::SemiBold)) / 2, qy + qs + px(22.0), &url, Color::white(), us, Weight::SemiBold);
        } else {
            let msg = "In attesa della rete";
            s.draw_text_ex(qcx + (qcw - text_width(msg, 19.0 * k, Weight::SemiBold)) / 2, qy + qs / 2, msg, Color::white(), 19.0 * k, Weight::SemiBold);
        }
        let note = "Premi Indietro per tornare alla Home";
        s.draw_text_ex(qcx + (qcw - text_width(note, 15.0 * k, Weight::Regular)) / 2, qcy + qch - px(40.0), note, Color::rgba(255, 255, 255, 170), 15.0 * k, Weight::Regular);
    }
}
