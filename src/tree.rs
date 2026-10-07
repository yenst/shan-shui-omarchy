//! Tree.* from the original.

use crate::draw::*;
use crate::geom::*;
use crate::noise::noise;
use crate::rng::*;
use std::f64::consts::PI;

/// The leaf-shaped blob profile shared by tree02, twig and tree07-ish leaves.
pub fn leaf_fun(x: f64) -> f64 {
    if x <= 1.0 {
        ((x * PI).sin() * x).powf(0.5)
    } else {
        -(((x - 2.0) * PI * (x - 2.0)).sin()).powf(0.5)
    }
}

fn round1(a: f64) -> f64 {
    (a * 10.0).round() / 10.0
}

fn trunk_noise(reso: usize) -> Vec<[f64; 2]> {
    (0..reso)
        .map(|i| [noise(i as f64 * 0.5, 0.0, 0.0), noise(i as f64 * 0.5, 0.5, 0.0)])
        .collect()
}

pub fn tree01(c: &mut Canv, x: f64, y: f64, hei: f64, wid: f64, col: Col) {
    let reso = 10;
    let ns = trunk_noise(reso);
    let (mut l1, mut l2) = (Vec::new(), Vec::new());
    for i in 0..reso {
        let rr = (reso - i) as f64;
        let nx = x;
        let ny = y - i as f64 * hei / reso as f64;
        if i as f64 >= reso as f64 / 4.0 {
            let mut j = 0.0;
            while j < rr / 5.0 {
                let bx = nx + (rand() - 0.5) * wid * 1.2 * rr;
                let by = ny + (rand() - 0.5) * wid;
                blob(
                    c,
                    bx,
                    by,
                    BlobArgs {
                        len: rand() * 20.0 * rr * 0.2 + 10.0,
                        wid: rand() * 6.0 + 3.0,
                        ang: (rand() - 0.5) * PI / 6.0,
                        col: Col { g: col.g, a: round1(rand() * 0.2 + col.a) },
                        ..Default::default()
                    },
                );
                j += 1.0;
            }
        }
        l1.push([nx + (ns[i][0] - 0.5) * wid - wid / 2.0, ny]);
        l2.push([nx + (ns[i][1] - 0.5) * wid + wid / 2.0, ny]);
    }
    line(c, l1, col, 1.5);
    line(c, l2, col, 1.5);
}

pub fn tree02(c: &mut Canv, x: f64, y: f64, hei: f64, wid: f64, clu: usize, col: Col) {
    let cl = clu as f64;
    for _ in 0..clu {
        let bx = x + gaussian() * cl * 4.0;
        let by = y + gaussian() * cl * 4.0;
        blob(
            c,
            bx,
            by,
            BlobArgs {
                ang: PI / 2.0,
                fun: &leaf_fun,
                wid: rand() * wid * 0.75 + wid * 0.5,
                len: rand() * hei * 0.75 + hei * 0.5,
                col,
                ..Default::default()
            },
        );
    }
}

pub fn tree03(c: &mut Canv, x: f64, y: f64, hei: f64, wid: f64, ben: &dyn Fn(f64) -> f64, col: Col) {
    let reso = 10;
    let ns = trunk_noise(reso);
    let mut blobs = Canv::new();
    let (mut l1, mut l2) = (Vec::new(), Vec::new());
    let shape = |x: f64| (50.0 * x + 1.0).ln() / 3.95;
    for i in 0..reso {
        let rr = (reso - i) as f64;
        let nx = x + ben(i as f64 / reso as f64) * 100.0;
        let ny = y - i as f64 * hei / reso as f64;
        if i as f64 >= reso as f64 / 5.0 {
            for _ in 0..(reso - i) * 2 {
                let ox = rand() * wid * 2.0 * shape(rr / reso as f64);
                let bx = nx + ox * sign();
                let by = ny + (rand() - 0.5) * wid * 2.0;
                blob(
                    &mut blobs,
                    bx,
                    by,
                    BlobArgs {
                        len: ox * 2.0,
                        wid: rand() * 6.0 + 3.0,
                        ang: (rand() - 0.5) * PI / 6.0,
                        col: Col { g: col.g, a: round3(rand() * 0.2 + col.a) },
                        ..Default::default()
                    },
                );
            }
        }
        l1.push([nx + ((ns[i][0] - 0.5) * wid - wid / 2.0) * rr / reso as f64, ny]);
        l2.push([nx + ((ns[i][1] - 0.5) * wid + wid / 2.0) * rr / reso as f64, ny]);
    }
    poly(c, concat(&l1, &reversed(&l2)), Some(WHITE), Some(col), 1.5);
    c.extend(blobs);
}

