//! Rasterize the scene into vertical tiles and tint them with the theme.
//!
//! The original draws gray ink onto a white SVG and multiplies it over paper.
//! We render the same grayscale image, then map its lightness onto a
//! two-color ramp from the theme's foreground (ink) to background (paper),
//! so dark themes get light ink on dark paper.

use crate::noise::noise;
use crate::rng::Rng;
use crate::scene::Item;
use crate::theme::Palette;
use std::sync::Arc;
use tiny_skia::{Color, FillRule, Paint, PathBuilder, Pixmap, Stroke, Transform};

/// Tile width in pixels.
pub const TILE_W: u32 = 512;
/// Height of the original's viewBox, in world units.
pub const VIEW_H: f64 = 700.0;

pub struct View {
    /// pixels per world unit
    pub scale: f64,
    pub height: u32,
    pub pal: Palette,
    /// paper grain strength, 0..1
    pub grain: f32,
}

/// The original's paper background: a mirrored 512² noise tile.
pub struct Paper {
    tex: Vec<f32>,
    texel: u32,
}

impl Paper {
    pub fn new(seed: u64, scale: f64) -> Self {
        let reso = 512usize;
        let mut r = Rng(seed ^ 0x9A9E_5EED);
        let mut tex = vec![1.0f32; reso * reso];
        for i in 0..=reso / 2 {
            for j in 0..=reso / 2 {
                let c = 245.0 + noise(i as f64 * 0.1, j as f64 * 0.1, 0.0) * 10.0 - r.f64() * 20.0;
                let t = (c / 255.0).min(1.0) as f32;
                for (x, y) in [(i, j), (reso - i, j), (i, reso - j), (reso - i, reso - j)] {
                    if x < reso && y < reso {
                        tex[y * reso + x] = t;
                    }
                }
            }
        }
        // one paper texel per CSS pixel of the original's 1.142 zoom
        let texel = (scale / 1.142).round().max(1.0) as u32;
        Paper { tex, texel }
    }

    fn at(&self, gx: i64, y: u32) -> f32 {
        let tx = (gx.div_euclid(self.texel as i64) & 511) as usize;
        let ty = ((y / self.texel) & 511) as usize;
        self.tex[ty * 512 + tx]
    }
}

pub fn pack(c: [u8; 3]) -> u32 {
    (c[0] as u32) << 16 | (c[1] as u32) << 8 | c[2] as u32
}

/// Render tile `idx` (pixels `idx*TILE_W ..`) as 0RGB pixels, row-major.
pub fn render_tile(items: &[Arc<Item>], idx: i64, view: &View, paper: &Paper) -> Vec<u32> {
    let (w, h) = (TILE_W, view.height);
    let mut pm = Pixmap::new(w, h).expect("tile size");
    pm.fill(Color::WHITE);

    let px0 = idx as f64 * w as f64;
    let ux0 = px0 / view.scale;
    let ux1 = (px0 + w as f64) / view.scale;

    let mut vis: Vec<&Arc<Item>> = items.iter().filter(|it| it.xr[1] >= ux0 && it.xr[0] <= ux1).collect();
    vis.sort_by(|a, b| a.y.total_cmp(&b.y).then(a.seq.cmp(&b.seq)));

    let s = view.scale as f32;
    let mut paint = Paint { anti_alias: true, ..Default::default() };
    for it in vis {
        let ts = Transform::from_row(s, 0.0, 0.0, s, (it.base_x * view.scale - px0) as f32, 0.0);
        let (lx0, lx1) = ((ux0 - it.base_x) as f32, (ux1 - it.base_x) as f32);
        for p in &it.prims {
            if p.bb[2] < lx0 || p.bb[0] > lx1 {
                continue;
            }
            let mut pb = PathBuilder::new();
            pb.move_to(p.pts[0][0], p.pts[0][1]);
            for q in &p.pts[1..] {
                pb.line_to(q[0], q[1]);
            }
            if let Some(f) = p.fill {
                let mut fb = pb.clone();
                fb.close();
                if let Some(path) = fb.finish() {
                    paint.set_color_rgba8(gray(f.g), gray(f.g), gray(f.g), alpha(f.a));
                    pm.fill_path(&path, &paint, FillRule::Winding, ts, None);
                }
            }
            if let Some((col, wid)) = p.stroke {
                if let Some(path) = pb.finish() {
                    paint.set_color_rgba8(gray(col.g), gray(col.g), gray(col.g), alpha(col.a));
                    let st = Stroke { width: wid, ..Default::default() };
                    pm.stroke_path(&path, &paint, &st, ts, None);
                }
            }
        }
    }

    let bg = view.pal.bg.map(|v| v as f32);
    let fg = view.pal.fg.map(|v| v as f32);
    let gx0 = idx * w as i64;
    let data = pm.data();
    let mut out = vec![0u32; (w * h) as usize];
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) as usize;
            let l = data[i * 4] as f32 / 255.0;
            let grain = 1.0 - view.grain * (1.0 - paper.at(gx0 + x as i64, y));
            let v = l * grain;
            let ch = |k: usize| (fg[k] + (bg[k] - fg[k]) * v).round().clamp(0.0, 255.0) as u8;
            out[i] = pack([ch(0), ch(1), ch(2)]);
        }
    }
    out
}

fn gray(g: f64) -> u8 {
    (g * 255.0).round().clamp(0.0, 255.0) as u8
}

fn alpha(a: f64) -> u8 {
    (a * 255.0).round().clamp(0.0, 255.0) as u8
}
