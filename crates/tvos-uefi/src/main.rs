#![no_std]
#![no_main]

mod efi;

use core::panic::PanicInfo;
use core::ptr::null_mut;
use efi::*;

#[repr(C)]
pub struct InputKey {
    pub scan_code: u16,
    pub unicode_char: u16,
}

#[repr(C)]
pub struct TextInput {
    pub reset: unsafe extern "efiapi" fn(*mut TextInput, bool) -> Status,
    pub read_key_stroke: unsafe extern "efiapi" fn(*mut TextInput, *mut InputKey) -> Status,
    pub wait_for_key: Handle,
}

static FONT_8X16: &[u8; 95 * 16] = include_bytes!("font8x16.bin");

#[derive(Clone, Copy, PartialEq, Eq)]
enum Screen {
    Launcher,
    YouTube,
    PrimeVideo,
    SamsungDeX,
    MediaPlayer,
    Monitor,
    Settings,
}

struct UefiFramebuffer {
    base: *mut u32,
    backbuffer: *mut u32,
    width: usize,
    height: usize,
    stride: usize,
}

impl UefiFramebuffer {
    #[inline(always)]
    fn blend_pixel(&mut self, x: usize, y: usize, color: u32) {
        if x < self.width && y < self.height {
            let idx = y * self.stride + x;
            let src_a = (color >> 24) & 0xFF;
            if src_a == 255 {
                unsafe { self.backbuffer.add(idx).write(color) };
            } else if src_a > 0 {
                let dst = unsafe { self.backbuffer.add(idx).read() };
                let inv_a = 255 - src_a;

                let src_r = (color >> 16) & 0xFF;
                let src_g = (color >> 8) & 0xFF;
                let src_b = color & 0xFF;

                let dst_r = (dst >> 16) & 0xFF;
                let dst_g = (dst >> 8) & 0xFF;
                let dst_b = dst & 0xFF;

                let r = (src_r * src_a + dst_r * inv_a) / 255;
                let g = (src_g * src_a + dst_g * inv_a) / 255;
                let b = (src_b * src_a + dst_b * inv_a) / 255;

                let blended = (0xFF << 24) | (r << 16) | (g << 8) | b;
                unsafe { self.backbuffer.add(idx).write(blended) };
            }
        }
    }

    fn fill_rect(&mut self, x: i32, y: i32, w: u32, h: u32, color: u32) {
        let x0 = x.max(0) as usize;
        let y0 = y.max(0) as usize;
        let x1 = ((x + w as i32).max(0) as usize).min(self.width);
        let y1 = ((y + h as i32).max(0) as usize).min(self.height);

        let alpha = (color >> 24) & 0xFF;
        if alpha == 255 {
            for cy in y0..y1 {
                for cx in x0..x1 {
                    unsafe { self.backbuffer.add(cy * self.stride + cx).write(color) };
                }
            }
        } else {
            for cy in y0..y1 {
                for cx in x0..x1 {
                    self.blend_pixel(cx, cy, color);
                }
            }
        }
    }

    fn draw_glass_card(&mut self, x: i32, y: i32, w: u32, h: u32, radius: i32, bg: u32, border: u32) {
        let w_i = w as i32;
        let h_i = h as i32;
        let r = radius.min(w_i / 2).min(h_i / 2);

        let x0 = x.max(0);
        let y0 = y.max(0);
        let x1 = (x + w_i).min(self.width as i32);
        let y1 = (y + h_i).min(self.height as i32);

        for cy in y0..y1 {
            let dy = cy - y;
            for cx in x0..x1 {
                let dx = cx - x;
                let mut dist_to_corner: Option<f32> = None;

                if dx < r && dy < r {
                    let cdx = (r - dx) as f32;
                    let cdy = (r - dy) as f32;
                    dist_to_corner = Some(sqrt_approx(cdx * cdx + cdy * cdy));
                } else if dx >= w_i - r && dy < r {
                    let cdx = (dx - (w_i - r)) as f32;
                    let cdy = (r - dy) as f32;
                    dist_to_corner = Some(sqrt_approx(cdx * cdx + cdy * cdy));
                } else if dx < r && dy >= h_i - r {
                    let cdx = (r - dx) as f32;
                    let cdy = (dy - (h_i - r)) as f32;
                    dist_to_corner = Some(sqrt_approx(cdx * cdx + cdy * cdy));
                } else if dx >= w_i - r && dy >= h_i - r {
                    let cdx = (dx - (w_i - r)) as f32;
                    let cdy = (dy - (h_i - r)) as f32;
                    dist_to_corner = Some(sqrt_approx(cdx * cdx + cdy * cdy));
                }

                let inside = match dist_to_corner {
                    None => true,
                    Some(d) => d <= r as f32,
                };

                if inside {
                    let is_border = match dist_to_corner {
                        Some(d) => d >= (r as f32 - 1.5),
                        None => dx == 0 || dx == w_i - 1 || dy == 0 || dy == h_i - 1,
                    };
                    self.blend_pixel(cx as usize, cy as usize, if is_border { border } else { bg });
                }
            }
        }
    }

