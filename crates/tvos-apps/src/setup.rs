//! First-boot setup: network state, QR code for the phone remote, TV remote (HDMI-CEC) state.
//! Shown once; the shell remembers it was completed.

use crate::qr;
use crate::sysinfo::SysInfo;
use tvos_core::color::Color;
use tvos_core::input::RemoteKey;
use tvos_core::surface::Surface;
use tvos_core::text::{text_width, Weight};

#[derive(Default)]
pub struct SetupScreen {
    pub sys: SysInfo,
    pub port: u16,
    pub pin: String,
}

impl SetupScreen {
    /// True when the user confirmed: setup is over.
    pub fn handle_key(&mut self, key: RemoteKey) -> bool {
        matches!(key, RemoteKey::Select | RemoteKey::Home)
    }

    fn remote_url(&self) -> Option<String> {
        self.sys.ip.as_ref().map(|ip| format!("http://{ip}:{}", self.port))
    }

    pub fn render(&self, s: &mut Surface) {
        let (w, h) = (s.width as i32, s.height as i32);
        let k = h as f32 / 720.0;
        let px = |v: f32| (v * k).round() as i32;
        s.draw_background();
        s.fill_rect(0, 0, w as u32, h as u32, Color::rgba(0, 0, 0, 70));

        // Left: greeting and the three things that must work
        let x = (w as f32 * 0.075) as i32;
        s.draw_text_shadow(x, (h as f32 * 0.12) as i32, "Configurazione iniziale", Color::rgba(255, 255, 255, 200), 22.0 * k, Weight::SemiBold);
        s.draw_text_shadow(x, (h as f32 * 0.165) as i32, "Benvenuto", Color::white(), 78.0 * k, Weight::Display);
        s.draw_text_shadow(x, (h as f32 * 0.33) as i32, "Collega il telefono e la TV è pronta.", Color::rgba(255, 255, 255, 220), 24.0 * k, Weight::Regular);

        let sys = &self.sys;
        let (net_title, net_detail) = sys.network_summary();
        let net_detail = if sys.ip.is_some() && sys.static_ip { format!("{net_detail} - indirizzo fisso") } else { net_detail };
        let cec = if sys.cec_ready {
            ("Attivo: usa il telecomando del televisore", true)
        } else if sys.cec_present {
            ("In attesa del televisore", false)
        } else {
            ("Non disponibile su questo televisore", false)
        };
        let rows = [
            (net_title.as_str(), net_detail, sys.ip.is_some()),
            (
                "Telecomando dal telefono",
                if sys.ip.is_some() { "Inquadra il codice QR con la fotocamera".to_string() } else { "Disponibile appena la rete è connessa".to_string() },
                sys.ip.is_some(),
            ),
            ("Telecomando della TV (HDMI-CEC)", cec.0.to_string(), cec.1),
        ];
        let (rw, rh, gap) = ((w as f32 * 0.46) as i32, px(78.0), px(12.0));
        let y0 = (h as f32 * 0.43) as i32;
        for (i, (title, detail, ok)) in rows.iter().enumerate() {
            let y = y0 + i as i32 * (rh + gap);
            s.draw_frosted_card(x, y, rw as u32, rh as u32, px(22.0), Color::rgba(30, 30, 40, 70), Color::rgba(255, 255, 255, 80));
            let dot = px(14.0);
            let dc = if *ok { Color::rgb(52, 211, 120) } else { Color::rgb(245, 180, 60) };
            s.fill_rrect(x + px(26.0), y + rh / 2 - dot / 2, dot as u32, dot as u32, dot / 2, dc);
            let tx = x + px(26.0) + dot + px(18.0);
            s.draw_text_ex(tx, y + px(13.0), title, Color::white(), 22.0 * k, Weight::SemiBold);
            s.draw_text_ex(tx, y + px(44.0), detail, Color::rgba(255, 255, 255, 200), 17.0 * k, Weight::Regular);
        }

        // Start button
        let label = "Premi OK per iniziare";
        let bsize = 21.0 * k;
        let (bw, bh) = (text_width(label, bsize, Weight::SemiBold) + px(56.0), px(54.0));
        let by = y0 + 3 * (rh + gap) + px(14.0);
        s.fill_rrect(x, by, bw as u32, bh as u32, bh / 2, Color::rgba(255, 255, 255, 235));
        s.draw_text_ex(x + px(28.0), by + (bh as f32 * 0.5 - bsize * 0.68) as i32, label, Color::rgb(25, 25, 32), bsize, Weight::SemiBold);

        // Right: QR card
        let (cx, cw) = ((w as f32 * 0.615) as i32, (w as f32 * 0.31) as i32);
        let (cy, ch) = ((h as f32 * 0.14) as i32, (h as f32 * 0.74) as i32);
        s.draw_frosted_card(cx, cy, cw as u32, ch as u32, px(30.0), Color::rgba(30, 30, 40, 70), Color::rgba(255, 255, 255, 90));
        let pad = px(30.0);
        s.draw_text_shadow(cx + pad, cy + pad, "Telecomando", Color::white(), 28.0 * k, Weight::Display);
        let qs = (cw - 2 * pad).min(ch - px(190.0));
        let (qx, qy) = (cx + (cw - qs) / 2, cy + px(82.0));
        match self.remote_url() {
            Some(url) => {
                qr::draw(s, qx, qy, qs, &url);
                let us = 19.0 * k;
                let uw = text_width(&url, us, Weight::SemiBold);
                s.draw_text_ex(cx + (cw - uw) / 2, qy + qs + px(18.0), &url, Color::white(), us, Weight::SemiBold);
            }
            None => {
                s.fill_rrect(qx, qy, qs as u32, qs as u32, qs / 14, Color::rgba(255, 255, 255, 30));
                let msg = "In attesa della rete";
                let ms = 19.0 * k;
                s.draw_text_ex(qx + (qs - text_width(msg, ms, Weight::SemiBold)) / 2, qy + qs / 2 - px(12.0), msg, Color::white(), ms, Weight::SemiBold);
            }
        }
        if !self.pin.is_empty() {
            let pin = format!("PIN per installare app: {}", self.pin);
            let ps = 17.0 * k;
            s.draw_text_ex(cx + (cw - text_width(&pin, ps, Weight::Regular)) / 2, qy + qs + px(52.0), &pin, Color::rgba(255, 255, 255, 200), ps, Weight::Regular);
        }
    }
}
