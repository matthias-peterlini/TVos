//! Vector icons drawn from signed-distance shapes: anti-aliased at any size, no image assets.

use crate::color::Color;
use crate::surface::Surface;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Icon {
    YouTube,
    Prime,
    Screen,
    Music,
    Chart,
    Gear,
    Remote,
    Wifi,
    Phone,
    Netflix,
    Plus,
}

type V = (f32, f32);

fn sub(a: V, b: V) -> V {
    (a.0 - b.0, a.1 - b.1)
}
fn len(v: V) -> f32 {
    (v.0 * v.0 + v.1 * v.1).sqrt()
}
fn circle(p: V, c: V, r: f32) -> f32 {
    len(sub(p, c)) - r
}
fn rrect(p: V, c: V, half: V, r: f32) -> f32 {
    let qx = (p.0 - c.0).abs() - (half.0 - r);
    let qy = (p.1 - c.1).abs() - (half.1 - r);
    let (ox, oy) = (qx.max(0.0), qy.max(0.0));
    (ox * ox + oy * oy).sqrt() + qx.max(qy).min(0.0) - r
}
fn segment(p: V, a: V, b: V, w: f32) -> f32 {
    let (pa, ba) = (sub(p, a), sub(b, a));
    let h = ((pa.0 * ba.0 + pa.1 * ba.1) / (ba.0 * ba.0 + ba.1 * ba.1)).clamp(0.0, 1.0);
    len((pa.0 - ba.0 * h, pa.1 - ba.1 * h)) - w
}
/// Convex triangle, any winding
fn triangle(p: V, pts: [V; 3]) -> f32 {
    let centroid = ((pts[0].0 + pts[1].0 + pts[2].0) / 3.0, (pts[0].1 + pts[1].1 + pts[2].1) / 3.0);
    let mut d = f32::MIN;
    for i in 0..3 {
        let (u, v) = (pts[i], pts[(i + 1) % 3]);
        let e = sub(v, u);
        let mut n = (e.1, -e.0);
        let l = len(n);
        n = (n.0 / l, n.1 / l);
        let c = sub(centroid, u);
        if n.0 * c.0 + n.1 * c.1 > 0.0 {
            n = (-n.0, -n.1);
        }
        let q = sub(p, u);
        d = d.max(n.0 * q.0 + n.1 * q.1);
    }
    d
}
/// Arc of radius r between angles a0..a1 (screen coordinates, y down), stroke half-width w
fn arc(p: V, c: V, r: f32, w: f32, a0: f32, a1: f32) -> f32 {
    let v = sub(p, c);
    let ang = v.1.atan2(v.0);
    if ang >= a0 && ang <= a1 {
        (len(v) - r).abs() - w
    } else {
        let e0 = (c.0 + r * a0.cos(), c.1 + r * a0.sin());
        let e1 = (c.0 + r * a1.cos(), c.1 + r * a1.sin());
        circle(p, e0, w).min(circle(p, e1, w))
    }
}
fn outline(d: f32, w: f32) -> f32 {
    d.abs() - w
}
fn minus(a: f32, b: f32) -> f32 {
    a.max(-b)
}

