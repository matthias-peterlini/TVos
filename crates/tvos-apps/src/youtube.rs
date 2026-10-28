use crate::launcher::Launch;
use crate::youtube_api::{api_key, fetch_trending, Video};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tvos_core::color::Color;
use tvos_core::icons::Icon;
use tvos_core::input::RemoteKey;
use tvos_core::surface::Surface;
use tvos_core::text::{text_width, Weight};
use tvos_core::theme::Theme;

const RED: Color = Color::rgb(239, 68, 68);
const RED_SOFT: Color = Color::rgb(248, 113, 113);

enum Feed {
    NoKey,
    Loading,
    Ready(Vec<Video>),
    Failed(String),
}

pub struct YouTubeApp {
    feed: Arc<Mutex<Feed>>,
    pub selected_index: usize,
    top_row: usize,
    visible_rows: usize,
    pub detail_open: bool,
    pending: Option<Launch>,
}

impl YouTubeApp {
    pub fn new() -> Self {
        let feed = Arc::new(Mutex::new(Feed::Loading));
        match api_key() {
            None => *feed.lock().unwrap() = Feed::NoKey,
            Some(key) => {
                let shared = feed.clone();
                // Background refresh: retry soon after a failure (network or clock not ready at boot), otherwise every 15 min
                thread::spawn(move || loop {
                    let wait = match fetch_trending(&key) {
                        Ok(v) => {
                            *shared.lock().unwrap() = Feed::Ready(v);
                            Duration::from_secs(15 * 60)
                        }
                        Err(e) => {
                            let mut f = shared.lock().unwrap();
                            // keep showing the old list if we have one
                            if !matches!(*f, Feed::Ready(_)) {
                                *f = Feed::Failed(e);
                            }
                            Duration::from_secs(20)
                        }
                    };
                    thread::sleep(wait);
                });
            }
        }
        Self { feed, selected_index: 0, top_row: 0, visible_rows: 3, detail_open: false, pending: None }
    }

    /// The video the user chose to watch, to be opened in the browser
    pub fn take_launch(&mut self) -> Option<Launch> {
        self.pending.take()
    }

    fn count(&self) -> usize {
        match &*self.feed.lock().unwrap() {
            Feed::Ready(v) => v.len(),
            _ => 0,
        }
    }

    pub fn handle_key(&mut self, key: RemoteKey) -> bool {
        if self.detail_open {
            match key {
                RemoteKey::Back => self.detail_open = false,
                RemoteKey::Select => {
                    if let Feed::Ready(v) = &*self.feed.lock().unwrap() {
                        if let Some(video) = v.get(self.selected_index) {
                            self.pending = Some(Launch::Web {
                                name: video.title.clone(),
                                url: format!("https://www.youtube.com/tv#/watch?v={}", video.id),
                            });
                        }
                    }
                }
                _ => return false,
            }
            return true;
        }
        let n = self.count();
        if n == 0 {
            return false;
        }
        let i = self.selected_index;
        match key {
            RemoteKey::Up if i >= 2 => self.selected_index -= 2,
            RemoteKey::Down if i + 2 < n => self.selected_index += 2,
            RemoteKey::Left if i % 2 == 1 => self.selected_index -= 1,
            RemoteKey::Right if i % 2 == 0 && i + 1 < n => self.selected_index += 1,
            RemoteKey::Select => self.detail_open = true,
            _ => return false,
        }
        true
    }

