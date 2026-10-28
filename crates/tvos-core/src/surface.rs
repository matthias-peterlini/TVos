use crate::background::{bg_blur_pixel, bg_pixel, BG_HEIGHT, BG_WIDTH};
use crate::color::Color;
use crate::icons::Icon;
use crate::text::{ascent, glyph, text_width, Weight};

pub struct Surface {
    pub width: usize,
    pub height: usize,
    pub buffer: Vec<u32>, // ARGB32
    /// Wallpaper scaled to the surface size (built once)
    bg: Vec<u32>,
    /// Blurred wallpaper with the dark glass tint already applied (built once)
    glass: Vec<u32>,
    /// Blurred wallpaper without tint (frosted panels over the bright photo)
    blur: Vec<u32>,
}

impl Surface {
    pub fn new(width: usize, height: usize) -> Self {
        let mut s = Self { width, height, buffer: Vec::new(), bg: Vec::new(), glass: Vec::new(), blur: Vec::new() };
        s.resize(width, height);
        s
    }

    pub fn resize(&mut self, width: usize, height: usize) {
        self.width = width;
        self.height = height;
        self.buffer = vec![0xFF07090E; width * height];
        self.bg = Vec::with_capacity(width * height);
        self.glass = Vec::with_capacity(width * height);
        self.blur = Vec::with_capacity(width * height);
        let tint = Color::rgba(16, 18, 26, 200);
        for y in 0..height {
            let sy = (y * BG_HEIGHT / height).min(BG_HEIGHT - 1);
            for x in 0..width {
                let sx = (x * BG_WIDTH / width).min(BG_WIDTH - 1);
                self.bg.push(bg_pixel(sx, sy).to_u32_argb());
                let blurred = bg_blur_pixel(sx, sy);
                self.blur.push(blurred.to_u32_argb());
                self.glass.push(Color::blend_over(tint, blurred).to_u32_argb());
            }
        }
    }

    /// Blit the Dolomiti background image (already scaled to the surface)
    pub fn draw_background(&mut self) {
        self.buffer.copy_from_slice(&self.bg);
    }

    /// Blurred wallpaper washed with `color` (`amount` 0..1): the app library takes the colour of the focused app
    pub fn draw_tinted_background(&mut self, color: Color, amount: f32) {
        let a = (amount.clamp(0.0, 1.0) * 256.0) as u32;
        let (r, g, b) = (color.r as u32, color.g as u32, color.b as u32);
        for (dst, src) in self.buffer.iter_mut().zip(self.blur.iter()) {
            let (sr, sg, sb) = ((src >> 16) & 255, (src >> 8) & 255, src & 255);
            let or = (sr * (256 - a) + r * a) >> 8;
            let og = (sg * (256 - a) + g * a) >> 8;
            let ob = (sb * (256 - a) + b * a) >> 8;
            *dst = 0xFF00_0000 | (or << 16) | (og << 8) | ob;
        }
    }

    /// Anti-aliased line segment with round caps
    pub fn draw_line(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, thick: f32, color: Color) {
        let pad = thick / 2.0 + 1.0;
        let (minx, maxx) = ((x0.min(x1) - pad).floor().max(0.0) as i32, (x0.max(x1) + pad).ceil().min(self.width as f32) as i32);
        let (miny, maxy) = ((y0.min(y1) - pad).floor().max(0.0) as i32, (y0.max(y1) + pad).ceil().min(self.height as f32) as i32);
        let (dx, dy) = (x1 - x0, y1 - y0);
        let len2 = (dx * dx + dy * dy).max(1e-6);
        for y in miny..maxy {
            for x in minx..maxx {
                let (px, py) = (x as f32 + 0.5 - x0, y as f32 + 0.5 - y0);
                let t = ((px * dx + py * dy) / len2).clamp(0.0, 1.0);
                let (qx, qy) = (px - t * dx, py - t * dy);
                let d = (qx * qx + qy * qy).sqrt() - thick / 2.0;
                let cov = (0.5 - d).clamp(0.0, 1.0);
                if cov > 0.0 {
                    let mut c = color;
                    c.a = (color.a as f32 * cov) as u8;
                    self.blend_pixel(x as usize, y as usize, c);
                }
            }
        }
    }

    /// Downward chevron centred on (cx, cy), `w` px wide
    pub fn draw_chevron(&mut self, cx: i32, cy: i32, w: i32, color: Color) {
        let (hw, hh) = (w as f32 / 2.0, w as f32 / 4.0);
        let t = (w as f32 / 7.0).max(2.0);
        self.draw_line(cx as f32 - hw, cy as f32 - hh, cx as f32, cy as f32 + hh, t, color);
        self.draw_line(cx as f32, cy as f32 + hh, cx as f32 + hw, cy as f32 - hh, t, color);
    }