pub fn branch(hei: f64, wid: f64, ang: f64, det: f64, ben: f64) -> [Vec<P>; 2] {
    let mut tl: Vec<P> = vec![[0.0, 0.0]];
    let (mut nx, mut ny) = (0.0, 0.0);
    let mut a0 = 0.0;
    let g = 3;
    for _ in 0..g {
        a0 += (ben / 2.0 + rand() * ben / 2.0) * sign();
        nx += a0.cos() * hei / g as f64;
        ny -= a0.sin() * hei / g as f64;
        tl.push([nx, ny]);
    }
    let last = tl[tl.len() - 1];
    let ta = last[1].atan2(last[0]);
    for p in tl.iter_mut() {
        let a = p[1].atan2(p[0]);
        let d = (p[0] * p[0] + p[1] * p[1]).sqrt();
        *p = [d * (a - ta + ang).cos(), d * (a - ta + ang).sin()];
    }

    let (mut t1, mut t2) = (Vec::new(), Vec::new());
    let span = det;
    let total = (tl.len() as f64 - 1.0) * span;
    let (mut lx, mut ly) = (0.0, 0.0);
    let mut i = 0.0;
    while i < total {
        let lp = tl[(i / span).floor() as usize];
        let np = tl[(i / span).ceil() as usize];
        let p = (i % span) / span;
        let nx = lp[0] * (1.0 - p) + np[0] * p;
        let ny = lp[1] * (1.0 - p) + np[1] * p;
        let an = (ny - ly).atan2(nx - lx);
        let woff = (noise(i * 0.3, 0.0, 0.0) - 0.5) * wid * hei / 80.0;
        let b = if p == 0.0 { rand() * wid } else { 0.0 };
        let nw = wid * ((total - i) / total * 0.5 + 0.5);
        let (c1, s1) = ((an + PI / 2.0).cos(), (an + PI / 2.0).sin());
        let (c2, s2) = ((an - PI / 2.0).cos(), (an - PI / 2.0).sin());
        t1.push([nx + c1 * (nw + woff + b), ny + s1 * (nw + woff + b)]);
        t2.push([nx + c2 * (nw - woff + b), ny + s2 * (nw - woff + b)]);
        lx = nx;
        ly = ny;
        i += 1.0;
    }
    [t1, t2]
}

#[derive(Clone, Copy)]
pub struct TwigArgs {
    pub dir: f64,
    pub sca: f64,
    pub wid: f64,
    pub ang: f64,
    pub lea: (bool, f64),
}

impl Default for TwigArgs {
    fn default() -> Self {
        TwigArgs { dir: 1.0, sca: 1.0, wid: 1.0, ang: 0.0, lea: (true, 12.0) }
    }
}

pub fn twig(c: &mut Canv, tx: f64, ty: f64, dep: i32, a: TwigArgs) {
    let mut tw = Vec::new();
    let tl = 10;
    let hs = rand() * 0.5 + 0.5;
    let tfun = |x: f64| -1.0 / (x + 1.0).powi(5) + 1.0;
    let a0 = rand() * PI / 6.0 * a.dir + a.ang;
    for i in 0..tl {
        let mx = a.dir * tfun(i as f64 / tl as f64) * 50.0 * a.sca * hs;
        let my = -(i as f64) * 5.0 * a.sca;
        let an = my.atan2(mx);
        let d = (mx * mx + my * my).sqrt();
        let nx = (an + a0).cos() * d;
        let ny = (an + a0).sin() * d;
        tw.push([nx + tx, ny + ty]);
        if (i == tl / 3 || i == tl * 2 / 3) && dep > 0 {
            twig(
                c,
                nx + tx,
                ny + ty,
                dep - 1,
                TwigArgs { ang: a.ang, sca: a.sca * 0.8, wid: a.wid, dir: a.dir * sign(), lea: a.lea },
            );
        }
        if i == tl - 1 && a.lea.0 {
            for j in 0..5 {
                let dj = (j as f64 - 2.5) * 5.0;
                blob(
                    c,
                    nx + tx + a.ang.cos() * dj * a.wid,
                    ny + ty + (a.ang.sin() * dj - a.lea.1 / (dep as f64 + 1.0)) * a.wid,
                    BlobArgs {
                        wid: (6.0 + 3.0 * rand()) * a.wid,
                        len: (15.0 + 12.0 * rand()) * a.wid,
                        ang: a.ang / 2.0 + PI / 2.0 + PI * 0.2 * (rand() - 0.5),
                        col: ink(round3(0.5 + dep as f64 * 0.2)),
                        fun: &leaf_fun,
                        ..Default::default()
                    },
                );
            }
        }
    }
    stroke(
        c,
        &tw,
        StrokeArgs { wid: 1.0, fun: &|x| (x * PI / 2.0).cos(), col: ink(0.5), ..Default::default() },
    );
}

