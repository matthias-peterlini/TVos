//! Top pill bar (home | Sorgenti | Impostazioni), the bottom chevron and small animation helpers.

use tvos_core::color::Color;
use tvos_core::surface::Surface;
use tvos_core::text::{text_width, Weight};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tab {
    Home,
    Sources,
    Settings,
}

impl Tab {
    pub const ALL: [Tab; 3] = [Tab::Home, Tab::Sources, Tab::Settings];

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn label(self) -> &'static str {
        match self {
            Tab::Home => "home",
            Tab::Sources => "Sorgenti",
            Tab::Settings => "Impostazioni",
        }
    }
}

/// Move `v` toward `target` with exponential smoothing; true while it is still moving.
pub fn ease(v: &mut f32, target: f32, dt: f32, rate: f32) -> bool {
    let d = target - *v;
    if d.abs() < 0.003 {
        *v = target;
        return false;
    }
    *v += d * (1.0 - (-dt * rate).exp());
    true
}

/// `pos` is the animated tab position (0.0 = home, 1.0 = Sorgenti, ...); `focused` rings the pill.
pub fn draw_bar(s: &mut Surface, pos: f32, focused: bool) {
    let (w, h) = (s.width as i32, s.height as i32);
    let k = h as f32 / 720.0;
    let px = |v: f32| (v * k).round() as i32;
    let size = 17.0 * k;
    let pad = px(24.0);
    let inset = px(6.0);
    let widths: Vec<i32> = Tab::ALL.iter().map(|t| text_width(t.label(), size, Weight::SemiBold) + 2 * pad).collect();
    let bar_w = widths.iter().sum::<i32>() + 2 * inset;
    let bar_h = px(48.0);
    let bar_x = (w - bar_w) / 2;
    let bar_y = px(24.0);
    s.draw_frosted_card(bar_x, bar_y, bar_w as u32, bar_h as u32, bar_h / 2, Color::rgba(40, 40, 50, 70), Color::rgba(255, 255, 255, 70));

    let mut xs = Vec::new();
    let mut x = bar_x + inset;
    for iw in &widths {
        xs.push(x);
        x += iw;
    }
    // The pill glides between the two neighbouring items
    let p = pos.clamp(0.0, (widths.len() - 1) as f32);
    let i = (p.floor() as usize).min(widths.len() - 2);
    let f = p - i as f32;
    let lerp = |a: i32, b: i32| (a as f32 + (b - a) as f32 * f).round() as i32;
    let (px_, pw) = (lerp(xs[i], xs[i + 1]), lerp(widths[i], widths[i + 1]));
    let (py, ph) = (bar_y + inset, bar_h - 2 * inset);
    s.fill_rrect(px_, py, pw as u32, ph as u32, ph / 2, Color::rgba(255, 255, 255, 235));
    if focused {
        s.stroke_rrect(px_ - 4, py - 4, (pw + 8) as u32, (ph + 8) as u32, ph / 2 + 4, 2.5, Color::rgba(255, 255, 255, 230));
    }
    for (n, tab) in Tab::ALL.iter().enumerate() {
        let near = (1.0 - (p - n as f32).abs()).clamp(0.0, 1.0);
        let col = Color::rgb(235, 235, 240).lerp(Color::rgb(25, 25, 32), near);
        let tw = text_width(tab.label(), size, Weight::SemiBold);
        s.draw_text_ex(xs[n] + (widths[n] - tw) / 2, bar_y + (bar_h as f32 * 0.5 - size * 0.68) as i32, tab.label(), col, size, Weight::SemiBold);
    }
}

/// The small "swipe down" hint at the bottom centre
pub fn draw_chevron(s: &mut Surface) {
    let k = s.height as f32 / 720.0;
    s.draw_chevron(s.width as i32 / 2, s.height as i32 - (26.0 * k) as i32, (26.0 * k) as i32, Color::rgba(255, 255, 255, 210));
}