    /// Blit the frosted (blurred + tinted) background into a rectangle
    pub fn draw_glass_background(&mut self, cx: i32, cy: i32, cw: u32, ch: u32) {
        let sw = self.width;
        let x0 = cx.max(0) as usize;
        let x1 = ((cx + cw as i32).max(0) as usize).min(sw);
        let y0 = cy.max(0) as usize;
        let y1 = ((cy + ch as i32).max(0) as usize).min(self.height);
        if x0 >= x1 {
            return;
        }
        for y in y0..y1 {
            self.buffer[y * sw + x0..y * sw + x1].copy_from_slice(&self.glass[y * sw + x0..y * sw + x1]);
        }
    }

    #[inline(always)]
    pub fn clear(&mut self, color: Color) {
        let val = color.to_u32_argb();
        self.buffer.fill(val);
    }

    #[inline(always)]
    pub fn get_pixel(&self, x: usize, y: usize) -> Color {
        if x < self.width && y < self.height {
            Color::from_u32_argb(self.buffer[y * self.width + x])
        } else {
            Color::transparent()
        }
    }

    #[inline(always)]
    pub fn set_pixel(&mut self, x: usize, y: usize, color: Color) {
        if x < self.width && y < self.height {
            self.buffer[y * self.width + x] = color.to_u32_argb();
        }
    }

    #[inline(always)]
    pub fn blend_pixel(&mut self, x: usize, y: usize, color: Color) {
        if x < self.width && y < self.height {
            let idx = y * self.width + x;
            let dst = Color::from_u32_argb(self.buffer[idx]);
            let blended = Color::blend_over(color, dst);
            self.buffer[idx] = blended.to_u32_argb();
        }
    }

    pub fn fill_rect(&mut self, x: i32, y: i32, w: u32, h: u32, color: Color) {
        let x0 = x.max(0) as usize;
        let y0 = y.max(0) as usize;
        let x1 = ((x + w as i32).max(0) as usize).min(self.width);
        let y1 = ((y + h as i32).max(0) as usize).min(self.height);

        if x0 >= x1 || y0 >= y1 {
            return;
        }

        if color.a == 255 {
            let val = color.to_u32_argb();
            for cy in y0..y1 {
                let row_start = cy * self.width + x0;
                let row_end = cy * self.width + x1;
                self.buffer[row_start..row_end].fill(val);
            }
        } else if color.a > 0 {
            for cy in y0..y1 {
                for cx in x0..x1 {
                    self.blend_pixel(cx, cy, color);
                }
            }
        }
    }

    pub fn draw_radial_glow(&mut self, center_x: i32, center_y: i32, radius: i32, color: Color) {
        let x0 = (center_x - radius).max(0);
        let y0 = (center_y - radius).max(0);
        let x1 = (center_x + radius).min(self.width as i32 - 1);
        let y1 = (center_y + radius).min(self.height as i32 - 1);

        let r2 = (radius * radius) as f32;
        let max_alpha = color.a as f32;

        for y in y0..=y1 {
            let dy = y - center_y;
            let dy2 = (dy * dy) as f32;
            for x in x0..=x1 {
                let dx = x - center_x;
                let dist2 = (dx * dx) as f32 + dy2;
                if dist2 <= r2 {
                    let f = 1.0 - (dist2 / r2);
                    let factor = f * f;
                    let mut c = color;
                    c.a = (max_alpha * factor) as u8;
                    self.blend_pixel(x as usize, y as usize, c);
                }
            }
        }
    }

    /// Signed distance from a pixel center to a rounded rect (negative inside)
    #[inline(always)]
    fn rrect_dist(px: f32, py: f32, x: f32, y: f32, w: f32, h: f32, r: f32) -> f32 {
        let (hx, hy) = (w / 2.0, h / 2.0);
        let qx = (px - (x + hx)).abs() - (hx - r);
        let qy = (py - (y + hy)).abs() - (hy - r);
        let (ox, oy) = (qx.max(0.0), qy.max(0.0));
        (ox * ox + oy * oy).sqrt() + qx.max(qy).min(0.0) - r
    }

    /// Anti-aliased filled rounded rectangle
    pub fn fill_rrect(&mut self, x: i32, y: i32, w: u32, h: u32, radius: i32, color: Color) {
        let r = (radius as f32).min(w as f32 / 2.0).min(h as f32 / 2.0);
        let (fx, fy, fw, fh) = (x as f32, y as f32, w as f32, h as f32);
        for cy in y.max(0)..(y + h as i32).min(self.height as i32) {
            for cx in x.max(0)..(x + w as i32).min(self.width as i32) {
                let d = Self::rrect_dist(cx as f32 + 0.5, cy as f32 + 0.5, fx, fy, fw, fh, r);
                let cov = (0.5 - d).clamp(0.0, 1.0);
                if cov > 0.0 {
                    let mut c = color;
                    c.a = (color.a as f32 * cov) as u8;
                    self.blend_pixel(cx as usize, cy as usize, c);
                }
            }
        }
    }

