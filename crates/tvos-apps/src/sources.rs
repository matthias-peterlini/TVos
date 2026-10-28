//! "Sorgenti": every input the TV can receive (antenna, Ethernet, Wi-Fi, HDMI), from real system data.

use crate::sysinfo::SysInfo;
use tvos_core::color::Color;
use tvos_core::input::RemoteKey;
use tvos_core::surface::Surface;
use tvos_core::text::{text_width, Weight};

struct Row {
    title: &'static str,
    detail: String,
    status: &'static str,
    on: bool,
}

#[derive(Default)]
pub struct SourcesScreen {
    pub sys: SysInfo,
    /// False while the top bar has the focus
    pub active: bool,
    selected: usize,
}

impl SourcesScreen {
    /// True when the user pressed Up on the first row: focus goes back to the top bar.
    pub fn handle_key(&mut self, key: RemoteKey) -> bool {
        match key {
            RemoteKey::Up if self.selected == 0 => return true,
            RemoteKey::Up => self.selected -= 1,
            RemoteKey::Down if self.selected + 1 < 4 => self.selected += 1,
            _ => {}
        }
        false
    }

    fn rows(&self) -> Vec<Row> {
        let sys = &self.sys;
        let ip = sys.ip.clone().unwrap_or_default();
        let link = |wireless: bool, none: &str| -> (String, &'static str, bool) {
            let mut found = sys.ifaces.iter().filter(|i| i.wireless == wireless);
            match found.clone().find(|i| i.up) {
                Some(i) => (format!("{} - {}", i.name, ip), "Connesso", true),
                None if found.next().is_some() => ("Nessuna connessione attiva".to_string(), "Non connesso", false),
                None => (none.to_string(), "Assente", false),
            }
        };
        let (eth_d, eth_s, eth_on) = link(false, "Nessun adattatore Ethernet");
        let (wifi_d, wifi_s, wifi_on) = link(true, "Nessun adattatore Wi-Fi");
        let hdmi: Vec<&str> = sys.connectors.iter().filter(|c| c.name.starts_with("HDMI")).map(|c| c.name.as_str()).collect();
        let hdmi_on = sys.connectors.iter().any(|c| c.name.starts_with("HDMI") && c.connected);
        let cec = if sys.cec_ready {
            "HDMI-CEC attivo"
        } else if sys.cec_present {
            "HDMI-CEC in attesa del televisore"
        } else {
            "HDMI-CEC non disponibile"
        };
        let hdmi_d = if hdmi.is_empty() { format!("Uscita non rilevata - {cec}") } else { format!("{} - {cec}", hdmi.join(", ")) };
        vec![
            Row {
                title: "Antenna - Digitale terrestre",
                detail: if sys.dvb { "Sintonizzatore DVB-T rilevato".into() } else { "Nessun sintonizzatore DVB-T collegato".into() },
                status: if sys.dvb { "Disponibile" } else { "Non disponibile" },
                on: sys.dvb,
            },
            Row { title: "Ethernet", detail: eth_d, status: eth_s, on: eth_on },
            Row { title: "Wi-Fi", detail: wifi_d, status: wifi_s, on: wifi_on },
            Row { title: "HDMI", detail: hdmi_d, status: if hdmi_on { "Collegato" } else { "Non collegato" }, on: hdmi_on },
        ]
    }

    pub fn render(&self, s: &mut Surface) {
        let (w, h) = (s.width as i32, s.height as i32);
        let k = h as f32 / 720.0;
        let px = |v: f32| (v * k).round() as i32;
        let (x, row_h, gap) = ((w as f32 * 0.07) as i32, (h as f32 * 0.145) as i32, (h as f32 * 0.016) as i32);
        let (pw, y0) = ((w as f32 * 0.86) as i32, (h as f32 * 0.17) as i32);
        let (tsize, dsize, ssize) = (30.0 * k, 20.0 * k, 20.0 * k);
        for (i, row) in self.rows().iter().enumerate() {
            let y = y0 + i as i32 * (row_h + gap);
            let focused = self.active && i == self.selected;
            let tint = if focused { Color::rgba(255, 255, 255, 40) } else { Color::rgba(30, 30, 40, 70) };
            s.draw_frosted_card(x, y, pw as u32, row_h as u32, px(26.0), tint, Color::rgba(255, 255, 255, 80));
            if focused {
                s.stroke_rrect(x - 3, y - 3, (pw + 6) as u32, (row_h + 6) as u32, px(29.0), 3.0, Color::rgba(255, 255, 255, 235));
            }
            let dot = px(14.0);
            let dc = if row.on { Color::rgb(52, 211, 120) } else { Color::rgba(255, 255, 255, 110) };
            s.fill_rrect(x + px(34.0), y + row_h / 2 - dot / 2, dot as u32, dot as u32, dot / 2, dc);
            let tx = x + px(34.0) + dot + px(22.0);
            s.draw_text_shadow(tx, y + row_h / 2 - (tsize * 0.95) as i32, row.title, Color::white(), tsize, Weight::SemiBold);
            s.draw_text_ex(tx, y + row_h / 2 + px(6.0), &row.detail, Color::rgba(255, 255, 255, 215), dsize, Weight::Regular);
            let sw = text_width(row.status, ssize, Weight::SemiBold);
            s.draw_text_shadow(x + pw - px(34.0) - sw, y + row_h / 2 - (ssize * 0.65) as i32, row.status, if row.on { Color::white() } else { Color::rgba(255, 255, 255, 170) }, ssize, Weight::SemiBold);
        }
    }
}
