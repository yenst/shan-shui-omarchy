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
use tiny_skia::{Color, FillRule, Mask, Paint, Path, PathBuilder, Pixmap, Stroke, Transform};

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
    pub draw: Option<Draw>,
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

/// Where the drawing front sits, in world units.
#[derive(Clone, Copy)]
pub struct Draw {
    /// screen width
    pub width: f64,
    /// distance from the left screen edge to the drawing front
    pub front: f64,
}

/// A pixel that gets inked once the reveal position reaches `key`.
#[derive(Clone, Copy)]
pub struct Reveal {
    pub key: f32,
    pub idx: u32,
    pub color: u32,
}

pub struct TileImage {
    /// What to show now: the finished tile, or bare paper when drawing.
    pub disp: Vec<u32>,
    /// Pixels still to ink, sorted by key (empty when not drawing).
    pub reveal: Vec<Reveal>,
}

/// When a stroke gets drawn: near where it sits, but also in the order the
/// item was painted, so outlines come before hatching before trees.
fn prim_key(it: &Item, k: usize, bb: &[f32; 4]) -> f64 {
    let n = it.prims.len().max(1) as f64;
    let cx = it.base_x + (bb[0] + bb[2]) as f64 / 2.0;
    let icx = (it.xr[0] + it.xr[1]) / 2.0;
    let iw = it.xr[1] - it.xr[0];
    0.5 * cx + 0.5 * icx + (k as f64 / n - 0.5) * iw * 0.6
}

/// Antialiased coverage of a primitive over its pixel bounds in the tile,
/// with the bounds' top-left corner.
fn coverage(
    p: &crate::scene::RPrim,
    fill: Option<&Path>,
    stroke: Option<(&(crate::draw::Col, Stroke), &Path)>,
    ts: Transform,
    w: u32,
    h: u32,
) -> Option<(Mask, usize, usize)> {
    let x0 = ((p.bb[0] * ts.sx + ts.tx).floor() as i64).max(0);
    let x1 = ((p.bb[2] * ts.sx + ts.tx).ceil() as i64).min(w as i64);
    let y0 = ((p.bb[1] * ts.sy).floor() as i64).max(0);
    let y1 = ((p.bb[3] * ts.sy).ceil() as i64).min(h as i64);
    let mut mask = Mask::new(u32::try_from(x1 - x0).ok()?, u32::try_from(y1 - y0).ok()?)?;
    let mts = ts.post_translate(-x0 as f32, -y0 as f32);
    if let Some(path) = fill {
        mask.fill_path(path, FillRule::Winding, true, mts);
    }
    if let Some(((_, st), path)) = stroke
        && let Some(outline) = path.stroke(st, ts.sx)
    {
        mask.fill_path(&outline, FillRule::Winding, true, mts);
    }
    Some((mask, x0 as usize, y0 as usize))
}

