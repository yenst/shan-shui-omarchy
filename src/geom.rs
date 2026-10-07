//! PolyTools and Util from the original.

pub type P = [f64; 2];

pub fn mid_pt(pl: &[P]) -> P {
    let n = pl.len() as f64;
    pl.iter()
        .fold([0.0, 0.0], |acc, v| [acc[0] + v[0] / n, acc[1] + v[1] / n])
}

pub fn distance(a: P, b: P) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

pub fn mapval(v: f64, istart: f64, istop: f64, ostart: f64, ostop: f64) -> f64 {
    ostart + (ostop - ostart) * ((v - istart) / (istop - istart))
}

pub fn loop_noise(ns: &mut [f64]) {
    let n = ns.len();
    let dif = ns[n - 1] - ns[0];
    let (mut lo, mut hi) = (100.0f64, -100.0f64);
    for i in 0..n {
        ns[i] += dif * (n - 1 - i) as f64 / (n - 1) as f64;
        lo = lo.min(ns[i]);
        hi = hi.max(ns[i]);
    }
    for v in ns.iter_mut() {
        *v = mapval(*v, lo, hi, 0.0, 1.0);
    }
}

/// Subdivide a polyline, `reso` points per segment.
pub fn div(pl: &[P], reso: f64) -> Vec<P> {
    let tl = (pl.len() as f64 - 1.0) * reso;
    let mut out = Vec::new();
    let mut i = 0.0;
    while i < tl {
        let last = pl[(i / reso).floor() as usize];
        let next = pl[(i / reso).ceil() as usize];
        let p = (i % reso) / reso;
        out.push([last[0] * (1.0 - p) + next[0] * p, last[1] * (1.0 - p) + next[1] * p]);
        i += 1.0;
    }
    if let Some(l) = pl.last() {
        out.push(*l);
    }
    out
}

pub fn offset(pts: &[P], dx: f64, dy: f64) -> Vec<P> {
    pts.iter().map(|p| [p[0] + dx, p[1] + dy]).collect()
}

pub fn reversed(pts: &[P]) -> Vec<P> {
    pts.iter().rev().copied().collect()
}

pub fn concat(a: &[P], b: &[P]) -> Vec<P> {
    let mut v = Vec::with_capacity(a.len() + b.len());
    v.extend_from_slice(a);
    v.extend_from_slice(b);
    v
}

/// Rational quadratic bezier through midpoints.
pub fn bezmh(pts: &[P], w: f64) -> Vec<P> {
    let pts: Vec<P> = if pts.len() == 2 {
        vec![pts[0], mid_pt(&[pts[0], pts[1]]), pts[1]]
    } else {
        pts.to_vec()
    };
    let mut out = Vec::new();
    let n = pts.len();
    for j in 0..n - 2 {
        let p0 = if j == 0 { pts[j] } else { mid_pt(&[pts[j], pts[j + 1]]) };
        let p1 = pts[j + 1];
        let p2 = if j == n - 3 { pts[j + 2] } else { mid_pt(&[pts[j + 1], pts[j + 2]]) };
        let pl = 20;
        let cnt = pl + usize::from(j == n - 3);
        for i in 0..cnt {
            let t = i as f64 / pl as f64;
            let u = (1.0 - t).powi(2) + 2.0 * t * (1.0 - t) * w + t * t;
            out.push([
                ((1.0 - t).powi(2) * p0[0] + 2.0 * t * (1.0 - t) * p1[0] * w + t * t * p2[0]) / u,
                ((1.0 - t).powi(2) * p0[1] + 2.0 * t * (1.0 - t) * p1[1] * w + t * t * p2[1]) / u,
            ]);
        }
    }
    out
}

fn line_expr(p0: P, p1: P) -> (f64, f64) {
    let den = p1[0] - p0[0];
    let m = if den == 0.0 { f64::INFINITY } else { (p1[1] - p0[1]) / den };
    (m, p0[1] - m * p0[0])
}

