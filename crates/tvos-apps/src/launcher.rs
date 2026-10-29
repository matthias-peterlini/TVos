//! Home screen, tvOS style: full-screen photo, big clock, greeting and a dock of app tiles.

use crate::clock;
use crate::nav::ease;
use crate::sysinfo::SysInfo;
use std::path::PathBuf;
use tvos_core::color::Color;
use tvos_core::icons::Icon;
use tvos_core::input::RemoteKey;
use tvos_core::surface::Surface;
use tvos_core::text::{text_width, Weight};
use tvos_store::{AppKind, InstalledApp};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ActiveScreen {
    Launcher,
    YouTube,
    PrimeVideo,
    ScreenMirror,
    MediaPlayer,
    SystemMonitor,
    Settings,
    AddApps,
}

/// An app the shell has to start outside of TVos' own screens.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Launch {
    Web { name: String, url: String },
    Native { name: String, exec: PathBuf },
}

#[derive(Clone)]
enum Action {
    Screen(ActiveScreen),
    Launch(Launch),
}

#[derive(Clone)]
enum Art {
    YouTube,
    Netflix,
    /// White glyph on a coloured gradient
    Glyph(Icon, u32),
    /// First letters of the name on the app's colour
    Letters(u32),
}

#[derive(Clone)]
struct HomeApp {
    id: String,
    name: String,
    art: Art,
    action: Action,
    /// Shown in the dock; every app is in the library
    pinned: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Mode {
    Dock,
    Library,
}

const LIB_COLS: usize = 4;
const LIB_PER_PAGE: usize = 8;

impl Art {
    /// Colour the library background takes while the app is focused
    fn color(&self) -> u32 {
        match self {
            Art::YouTube => 0xE62117,
            Art::Netflix => 0xB20710,
            Art::Glyph(_, c) | Art::Letters(c) => *c,
        }
    }
}

pub struct Launcher {
    /// Position in the dock (pinned apps)
    pub selected_index: usize,
    pub active_screen: ActiveScreen,
    pub server_ip: String,
    pub web_port: u16,
    pub sys: SysInfo,
    /// Shown after "Ciao,"
    pub name: String,
    apps: Vec<HomeApp>,
    installed: Vec<InstalledApp>,
    scroll: usize,
    pub pending_launch: Option<Launch>,
    mode: Mode,
    lib_sel: usize,
    /// 0 = home, 1 = app library (animated)
    lib_t: f32,
    tint: [f32; 3],
}

fn user_name() -> String {
    std::env::var("TVOS_NAME")
        .ok()
        .or_else(|| std::fs::read_to_string("/etc/tvos/name").ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "ticly Studio".to_string())
}

fn art_for(app: &InstalledApp) -> Art {
    if app.id == "netflix" {
        Art::Netflix
    } else {
        Art::Letters(app.color)
    }
}

fn rgb_of(c: u32) -> [f32; 3] {
    [((c >> 16) & 255) as f32, ((c >> 8) & 255) as f32, (c & 255) as f32]
}

impl Launcher {
    pub fn new(server_ip: String) -> Self {
        let mut l = Self {
            selected_index: 0,
            active_screen: ActiveScreen::Launcher,
            server_ip,
            web_port: 8080,
            sys: SysInfo::default(),
            name: user_name(),
            apps: Vec::new(),
            installed: Vec::new(),
            scroll: 0,
            pending_launch: None,
            mode: Mode::Dock,
            lib_sel: 0,
            lib_t: 0.0,
            tint: [0.0; 3],
        };
        l.rebuild();
        l
    }

    fn dock(&self) -> Vec<usize> {
        (0..self.apps.len()).filter(|&i| self.apps[i].pinned).collect()
    }

    pub fn in_library(&self) -> bool {
        self.lib_t > 0.5
    }

    pub fn installed_count(&self) -> usize {
        self.installed.len()
    }

    /// Back to the plain home screen
    pub fn reset(&mut self) {
        self.mode = Mode::Dock;
    }

    fn target(&self) -> f32 {
        if self.mode == Mode::Library { 1.0 } else { 0.0 }
    }

    fn tint_target(&self) -> [f32; 3] {
        self.apps.get(self.lib_sel).map_or([0.0; 3], |a| rgb_of(a.art.color()))
    }