    /// Anti-aliased rounded rectangle filled with a top-to-bottom gradient
    pub fn fill_rrect_gradient(&mut self, x: i32, y: i32, w: u32, h: u32, radius: i32, top: Color, bottom: Color) {
        let r = (radius as f32).min(w as f32 / 2.0).min(h as f32 / 2.0);
        let (fx, fy, fw, fh) = (x as f32, y as f32, w as f32, h as f32);
        for cy in y.max(0)..(y + h as i32).min(self.height as i32) {
            let base = top.lerp(bottom, (cy - y) as f32 / h.max(1) as f32);
            for cx in x.max(0)..(x + w as i32).min(self.width as i32) {
                let d = Self::rrect_dist(cx as f32 + 0.5, cy as f32 + 0.5, fx, fy, fw, fh, r);
                let cov = (0.5 - d).clamp(0.0, 1.0);
                if cov > 0.0 {
                    let mut c = base;
                    c.a = (base.a as f32 * cov) as u8;
                    self.blend_pixel(cx as usize, cy as usize, c);
                }
            }
        }
    }

    /// Text with a soft dark shadow, readable over a bright photo
    pub fn draw_text_shadow(&mut self, x: i32, y: i32, text: &str, color: Color, size: f32, weight: Weight) {
        self.draw_text_ex(x + 2, y + 3, text, Color::rgba(0, 0, 0, 90), size, weight);
        self.draw_text_ex(x + 1, y + 1, text, Color::rgba(0, 0, 0, 70), size, weight);
        self.draw_text_ex(x, y, text, color, size, weight);
    }

    /// Anti-aliased rounded rectangle outline (inside the rect), `bw` px thick
    pub fn stroke_rrect(&mut self, x: i32, y: i32, w: u32, h: u32, radius: i32, bw: f32, color: Color) {
        let r = (radius as f32).min(w as f32 / 2.0).min(h as f32 / 2.0);
        let (fx, fy, fw, fh) = (x as f32, y as f32, w as f32, h as f32);
        for cy in y.max(0)..(y + h as i32).min(self.height as i32) {
            for cx in x.max(0)..(x + w as i32).min(self.width as i32) {
                let d = Self::rrect_dist(cx as f32 + 0.5, cy as f32 + 0.5, fx, fy, fw, fh, r);
                let outer = (0.5 - d).clamp(0.0, 1.0);
                let inner = (0.5 - (d + bw)).clamp(0.0, 1.0);
                let cov = outer * (1.0 - inner);
                if cov > 0.0 {
                    let mut c = color;
                    c.a = (color.a as f32 * cov) as u8;
                    self.blend_pixel(cx as usize, cy as usize, c);
                }
            }
        }
    }

    /// Dark glass card: blurred background + dark tint + hairline border + top highlight
    pub fn draw_glass_card(&mut self, x: i32, y: i32, w: u32, h: u32, radius: i32, bg_color: Color, border_color: Color) {
        self.glass_card_impl(x, y, w, h, radius, bg_color, border_color, false);
    }

    /// Light frosted card (tvOS style): the blurred photo itself, only lightly tinted by `tint`
    pub fn draw_frosted_card(&mut self, x: i32, y: i32, w: u32, h: u32, radius: i32, tint: Color, border_color: Color) {
        self.glass_card_impl(x, y, w, h, radius, tint, border_color, true);
    }