fn intersect(l0: [P; 2], l1: [P; 2]) -> bool {
    let (m0, k0) = line_expr(l0[0], l0[1]);
    let (m1, k1) = line_expr(l1[0], l1[1]);
    let den = m0 - m1;
    if den == 0.0 {
        return false;
    }
    let x = (k1 - k0) / den;
    let y = m0 * x + k0;
    let on_seg = |ln: [P; 2]| {
        ln[0][0].min(ln[1][0]) <= x
            && x <= ln[0][0].max(ln[1][0])
            && ln[0][1].min(ln[1][1]) <= y
            && y <= ln[0][1].max(ln[1][1])
    };
    on_seg(l0) && on_seg(l1)
}

fn pt_in_poly(pt: P, pl: &[P]) -> bool {
    let mut count = 0;
    for i in 0..pl.len() {
        let np = pl[(i + 1) % pl.len()];
        if intersect([pl[i], np], [pt, [pt[0] + 999.0, pt[1] + 999.0]]) {
            count += 1;
        }
    }
    count % 2 == 1
}

fn ln_in_poly(ln: [P; 2], pl: &[P]) -> bool {
    let ep = 0.01;
    let lnc = [
        [ln[0][0] * (1.0 - ep) + ln[1][0] * ep, ln[0][1] * (1.0 - ep) + ln[1][1] * ep],
        [ln[0][0] * ep + ln[1][0] * (1.0 - ep), ln[0][1] * ep + ln[1][1] * (1.0 - ep)],
    ];
    for i in 0..pl.len() {
        let np = pl[(i + 1) % pl.len()];
        if intersect(lnc, [pl[i], np]) {
            return false;
        }
    }
    pt_in_poly(mid_pt(&ln), pl)
}

fn sides_of(pl: &[P]) -> Vec<f64> {
    (0..pl.len()).map(|i| distance(pl[i], pl[(i + 1) % pl.len()])).collect()
}

fn area_of(pl: &[P]) -> f64 {
    let s = sides_of(pl);
    let (a, b, c) = (s[0], s[1], s[2]);
    let p = (a + b + c) / 2.0;
    (p * (p - a) * (p - b) * (p - c)).sqrt()
}

fn sliver_ratio(pl: &[P]) -> f64 {
    area_of(pl) / sides_of(pl).iter().sum::<f64>()
}

fn best_ear(pl: &[P], convex: bool, optimize: bool) -> (Vec<P>, Vec<P>) {
    let n = pl.len();
    let mut cuts = Vec::new();
    for i in 0..n {
        let pt = pl[i];
        let lp = pl[if i != 0 { i - 1 } else { n - 1 }];
        let np = pl[(i + 1) % n];
        if convex || ln_in_poly([lp, np], pl) {
            let mut q = pl.to_vec();
            q.remove(i);
            let c = (vec![lp, pt, np], q);
            if !optimize {
                return c;
            }
            cuts.push(c);
        }
    }
    let mut best = (pl.to_vec(), Vec::new());
    let mut best_ratio = 0.0;
    for c in cuts {
        let r = sliver_ratio(&c.0);
        if r >= best_ratio {
            best_ratio = r;
            best = c;
        }
    }
    best
}

fn shatter(pl: &[P], a: f64, depth: u32, out: &mut Vec<Vec<P>>) {
    if pl.len() < 3 {
        return;
    }
    // The original keeps splitting on NaN areas (degenerate triangles), which
    // can blow up exponentially; treat those as done instead.
    let ar = area_of(pl);
    if !(ar >= a) || depth > 24 {
        out.push(pl.to_vec());
        return;
    }
    let s = sides_of(pl);
    let mut ind = 0;
    for i in 0..s.len() {
        if s[i] > s[ind] {
            ind = i;
        }
    }
    let nind = (ind + 1) % pl.len();
    let lind = (ind + 2) % pl.len();
    let mid = mid_pt(&[pl[ind], pl[nind]]);
    shatter(&[pl[ind], mid, pl[lind]], a, depth + 1, out);
    shatter(&[pl[lind], pl[nind], mid], a, depth + 1, out);
}

pub fn triangulate(pl: &[P], area: f64, convex: bool, optimize: bool) -> Vec<Vec<P>> {
    let mut out = Vec::new();
    let mut rest = pl.to_vec();
    while rest.len() > 3 {
        let (ear, q) = best_ear(&rest, convex, optimize);
        shatter(&ear, area, 0, &mut out);
        rest = q;
    }
    shatter(&rest, area, 0, &mut out);
    out
}