    pub fn render(&mut self, surface: &mut Surface) {
        let w = surface.width as i32;
        let h = surface.height as i32;

        surface.clear(Theme::BG_BASE);
        surface.draw_radial_glow(w / 4, h / 3, 350, Color::rgba(239, 68, 68, 45));
        surface.draw_radial_glow((3 * w) / 4, (2 * h) / 3, 320, Theme::ORB_AMBER);

        surface.draw_icon(Icon::YouTube, 82, 66, 36, RED);
        surface.draw_text_ex(112, 46, "YouTube", Theme::TEXT_PRIMARY, 30.0, Weight::Bold);
        surface.draw_badge(
            112 + text_width("YouTube", 30.0, Weight::Bold) + 16,
            50,
            "Più visti in Italia",
            Color::rgba(239, 68, 68, 30),
            Color::rgba(239, 68, 68, 80),
            RED_SOFT,
        );
        surface.draw_text(60, 98, "Frecce per scegliere, [OK] per i dettagli, [BACK] per tornare alla Home.", Theme::TEXT_MUTED, 1);

        let feed = self.feed.lock().unwrap();
        let videos = match &*feed {
            Feed::Ready(v) => v,
            other => {
                let (title, detail) = match other {
                    Feed::NoKey => ("Chiave API di YouTube non trovata", "Copiala in /etc/tvos/youtube.key e riavvia.".to_string()),
                    Feed::Loading => ("Caricamento in corso...", "Sto scaricando i video più visti.".to_string()),
                    Feed::Failed(e) => ("Impossibile caricare i video", format!("{e}. Riprovo tra pochi secondi.")),
                    Feed::Ready(_) => unreachable!(),
                };
                surface.draw_glass_card(60, 140, (w - 120) as u32, 120, 24, Theme::CARD_BG, Theme::CARD_BORDER);
                surface.draw_text_ex(90, 165, title, Theme::TEXT_PRIMARY, 22.0, Weight::SemiBold);
                surface.draw_text_ex(90, 205, &detail, Theme::TEXT_SECONDARY, 15.0, Weight::Regular);
                return;
            }
        };

        if self.detail_open {
            if let Some(v) = videos.get(self.selected_index) {
                draw_detail(surface, v);
            }
            return;
        }

        // Grid: 2 columns, as many rows as fit; scrolls to keep the selection visible
        let gap = 20;
        let start_x = 60;
        let start_y = 135;
        let card_w = ((w - 120 - gap) / 2) as u32;
        let card_h = 150u32;
        self.visible_rows = (((h - start_y - 30) / (card_h as i32 + gap)).max(1)) as usize;
        let sel_row = self.selected_index / 2;
        if sel_row < self.top_row {
            self.top_row = sel_row;
        } else if sel_row >= self.top_row + self.visible_rows {
            self.top_row = sel_row + 1 - self.visible_rows;
        }

        for (i, v) in videos.iter().enumerate() {
            let row = i / 2;
            if row < self.top_row || row >= self.top_row + self.visible_rows {
                continue;
            }
            let cx = start_x + (i % 2) as i32 * (card_w as i32 + gap);
            let cy = start_y + (row - self.top_row) as i32 * (card_h as i32 + gap);
            let focused = self.selected_index == i;
            if focused {
                surface.draw_radial_glow(cx + card_w as i32 / 2, cy + card_h as i32 / 2, 200, Color::rgba(239, 68, 68, 70));
            }
            let bg = if focused { Color::rgba(35, 18, 22, 235) } else { Theme::CARD_BG };
            let border = if focused { Color::rgba(239, 68, 68, 220) } else { Theme::CARD_BORDER };
            surface.draw_glass_card(cx, cy, card_w, card_h, 20, bg, border);

            // Thumbnail placeholder (16:9)
            let (tw, th) = (176u32, 99u32);
            let (tx, ty) = (cx + 20, cy + 20);
            surface.draw_glass_card(tx, ty, tw, th, 12, Color::rgba(239, 68, 68, 25), Color::rgba(239, 68, 68, 60));
            surface.draw_icon(Icon::YouTube, tx + tw as i32 / 2, ty + th as i32 / 2 - 6, 34, Color::rgba(248, 113, 113, 200));
            let dw = text_width(&v.duration, 12.0, Weight::SemiBold) as u32 + 14;
            surface.fill_rrect(tx + tw as i32 - dw as i32 - 6, ty + th as i32 - 24, dw, 18, 6, Color::rgba(0, 0, 0, 200));
            surface.draw_text_ex(tx + tw as i32 - dw as i32 + 1, ty + th as i32 - 22, &v.duration, Theme::TEXT_PRIMARY, 12.0, Weight::SemiBold);

            // Text
            let info_x = tx + tw as i32 + 18;
            let info_w = (cx + card_w as i32 - 20) - info_x;
            for (n, line) in wrap_px(&v.title, 15.0, Weight::SemiBold, info_w, 3).iter().enumerate() {
                surface.draw_text_ex(info_x, cy + 18 + n as i32 * 21, line, Theme::TEXT_PRIMARY, 15.0, Weight::SemiBold);
            }
            let ch = wrap_px(&v.channel, 12.5, Weight::Regular, info_w, 1);
            surface.draw_text_ex(info_x, cy + 88, ch.first().map_or("", |s| s.as_str()), Theme::TEXT_SECONDARY, 12.5, Weight::Regular);
            let vw = wrap_px(&v.views, 12.5, Weight::Regular, info_w, 1);
            surface.draw_text_ex(info_x, cy + 108, vw.first().map_or("", |s| s.as_str()), Theme::TEXT_MUTED, 12.5, Weight::Regular);
        }
    }
}