    #[allow(clippy::too_many_arguments)]
    fn glass_card_impl(
        &mut self,
        x: i32,
        y: i32,
        w: u32,
        h: u32,
        radius: i32,
        bg_color: Color,
        border_color: Color,
        frosted: bool,
    ) {
        let r = (radius as f32).min(w as f32 / 2.0).min(h as f32 / 2.0);
        let (fx, fy, fw, fh) = (x as f32, y as f32, w as f32, h as f32);
        let (sw, sh) = (self.width, self.height);
        let strip_corner = r.ceil() as i32 + 1;
        for cy in y.max(0)..(y + h as i32).min(sh as i32) {
            let dy = cy - y;
            // Pixels at least 2px away from every edge and outside the corner squares are plain glass
            let edge_row = dy < 2 || dy >= h as i32 - 2;
            let strip = if dy < strip_corner || dy >= h as i32 - strip_corner { strip_corner } else { 2 };
            for cx in x.max(0)..(x + w as i32).min(sw as i32) {
                let dx = cx - x;
                let idx = cy as usize * sw + cx as usize;
                let deep = !edge_row && dx >= strip && dx < w as i32 - strip;
                let (cov, d) = if deep {
                    (1.0, -(r + 2.0))
                } else {
                    let d = Self::rrect_dist(cx as f32 + 0.5, cy as f32 + 0.5, fx, fy, fw, fh, r);
                    ((0.5 - d).clamp(0.0, 1.0), d)
                };
                if cov <= 0.0 {
                    continue;
                }
                let base = if frosted { self.blur[idx] } else { self.glass[idx] };
                let glass = Color::blend_over(bg_color, Color::from_u32_argb(base));
                if deep {
                    self.buffer[idx] = Color { a: 255, ..glass }.to_u32_argb();
                    continue;
                }
                let dst = Color::from_u32_argb(self.buffer[idx]);
                let mut out = dst.lerp(Color { a: 255, ..glass }, cov);
                out.a = 255;

                let ring = cov * (1.0 - (0.5 - (d + 1.0)).clamp(0.0, 1.0));
                if ring > 0.0 {
                    let mut b = border_color;
                    b.a = (border_color.a as f32 * ring) as u8;
                    out = Color::blend_over(b, out);
                }
                if dy == 1 && d < -r * 0.5 {
                    out = Color::blend_over(Color::rgba(255, 255, 255, 40), out);
                }
                self.buffer[idx] = out.to_u32_argb();
            }
        }
    }

    /// Draw text. `scale` maps to a pixel size (14 px per step); scale>=2 uses Bold.
    pub fn draw_text(&mut self, start_x: i32, start_y: i32, text: &str, color: Color, scale: usize) {
        let w = if scale >= 2 { Weight::Bold } else { Weight::Regular };
        self.draw_text_ex(start_x, start_y, text, color, 14.0 * scale.max(1) as f32, w);
    }

    /// Draw anti-aliased TTF text; (start_x, start_y) is the top-left of the line box.
    pub fn draw_text_ex(&mut self, start_x: i32, start_y: i32, text: &str, color: Color, size: f32, weight: Weight) {
        let asc = ascent(size, weight);
        let line_h = (size * 1.35) as i32;
        let (mut pen, mut cur_y) = (start_x as f32, start_y);
        for ch in text.chars() {
            if ch == '\n' {
                cur_y += line_h;
                pen = start_x as f32;
                continue;
            }
            let g = glyph(ch, size, weight);
            let (m, bmp) = (&g.0, &g.1);
            let gx = pen.round() as i32 + m.xmin;
            let gy = cur_y + asc.round() as i32 - m.height as i32 - m.ymin;
            for row in 0..m.height {
                for col in 0..m.width {
                    let cov = bmp[row * m.width + col];
                    if cov > 0 {
                        let (px, py) = (gx + col as i32, gy + row as i32);
                        if px >= 0 && py >= 0 {
                            let mut c = color;
                            c.a = (color.a as u32 * cov as u32 / 255) as u8;
                            self.blend_pixel(px as usize, py as usize, c);
                        }
                    }
                }
            }
            pen += m.advance_width;
        }
    }

    pub fn draw_badge(&mut self, x: i32, y: i32, text: &str, bg: Color, border: Color, text_color: Color) {
        let tw = text_width(text, 13.0, Weight::SemiBold);
        let badge_w = tw as u32 + 24;
        self.draw_glass_card(x, y, badge_w, 28, 14, bg, border);
        self.draw_text_ex(x + 12, y + 5, text, text_color, 13.0, Weight::SemiBold);
    }

    /// Draw a horizontal divider line
    pub fn draw_divider(&mut self, x: i32, y: i32, w: u32, color: Color) {
        self.fill_rect(x, y, w, 1, color);
    }

    /// Feature row (icon + title + subtitle) like the ticly login feature list
    pub fn draw_feature_item(&mut self, x: i32, y: i32, w: u32, icon: Icon, title: &str, subtitle: &str) {
        self.draw_glass_card(x, y, w, 64, 16, Color::rgba(255, 255, 255, 8), Color::rgba(255, 255, 255, 22));
        self.draw_glass_card(x + 14, y + 14, 36, 36, 10, Color::rgba(255, 255, 255, 14), Color::rgba(255, 255, 255, 30));
        self.draw_icon(icon, x + 32, y + 32, 20, Color::rgba(255, 255, 255, 220));
        self.draw_text_ex(x + 62, y + 12, title, Color::white(), 15.0, Weight::SemiBold);
        self.draw_text_ex(x + 62, y + 34, subtitle, Color::rgba(255, 255, 255, 140), 12.5, Weight::Regular);
    }
}