    fn draw_radial_glow(&mut self, cx: i32, cy: i32, radius: i32, color: u32) {
        let x0 = (cx - radius).max(0);
        let y0 = (cy - radius).max(0);
        let x1 = (cx + radius).min(self.width as i32 - 1);
        let y1 = (cy + radius).min(self.height as i32 - 1);

        let r2 = (radius * radius) as f32;
        let base_alpha = ((color >> 24) & 0xFF) as f32;
        let rgb = color & 0x00FFFFFF;

        for y in y0..=y1 {
            let dy = y - cy;
            let dy2 = (dy * dy) as f32;
            for x in x0..=x1 {
                let dx = x - cx;
                let dist2 = (dx * dx) as f32 + dy2;
                if dist2 <= r2 {
                    let factor = (1.0 - (dist2 / r2)) * (1.0 - (dist2 / r2));
                    let a = (base_alpha * factor) as u32;
                    if a > 0 {
                        self.blend_pixel(x as usize, y as usize, (a << 24) | rgb);
                    }
                }
            }
        }
    }

    fn draw_text(&mut self, start_x: i32, start_y: i32, text: &str, color: u32, scale: usize) {
        let scale = scale.max(1);
        let mut cur_x = start_x;
        let mut cur_y = start_y;

        for ch in text.chars() {
            if ch == '\n' {
                cur_y += (16 * scale + 4) as i32;
                cur_x = start_x;
                continue;
            }

            let ascii = ch as usize;
            let glyph = if (32..127).contains(&ascii) {
                &FONT_8X16[(ascii - 32) * 16..(ascii - 32 + 1) * 16]
            } else {
                &[0x3C, 0x7E, 0xFF, 0xFF, 0xFF, 0xFF, 0x7E, 0x3C, 0, 0, 0, 0, 0, 0, 0, 0]
            };

            for (row_idx, &row_byte) in glyph.iter().enumerate() {
                for col_idx in 0..8 {
                    if ((row_byte >> (7 - col_idx)) & 1) == 1 {
                        let px = cur_x + (col_idx * scale) as i32;
                        let py = cur_y + (row_idx * scale) as i32;
                        if scale == 1 {
                            self.blend_pixel(px as usize, py as usize, color);
                        } else {
                            self.fill_rect(px, py, scale as u32, scale as u32, color);
                        }
                    }
                }
            }
            cur_x += (8 * scale) as i32;
        }
    }

    fn draw_badge(&mut self, x: i32, y: i32, text: &str, bg: u32, border: u32, text_color: u32) {
        let text_w = (text.len() * 8) as u32;
        let badge_w = text_w + 24;
        self.draw_glass_card(x, y, badge_w, 24, 12, bg, border);
        self.draw_text(x + 12, y + 4, text, text_color, 1);
    }

    fn flip(&mut self) {
        unsafe {
            core::ptr::copy_nonoverlapping(self.backbuffer, self.base, self.height * self.stride);
        }
    }
}

fn sqrt_approx(x: f32) -> f32 {
    let mut i = x.to_bits();
    i = 0x1fbd1df5 + (i >> 1);
    let mut y = f32::from_bits(i);
    y = 0.5 * (y + x / y);
    y = 0.5 * (y + x / y);
    y
}