fn bark(c: &mut Canv, x: f64, y: f64, wid: f64, ang: f64) {
    let pts = blob_pts(x, y, &BlobArgs { len: 10.0 + 10.0 * rand(), wid, ang, noi: 0.5, ..Default::default() });
    let fr = rand();
    stroke(
        c,
        &pts,
        StrokeArgs {
            wid: 0.8,
            noi: 0.0,
            col: ink(0.4),
            out: 0.0,
            fun: &|x| ((x + fr) * PI * 3.0).sin(),
            ..Default::default()
        },
    );
}

/// Bark texture along a trunk outline. Like the original (via shared array
/// references) this also nudges some of the outline points it is given.
pub fn barkify(c: &mut Canv, x: f64, y: f64, tr: &mut [Vec<P>; 2]) {
    let n = tr[0].len();
    for i in 2..n.saturating_sub(1) {
        let a0 = (tr[0][i][1] - tr[0][i - 1][1]).atan2(tr[0][i][0] - tr[0][i - 1][0]);
        let a1 = (tr[1][i][1] - tr[1][i - 1][1]).atan2(tr[1][i][0] - tr[1][i - 1][0]);
        let p = rand();
        let nx = tr[0][i][0] * (1.0 - p) + tr[1][i][0] * p;
        let ny = tr[0][i][1] * (1.0 - p) + tr[1][i][1] * p;
        if rand() < 0.2 {
            blob(
                c,
                nx + x,
                ny + y,
                BlobArgs {
                    noi: 1.0,
                    len: 15.0,
                    wid: 6.0 - (p - 0.5).abs() * 10.0,
                    ang: (a0 + a1) / 2.0,
                    col: ink(0.6),
                    ..Default::default()
                },
            );
        } else {
            bark(c, nx + x, ny + y, 5.0 - (p - 0.5).abs() * 10.0, (a0 + a1) / 2.0);
        }

        if rand() < 0.05 {
            let jl = rand() * 2.0 + 2.0;
            let xya = choice(&[(tr[0][i][0], tr[0][i][1], a0), (tr[1][i][0], tr[1][i][1], a1)]);
            let mut j = 0.0;
            while j < jl {
                blob(
                    c,
                    xya.0 + x + xya.2.cos() * (j - jl / 2.0) * 4.0,
                    xya.1 + y + xya.2.sin() * (j - jl / 2.0) * 4.0,
                    BlobArgs { wid: 4.0, len: 4.0 + 6.0 * rand(), ang: a0 + PI / 2.0, col: ink(0.6), ..Default::default() },
                );
                j += 1.0;
            }
        }
    }

    // trflist = trlist[0] ++ reverse(trlist[1]), as (side, index) references
    let refs: Vec<(usize, usize)> =
        (0..tr[0].len()).map(|i| (0, i)).chain((0..tr[1].len()).rev().map(|i| (1, i))).collect();
    let mut groups: Vec<Vec<(usize, usize)>> = vec![Vec::new()];
    for r in refs {
        if rand() < 0.5 {
            groups.push(Vec::new());
        } else {
            groups.last_mut().unwrap().push(r);
        }
    }
    for (i, g) in groups.iter().enumerate() {
        let src: Vec<P> = g.iter().map(|&(s, k)| tr[s][k]).collect();
        let mut pts = div(&src, 4.0);
        for (j, p) in pts.iter_mut().enumerate() {
            p[0] += (noise(i as f64, j as f64 * 0.1, 1.0) - 0.5) * (15.0 + 5.0 * gaussian());
            p[1] += (noise(i as f64, j as f64 * 0.1, 2.0) - 0.5) * (15.0 + 5.0 * gaussian());
        }
        // the last subdivided point is the original point object in JS
        if let (Some(&(s, k)), Some(&last)) = (g.last(), pts.last()) {
            tr[s][k] = last;
        }
        stroke(c, &offset(&pts, x, y), StrokeArgs { wid: 1.5, col: ink(0.7), out: 0.0, ..Default::default() });
    }
}