fn draw_detail(surface: &mut Surface, v: &Video) {
    let w = surface.width as i32;
    let (x, y) = (60, 135);
    let cw = (w - 120) as u32;
    surface.draw_glass_card(x, y, cw, 360, 24, Color::rgba(14, 10, 14, 240), Color::rgba(239, 68, 68, 140));
    let inner = cw as i32 - 80;
    for (n, line) in wrap_px(&v.title, 28.0, Weight::Bold, inner, 3).iter().enumerate() {
        surface.draw_text_ex(x + 40, y + 36 + n as i32 * 38, line, Theme::TEXT_PRIMARY, 28.0, Weight::Bold);
    }
    surface.draw_text_ex(x + 40, y + 160, &v.channel, RED_SOFT, 18.0, Weight::SemiBold);
    surface.draw_text_ex(x + 40, y + 192, &format!("{}  -  durata {}", v.views, v.duration), Theme::TEXT_SECONDARY, 15.0, Weight::Regular);
    surface.draw_text_ex(x + 40, y + 232, &format!("youtube.com/watch?v={}", v.id), Theme::TEXT_AMBER, 16.0, Weight::Regular);
    surface.fill_rrect(x + 40, y + 282, 250, 44, 22, Color::rgba(239, 68, 68, 230));
    surface.draw_text_ex(x + 40 + 28, y + 292, "[OK]  Guarda", Color::white(), 17.0, Weight::SemiBold);
    surface.draw_text_ex(x + 320, y + 293, "[BACK] torna all'elenco  -  [HOME] chiude il video", Theme::TEXT_MUTED, 14.0, Weight::Regular);
}

/// Greedy word wrap by pixel width; the last allowed line is cut with an ellipsis.
fn wrap_px(text: &str, size: f32, weight: Weight, max_w: i32, max_lines: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut cur = String::new();
    for word in text.split_whitespace() {
        let candidate = if cur.is_empty() { word.to_string() } else { format!("{cur} {word}") };
        if text_width(&candidate, size, weight) <= max_w || cur.is_empty() {
            cur = candidate;
        } else {
            lines.push(std::mem::take(&mut cur));
            cur = word.to_string();
        }
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    if lines.len() > max_lines {
        lines.truncate(max_lines);
    }
    // Hard-cut any line that is still too wide (very long single words) or was truncated
    let truncated = text.split_whitespace().count() > lines.iter().map(|l| l.split_whitespace().count()).sum::<usize>();
    let last = lines.len().saturating_sub(1);
    for (i, l) in lines.iter_mut().enumerate() {
        let must_ellipsize = (i == last && truncated) || text_width(l, size, weight) > max_w;
        if must_ellipsize {
            while text_width(&format!("{l}…"), size, weight) > max_w && l.pop().is_some() {}
            l.push('…');
        }
    }
    lines
}
