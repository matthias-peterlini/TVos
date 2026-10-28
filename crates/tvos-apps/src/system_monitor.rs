use crate::sysinfo::SysInfo;
use tvos_core::color::Color;
use tvos_core::input::RemoteKey;
use tvos_core::surface::Surface;
use tvos_core::theme::Theme;

#[derive(Default)]
pub struct SystemMonitor {
    pub sys: SysInfo,
}

fn or_nd<T: std::fmt::Display>(v: Option<T>, unit: &str) -> String {
    v.map_or("n/d".to_string(), |v| format!("{v}{unit}"))
}

impl SystemMonitor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn handle_key(&mut self, _key: RemoteKey) -> bool {
        false
    }

    pub fn render(&self, surface: &mut Surface) {
        let w = surface.width as i32;
        let h = surface.height as i32;
        let s = &self.sys;

        surface.clear(Theme::BG_BASE);
        surface.draw_radial_glow(w / 4, h / 2, 350, Theme::ORB_CYAN);
        surface.draw_radial_glow((3 * w) / 4, h / 2, 320, Theme::ORB_AMBER);

        surface.draw_text(60, 50, "Monitor di sistema", Theme::TEXT_PRIMARY, 2);
        surface.draw_text(60, 90, "Premi [BACK] sul telecomando per tornare alla Home", Theme::TEXT_MUTED, 1);

        let grid_w = ((w - 120 - 30) / 2).min(420) as u32;
        let grid_h = 190u32;
        let gap = 30;
        let start_x = 60;
        let start_y = 140;
        let p2_x = start_x + grid_w as i32 + gap;
        let row2_y = start_y + grid_h as i32 + gap;

        // CPU
        surface.draw_glass_card(start_x, start_y, grid_w, grid_h, 24, Theme::CARD_BG, Theme::CARD_BORDER);
        surface.draw_text(start_x + 30, start_y + 25, "PROCESSORE", Theme::TEXT_MUTED, 1);
        let load = s.cpu_percent.map_or("n/d".to_string(), |p| format!("{p:.0}%"));
        surface.draw_text(start_x + 30, start_y + 50, &format!("Carico {load}"), Theme::TEXT_PRIMARY, 2);
        let hw = s.hardware.clone().unwrap_or_else(|| "Hardware non rilevato".into());
        surface.draw_text(start_x + 30, start_y + 100, &hw, Theme::TEXT_SECONDARY, 1);
        surface.draw_text(start_x + 30, start_y + 125, &format!("{} core", s.cpu_cores), Theme::TEXT_SECONDARY, 1);
        let bar_w = grid_w - 60;
        surface.fill_rrect(start_x + 30, start_y + 155, bar_w, 8, 4, Color::rgba(255, 255, 255, 30));
        if let Some(p) = s.cpu_percent {
            surface.fill_rrect(start_x + 30, start_y + 155, ((bar_w as f32) * p / 100.0).max(8.0) as u32, 8, 4, Theme::AMBER);
        }

        // RAM
        surface.draw_glass_card(p2_x, start_y, grid_w, grid_h, 24, Theme::CARD_BG, Theme::CARD_BORDER);
        surface.draw_text(p2_x + 30, start_y + 25, "MEMORIA RAM", Theme::TEXT_MUTED, 1);
        let ram = match (s.ram_used_mb, s.ram_total_mb) {
            (Some(u), Some(t)) => format!("{u} MB / {t} MB"),
            _ => "n/d".into(),
        };
        surface.draw_text(p2_x + 30, start_y + 50, &ram, Theme::TEXT_PRIMARY, 2);
        surface.draw_text(p2_x + 30, start_y + 100, "Memoria in uso dal sistema (esclusa la cache)", Theme::TEXT_SECONDARY, 1);
        surface.fill_rrect(p2_x + 30, start_y + 155, bar_w, 8, 4, Color::rgba(255, 255, 255, 30));
        if let (Some(u), Some(t)) = (s.ram_used_mb, s.ram_total_mb) {
            let frac = u as f32 / t.max(1) as f32;
            surface.fill_rrect(p2_x + 30, start_y + 155, ((bar_w as f32) * frac).max(8.0) as u32, 8, 4, Theme::EMERALD);
        }

        // Display & HDMI-CEC
        surface.draw_glass_card(start_x, row2_y, grid_w, grid_h, 24, Theme::CARD_BG, Theme::CARD_BORDER);
        surface.draw_text(start_x + 30, row2_y + 25, "SCHERMO E HDMI-CEC", Theme::TEXT_MUTED, 1);
        surface.draw_text(start_x + 30, row2_y + 50, &format!("{w} x {h}"), Theme::TEXT_PRIMARY, 2);
        let (cec_text, cec_col) = if s.cec_ready {
            ("CEC: collegato, telecomando TV attivo", Theme::EMERALD)
        } else if s.cec_present {
            ("CEC: adattatore presente, TV non risponde", Theme::AMBER)
        } else {
            ("CEC: nessun adattatore (usa il telefono)", Theme::TEXT_MUTED)
        };
        surface.draw_text(start_x + 30, row2_y + 105, cec_text, cec_col, 1);

        // Temperature, network, uptime
        surface.draw_glass_card(p2_x, row2_y, grid_w, grid_h, 24, Theme::CARD_BG, Theme::CARD_BORDER);
        surface.draw_text(p2_x + 30, row2_y + 25, "TEMPERATURA E RETE", Theme::TEXT_MUTED, 1);
        let (temp, state, col) = match s.temp_c {
            Some(t) if t >= 75.0 => (format!("{t:.1} C"), "Alta", Theme::ROSE),
            Some(t) if t >= 60.0 => (format!("{t:.1} C"), "Calda", Theme::AMBER),
            Some(t) => (format!("{t:.1} C"), "Normale", Theme::EMERALD),
            None => ("n/d".to_string(), "sensore non disponibile", Theme::TEXT_MUTED),
        };
        surface.draw_text(p2_x + 30, row2_y + 50, &temp, Theme::TEXT_PRIMARY, 2);
        surface.draw_text(p2_x + 30, row2_y + 100, state, col, 1);
        surface.draw_text(p2_x + 30, row2_y + 125, &format!("IP: {}", or_nd(s.ip.as_deref(), "")), Theme::TEXT_SECONDARY, 1);
        let kernel = format!("Acceso da {} | Linux {}", s.uptime_text(), or_nd(s.kernel.as_deref(), ""));
        surface.draw_text(p2_x + 30, row2_y + 150, &kernel, Theme::TEXT_SECONDARY, 1);
    }
}