/// Render tile `idx` (pixels `idx*TILE_W ..`) as 0RGB pixels, row-major.
pub fn render_tile(items: &[Arc<Item>], idx: i64, view: &View, paper: &Paper) -> TileImage {
    let (w, h) = (TILE_W, view.height);
    let mut pm = Pixmap::new(w, h).expect("tile size");
    pm.fill(Color::WHITE);
    // per pixel: the draw key of the topmost primitive covering it
    let mut keys = if view.draw.is_some() { vec![f32::NAN; (w * h) as usize] } else { Vec::new() };
    // per pixel: the draw key and lightness of what lies beneath that primitive
    let mut under = if view.draw.is_some() { vec![(f32::NAN, 255u8); (w * h) as usize] } else { Vec::new() };

    let px0 = idx as f64 * w as f64;
    let ux0 = px0 / view.scale;
    let ux1 = (px0 + w as f64) / view.scale;

    let mut vis: Vec<&Arc<Item>> = items.iter().filter(|it| it.xr[1] >= ux0 && it.xr[0] <= ux1).collect();
    vis.sort_by(|a, b| a.y.total_cmp(&b.y).then(a.seq.cmp(&b.seq)));

    let s = view.scale as f32;
    let mut paint = Paint { anti_alias: true, ..Default::default() };
    for it in vis {
        let tx = (it.base_x * view.scale - px0) as f32;
        let ts = Transform::from_row(s, 0.0, 0.0, s, tx, 0.0);
        let (lx0, lx1) = ((ux0 - it.base_x) as f32, (ux1 - it.base_x) as f32);
        for (k, p) in it.prims.iter().enumerate() {
            if p.bb[2] < lx0 || p.bb[0] > lx1 {
                continue;
            }
            let mut pb = PathBuilder::new();
            pb.move_to(p.pts[0][0], p.pts[0][1]);
            for q in &p.pts[1..] {
                pb.line_to(q[0], q[1]);
            }
            let fill_path = p.fill.and_then(|_| {
                let mut fb = pb.clone();
                fb.close();
                fb.finish()
            });
            let stroke = p.stroke.map(|(col, wid)| (col, Stroke { width: wid, ..Default::default() }));
            let stroke_path = stroke.as_ref().and_then(|_| pb.finish());

            if view.draw.is_some()
                && let Some((mask, bx0, by0)) = coverage(p, fill_path.as_ref(), stroke.as_ref().zip(stroke_path.as_ref()), ts, w, h)
            {
                // each stroke draws itself along its long axis; tall ones grow upward
                let key0 = prim_key(it, k, &p.bb);
                let (bw, bh) = ((p.bb[2] - p.bb[0]) as f64, (p.bb[3] - p.bb[1]) as f64);
                let span = (bw.max(bh) * 0.12).min(30.0);
                let mw = mask.width() as usize;
                for (j, row) in mask.data().chunks_exact(mw).enumerate() {
                    let py = by0 + j;
                    for (i, &m) in row.iter().enumerate() {
                        if m == 0 {
                            continue;
                        }
                        let pxl = bx0 + i;
                        let prog = if bw >= bh {
                            ((pxl as f64 + 0.5 - tx as f64) / view.scale - p.bb[0] as f64) / bw
                        } else {
                            (p.bb[3] as f64 - (py as f64 + 0.5) / view.scale) / bh
                        };
                        // remember what this primitive covers up, and when that was drawn
                        let px = py * w as usize + pxl;
                        under[px] = (keys[px], pm.data()[px * 4]);
                        keys[px] = (key0 + prog.clamp(0.0, 1.0) * span) as f32;
                    }
                }
            }

            if let (Some(f), Some(path)) = (p.fill, &fill_path) {
                paint.set_color_rgba8(gray(f.g), gray(f.g), gray(f.g), alpha(f.a));
                pm.fill_path(path, &paint, FillRule::Winding, ts, None);
            }
            if let (Some((col, st)), Some(path)) = (&stroke, &stroke_path) {
                paint.set_color_rgba8(gray(col.g), gray(col.g), gray(col.g), alpha(col.a));
                pm.stroke_path(path, &paint, st, ts, None);
            }
        }
    }

    let bg = view.pal.bg.map(|v| v as f32);
    let fg = view.pal.fg.map(|v| v as f32);
    let tint = |v: f32| {
        let ch = |k: usize| (fg[k] + (bg[k] - fg[k]) * v).round().clamp(0.0, 255.0) as u8;
        pack([ch(0), ch(1), ch(2)])
    };
    let gx0 = idx * w as i64;
    let data = pm.data();
    let mut disp = vec![0u32; (w * h) as usize];
    let mut reveal = Vec::new();
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) as usize;
            let l = data[i * 4] as f32 / 255.0;
            let grain = 1.0 - view.grain * (1.0 - paper.at(gx0 + x as i64, y));
            let color = tint(l * grain);
            match view.draw {
                Some(d) if !keys[i].is_nan() || data[i * 4] < 255 => {
                    // every pixel is drawn while on screen, finished by 30% from the left
                    let wx = (gx0 as f64 + x as f64 + 0.5) / view.scale;
                    let raw = if keys[i].is_nan() { wx + d.front - 0.65 * d.width } else { keys[i] as f64 };
                    let clamp = |k: f64| k.clamp(wx + d.front - d.width, wx + d.front - 0.3 * d.width) as f32;
                    let key = clamp(raw);
                    // show what's beneath first, so later strokes don't punch paper holes
                    let (ukey, ul) = under[i];
                    if !ukey.is_nan() && ul < 255 && clamp(ukey as f64) < key {
                        let ucolor = tint(ul as f32 / 255.0 * grain);
                        reveal.push(Reveal { key: clamp(ukey as f64), idx: i as u32, color: ucolor });
                    }
                    reveal.push(Reveal { key, idx: i as u32, color });
                    disp[i] = tint(grain);
                }
                _ => disp[i] = color,
            }
        }
    }
    reveal.sort_unstable_by(|a, b| a.key.total_cmp(&b.key));
    TileImage { disp, reveal }
}

fn gray(g: f64) -> u8 {
    (g * 255.0).round().clamp(0.0, 255.0) as u8
}

fn alpha(a: f64) -> u8 {
    (a * 255.0).round().clamp(0.0, 255.0) as u8
}
