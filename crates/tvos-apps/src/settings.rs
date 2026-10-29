//! "Impostazioni": category menu on the left, glass panel with the details on the right.

use crate::clock;
use crate::qr;
use crate::sysinfo::SysInfo;
use tvos_core::color::Color;
use tvos_core::input::RemoteKey;
use tvos_core::surface::Surface;
use tvos_core::text::{text_width, Weight};

const MENU: [&str; 6] = ["Generali", "Telecomando", "Aspetto", "Rete", "Gestione sistema", "Informazioni"];

pub struct SettingsApp {
    pub server_ip: String,
    pub server_port: u16,
    pub pin: String,
    pub name: String,
    pub installed_apps: usize,
    pub sys: SysInfo,
    menu: usize,
}

impl SettingsApp {
    pub fn new(server_ip: String, server_port: u16) -> Self {
        Self { server_ip, server_port, pin: String::new(), name: String::new(), installed_apps: 0, sys: SysInfo::default(), menu: 0 }
    }

    /// True when the user pressed Up on the first entry: focus goes back to the top bar.
    pub fn handle_key(&mut self, key: RemoteKey) -> bool {
        match key {
            RemoteKey::Up if self.menu == 0 => return true,
            RemoteKey::Up => self.menu -= 1,
            RemoteKey::Down if self.menu + 1 < MENU.len() => self.menu += 1,
            _ => {}
        }
        false
    }

    fn remote_url(&self) -> String {
        format!("http://{}:{}", self.server_ip, self.server_port)
    }