    /// Advance the animations by `dt` seconds; true while something is still moving.
    pub fn tick(&mut self, dt: f32) -> bool {
        let target = self.target();
        let mut moving = ease(&mut self.lib_t, target, dt, 11.0);
        let t = self.tint_target();
        for i in 0..3 {
            moving |= ease(&mut self.tint[i], t[i], dt, 9.0);
        }
        moving
    }

    /// Jump to the end of every animation (screenshots)
    pub fn settle(&mut self) {
        self.lib_t = self.target();
        self.tint = self.tint_target();
    }

    /// Replace the list of installed apps; the selection stays on the same app when it still exists.
    pub fn set_installed(&mut self, installed: Vec<InstalledApp>) {
        if installed == self.installed {
            return;
        }
        let keep = self.dock().get(self.selected_index).map(|&i| self.apps[i].id.clone());
        let keep_lib = self.apps.get(self.lib_sel).map(|a| a.id.clone());
        self.installed = installed;
        self.rebuild();
        let dock = self.dock();
        if let Some(p) = keep.and_then(|id| dock.iter().position(|&i| self.apps[i].id == id)) {
            self.selected_index = p;
        }
        self.selected_index = self.selected_index.min(dock.len().saturating_sub(1));
        self.lib_sel = keep_lib.and_then(|id| self.apps.iter().position(|a| a.id == id)).unwrap_or(0).min(self.apps.len().saturating_sub(1));
    }

    fn rebuild(&mut self) {
        let mut apps = vec![HomeApp {
            id: "@youtube".into(),
            name: "YouTube".into(),
            art: Art::YouTube,
            action: Action::Screen(ActiveScreen::YouTube),
            pinned: true,
        }];
        for a in &self.installed {
            let launch = match a.kind {
                AppKind::Web => Launch::Web { name: a.name.clone(), url: a.url.clone() },
                AppKind::Native => Launch::Native { name: a.name.clone(), exec: a.exec.clone() },
            };
            apps.push(HomeApp { id: a.id.clone(), name: a.name.clone(), art: art_for(a), action: Action::Launch(launch), pinned: true });
        }
        let system = [
            ("@apps", "Installa app", Icon::Plus, 0x5E5CE6, ActiveScreen::AddApps),
            ("@monitor", "Monitor", Icon::Chart, 0x10B981, ActiveScreen::SystemMonitor),
        ];
        for (id, name, icon, color, screen) in system {
            apps.push(HomeApp { id: id.into(), name: name.into(), art: Art::Glyph(icon, color), action: Action::Screen(screen), pinned: false });
        }
        self.apps = apps;
    }

    fn launch(&mut self, idx: usize) {
        if let Some(app) = self.apps.get(idx) {
            match app.action.clone() {
                Action::Screen(s) => self.active_screen = s,
                Action::Launch(l) => self.pending_launch = Some(l),
            }
        }
    }

    /// Returns true when Up should move the focus to the top bar.
    pub fn handle_key(&mut self, key: RemoteKey) -> bool {
        match self.mode {
            Mode::Dock => {
                let dock = self.dock();
                match key {
                    RemoteKey::Left if self.selected_index > 0 => self.selected_index -= 1,
                    RemoteKey::Right if self.selected_index + 1 < dock.len() => self.selected_index += 1,
                    RemoteKey::Down => {
                        self.lib_sel = dock.get(self.selected_index).copied().unwrap_or(0);
                        self.mode = Mode::Library;
                    }
                    RemoteKey::Select => {
                        if let Some(&i) = dock.get(self.selected_index) {
                            self.launch(i);
                        }
                    }
                    RemoteKey::Up => return true,
                    _ => {}
                }
            }
            Mode::Library => {
                let (n, i) = (self.apps.len(), self.lib_sel);
                match key {
                    RemoteKey::Left if i % LIB_COLS > 0 => self.lib_sel -= 1,
                    RemoteKey::Right if i % LIB_COLS < LIB_COLS - 1 && i + 1 < n => self.lib_sel += 1,
                    RemoteKey::Up if i >= LIB_COLS => self.lib_sel -= LIB_COLS,
                    RemoteKey::Up | RemoteKey::Back => {
                        if let Some(p) = self.dock().iter().position(|&d| d == self.lib_sel) {
                            self.selected_index = p;
                        }
                        self.mode = Mode::Dock;
                    }
                    RemoteKey::Down if i + LIB_COLS < n => self.lib_sel += LIB_COLS,
                    RemoteKey::Down if (i / LIB_COLS) < (n - 1) / LIB_COLS => self.lib_sel = n - 1,
                    RemoteKey::Select => self.launch(i),
                    _ => {}
                }
            }
        }
        false
    }

