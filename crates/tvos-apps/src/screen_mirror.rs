use tvos_core::color::Color;
use tvos_core::input::RemoteKey;
use tvos_core::surface::Surface;
use tvos_core::theme::Theme;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CastProtocol {
    SamsungDeX,
    AndroidMiracast,
    AppleAirPlay,
}

pub struct ScreenMirrorApp {
    pub is_connected: bool,
    pub device_name: String,
    pub active_protocol: CastProtocol,
    pub resolution: &'static str,
    pub fps: u32,
    pub latency_ms: u32,
}

impl ScreenMirrorApp {
    pub fn new() -> Self {
        Self {
            is_connected: false,
            device_name: "Samsung Galaxy S24 Ultra".to_string(),
            active_protocol: CastProtocol::SamsungDeX,
            resolution: "1920x1080 Full HD",
            fps: 60,
            latency_ms: 28,
        }
    }

    pub fn handle_key(&mut self, key: RemoteKey) -> bool {
        match key {
            RemoteKey::Select => {
                // Toggle simulated connection
                self.is_connected = !self.is_connected;
                true
            }
            RemoteKey::Back => {
                if self.is_connected {
                    self.is_connected = false;
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    pub fn render(&self, surface: &mut Surface) {
        let w = surface.width as i32;
        let h = surface.height as i32;

        let dex_blue = Color::rgb(37, 99, 235); // #2563eb
        let dex_cyan = Color::rgb(6, 182, 212);

        surface.clear(Theme::BG_BASE);
        surface.draw_radial_glow(w / 3, h / 3, 380, Color::rgba(37, 99, 235, 45));
        surface.draw_radial_glow((2 * w) / 3, (2 * h) / 3, 340, Theme::ORB_AMBER);

        // Header
        surface.draw_text(60, 45, "ticly // Wireless DeX & Schermo Telefono", Theme::TEXT_PRIMARY, 2);
        surface.draw_badge(
            620,
            45,
            "Miracast WFD + AirPlay + DeX UIBC",
            Color::rgba(37, 99, 235, 30),
            Color::rgba(37, 99, 235, 80),
            dex_cyan,
        );
        surface.draw_text(60, 85, "Condividi lo schermo o usa la modalità Desktop Samsung DeX senza cavi.", Theme::TEXT_MUTED, 1);

        if self.is_connected {
            // Active Casting Session
            let card_w = (w - 120) as u32;
            let card_h = (h - 180) as u32;
            let card_x = 60;
            let card_y = 120;

            surface.draw_glass_card(card_x, card_y, card_w, card_h, 24, Color::rgba(10, 16, 28, 250), dex_blue);

            // Simulation Desktop Box
            let box_w = (card_w - 60) as u32;
            let box_h = (card_h - 130) as u32;
            surface.draw_glass_card(card_x + 30, card_y + 20, box_w, box_h, 16, Color::rgb(8, 12, 20), Color::rgba(255, 255, 255, 20));

            // Samsung DeX Desktop Canvas Mockup
            surface.draw_text(card_x + 60, card_y + 50, "[ SAMSUNG DeX WIRELESS ATTIVO ]", dex_cyan, 2);
            let dev_str = format!("Dispositivo collegato: {}", self.device_name);
            surface.draw_text(card_x + 60, card_y + 95, &dev_str, Theme::TEXT_PRIMARY, 2);

            let stats = format!(
                "Risoluzione: {} | {} FPS | Latenza: {}ms | Decodifica: H.264 HW VideoCore IV",
                self.resolution, self.fps, self.latency_ms
            );
            surface.draw_text(card_x + 60, card_y + 140, &stats, Theme::EMERALD, 1);
            surface.draw_text(
                card_x + 60,
                card_y + 175,
                "Canale di Ritorno UIBC: Puoi usare il telecomando TV o mouse/tastiera Bluetooth!",
                Theme::TEXT_SECONDARY,
                1,
            );

            // Control hint
            surface.draw_text(
                card_x + 40,
                card_y + (card_h as i32) - 45,
                "Premi [OK] o [BACK] sul telecomando per disconnettere la sessione DeX",
                Theme::TEXT_AMBER,
                1,
            );
            return;
        }

        // Standby Screen: How to connect guide
        let card_w = (w - 120) as u32;
        let card_h = 440u32;
        let card_x = 60;
        let card_y = 130;

        surface.draw_glass_card(card_x, card_y, card_w, card_h, 24, Theme::CARD_BG, Theme::CARD_BORDER);

        // Receiver state badge
        surface.draw_badge(
            card_x + 40,
            card_y + 35,
            "RICEVITORE IN ASCOLTO (SSID: TVos-DeX-Sink)",
            Color::rgba(16, 185, 129, 25),
            Color::rgba(16, 185, 129, 70),
            Theme::EMERALD,
        );

        surface.draw_text(
            card_x + 40,
            card_y + 75,
            "Collega il tuo smartphone alla TV in un istante:",
            Theme::TEXT_PRIMARY,
            2,
        );

        // 3 Guide Columns
        let col_w = 340u32;
        let col_h = 240u32;
        let col_gap = 25;
        let col_y = card_y + 125;

        // Column 1: Samsung DeX
        let col1_x = card_x + 40;
        surface.draw_glass_card(col1_x, col_y, col_w, col_h, 18, Color::rgba(37, 99, 235, 18), dex_blue);
        surface.draw_text(col1_x + 20, col_y + 20, "SAMSUNG DeX WIRELESS", dex_cyan, 1);
        surface.draw_text(col1_x + 20, col_y + 45, "Desktop PC su TV", Theme::TEXT_PRIMARY, 2);
        surface.draw_text(col1_x + 20, col_y + 90, "1. Scorri giu dal bordo in alto", Theme::TEXT_SECONDARY, 1);
        surface.draw_text(col1_x + 20, col_y + 115, "2. Tocca l'icona 'DeX'", Theme::TEXT_SECONDARY, 1);
        surface.draw_text(col1_x + 20, col_y + 140, "3. Seleziona 'ticly TVos'", Theme::TEXT_SECONDARY, 1);
        surface.draw_text(col1_x + 20, col_y + 185, ">> Supporta mouse e tastiera!", dex_cyan, 1);

        // Column 2: Android Smart View / Miracast
        let col2_x = col1_x + (col_w as i32) + col_gap;
        surface.draw_glass_card(col2_x, col_y, col_w, col_h, 18, Color::rgba(255, 255, 255, 6), Theme::CARD_BORDER);
        surface.draw_text(col2_x + 20, col_y + 20, "ANDROID SCREEN CAST", Theme::TEXT_AMBER, 1);
        surface.draw_text(col2_x + 20, col_y + 45, "Smart View / Mirror", Theme::TEXT_PRIMARY, 2);
        surface.draw_text(col2_x + 20, col_y + 90, "1. Apri i Quick Settings", Theme::TEXT_SECONDARY, 1);
        surface.draw_text(col2_x + 20, col_y + 115, "2. Premi 'Trasmetti schermo'", Theme::TEXT_SECONDARY, 1);
        surface.draw_text(col2_x + 20, col_y + 140, "3. Tocca 'ticly TVos'", Theme::TEXT_SECONDARY, 1);
        surface.draw_text(col2_x + 20, col_y + 185, ">> Duplicazione speculare live", Theme::TEXT_MUTED, 1);

        // Column 3: Apple iPhone / iPad AirPlay
        let col3_x = col2_x + (col_w as i32) + col_gap;
        surface.draw_glass_card(col3_x, col_y, col_w, col_h, 18, Color::rgba(255, 255, 255, 6), Theme::CARD_BORDER);
        surface.draw_text(col3_x + 20, col_y + 20, "APPLE AIRPLAY", Color::rgb(244, 114, 182), 1);
        surface.draw_text(col3_x + 20, col_y + 45, "iPhone & iPad", Theme::TEXT_PRIMARY, 2);
        surface.draw_text(col3_x + 20, col_y + 90, "1. Apri Centro di Controllo", Theme::TEXT_SECONDARY, 1);
        surface.draw_text(col3_x + 20, col_y + 115, "2. Premi 'Duplica Schermo'", Theme::TEXT_SECONDARY, 1);
        surface.draw_text(col3_x + 20, col_y + 140, "3. Seleziona 'ticly TVos'", Theme::TEXT_SECONDARY, 1);
        surface.draw_text(col3_x + 20, col_y + 185, ">> Audio e video streaming", Theme::TEXT_MUTED, 1);

        // Test hint
        surface.draw_text(
            card_x + 40,
            card_y + 390,
            ">> Premi [OK] sul telecomando per simulare la connessione immediata di Samsung DeX!",
            Theme::TEXT_AMBER,
            1,
        );
    }
}