    fn lines(&self, screen: (usize, usize)) -> Vec<(String, String)> {
        let sys = &self.sys;
        let nd = || "n/d".to_string();
        let yn = |b: bool| if b { "Sì" } else { "No" }.to_string();
        let cec = || {
            if sys.cec_ready {
                "Attivo".to_string()
            } else if sys.cec_present {
                "In attesa del televisore".into()
            } else {
                "Non disponibile".into()
            }
        };
        match MENU[self.menu] {
            "Generali" => {
                let unix = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64);
                let tz = if clock::italy_offset(unix) == 3600 { "Europa/Roma (CET)" } else { "Europa/Roma (CEST)" };
                vec![
                    ("Nome".into(), self.name.clone()),
                    ("Lingua".into(), "Italiano".into()),
                    ("Fuso orario".into(), tz.into()),
                    ("Ora".into(), clock::now_text()),
                    ("Telecomando TV (HDMI-CEC)".into(), cec()),
                ]
            }
            "Telecomando" => vec![
                ("Indirizzo".into(), if sys.ip.is_some() { self.remote_url() } else { "In attesa della rete".into() }),
                ("Telefono".into(), "Inquadra il QR".into()),
                ("HDMI-CEC".into(), cec()),
                ("PIN app".into(), if self.pin.is_empty() { nd() } else { self.pin.clone() }),
            ],
            "Aspetto" => vec![
                ("Sfondo".into(), "Dolomiti (predefinito)".into()),
                ("Carattere".into(), "Inter".into()),
                ("Carattere orologio".into(), "Google Sans Flex".into()),
                ("Risoluzione".into(), format!("{} x {}", screen.0, screen.1)),
            ],
            "Rete" => {
                let mut v = vec![("Indirizzo IP".to_string(), sys.ip.clone().unwrap_or_else(nd))];
                if sys.ip.is_some() {
                    v.push(("Tipo di indirizzo".into(), if sys.static_ip { "Fisso, mantenuto al riavvio".into() } else { "Automatico (DHCP)".into() }));
                }
                for nic in &sys.ifaces {
                    let kind = if nic.wireless { "Wi-Fi" } else { "Ethernet" };
                    v.push((format!("{} ({kind})", nic.name), if nic.up { "Connessa" } else { "Non connessa" }.into()));
                }
                v.push(("Bluetooth".into(), if sys.bluetooth { "Adattatore rilevato".into() } else { "Nessun adattatore".into() }));
                v
            }
            "Gestione sistema" => {
                let ram = match (sys.ram_used_mb, sys.ram_total_mb) {
                    (Some(u), Some(t)) => format!("{u} / {t} MB"),
                    _ => nd(),
                };
                vec![
                    ("Memoria".into(), ram),
                    ("Processore".into(), sys.cpu_percent.map_or_else(nd, |c| format!("{c:.0} %"))),
                    ("Temperatura".into(), sys.temp_c.map_or_else(nd, |t| format!("{t:.0} °C"))),
                    ("Acceso da".into(), sys.uptime_text()),
                    ("App installate".into(), self.installed_apps.to_string()),
                    ("Installa dal telefono".into(), format!("{}/apps", self.remote_url())),
                ]
            }
            _ => vec![
                ("Sistema".into(), format!("TVos {}", env!("CARGO_PKG_VERSION"))),
                ("Dispositivo".into(), sys.hardware.clone().unwrap_or_else(nd)),
                ("Kernel".into(), sys.kernel.clone().unwrap_or_else(nd)),
                ("Core".into(), if sys.cpu_cores > 0 { sys.cpu_cores.to_string() } else { nd() }),
                ("Bluetooth".into(), yn(sys.bluetooth)),
            ],
        }
    }

    pub fn render(&self, s: &mut Surface) {
        let (w, h) = (s.width as i32, s.height as i32);
        let k = h as f32 / 720.0;
        let px = |v: f32| (v * k).round() as i32;
        let (top, bottom) = ((h as f32 * 0.17) as i32, (h as f32 * 0.88) as i32);
        let (mx, mw) = ((w as f32 * 0.065) as i32, (w as f32 * 0.23) as i32);
        let (rx, rw) = (mx + mw + px(20.0), (w as f32 * 0.935) as i32 - (mx + mw + px(20.0)));

        // Menu
        let ph = bottom - top;
        s.draw_frosted_card(mx, top, mw as u32, ph as u32, px(26.0), Color::rgba(30, 30, 40, 60), Color::rgba(255, 255, 255, 80));
        let (item_h, ipad) = (px(52.0), px(14.0));
        let msize = 21.0 * k;
        for (i, name) in MENU.iter().enumerate() {
            let y = top + ipad + i as i32 * (item_h + px(8.0));
            let on = i == self.menu;
            if on {
                s.fill_rrect(mx + ipad, y, (mw - 2 * ipad) as u32, item_h as u32, px(26.0) - ipad, Color::rgba(255, 255, 255, 225));
            }
            let col = if on { Color::rgb(25, 25, 32) } else { Color::white() };
            s.draw_text_ex(mx + ipad + px(18.0), y + (item_h as f32 * 0.5 - msize * 0.68) as i32, name, col, msize, Weight::SemiBold);
        }

        // Panel
        s.draw_frosted_card(rx, top, rw as u32, ph as u32, px(26.0), Color::rgba(30, 30, 40, 60), Color::rgba(255, 255, 255, 80));
        s.draw_text_ex(rx + px(40.0), top + px(30.0), MENU[self.menu], Color::white(), 34.0 * k, Weight::Display);
        // The remote page also shows its QR code on the right of the panel
        let mut rows_w = rw - px(80.0);
        if MENU[self.menu] == "Telecomando" && self.sys.ip.is_some() {
            let qs = px(250.0).min(ph - px(130.0));
            qr::draw(s, rx + rw - px(40.0) - qs, top + px(100.0), qs, &self.remote_url());
            rows_w -= qs + px(36.0);
        }
        let (lsize, vsize) = (20.0 * k, 20.0 * k);
        let row_h = px(48.0);
        for (i, (label, value)) in self.lines((s.width, s.height)).iter().enumerate() {
            let y = top + px(100.0) + i as i32 * row_h;
            if y + row_h > bottom {
                break;
            }
            s.fill_rect(rx + px(40.0), y + row_h - 1, rows_w as u32, 1, Color::rgba(255, 255, 255, 45));
            s.draw_text_ex(rx + px(40.0), y + (row_h as f32 * 0.5 - lsize * 0.68) as i32, label, Color::rgba(255, 255, 255, 215), lsize, Weight::Regular);
            let vw = text_width(value, vsize, Weight::SemiBold);
            s.draw_text_ex(rx + px(40.0) + rows_w - vw, y + (row_h as f32 * 0.5 - vsize * 0.68) as i32, value, Color::white(), vsize, Weight::SemiBold);
        }
    }
}
