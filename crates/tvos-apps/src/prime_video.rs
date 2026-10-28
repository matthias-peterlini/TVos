use tvos_core::color::Color;
use tvos_core::input::RemoteKey;
use tvos_core::surface::Surface;
use tvos_core::theme::Theme;

pub struct PrimeItem {
    pub title: &'static str,
    pub genre: &'static str,
    pub match_pct: &'static str,
}

pub struct PrimeVideoApp {
    pub selected_index: usize,
    pub is_streaming: bool,
    pub items: Vec<PrimeItem>,
}

impl PrimeVideoApp {
    pub fn new() -> Self {
        Self {
            selected_index: 0,
            is_streaming: false,
            items: vec![
                PrimeItem {
                    title: "The Boys",
                    genre: "Azione / Fantascienza",
                    match_pct: "98% Compatibile",
                },
                PrimeItem {
                    title: "Il Signore degli Anelli",
                    genre: "Fantasy / Avventura",
                    match_pct: "95% Compatibile",
                },
                PrimeItem {
                    title: "Fallout",
                    genre: "Post-Apocalittico",
                    match_pct: "99% Compatibile",
                },
                PrimeItem {
                    title: "Reacher",
                    genre: "Crimine / Thriller",
                    match_pct: "92% Compatibile",
                },
            ],
        }
    }

    pub fn handle_key(&mut self, key: RemoteKey) -> bool {
        if self.is_streaming {
            match key {
                RemoteKey::Select | RemoteKey::PlayPause | RemoteKey::Back => {
                    self.is_streaming = false;
                    return true;
                }
                _ => return false,
            }
        }

        match key {
            RemoteKey::Left => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
                true
            }
            RemoteKey::Right => {
                if self.selected_index + 1 < self.items.len() {
                    self.selected_index += 1;
                }
                true
            }
            RemoteKey::Select => {
                self.is_streaming = true;
                true
            }
            _ => false,
        }
    }

    pub fn render(&self, surface: &mut Surface) {
        let w = surface.width as i32;
        let h = surface.height as i32;

        let prime_blue = Color::rgb(0, 168, 225); // #00a8e1
        let prime_bg = Color::rgba(14, 23, 36, 230);

        surface.clear(Theme::BG_BASE);
        surface.draw_radial_glow(w / 3, h / 3, 380, Color::rgba(0, 168, 225, 45));
        surface.draw_radial_glow((2 * w) / 3, (2 * h) / 3, 320, Theme::ORB_VIOLET);

        // Header
        surface.draw_text(60, 45, "prime video", prime_blue, 3);
        surface.draw_badge(
            340,
            50,
            "Widevine DRM L3 Certificato",
            Color::rgba(0, 168, 225, 30),
            Color::rgba(0, 168, 225, 80),
            prime_blue,
        );
        surface.draw_text(60, 100, "Incluso con Prime. Scegli un titolo e premi [OK] per riprodurre.", Theme::TEXT_MUTED, 1);

        if self.is_streaming {
            let cur_item = &self.items[self.selected_index];
            let card_w = (w - 120) as u32;
            let card_h = (h - 180) as u32;
            let card_x = 60;
            let card_y = 130;

            surface.draw_glass_card(card_x, card_y, card_w, card_h, 24, Color::rgba(10, 15, 25, 250), prime_blue);

            surface.draw_text(card_x + 60, card_y + 50, "[ STREAMING PROTETTO ATTIVO ]", prime_blue, 2);
            surface.draw_text(card_x + 60, card_y + 100, cur_item.title, Theme::TEXT_PRIMARY, 3);
            surface.draw_text(card_x + 60, card_y + 160, cur_item.genre, Theme::TEXT_SECONDARY, 1);
            surface.draw_text(card_x + 60, card_y + 195, "Flusso crittografato AES-128 con Widevine CDM ARMv8", Theme::EMERALD, 1);
            surface.draw_text(card_x + 60, card_y + 225, "Decodifica H.264 / AAC 5.1 su HDMI", Theme::TEXT_MUTED, 1);

            surface.draw_text(card_x + 60, card_y + (card_h as i32) - 50, "Premi [OK] o [BACK] per tornare al catalogo", Theme::TEXT_AMBER, 1);
            return;
        }

        // Horizontal Row of Movies/Series
        let card_w = 265u32;
        let card_h = 360u32;
        let gap = 30;
        let start_x = 60;
        let start_y = 150;

        for (i, item) in self.items.iter().enumerate() {
            let cx = start_x + (i as i32) * (card_w as i32 + gap);
            let is_focused = self.selected_index == i;

            if is_focused {
                surface.draw_radial_glow(cx + (card_w as i32) / 2, start_y + (card_h as i32) / 2, 240, Color::rgba(0, 168, 225, 80));
            }

            let bg = if is_focused { Color::rgba(16, 36, 58, 240) } else { prime_bg };
            let border = if is_focused { prime_blue } else { Theme::CARD_BORDER };

            surface.draw_glass_card(cx, start_y, card_w, card_h, 22, bg, border);

            // Poster box
            let poster_w = (card_w - 30) as u32;
            let poster_h = 200u32;
            let poster_x = cx + 15;
            let poster_y = start_y + 15;
            surface.draw_glass_card(poster_x, poster_y, poster_w, poster_h, 14, Color::rgba(0, 168, 225, 30), Color::rgba(0, 168, 225, 70));
            surface.draw_text(poster_x + 30, poster_y + 90, "[ PRIME POSTER ]", Color::rgb(147, 197, 253), 1);

            // Details
            surface.draw_text(cx + 20, start_y + 235, item.title, Theme::TEXT_PRIMARY, 2);
            surface.draw_text(cx + 20, start_y + 275, item.genre, Theme::TEXT_SECONDARY, 1);
            surface.draw_text(cx + 20, start_y + 300, item.match_pct, Theme::EMERALD, 1);

            if is_focused {
                surface.draw_text(cx + 20, start_y + 330, ">> Premi [OK] per guardare", prime_blue, 1);
            }
        }
    }
}