    pub fn render(&mut self, surface: &mut Surface) {
        let h = surface.height as f32;
        if self.lib_t > 0.001 {
            let c = Color::rgb(self.tint[0] as u8, self.tint[1] as u8, self.tint[2] as u8);
            surface.draw_tinted_background(c, 0.6 * self.lib_t);
        } else {
            surface.draw_background();
        }
        if self.lib_t < 0.5 {
            let a = self.lib_t * 2.0;
            self.render_home(surface, -(a * 0.12 * h) as i32);
        } else {
            let b = (self.lib_t - 0.5) * 2.0;
            self.render_library(surface, b);
        }
    }

    fn render_home(&mut self, surface: &mut Surface, dy: i32) {
        let w = surface.width as i32;
        let h = surface.height as i32;
        let s = h as f32 / 720.0;
        let px = |v: f32| (v * s).round() as i32;
        let white = Color::white();

        // Status chips, top right
        let chip = px(44.0);
        let chips = [(Icon::Remote, self.sys.cec_ready), (Icon::Wifi, self.sys.ip.is_some())];
        for (i, (icon, on)) in chips.iter().enumerate() {
            let x = w - px(36.0) - chip - i as i32 * (chip + px(12.0));
            let y = px(30.0) + dy;
            surface.draw_frosted_card(x, y, chip as u32, chip as u32, chip / 2, Color::rgba(255, 255, 255, 30), Color::rgba(255, 255, 255, 70));
            let c = if *on { white } else { Color::rgba(255, 255, 255, 110) };
            surface.draw_icon(*icon, x + chip / 2, y + chip / 2, px(22.0), c);
        }

        // Clock and greeting: Google Sans Flex, wide and slightly rounded
        let time = clock::now_text();
        let probe = text_width(&time, 100.0, Weight::Display).max(1) as f32;
        let size = (100.0 * w as f32 * 0.36 / probe).min(150.0 * s);
        surface.draw_text_ex((w - text_width(&time, size, Weight::Display)) / 2, (h as f32 * 0.16) as i32 + dy, &time, white, size, Weight::Display);
        let greet = format!("Ciao, {}", self.name);
        let gsize = 46.0 * s;
        surface.draw_text_ex((w - text_width(&greet, gsize, Weight::Display)) / 2, (h as f32 * 0.465) as i32 + dy, &greet, white, gsize, Weight::Display);

        // Dock
        let dock = self.dock();
        let count = dock.len();
        let tile_w = (w as f32 * 0.19) as i32;
        let tile_h = (tile_w as f32 * 0.57) as i32;
        let grow = (tile_w as f32 * 0.05) as i32; // the focused tile grows by this much on each side
        let gap = (tile_w as f32 * 0.10) as i32; // wider than 2 * grow so the focused tile never touches its neighbours
        let pad = (tile_w as f32 * 0.10) as i32;
        let label = px(40.0); // room under the tiles for the focused app's name, inside the bar
        let fit = (((w as f32 * 0.92) as i32 - 2 * pad + gap) / (tile_w + gap)).max(1) as usize;
        let visible = fit.min(count);
        if self.selected_index < self.scroll {
            self.scroll = self.selected_index;
        } else if self.selected_index >= self.scroll + visible {
            self.scroll = self.selected_index + 1 - visible;
        }
        let bar_w = visible as i32 * tile_w + (visible as i32 - 1) * gap + 2 * pad;
        let bar_h = tile_h + 2 * pad + label;
        let bar_x = (w - bar_w) / 2;
        let bar_y = (h as f32 * 0.91) as i32 - bar_h + dy;
        // Bar corners concentric with the tiles': tile radius + padding
        surface.draw_frosted_card(bar_x, bar_y, bar_w as u32, bar_h as u32, tile_h * 16 / 100 + pad, Color::rgba(255, 255, 255, 38), Color::rgba(255, 255, 255, 80));

        let mut focused: Option<(usize, i32)> = None;
        for slot in 0..visible {
            let idx = self.scroll + slot;
            let x = bar_x + pad + slot as i32 * (tile_w + gap);
            if idx == self.selected_index {
                focused = Some((idx, x));
                continue;
            }
            let app = &self.apps[dock[idx]];
            draw_tile(surface, &app.art, &app.name, x, bar_y + pad, tile_w, tile_h);
        }
        if let Some((idx, x)) = focused {
            // The focused tile grows, gets a ring and a soft shadow
            let app = &self.apps[dock[idx]];
            let (fx, fy, fw, fh) = (x - grow, bar_y + pad - grow, tile_w + 2 * grow, tile_h + 2 * grow);
            surface.draw_radial_glow(fx + fw / 2, fy + fh + px(6.0), fw * 3 / 5, Color::rgba(0, 0, 0, 120));
            draw_tile(surface, &app.art, &app.name, fx, fy, fw, fh);
            surface.stroke_rrect(fx - 2, fy - 2, (fw + 4) as u32, (fh + 4) as u32, fh * 16 / 100 + 2, 3.0, Color::rgba(255, 255, 255, 235));
            let nsize = 22.0 * s;
            surface.draw_text_ex(fx + (fw - text_width(&app.name, nsize, Weight::SemiBold)) / 2, fy + fh + px(6.0), &app.name, white, nsize, Weight::SemiBold);
        }
        // More apps off screen: small arrows at the dock edges
        let arrow = 26.0 * s;
        if self.scroll > 0 {
            surface.draw_text_ex(bar_x - px(28.0), bar_y + pad + tile_h / 2 - px(18.0), "‹", white, arrow * 1.6, Weight::SemiBold);
        }
        if self.scroll + visible < count {
            surface.draw_text_ex(bar_x + bar_w + px(12.0), bar_y + pad + tile_h / 2 - px(18.0), "›", white, arrow * 1.6, Weight::SemiBold);
        }
    }

