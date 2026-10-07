//! Drawing primitives: the original emits SVG polylines; we collect them as
//! grayscale vector primitives and rasterize later.

use crate::geom::{P, loop_noise, offset};
use crate::noise::noise;
use crate::rng::rand;
use std::f64::consts::PI;

/// A grayscale color, `g` in 0..1 (1 = paper white), with alpha.
#[derive(Clone, Copy, Debug)]
pub struct Col {
    pub g: f64,
    pub a: f64,
}

pub fn rgba(v: f64, a: f64) -> Col {
    Col { g: v / 255.0, a }
}

/// The original's ink color, `rgba(100,100,100,a)`.
pub fn ink(a: f64) -> Col {
    rgba(100.0, a)
}

pub const WHITE: Col = Col { g: 1.0, a: 1.0 };

pub fn round3(a: f64) -> f64 {
    (a * 1000.0).round() / 1000.0
}

pub struct Prim {
    pub pts: Vec<P>,
    pub fill: Option<Col>,
    pub stroke: Option<(Col, f64)>,
}

pub type Canv = Vec<Prim>;

/// SVG `<polyline>`: fill closes the shape implicitly, the stroke stays open.
pub fn poly(c: &mut Canv, pts: Vec<P>, fill: Option<Col>, stroke: Option<Col>, wid: f64) {
    let fill = fill.filter(|f| f.a > 0.0);
    let stroke = stroke.filter(|s| wid > 0.0 && s.a > 0.0).map(|s| (s, wid));
    if fill.is_none() && stroke.is_none() {
        return;
    }
    let pts: Vec<P> = pts.into_iter().filter(|p| p[0].is_finite() && p[1].is_finite()).collect();
    if pts.len() < 2 {
        return;
    }
    c.push(Prim { pts, fill, stroke });
}

pub fn fill(c: &mut Canv, pts: Vec<P>, col: Col) {
    poly(c, pts, Some(col), None, 0.0);
}

pub fn line(c: &mut Canv, pts: Vec<P>, col: Col, wid: f64) {
    poly(c, pts, None, Some(col), wid);
}

fn sin_pi(x: f64) -> f64 {
    (x * PI).sin()
}

pub fn one(_: f64) -> f64 {
    1.0
}

pub struct StrokeArgs<'a> {
    pub xof: f64,
    pub yof: f64,
    pub wid: f64,
    pub col: Col,
    pub noi: f64,
    pub out: f64,
    pub fun: &'a dyn Fn(f64) -> f64,
}

impl Default for StrokeArgs<'_> {
    fn default() -> Self {
        StrokeArgs { xof: 0.0, yof: 0.0, wid: 2.0, col: rgba(200.0, 0.9), noi: 0.5, out: 1.0, fun: &sin_pi }
    }
}

/// A brush stroke along a polyline with varying width.
pub fn stroke(c: &mut Canv, pts: &[P], a: StrokeArgs) {
    if pts.is_empty() {
        return;
    }
    let n = pts.len();
    let n0 = rand() * 10.0;
    let mut v0 = Vec::with_capacity(n);
    let mut v1 = Vec::with_capacity(n);
    for i in 1..n.saturating_sub(1) {
        let mut w = a.wid * (a.fun)(i as f64 / n as f64);
        w = w * (1.0 - a.noi) + w * a.noi * noise(i as f64 * 0.5, n0, 0.0);
        let a1 = (pts[i][1] - pts[i - 1][1]).atan2(pts[i][0] - pts[i - 1][0]);
        let a2 = (pts[i][1] - pts[i + 1][1]).atan2(pts[i][0] - pts[i + 1][0]);
        let mut an = (a1 + a2) / 2.0;
        if an < a2 {
            an += PI;
        }
        v0.push([pts[i][0] + w * an.cos(), pts[i][1] + w * an.sin()]);
        v1.push([pts[i][0] - w * an.cos(), pts[i][1] - w * an.sin()]);
    }
    let mut vtx = Vec::with_capacity(2 * n + 2);
    vtx.push(pts[0]);
    vtx.extend(v0);
    vtx.push(pts[n - 1]);
    vtx.extend(v1.into_iter().rev());
    vtx.push(pts[0]);
    poly(c, offset(&vtx, a.xof, a.yof), Some(a.col), Some(a.col), a.out);
}

fn blob_fun(x: f64) -> f64 {
    if x <= 1.0 { (x * PI).sin().powf(0.5) } else { -((x + 1.0) * PI).sin().powf(0.5) }
}

pub struct BlobArgs<'a> {
    pub len: f64,
    pub wid: f64,
    pub ang: f64,
    pub col: Col,
    pub noi: f64,
    pub fun: &'a dyn Fn(f64) -> f64,
}