fn trunk_stroke(c: &mut Canv, pts: &[P], x: f64, y: f64, alpha: f64) {
    stroke(
        c,
        &offset(pts, x, y),
        StrokeArgs { col: ink(round3(alpha)), wid: 2.5, fun: &|_| 1f64.sin(), noi: 0.9, out: 0.0, ..Default::default() },
    );
}

fn joined(tr: &[Vec<P>; 2]) -> Vec<P> {
    concat(&tr[0], &reversed(&tr[1]))
}

pub fn tree04(c: &mut Canv, x: f64, y: f64, hei: f64, wid: f64, col: Col) {
    let mut tx = Canv::new();
    let mut tw = Canv::new();
    let mut tr = branch(hei, wid, -PI / 2.0, 10.0, PI * 0.2);
    barkify(&mut tx, x, y, &mut tr);
    let trlist = joined(&tr);
    let n = trlist.len() as f64;
    let mut trm = Vec::new();
    for (i, &tp) in trlist.iter().enumerate() {
        let fi = i as f64;
        if (fi >= n * 0.3 && fi <= n * 0.7 && rand() < 0.1) || fi == n / 2.0 - 1.0 {
            let ba = PI * 0.2 - PI * 1.4 * f64::from(u8::from(fi > n / 2.0));
            let mut br = branch(hei * (rand() + 1.0) * 0.3, wid * 0.5, ba, 10.0, PI * 0.2);
            br[0].remove(0);
            br[1].remove(0);
            let mut moved = [offset(&br[0], tp[0], tp[1]), offset(&br[1], tp[0], tp[1])];
            barkify(&mut tx, x, y, &mut moved);
            let bl = br[0].len();
            for j in 0..bl {
                if rand() < 0.2 || j == bl - 1 {
                    twig(
                        &mut tw,
                        br[0][j][0] + tp[0] + x,
                        br[0][j][1] + tp[1] + y,
                        1,
                        TwigArgs {
                            wid: hei / 300.0,
                            ang: if ba > -PI / 2.0 { ba } else { ba + PI },
                            sca: 0.5 * hei / 300.0,
                            dir: if ba > -PI / 2.0 { 1.0 } else { -1.0 },
                            ..Default::default()
                        },
                    );
                }
            }
            trm.extend(offset(&joined(&br), tp[0], tp[1]));
        } else {
            trm.push(tp);
        }
    }
    poly(c, offset(&trm, x, y), Some(WHITE), Some(col), 0.0);
    if trm.len() >= 2 {
        trm.remove(0);
        trm.pop();
    }
    trunk_stroke(c, &trm, x, y, 0.4 + rand() * 0.1);
    c.extend(tx);
    c.extend(tw);
}

pub fn tree05(c: &mut Canv, x: f64, y: f64, hei: f64, wid: f64, col: Col) {
    let mut tx = Canv::new();
    let mut tw = Canv::new();
    let mut tr = branch(hei, wid, -PI / 2.0, 10.0, 0.0);
    barkify(&mut tx, x, y, &mut tr);
    let trlist = joined(&tr);
    let n = trlist.len() as f64;
    let mut trm = Vec::new();
    for (i, &tp) in trlist.iter().enumerate() {
        let fi = i as f64;
        let p = (fi - n * 0.5).abs() / (n * 0.5);
        if (fi >= n * 0.2 && fi <= n * 0.8 && i % 3 == 0 && rand() > p) || fi == n / 2.0 - 1.0 {
            let bar = rand() * 0.2;
            let ba = -bar * PI - (1.0 - bar * 2.0) * PI * f64::from(u8::from(fi > n / 2.0));
            let mut br = branch(hei * (0.3 * p - rand() * 0.05), wid * 0.5, ba, 10.0, 0.5);
            br[0].remove(0);
            br[1].remove(0);
            let bl = br[0].len();
            for j in 0..bl {
                if j % 20 == 0 || j == bl - 1 {
                    twig(
                        &mut tw,
                        br[0][j][0] + tp[0] + x,
                        br[0][j][1] + tp[1] + y,
                        0,
                        TwigArgs {
                            wid: hei / 300.0,
                            ang: if ba > -PI / 2.0 { ba } else { ba + PI },
                            sca: 0.2 * hei / 300.0,
                            dir: if ba > -PI / 2.0 { 1.0 } else { -1.0 },
                            lea: (true, 5.0),
                        },
                    );
                }
            }
            trm.extend(offset(&joined(&br), tp[0], tp[1]));
        } else {
            trm.push(tp);
        }
    }
    poly(c, offset(&trm, x, y), Some(WHITE), Some(col), 0.0);
    if trm.len() >= 2 {
        trm.remove(0);
        trm.pop();
    }
    trunk_stroke(c, &trm, x, y, 0.4 + rand() * 0.1);
    c.extend(tx);
    c.extend(tw);
}