/// Distance field of an icon in a [-1, 1] box (negative = inside)
fn field(icon: Icon, p: V) -> f32 {
    match icon {
        Icon::YouTube => {
            let body = rrect(p, (0.0, 0.0), (0.92, 0.65), 0.28);
            minus(body, triangle(p, [(-0.2, -0.3), (-0.2, 0.3), (0.34, 0.0)]))
        }
        Icon::Prime => {
            let ring = outline(rrect(p, (0.0, 0.0), (0.82, 0.82), 0.3), 0.09);
            ring.min(triangle(p, [(-0.2, -0.35), (-0.2, 0.35), (0.38, 0.0)]))
        }
        Icon::Screen => {
            let panel = outline(rrect(p, (0.0, -0.12), (0.88, 0.58), 0.14), 0.09);
            let stand = segment(p, (0.0, 0.48), (0.0, 0.72), 0.09);
            let foot = segment(p, (-0.38, 0.78), (0.38, 0.78), 0.09);
            panel.min(stand).min(foot)
        }
        Icon::Music => {
            let head = circle(p, (-0.35, 0.55), 0.3);
            let stem = segment(p, (-0.08, 0.5), (-0.08, -0.7), 0.09);
            let flag = segment(p, (-0.08, -0.7), (0.55, -0.38), 0.11);
            head.min(stem).min(flag)
        }
        Icon::Chart => {
            let a = rrect(p, (-0.55, 0.38), (0.2, 0.42), 0.07);
            let b = rrect(p, (0.0, 0.1), (0.2, 0.7), 0.07);
            let c = rrect(p, (0.55, -0.2), (0.2, 1.0), 0.07);
            a.min(b).min(c)
        }
        Icon::Gear => {
            let mut body = circle(p, (0.0, 0.0), 0.55);
            for k in 0..8 {
                let a = k as f32 * std::f32::consts::FRAC_PI_4;
                let q = (p.0 * a.cos() + p.1 * a.sin(), -p.0 * a.sin() + p.1 * a.cos());
                body = body.min(rrect(q, (0.66, 0.0), (0.22, 0.17), 0.05));
            }
            minus(body, circle(p, (0.0, 0.0), 0.24))
        }
        Icon::Remote => {
            let shell = outline(rrect(p, (0.0, 0.0), (0.42, 0.92), 0.3), 0.09);
            let pad = outline(circle(p, (0.0, -0.3), 0.2), 0.07);
            let b1 = circle(p, (-0.16, 0.38), 0.08);
            let b2 = circle(p, (0.16, 0.38), 0.08);
            let b3 = circle(p, (0.0, 0.64), 0.08);
            shell.min(pad).min(b1).min(b2).min(b3)
        }
        Icon::Wifi => {
            let (c, up0, up1) = ((0.0, 0.62), -2.36, -0.78);
            let a1 = arc(p, c, 0.45, 0.085, up0, up1);
            let a2 = arc(p, c, 0.85, 0.085, up0, up1);
            let a3 = arc(p, c, 1.25, 0.085, up0, up1);
            circle(p, c, 0.11).min(a1).min(a2).min(a3)
        }
        Icon::Plus => segment(p, (-0.62, 0.0), (0.62, 0.0), 0.13).min(segment(p, (0.0, -0.62), (0.0, 0.62), 0.13)),
        Icon::Netflix => {
            let left = rrect(p, (-0.42, 0.0), (0.16, 0.95), 0.0);
            let right = rrect(p, (0.42, 0.0), (0.16, 0.95), 0.0);
            let (a, b, c, d) = ((-0.58, -0.95), (-0.24, -0.95), (0.58, 0.95), (0.24, 0.95));
            left.min(right).min(triangle(p, [a, b, c])).min(triangle(p, [a, c, d]))
        }
        Icon::Phone => {
            let shell = outline(rrect(p, (0.0, 0.0), (0.5, 0.9), 0.16), 0.09);
            let speaker = segment(p, (-0.14, -0.62), (0.14, -0.62), 0.05);
            let btn = circle(p, (0.0, 0.62), 0.09);
            shell.min(speaker).min(btn)
        }
    }
}

impl Surface {
    /// Draw `icon` centred on (cx, cy) inside a `size` x `size` px box.
    pub fn draw_icon(&mut self, icon: Icon, cx: i32, cy: i32, size: i32, color: Color) {
        let half = size as f32 / 2.0;
        let x0 = cx - size / 2 - 1;
        let y0 = cy - size / 2 - 1;
        for py in y0..y0 + size + 3 {
            for px in x0..x0 + size + 3 {
                let p = ((px as f32 + 0.5 - cx as f32) / half, (py as f32 + 0.5 - cy as f32) / half);
                let cov = (0.5 - field(icon, p) * half).clamp(0.0, 1.0);
                if cov > 0.0 && px >= 0 && py >= 0 {
                    let mut c = color;
                    c.a = (color.a as f32 * cov) as u8;
                    self.blend_pixel(px as usize, py as usize, c);
                }
            }
        }
    }
}
