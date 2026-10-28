use tvos_core::color::Color;
use tvos_core::input::RemoteKey;
use tvos_core::surface::Surface;
use tvos_core::theme::Theme;

pub struct MediaPlayer {
    pub is_playing: bool,
    pub current_track: usize,
    pub progress: f32,
    pub volume: u8,
}

impl MediaPlayer {
    pub fn new() -> Self {
        Self {
            is_playing: true,
            current_track: 0,
            progress: 0.35,
            volume: 80,
        }
    }

    pub fn handle_key(&mut self, key: RemoteKey) -> bool {
        match key {
            RemoteKey::PlayPause | RemoteKey::Select => {
                self.is_playing = !self.is_playing;
                true
            }
            RemoteKey::Right => {
                self.progress = (self.progress + 0.05).min(1.0);
                true
            }
            RemoteKey::Left => {
                self.progress = (self.progress - 0.05).max(0.0);
                true
            }
            RemoteKey::VolumeUp => {
                self.volume = (self.volume + 5).min(100);
                true
            }
            RemoteKey::VolumeDown => {
                self.volume = self.volume.saturating_sub(5);
                true
            }
            _ => false,
        }
    }

    pub fn update(&mut self) {
        if self.is_playing {
            self.progress += 0.001;
            if self.progress > 1.0 {
                self.progress = 0.0;
                self.current_track = (self.current_track + 1) % 3;
            }
        }
    }

    pub fn render(&self, surface: &mut Surface) {
        let w = surface.width as i32;
        let h = surface.height as i32;

        surface.clear(Theme::BG_BASE);
        surface.draw_radial_glow(w / 3, h / 2, 350, Theme::ORB_VIOLET);
        surface.draw_radial_glow((2 * w) / 3, h / 2, 300, Theme::ORB_AMBER);

        // Header
        surface.draw_text(60, 50, "ticly // Media Player", Theme::TEXT_PRIMARY, 2);
        surface.draw_text(60, 90, "Premi [BACK] sul telecomando per tornare alla Home", Theme::TEXT_MUTED, 1);

        // Glassmorphic Player Card
        let card_w = 780u32;
        let card_h = 420u32;
        let card_x = (w - card_w as i32) / 2;
        let card_y = 140;

        surface.draw_glass_card(card_x, card_y, card_w, card_h, 28, Theme::CARD_BG, Theme::CARD_BORDER);

        // Album Art Box
        let art_size = 200u32;
        let art_x = card_x + 50;
        let art_y = card_y + 50;
        surface.draw_glass_card(art_x, art_y, art_size, art_size, 20, Color::rgba(139, 92, 246, 60), Theme::VIOLET);
        surface.draw_text(art_x + 45, art_y + 85, "[ MUSIC ]", Theme::TEXT_PRIMARY, 2);

        // Track Info
        let tracks = [
            ("Ambient Horizon (Direct Bare-Metal)", "Rust Sound Engine - 48kHz Stereo"),
            ("Synthwave Odyssey", "Broadcom VideoCore Stream"),
            ("Neon Lights Lounge", "ticly Audio Live Network"),
        ];
        let (title, artist) = tracks[self.current_track];

        let info_x = art_x + (art_size as i32) + 50;
        surface.draw_text(info_x, art_y + 30, title, Theme::TEXT_PRIMARY, 2);
        surface.draw_text(info_x, art_y + 75, artist, Theme::TEXT_SECONDARY, 1);

        // State badge
        let state_txt = if self.is_playing { "IN RIPRODUZIONE" } else { "IN PAUSA" };
        let state_col = if self.is_playing { Theme::EMERALD } else { Theme::AMBER };
        surface.draw_badge(info_x, art_y + 115, state_txt, Theme::BADGE_BG, Theme::BADGE_BORDER, state_col);

        // Progress bar
        let bar_x = card_x + 50;
        let bar_y = card_y + 280;
        let bar_w = (card_w - 100) as u32;
        let bar_h = 10u32;

        // Background bar
        surface.fill_rect(bar_x, bar_y, bar_w, bar_h, Color::rgba(255, 255, 255, 30));
        // Fill bar
        let fill_w = ((bar_w as f32) * self.progress) as u32;
        surface.fill_rect(bar_x, bar_y, fill_w, bar_h, Theme::AMBER);

        // Controls hint
        let hint_y = card_y + 320;
        let play_icon = if self.is_playing { "[||] PAUSA (OK)" } else { "[>] RIPRODUCI (OK)" };
        surface.draw_text(bar_x, hint_y, play_icon, Theme::TEXT_AMBER, 1);
        surface.draw_text(bar_x + 220, hint_y, "[SX/DX] Salta avanti/indietro", Theme::TEXT_SECONDARY, 1);
        let vol_str = format!("Volume TV: {}%", self.volume);
        surface.draw_text(bar_x + 550, hint_y, &vol_str, Theme::TEXT_PRIMARY, 1);
    }
}