    /// App library: header (clock, pinned apps, name) and a 4x2 grid of every app. `b` is the entry progress 0..1.
    fn render_library(&mut self, surface: &mut Surface, b: f32) {
        let w = surface.width as i32;
        let h = surface.height as i32;
        let s = h as f32 / 720.0;
        let px = |v: f32| (v * s).round() as i32;
        let white = Color::white();
        let (hdy, gdy) = (-((1.0 - b) * 0.06 * h as f32) as i32, ((1.0 - b) * 0.08 * h as f32) as i32);

        // Header: clock left, pinned apps centre, name right
        let dock = self.dock();
        let mw = (w as f32 * 0.095) as i32;
        let mh = (mw as f32 * 0.57) as i32;
        let (mgap, mpad) = (px(10.0), px(9.0));
        let n = dock.len().min(5) as i32;
        let hb_w = n * mw + (n - 1).max(0) * mgap + 2 * mpad;
        let hb_h = mh + 2 * mpad;
        let hb_y = px(30.0) + hdy;
        let hb_x = (w - hb_w) / 2;
        surface.draw_frosted_card(hb_x, hb_y, hb_w as u32, hb_h as u32, mh / 3, Color::rgba(20, 20, 30, 70), Color::rgba(255, 255, 255, 80));
        for (slot, &i) in dock.iter().take(n as usize).enumerate() {
            let app = &self.apps[i];
            draw_tile(surface, &app.art, &app.name, hb_x + mpad + slot as i32 * (mw + mgap), hb_y + mpad, mw, mh);
        }
        let time = clock::now_text();
        let csize = 40.0 * s;
        surface.draw_text_ex(px(48.0), hb_y + hb_h / 2 - (csize * 0.68) as i32, &time, white, csize, Weight::Display);
        let nsize = 24.0 * s;
        let nw = text_width(&self.name, nsize, Weight::Display);
        surface.draw_text_ex(w - px(48.0) - nw, hb_y + hb_h / 2 - (nsize * 0.68) as i32, &self.name, white, nsize, Weight::Display);

        // Grid
        let tile_w = (w as f32 * 0.205) as i32;
        let tile_h = (tile_w as f32 * 0.57) as i32;
        let gap = (w as f32 * 0.0215) as i32;
        let x0 = (w - (LIB_COLS as i32 * tile_w + (LIB_COLS as i32 - 1) * gap)) / 2;
        let y0 = (h as f32 * 0.29) as i32 + gdy;
        let pitch = tile_h + (h as f32 * 0.10) as i32;
        let page = self.lib_sel / LIB_PER_PAGE;
        let first = page * LIB_PER_PAGE;
        let last = (first + LIB_PER_PAGE).min(self.apps.len());
        let cell = |i: usize| {
            let g = (i - first) as i32;
            (x0 + (g % LIB_COLS as i32) * (tile_w + gap), y0 + (g / LIB_COLS as i32) * pitch)
        };
        for i in first..last {
            if i != self.lib_sel {
                let (x, y) = cell(i);
                draw_tile(surface, &self.apps[i].art, &self.apps[i].name, x, y, tile_w, tile_h);
            }
        }
        if self.lib_sel < self.apps.len() {
            let app = &self.apps[self.lib_sel];
            let (x, y) = cell(self.lib_sel);
            let grow = (tile_w as f32 * 0.06) as i32;
            let (fx, fy, fw, fh) = (x - grow, y - grow, tile_w + 2 * grow, tile_h + 2 * grow);
            surface.draw_radial_glow(fx + fw / 2, fy + fh + px(6.0), fw * 3 / 5, Color::rgba(0, 0, 0, 110));
            draw_tile(surface, &app.art, &app.name, fx, fy, fw, fh);
            surface.stroke_rrect(fx - 2, fy - 2, (fw + 4) as u32, (fh + 4) as u32, fh * 16 / 100 + 2, 3.0, Color::rgba(255, 255, 255, 235));
            let nsize = 22.0 * s;
            surface.draw_text_ex(fx + (fw - text_width(&app.name, nsize, Weight::SemiBold)) / 2, fy + fh + px(12.0), &app.name, white, nsize, Weight::SemiBold);
        }

        // Page dots
        let pages = self.apps.len().div_ceil(LIB_PER_PAGE);
        if pages > 1 {
            let (r, step) = (px(5.0), px(20.0));
            let x_start = w / 2 - (pages as i32 - 1) * step / 2;
            for p in 0..pages {
                let a = if p == page { 255 } else { 110 };
                surface.fill_rrect(x_start + p as i32 * step - r, (h as f32 * 0.93) as i32 + gdy - r, (2 * r) as u32, (2 * r) as u32, r, Color::rgba(255, 255, 255, a));
            }
        }
    }
}

fn shade(color: u32) -> (Color, Color) {
    let c = Color::from_hex(color);
    (c.lerp(Color::white(), 0.16), c.lerp(Color::black(), 0.28))
}

fn draw_tile(s: &mut Surface, art: &Art, name: &str, x: i32, y: i32, w: i32, h: i32) {
    let r = h * 16 / 100;
    let u = h as f32 / 138.0;
    let (cx, cy) = (x + w / 2, y + h / 2);
    match art {
        Art::YouTube => {
            s.fill_rrect_gradient(x, y, w as u32, h as u32, r, Color::rgb(255, 255, 255), Color::rgb(236, 236, 238));
            let ic = (h as f32 * 0.42) as i32;
            let size = 25.0 * u;
            let tw = text_width("YouTube", size, Weight::SemiBold);
            let gap = (9.0 * u) as i32;
            let x0 = x + (w - (ic + gap + tw)) / 2;
            s.draw_icon(Icon::YouTube, x0 + ic / 2, cy, ic, Color::rgb(255, 0, 0));
            s.draw_text_ex(x0 + ic + gap, cy - (size * 0.6) as i32, "YouTube", Color::rgb(40, 40, 40), size, Weight::SemiBold);
        }
        Art::Netflix => {
            s.fill_rrect_gradient(x, y, w as u32, h as u32, r, Color::rgb(30, 14, 16), Color::rgb(10, 4, 6));
            s.draw_icon(Icon::Netflix, cx, cy, (h as f32 * 0.50) as i32, Color::rgb(229, 9, 20));
        }
        Art::Glyph(icon, color) => {
            let (top, bottom) = shade(*color);
            s.fill_rrect_gradient(x, y, w as u32, h as u32, r, top, bottom);
            s.draw_icon(*icon, cx, cy, (h as f32 * 0.46) as i32, Color::white());
        }
        Art::Letters(color) => {
            let (top, bottom) = shade(*color);
            s.fill_rrect_gradient(x, y, w as u32, h as u32, r, top, bottom);
            let letters: String = name.chars().filter(|c| c.is_alphanumeric()).take(2).collect::<String>();
            let letters = {
                let mut it = letters.chars();
                it.next().map(|f| f.to_uppercase().collect::<String>() + it.as_str()).unwrap_or_default()
            };
            let size = 54.0 * u;
            let tw = text_width(&letters, size, Weight::Bold);
            s.draw_text_ex(cx - tw / 2, cy - (size * 0.6) as i32, &letters, Color::white(), size, Weight::Bold);
        }
    }
}