#[no_mangle]
pub extern "efiapi" fn efi_main(_image: Handle, system_table: *const SystemTable) -> Status {
    let st = unsafe { &*system_table };
    let bs = unsafe { &*st.boot_services };

    // Locate GraphicsOutputProtocol
    let mut gop_ptr: *mut GraphicsOutput = null_mut();
    let status = unsafe {
        (bs.locate_protocol)(
            &GRAPHICS_OUTPUT_PROTOCOL,
            null_mut(),
            &mut gop_ptr as *mut *mut GraphicsOutput as *mut *mut core::ffi::c_void,
        )
    };

    if status != SUCCESS || gop_ptr.is_null() {
        return status;
    }

    let gop = unsafe { &mut *gop_ptr };
    let mode = unsafe { &*gop.mode };
    let info = unsafe { &*mode.info };

    let width = info.width as usize;
    let height = info.height as usize;
    let stride = info.pixels_per_scan_line as usize;
    let base = mode.frame_buffer_base as *mut u32;

    // Allocate backbuffer for double buffering
    let buffer_bytes = height * stride * 4;
    let pages = (buffer_bytes + 4095) / 4096;
    let mut backbuffer_addr: u64 = 0;
    let alloc_status = unsafe { (bs.allocate_pages)(2, 2, pages, &mut backbuffer_addr) }; // AllocateAnyPages, LoaderData

    let backbuffer = if alloc_status == SUCCESS {
        backbuffer_addr as *mut u32
    } else {
        base // Fallback to direct rendering if out of pages
    };

    let mut fb = UefiFramebuffer {
        base,
        backbuffer,
        width,
        height,
        stride,
    };

    let con_in = st.con_in as *mut TextInput;
    let mut screen = Screen::Launcher;
    let mut selected_app: usize = 0;

    let apps = [
        ("YouTube TV", "Video & Live 1080p", "[ Y ]", 0xFFEF4444),
        ("Prime Video", "Serie & Film DRM", "[ P ]", 0xFF00A8E1),
        ("Samsung DeX", "Schermo & Miracast", "[ D ]", 0xFF2563EB),
        ("Media Player", "Musica locale", "[ > ]", 0xFFF59E0B),
        ("Monitor", "CPU, 1GB RAM & Temp", "[ # ]", 0xFF10B981),
        ("Impostazioni", "Wi-Fi, BT & Remote", "[ * ]", 0xFF8B5CF6),
    ];

    loop {
        // 1. Process Input
        if !con_in.is_null() {
            let mut key = InputKey { scan_code: 0, unicode_char: 0 };
            let key_status = unsafe { ((*con_in).read_key_stroke)(con_in, &mut key) };

            if key_status == SUCCESS {
                match screen {
                    Screen::Launcher => {
                        match key.scan_code {
                            4 => { // Left
                                if selected_app > 0 { selected_app -= 1; }
                            }
                            3 => { // Right
                                if selected_app < 5 { selected_app += 1; }
                            }
                            _ => {}
                        }
                        if key.unicode_char == 13 { // Enter
                            screen = match selected_app {
                                0 => Screen::YouTube,
                                1 => Screen::PrimeVideo,
                                2 => Screen::SamsungDeX,
                                3 => Screen::MediaPlayer,
                                4 => Screen::Monitor,
                                5 => Screen::Settings,
                                _ => Screen::Launcher,
                            };
                        }
                    }
                    _ => {
                        // Any other screen: Esc (0x17) or Enter returns to Launcher
                        if key.scan_code == 0x17 || key.unicode_char == 27 || key.unicode_char == 13 {
                            screen = Screen::Launcher;
                        }
                    }
                }
            }
        }

        // 2. Render Screen
        let w = fb.width as i32;
        let h = fb.height as i32;

        // Base background: #07090e
        fb.fill_rect(0, 0, fb.width as u32, fb.height as u32, 0xFF07090E);

        // Ambient glows
        fb.draw_radial_glow(w / 4, h / 3, 300, 0x2CF59E0B);
        fb.draw_radial_glow((3 * w) / 4, (2 * h) / 3, 320, 0x288B5CF6);
        fb.draw_radial_glow(w / 2, h - 50, 240, 0x2206B6D4);

        match screen {
            Screen::Launcher => {
                // Top status badges
                fb.draw_badge(60, 25, "VirtualBox UEFI VM - Connessa", 0x2010B981, 0x5010B981, 0xFF10B981);
                fb.draw_badge(360, 25, "Samsung DeX Sink: Pronto", 0x202563EB, 0x502563EB, 0xFF60A5FA);

                // Branding
                let center_x = w / 2;
                fb.draw_text(center_x - 70, 55, "ticly", 0xFFFFFFFF, 4);
                fb.draw_badge(center_x - 180, 135, "Sistema Operativo TVos x86_64 UEFI", 0x20F59E0B, 0x50F59E0B, 0xFFFBBF24);
                fb.draw_text(center_x - 170, 175, "Premi [Frecce SX / DX] e [INVIO] per navigare", 0x99FFFFFF, 1);

                // 6 Cards Grid
                let card_w = 145u32;
                let card_h = 220u32;
                let gap = 16;
                let total_w = 6 * card_w as i32 + 5 * gap;
                let start_x = (w - total_w) / 2;
                let start_y = 230;

                for (i, &(title, desc, icon, accent_col)) in apps.iter().enumerate() {
                    let card_x = start_x + (i as i32) * (card_w as i32 + gap);
                    let is_focused = selected_app == i;

                    if is_focused {
                        fb.draw_radial_glow(card_x + (card_w as i32) / 2, start_y + (card_h as i32) / 2, 160, (0x44 << 24) | (accent_col & 0xFFFFFF));
                    }

                    let bg = if is_focused { 0xF0202232 } else { 0xD210121A };
                    let border = if is_focused { accent_col } else { 0x25FFFFFF };

                    fb.draw_glass_card(card_x, start_y, card_w, card_h, 18, bg, border);

                    // Icon box
                    let ib_x = card_x + 16;
                    let ib_y = start_y + 18;
                    fb.draw_glass_card(ib_x, ib_y, 36, 36, 10, 0x15FFFFFF, 0x25FFFFFF);
                    fb.draw_text(ib_x + 8, ib_y + 10, icon, accent_col, 1);

                    // Title & Desc
                    fb.draw_text(card_x + 16, start_y + 70, title, 0xFFFFFFFF, 1);
                    fb.draw_text(card_x + 16, start_y + 110, desc, 0x99FFFFFF, 1);

                    if is_focused {
                        fb.draw_text(card_x + 16, start_y + 180, ">> [INVIO]", accent_col, 1);
                    }
                }

                // Bottom bar
                let b_w = (w - 120) as u32;
                fb.draw_glass_card(60, h - 90, b_w, 65, 18, 0xD210121A, 0x25FFFFFF);
                fb.draw_text(85, h - 75, "COMANDI MACCHINA VIRTUALE VIRTUALBOX:", 0x66FFFFFF, 1);
                fb.draw_text(85, h - 55, "Frecce Sinistra/Destra per scorrere | Invio per entrare | Esc per uscire", 0xFFFFFFFF, 1);
            }
            Screen::YouTube => {
                fb.draw_text(60, 50, "ticly // YouTube TV Live", 0xFFEF4444, 3);
                fb.draw_text(60, 105, "Premi [ESC] o [INVIO] per tornare alla Home", 0x99FFFFFF, 1);
                fb.draw_glass_card(60, 150, (w - 120) as u32, 440, 24, 0xF0141620, 0xFFEF4444);
                fb.draw_text(100, 200, "[ STREAMING YOUTUBE 1080p ATTIVO ]", 0xFFEF4444, 2);
                fb.draw_text(100, 250, "Titolo: Raspberry Pi 3B & VirtualBox Rust Engine", 0xFFFFFFFF, 2);
                fb.draw_text(100, 300, "Canale: Tech Lab - 142K visualizzazioni", 0x99FFFFFF, 1);
                fb.draw_text(100, 350, "Decodifica H.264 GOP Framebuffer attiva a 60 FPS", 0xFF10B981, 1);
            }
            Screen::PrimeVideo => {
                fb.draw_text(60, 50, "prime video", 0xFF00A8E1, 3);
                fb.draw_text(60, 105, "Premi [ESC] o [INVIO] per tornare alla Home", 0x99FFFFFF, 1);
                fb.draw_glass_card(60, 150, (w - 120) as u32, 440, 24, 0xF0101824, 0xFF00A8E1);
                fb.draw_text(100, 200, "[ RIPRODUZIONE STREAMING PROTETTO ]", 0xFF00A8E1, 2);
                fb.draw_text(100, 250, "Serie TV: The Boys - Stagione 4", 0xFFFFFFFF, 2);
                fb.draw_text(100, 300, "Widevine DRM Level 3 (L3) Attivo - AES-128 Decryption", 0xFF10B981, 1);
            }
            Screen::SamsungDeX => {
                fb.draw_text(60, 50, "Samsung DeX Wireless", 0xFF2563EB, 3);
                fb.draw_text(60, 105, "Premi [ESC] o [INVIO] per tornare alla Home", 0x99FFFFFF, 1);
                fb.draw_glass_card(60, 150, (w - 120) as u32, 440, 24, 0xF0101828, 0xFF2563EB);
                fb.draw_text(100, 200, "[ SAMSUNG DeX WIRELESS ATTIVO SULLO SCHERMO ]", 0xFF60A5FA, 2);
                fb.draw_text(100, 250, "Dispositivo: Samsung Galaxy S24 Ultra", 0xFFFFFFFF, 2);
                fb.draw_text(100, 300, "Protocollo: Miracast Wi-Fi Display (WFD) + RTSP Port 7236", 0xFF10B981, 1);
                fb.draw_text(100, 350, "Canale di Ritorno UIBC: Supporto Mouse e Telecomando TV", 0xFFF59E0B, 1);
            }
            Screen::MediaPlayer => {
                fb.draw_text(60, 50, "Media Player", 0xFFF59E0B, 3);
                fb.draw_text(60, 105, "Premi [ESC] o [INVIO] per tornare alla Home", 0x99FFFFFF, 1);
                fb.draw_glass_card(60, 150, (w - 120) as u32, 440, 24, 0xF0181612, 0xFFF59E0B);
                fb.draw_text(100, 200, "[ IN RIPRODUZIONE AUDIO ]", 0xFFF59E0B, 2);
                fb.draw_text(100, 250, "Brano: Ambient Horizon (Rust Sound Engine)", 0xFFFFFFFF, 2);
                fb.draw_text(100, 300, "Output: Stereo 48kHz / HDMI & Bluetooth A2DP", 0xFF10B981, 1);
            }
            Screen::Monitor => {
                fb.draw_text(60, 50, "Monitor Hardware Sistema", 0xFF10B981, 3);
                fb.draw_text(60, 105, "Premi [ESC] o [INVIO] per tornare alla Home", 0x99FFFFFF, 1);
                fb.draw_glass_card(60, 150, (w - 120) as u32, 440, 24, 0xF0121C16, 0xFF10B981);
                fb.draw_text(100, 200, "CPU: x86_64 VirtualBox / Broadcom BCM2837 ARMv8", 0xFFFFFFFF, 2);
                fb.draw_text(100, 250, "RAM Utilizzata: ~42 MB / 1024 MB (Impronta minima)", 0xFF10B981, 2);
                fb.draw_text(100, 300, "Risoluzione Video: GOP Framebuffer 32-bit ARGB", 0xFFF59E0B, 1);
            }
            Screen::Settings => {
                fb.draw_text(60, 50, "Impostazioni TVos", 0xFF8B5CF6, 3);
                fb.draw_text(60, 105, "Premi [ESC] o [INVIO] per tornare alla Home", 0x99FFFFFF, 1);
                fb.draw_glass_card(60, 150, (w - 120) as u32, 440, 24, 0xF0181424, 0xFF8B5CF6);
                fb.draw_text(100, 200, "Wi-Fi: BCM43438 Wireless LAN Attiva (Home-WiFi 5GHz)", 0xFFFFFFFF, 2);
                fb.draw_text(100, 250, "Bluetooth: Soundbar Sala Connessa (A2DP Audio)", 0xFF60A5FA, 2);
                fb.draw_text(100, 300, "Telecomando Web Smartphone: http://192.168.1.5:8090", 0xFFF59E0B, 2);
            }
        }

        fb.flip();

        // Brief stall ~16ms
        unsafe { (bs.stall)(16_000) };
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