#[allow(clippy::too_many_arguments)]
fn frac_tree06(
    tx: &mut Canv,
    tw: &mut Canv,
    xoff: f64,
    yoff: f64,
    dep: i32,
    hei: f64,
    wid: f64,
    ang: f64,
    ben: f64,
) -> Vec<P> {
    let mut tr = branch(hei, wid, ang, hei / 20.0, ben);
    barkify(tx, xoff, yoff, &mut tr);
    let trlist = joined(&tr);
    let n = trlist.len();
    let nf = n as f64;
    let mut trm = Vec::new();
    for (i, &tp) in trlist.iter().enumerate() {
        let fi = i as f64;
        let pick = (rand() < 0.025 && fi >= nf * 0.2 && fi <= nf * 0.8)
            || i as i64 == (n / 2) as i64 - 1
            || i == n / 2 + 1;
        if pick && dep > 0 {
            let bar = 0.02 + rand() * 0.08;
            let ba = bar * PI - bar * 2.0 * PI * f64::from(u8::from(fi > nf / 2.0));
            let brlist = frac_tree06(
                tx,
                tw,
                tp[0] + xoff,
                tp[1] + yoff,
                dep - 1,
                hei * (0.7 + rand() * 0.2),
                wid * 0.6,
                ang + ba,
                0.55,
            );
            for b in &brlist {
                if rand() < 0.03 {
                    twig(
                        tw,
                        b[0] + tp[0] + xoff,
                        b[1] + tp[1] + yoff,
                        2,
                        TwigArgs {
                            ang: ba * (rand() * 0.5 + 0.75),
                            sca: 0.3,
                            dir: if ba > 0.0 { 1.0 } else { -1.0 },
                            lea: (false, 0.0),
                            ..Default::default()
                        },
                    );
                }
            }
            trm.extend(offset(&brlist, tp[0], tp[1]));
        } else {
            trm.push(tp);
        }
    }
    trm
}

pub fn tree06(c: &mut Canv, x: f64, y: f64, hei: f64, wid: f64, col: Col) {
    let mut tx = Canv::new();
    let mut tw = Canv::new();
    let mut trm = frac_tree06(&mut tx, &mut tw, x, y, 3, hei, wid, -PI / 2.0, 0.0);
    poly(c, offset(&trm, x, y), Some(WHITE), Some(col), 0.0);
    if trm.len() >= 2 {
        trm.remove(0);
        trm.pop();
    }
    trunk_stroke(c, &trm, x, y, 0.4 + rand() * 0.1);
    c.extend(tx);
    c.extend(tw);
}