impl Default for BlobArgs<'_> {
    fn default() -> Self {
        BlobArgs { len: 20.0, wid: 5.0, ang: 0.0, col: rgba(200.0, 0.9), noi: 0.5, fun: &blob_fun }
    }
}

pub fn blob_pts(x: f64, y: f64, a: &BlobArgs) -> Vec<P> {
    let reso = 20;
    let lalist: Vec<(f64, f64)> = (0..=reso)
        .map(|i| {
            let p = i as f64 / reso as f64 * 2.0;
            let xo = a.len / 2.0 - (p - 1.0).abs() * a.len;
            let yo = (a.fun)(p) * a.wid / 2.0;
            ((xo * xo + yo * yo).sqrt(), yo.atan2(xo))
        })
        .collect();
    let n0 = rand() * 10.0;
    let mut ns: Vec<f64> = (0..=reso).map(|i| noise(i as f64 * 0.05, n0, 0.0)).collect();
    loop_noise(&mut ns);
    lalist
        .iter()
        .zip(ns)
        .map(|(&(l, an), n)| {
            let s = n * a.noi + (1.0 - a.noi);
            [x + (an + a.ang).cos() * l * s, y + (an + a.ang).sin() * l * s]
        })
        .collect()
}

pub fn blob(c: &mut Canv, x: f64, y: f64, a: BlobArgs) {
    let pts = blob_pts(x, y, &a);
    fill(c, pts, a.col);
}

fn tex_noi(x: f64) -> f64 {
    30.0 / x
}

fn tex_col(_: f64) -> Col {
    ink(round3(rand() * 0.3))
}

fn tex_dis() -> f64 {
    if rand() > 0.5 { (1.0 / 3.0) * rand() } else { 2.0 / 3.0 + (1.0 / 3.0) * rand() }
}

pub struct TexArgs<'a> {
    pub xof: f64,
    pub yof: f64,
    pub tex: usize,
    pub wid: f64,
    pub len: f64,
    pub sha: usize,
    pub noi: &'a dyn Fn(f64) -> f64,
    pub col: &'a dyn Fn(f64) -> Col,
    pub dis: &'a dyn Fn() -> f64,
}

impl Default for TexArgs<'_> {
    fn default() -> Self {
        TexArgs {
            xof: 0.0,
            yof: 0.0,
            tex: 400,
            wid: 1.5,
            len: 0.2,
            sha: 0,
            noi: &tex_noi,
            col: &tex_col,
            dis: &tex_dis,
        }
    }
}

/// Hatching strokes laid across a grid of layered contour lines.
pub fn texture(c: &mut Canv, pl: &[Vec<P>], a: TexArgs) {
    let reso = [pl.len(), pl[0].len()];
    let mut texlist: Vec<Vec<P>> = Vec::with_capacity(a.tex);
    for i in 0..a.tex {
        let mid = ((a.dis)() * reso[1] as f64) as i64;
        let hlen = (rand() * (reso[1] as f64 * a.len)).floor() as i64;
        let start = (mid - hlen).clamp(0, reso[1] as i64) as usize;
        let end = (mid + hlen).clamp(0, reso[1] as i64) as usize;
        let layer = i as f64 / a.tex as f64 * (reso[0] - 1) as f64;
        let (lo, hi) = (layer.floor() as usize, layer.ceil() as usize);
        let p = layer - layer.floor();
        let mut row = Vec::with_capacity(end.saturating_sub(start));
        for j in start..end {
            let x = pl[lo][j][0] * p + pl[hi][j][0] * (1.0 - p);
            let y = pl[lo][j][1] * p + pl[hi][j][1] * (1.0 - p);
            let k = (a.noi)(layer + 1.0);
            row.push([
                x + k * (noise(x, j as f64 * 0.5, 0.0) - 0.5),
                y + k * (noise(y, j as f64 * 0.5, 0.0) - 0.5),
            ]);
        }
        texlist.push(row);
    }
    let n = texlist.len();
    if a.sha != 0 {
        for j in (0..n).step_by(2) {
            stroke(
                c,
                &offset(&texlist[j], a.xof, a.yof),
                StrokeArgs { col: ink(0.1), wid: a.sha as f64, ..Default::default() },
            );
        }
    }
    for j in (a.sha..n).step_by(1 + a.sha) {
        stroke(
            c,
            &offset(&texlist[j], a.xof, a.yof),
            StrokeArgs { col: (a.col)(j as f64 / n as f64), wid: a.wid, ..Default::default() },
        );
    }
}