pub fn tree07(c: &mut Canv, x: f64, y: f64, hei: f64, wid: f64) {
    let ben = |x: f64| x.sqrt() * 0.2;
    let reso = 10;
    let ns = trunk_noise(reso);
    let leaf = |x: f64| {
        if x <= 1.0 {
            2.75 * x * (1.0 - x).powf(1.0 / 1.8)
        } else {
            2.75 * (x - 2.0) * (x - 1.0).powf(1.0 / 1.8)
        }
    };
    let (mut l1, mut l2) = (Vec::new(), Vec::new());
    let mut tris = Vec::new();
    for i in 0..reso {
        let rr = (reso - i) as f64;
        let nx = x + ben(i as f64 / reso as f64) * 100.0;
        let ny = y - i as f64 * hei / reso as f64;
        if i as f64 >= reso as f64 / 4.0 {
            let bx = nx + (rand() - 0.5) * wid * 1.2 * rr * 0.5;
            let by = ny + (rand() - 0.5) * wid * 0.5;
            let bpl = blob_pts(
                bx,
                by,
                &BlobArgs {
                    len: rand() * 50.0 + 20.0,
                    wid: rand() * 12.0 + 12.0,
                    ang: -rand() * PI / 6.0,
                    fun: &leaf,
                    ..Default::default()
                },
            );
            tris.extend(triangulate(&bpl, 50.0, true, false));
        }
        l1.push([nx + (ns[i][0] - 0.5) * wid - wid / 2.0, ny]);
        l2.push([nx + (ns[i][1] - 0.5) * wid + wid / 2.0, ny]);
    }
    let mut all = triangulate(&concat(&l1, &reversed(&l2)), 50.0, true, true);
    all.extend(tris);
    for t in all {
        let m = mid_pt(&t);
        let v = (noise(m[0] * 0.02, m[1] * 0.02, 0.0) * 200.0 + 50.0).floor();
        fill(c, t, rgba(v, 0.8));
    }
}

fn frac_tree08(c: &mut Canv, xoff: f64, yoff: f64, dep: i32, ang: f64, len: f64, ben: f64) {
    let spt = [xoff, yoff];
    let ept = [xoff + ang.cos() * len, yoff + ang.sin() * len];
    let bsign = sign();
    let mut trm = div(&[[xoff, yoff], [xoff + len, yoff]], 10.0);
    let n = trm.len() as f64;
    for (i, p) in trm.iter_mut().enumerate() {
        p[1] += bsign * (i as f64 / n * PI).sin() * 2.0;
    }
    for p in trm.iter_mut() {
        let d = distance(*p, spt);
        let a = (p[1] - spt[1]).atan2(p[0] - spt[0]);
        *p = [spt[0] + d * (a + ang).cos(), spt[1] + d * (a + ang).sin()];
    }
    let fun: &dyn Fn(f64) -> f64 = if dep == 0 { &|x| (0.5 * PI * x).cos() } else { &one };
    stroke(c, &trm, StrokeArgs { fun, wid: 0.8, col: ink(0.5), ..Default::default() });
    if dep != 0 {
        let nben = ben + sign() * PI * 0.001 * (dep * dep) as f64;
        if rand() < 0.5 {
            let a1 = choice(&[norm_rand(-1.0, 0.5), norm_rand(0.5, 1.0)]);
            frac_tree08(c, ept[0], ept[1], dep - 1, ang + ben + PI * a1 * 0.2, len * norm_rand(0.8, 0.9), nben);
            let a2 = choice(&[norm_rand(-1.0, -0.5), norm_rand(0.5, 1.0)]);
            frac_tree08(c, ept[0], ept[1], dep - 1, ang + ben + PI * a2 * 0.2, len * norm_rand(0.8, 0.9), nben);
        } else {
            frac_tree08(c, ept[0], ept[1], dep - 1, ang + ben, len * norm_rand(0.8, 0.9), nben);
        }
    }
}

pub fn tree08(c: &mut Canv, x: f64, y: f64, hei: f64, wid: f64, col: Col) {
    let mut tw = Canv::new();
    let ang = norm_rand(-1.0, 1.0) * PI * 0.2;
    let tr = branch(hei, wid, -PI / 2.0 + ang, hei / 20.0, PI * 0.2);
    let trlist = joined(&tr);
    let half = trlist.len() / 2;
    for (i, tp) in trlist.iter().enumerate() {
        if rand() < 0.2 {
            let dep = (4.0 * rand()).floor() as i32;
            frac_tree08(&mut tw, x + tp[0], y + tp[1], dep, -PI / 2.0 - ang * rand(), 15.0, 0.0);
        } else if i == half {
            frac_tree08(&mut tw, x + tp[0], y + tp[1], 3, -PI / 2.0 + ang, 15.0, 0.0);
        }
    }
    poly(c, offset(&trlist, x, y), Some(WHITE), Some(col), 0.0);
    trunk_stroke(c, &trlist, x, y, 0.6 + rand() * 0.1);
    c.extend(tw);
}
