//! Mount.* from the original.

use crate::arch;
use crate::draw::*;
use crate::geom::*;
use crate::noise::noise;
use crate::rng::*;
use crate::tree;
use std::f64::consts::PI;

fn foot(c: &mut Canv, pl: &[Vec<P>], xof: f64, yof: f64) {
    let mut ft: Vec<Vec<P>> = Vec::new();
    let span = 10;
    let mut ni = 0;
    for i in 0..pl.len().saturating_sub(2) {
        if i != ni {
            continue;
        }
        ni = (ni + choice(&[1, 2])).min(pl.len() - 1);
        let row = &pl[i];
        let rl = row.len();
        let (mut a, mut b) = (Vec::new(), Vec::new());
        let mut j = 0;
        while (j as f64) < (rl as f64 / 8.0).min(10.0) {
            let k = noise(j as f64 * 0.1, i as f64, 0.0) * 10.0;
            a.push([row[j][0] + k, row[j][1]]);
            b.push([row[rl - 1 - j][0] - k, row[rl - 1 - j][1]]);
            j += 1;
        }
        a.reverse();
        b.reverse();
        for j in 0..span {
            let p = j as f64 / span as f64;
            let x1 = row[0][0] * (1.0 - p) + pl[ni][0][0] * p;
            let mut y1 = row[0][1] * (1.0 - p) + pl[ni][0][1] * p;
            let x2 = row[rl - 1][0] * (1.0 - p) + pl[ni][rl - 1][0] * p;
            let mut y2 = row[rl - 1][1] * (1.0 - p) + pl[ni][rl - 1][1] * p;
            let vib = -1.7 * (p - 1.0) * p.powf(1.0 / 5.0);
            let k = vib * 5.0 + noise(xof * 0.05, i as f64, 0.0) * 5.0;
            y1 += k;
            y2 += k;
            a.push([x1, y1]);
            b.push([x2, y2]);
        }
        ft.push(a);
        ft.push(b);
    }
    for f in &ft {
        fill(c, offset(f, xof, yof), WHITE);
    }
    for f in &ft {
        stroke(
            c,
            &offset(f, xof, yof),
            StrokeArgs { col: ink(round3(0.1 + rand() * 0.1)), wid: 1.0, ..Default::default() },
        );
    }
}

/// Points of the contour grid that satisfy `grow` (the original `vegetate`).
fn vegetate(pl: &[Vec<P>], mut grow: impl FnMut(usize, usize, P) -> bool) -> Vec<P> {
    let mut v = Vec::new();
    for (i, row) in pl.iter().enumerate() {
        for (j, &p) in row.iter().enumerate() {
            if grow(i, j, p) {
                v.push(p);
            }
        }
    }
    v
}

fn veg_col(x: f64, y: f64, base: f64) -> Col {
    ink(round3(noise(0.01 * x, 0.01 * y, 0.0) * 0.5 * 0.3 + base))
}

#[allow(clippy::too_many_arguments)]
pub fn mountain(c: &mut Canv, xoff: f64, yoff: f64, seed: f64, hei: f64, wid: f64, tex: usize, veg: bool) {
    let (h, w) = (hei, wid);
    let reso = [10usize, 50usize];
    let mut pl: Vec<Vec<P>> = Vec::new();
    let mut hoff = 0.0;
    for j in 0..reso[0] {
        hoff += rand() * yoff / 100.0;
        let row = (0..reso[1])
            .map(|i| {
                let x = (i as f64 / reso[1] as f64 - 0.5) * PI;
                let y = x.cos() * noise(x + 10.0, j as f64 * 0.15, seed);
                let p = 1.0 - j as f64 / reso[0] as f64;
                [(x / PI) * w * p, -y * h * p + hoff]
            })
            .collect();
        pl.push(row);
    }
    let last = reso[1] - 1;

    // RIM
    for v in vegetate(&pl, |i, j, p| {
        let ns = noise(j as f64 * 0.1, seed, 0.0);
        i == 0 && ns * ns * ns < 0.1 && p[1].abs() / h > 0.2
    }) {
        tree::tree02(c, v[0] + xoff, v[1] + yoff - 5.0, 16.0, 8.0, 2, veg_col(v[0], v[1], 0.5));
    }

    // WHITE BG
    fill(c, offset(&concat(&pl[0], &[[0.0, reso[0] as f64 * 4.0]]), xoff, yoff), WHITE);
    // OUTLINE
    stroke(c, &offset(&pl[0], xoff, yoff), StrokeArgs { col: ink(0.3), noi: 1.0, wid: 3.0, ..Default::default() });

    foot(c, &pl, xoff, yoff);
    texture(c, &pl, TexArgs { xof: xoff, yof: yoff, tex, sha: choice(&[0, 0, 0, 0, 5]), ..Default::default() });

    // TOP
    for v in vegetate(&pl, |i, j, p| {
        let ns = noise(i as f64 * 0.1, j as f64 * 0.1, seed + 2.0);
        ns * ns * ns < 0.1 && p[1].abs() / h > 0.5
    }) {
        tree::tree02(c, v[0] + xoff, v[1] + yoff, 16.0, 8.0, 5, veg_col(v[0], v[1], 0.5));
    }

    if veg {
        // MIDDLE
        let vl = vegetate(&pl, |i, j, p| {
            let ns = noise(i as f64 * 0.2, j as f64 * 0.05, seed);
            j % 2 == 1 && ns * ns * ns * ns < 0.012 && p[1].abs() / h < 0.3
        });
        for i in 0..vl.len() {
            let mut counter = 0;
            let mut dense = false;
            for j in 0..vl.len() {
                if i != j && (vl[i][0] - vl[j][0]).powi(2) + (vl[i][1] - vl[j][1]).powi(2) < 30.0 * 30.0 {
                    counter += 1;
                }
                if counter > 2 {
                    dense = true;
                    break;
                }
            }
            if dense {
                let (x, y) = (vl[i][0], vl[i][1]);
                let ht = (h + y) / h * 70.0;
                let ht = ht * 0.3 + rand() * ht * 0.7;
                tree::tree01(c, x + xoff, y + yoff, ht, rand() * 3.0 + 1.0, veg_col(x, y, 0.3));
            }
        }

        // BOTTOM
        for v in vegetate(&pl, |i, j, _| {
            let ns = noise(i as f64 * 0.2, j as f64 * 0.05, seed);
            (j == 0 || j == last) && ns * ns * ns * ns < 0.012
        }) {
            let (x, y) = (v[0], v[1]);
            let ht = (h + y) / h * 120.0;
            let ht = ht * 0.5 + rand() * ht * 0.5;
            let bc = rand() * 0.1;
            let bp = 1.0;
            tree::tree03(c, x + xoff, y + yoff, ht, 5.0, &|x| (x * bc).powf(bp), veg_col(x, y, 0.3));
        }
    }

    // BOTT ARCH
    for v in vegetate(&pl, |i, j, _| {
        let ns = noise(i as f64 * 0.2, j as f64 * 0.05, seed + 10.0);
        i != 0 && (j == 1 || j == last - 1) && ns * ns * ns * ns < 0.008
    }) {
        match choice(&[0, 0, 1, 1, 1, 2]) {
            1 => {
                let a = arch::Arch02 {
                    wid: norm_rand(40.0, 70.0),
                    sto: choice(&[1, 2, 2, 3]),
                    rot: rand(),
                    sty: choice(&[1, 2, 3]),
                    ..Default::default()
                };
                arch::arch02(c, v[0] + xoff, v[1] + yoff, seed, a);
            }
            2 => arch::arch04(c, v[0] + xoff, v[1] + yoff, seed, choice(&[1, 1, 1, 2, 2])),
            _ => {}
        }
    }

    // TOP ARCH
    for v in vegetate(&pl, |i, j, _| i == 1 && (j as f64 - reso[1] as f64 / 2.0).abs() < 1.0 && rand() < 0.02) {
        arch::arch03(c, v[0] + xoff, v[1] + yoff, seed, choice(&[5, 7]), 40.0 + rand() * 20.0);
    }

    // TRANSM
    for v in vegetate(&pl, |i, j, _| {
        let ns = noise(i as f64 * 0.2, j as f64 * 0.05, seed + 20.0 * PI);
        i % 2 == 0 && (j == 1 || j == last - 1) && ns * ns * ns * ns < 0.002
    }) {
        arch::transmission_tower01(c, v[0] + xoff, v[1] + yoff);
    }

    // BOTT ROCK
    for v in vegetate(&pl, |_, j, _| (j == 0 || j == last) && rand() < 0.1) {
        rock(c, v[0] + xoff, v[1] + yoff, seed, 20.0 + rand() * 20.0, 20.0 + rand() * 20.0, 40, 2);
    }
}

#[allow(clippy::too_many_arguments)]
pub fn flat_mount(c: &mut Canv, xoff: f64, yoff: f64, seed: f64, hei: f64, wid: f64, tex: usize, cho: f64) {
    let reso = [5usize, 50usize];
    let mut pl: Vec<Vec<P>> = Vec::new();
    let mut flat: Vec<Vec<P>> = Vec::new();
    let mut hoff = 0.0;
    for j in 0..reso[0] {
        hoff += rand() * yoff / 100.0;
        let mut row: Vec<P> = Vec::new();
        let mut fl: Vec<P> = Vec::new();
        for i in 0..reso[1] {
            let x = (i as f64 / reso[1] as f64 - 0.5) * PI;
            let y = ((x * 2.0).cos() + 1.0) * noise(x + 10.0, j as f64 * 0.1, seed);
            let p = 1.0 - j as f64 / reso[0] as f64 * 0.6;
            let nx = (x / PI) * wid * p;
            let mut ny = -y * hei * p + hoff;
            let hh = 100.0;
            if ny < -hh * cho + hoff {
                ny = -hh * cho + hoff;
                if fl.len() % 2 == 0 {
                    fl.push([nx, ny]);
                }
            } else if fl.len() % 2 == 1 {
                fl.push(*row.last().unwrap());
            }
            row.push([nx, ny]);
        }
        pl.push(row);
        flat.push(fl);
    }

    // WHITE BG
    fill(c, offset(&concat(&pl[0], &[[0.0, reso[0] as f64 * 4.0]]), xoff, yoff), WHITE);
    // OUTLINE
    stroke(c, &offset(&pl[0], xoff, yoff), StrokeArgs { col: ink(0.3), noi: 1.0, wid: 3.0, ..Default::default() });

    texture(
        c,
        &pl,
        TexArgs {
            xof: xoff,
            yof: yoff,
            tex,
            wid: 2.0,
            dis: &|| if rand() > 0.5 { 0.1 + 0.4 * rand() } else { 0.9 - 0.4 * rand() },
            ..Default::default()
        },
    );

    let (mut g1, mut g2): (Vec<P>, Vec<P>) = (Vec::new(), Vec::new());
    for f in flat.iter().step_by(2) {
        if f.len() >= 2 {
            g1.push(f[0]);
            g2.push(f[f.len() - 1]);
        }
    }
    if g1.is_empty() {
        return;
    }
    let wb = [g1[0][0], g2[0][0]];
    for i in 0..3 {
        let p = 0.8 - i as f64 * 0.2;
        g1.insert(0, [wb[0] * p, g1[0][1] - 5.0]);
        g2.insert(0, [wb[1] * p, g2[0][1] - 5.0]);
    }
    let wb = [g1[g1.len() - 1][0], g2[g2.len() - 1][0]];
    for i in 0..3 {
        let p = 0.6 - (i * i) as f64 * 0.1;
        g1.push([wb[0] * p, g1[g1.len() - 1][1] + 1.0]);
        g2.push([wb[1] * p, g2[g2.len() - 1][1] + 1.0]);
    }
    let d = 5;
    let g1 = div(&g1, d as f64);
    let g2 = div(&g2, d as f64);
    let r1 = reversed(&g1);
    let mut gr = concat(&r1, &concat(&g2, &[r1[0]]));
    let last = gr.len() - 1;
    for i in 0..gr.len() {
        // first and last entries are the same point object in JS
        if i == last {
            gr[last][0] = gr[0][0];
        }
        let v = (1.0 - ((i % d) as f64 - d as f64 / 2.0).abs() / (d as f64 / 2.0)) * 0.12;
        gr[i][0] *= 1.0 - v + noise(gr[i][1] * 0.5, 0.0, 0.0) * v;
        if i == last {
            gr[0][0] = gr[last][0];
        }
    }
    fill(c, offset(&gr, xoff, yoff), WHITE);
    stroke(c, &offset(&gr, xoff, yoff), StrokeArgs { wid: 3.0, col: ink(0.2), ..Default::default() });

    let mut bd = [f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY];
    for p in &gr {
        bd[0] = bd[0].min(p[0]);
        bd[1] = bd[1].max(p[0]);
        bd[2] = bd[2].min(p[1]);
        bd[3] = bd[3].max(p[1]);
    }
    flat_dec(c, xoff, yoff, bd);
}

/// Decorations on a flat mount; `bd` = [xmin, xmax, ymin, ymax].
fn flat_dec(c: &mut Canv, xoff: f64, yoff: f64, bd: [f64; 4]) {
    let [xmin, xmax, ymin, ymax] = bd;
    let ymid = (ymin + ymax) / 2.0;
    let tt = choice(&[0, 0, 1, 2, 3, 4]);

    // Loop bounds are re-drawn every iteration in the original.
    let mut j = 0.0;
    while j < rand() * 5.0 {
        rock(
            c,
            xoff + norm_rand(xmin, xmax),
            yoff + ymid + norm_rand(-10.0, 10.0) + 10.0,
            rand() * 100.0,
            10.0 + rand() * 20.0,
            10.0 + rand() * 20.0,
            40,
            2,
        );
        j += 1.0;
    }
    let mut j = 0;
    while j < choice(&[0, 0, 1, 2]) {
        let xr = xoff + norm_rand(xmin, xmax);
        let yr = yoff + ymid + norm_rand(-5.0, 5.0) + 20.0;
        let mut k = 0.0;
        while k < 2.0 + rand() * 3.0 {
            tree::tree08(c, xr + norm_rand(-30.0, 30.0).max(xmin).min(xmax), yr, 60.0 + rand() * 40.0, 1.0, ink(0.5));
            k += 1.0;
        }
        j += 1;
    }

    let big_rocks = |c: &mut Canv, n: f64| {
        let mut j = 0.0;
        while j < rand() * n {
            rock(
                c,
                xoff + norm_rand(xmin, xmax),
                yoff + ymid + norm_rand(-5.0, 5.0) + 20.0,
                rand() * 100.0,
                50.0 + rand() * 20.0,
                40.0 + rand() * 20.0,
                40,
                5,
            );
            j += 1.0;
        }
    };

    if tt == 0 {
        big_rocks(c, 3.0);
    }
    if tt == 1 {
        let pmin = rand() * 0.5;
        let pmax = rand() * 0.5 + 0.5;
        let x0 = xmin * (1.0 - pmin) + xmax * pmin;
        let x1 = xmin * (1.0 - pmax) + xmax * pmax;
        let mut i = x0;
        while i < x1 {
            tree::tree05(c, xoff + i + 20.0 * norm_rand(-1.0, 1.0), yoff + ymid + 20.0, 100.0 + rand() * 200.0, 5.0, ink(0.5));
            i += 30.0;
        }
        big_rocks(c, 4.0);
    } else if tt == 2 {
        let mut i = 0;
        while i < choice(&[1, 1, 1, 1, 2, 2, 3]) {
            let xr = norm_rand(xmin, xmax);
            tree::tree04(c, xoff + xr, yoff + ymid + 20.0, 300.0, 6.0, ink(0.5));
            let mut j = 0;
            while (j as f64) < rand() * 2.0 {
                rock(
                    c,
                    xoff + xmin.max(xmax.min(xr + norm_rand(-50.0, 50.0))),
                    yoff + ymid + norm_rand(-5.0, 5.0) + 20.0,
                    (j * i) as f64 * rand() * 100.0,
                    50.0 + rand() * 20.0,
                    40.0 + rand() * 20.0,
                    40,
                    5,
                );
                j += 1;
            }
            i += 1;
        }
    } else if tt == 3 {
        let mut i = 0;
        while i < choice(&[1, 1, 1, 1, 2, 2, 3]) {
            tree::tree06(c, xoff + norm_rand(xmin, xmax), yoff + ymid, 60.0 + rand() * 60.0, 6.0, ink(0.5));
            i += 1;
        }
    } else if tt == 4 {
        let pmin = rand() * 0.5;
        let pmax = rand() * 0.5 + 0.5;
        let x0 = xmin * (1.0 - pmin) + xmax * pmin;
        let x1 = xmin * (1.0 - pmax) + xmax * pmax;
        let mut i = x0;
        while i < x1 {
            tree::tree07(c, xoff + i + 20.0 * norm_rand(-1.0, 1.0), yoff + ymid + norm_rand(-1.0, 1.0), norm_rand(40.0, 80.0), 4.0);
            i += 20.0;
        }
    }

    let mut i = 0.0;
    while i < 50.0 * rand() {
        tree::tree02(c, xoff + norm_rand(xmin, xmax), yoff + norm_rand(ymin, ymax), 16.0, 8.0, 5, ink(0.5));
        i += 1.0;
    }

    let ts = choice(&[0, 0, 0, 0, 1]);
    if ts == 1 && tt != 4 {
        let x = xoff + norm_rand(xmin, xmax);
        let y = yoff + ymid + 20.0;
        arch::arch01(c, x, y, rand(), norm_rand(80.0, 100.0), norm_rand(160.0, 200.0), rand());
    }
}

pub fn dist_mount(c: &mut Canv, xoff: f64, yoff: f64, seed: f64, hei: f64, len: f64, seg: usize) {
    let span = 10.0;
    let segf = seg as f64;
    let mut pl: Vec<Vec<P>> = Vec::new();
    let mut i = 0usize;
    while (i as f64) < len / span / segf {
        let mut row: Vec<P> = Vec::new();
        for j in 0..=seg {
            let k = (i * seg + j) as f64;
            row.push([
                xoff + k * span,
                yoff - hei * noise(k * 0.05, seed, 0.0) * ((PI * k) / (len / span)).sin().powf(0.5),
            ]);
        }
        let mut j = 0;
        while (j as f64) < segf / 2.0 + 1.0 {
            let k = (i * seg + j * 2) as f64;
            row.insert(
                0,
                [xoff + k * span, yoff + 24.0 * noise(k * 0.05, 2.0, seed) * ((PI * k) / (len / span)).sin()],
            );
            j += 1;
        }
        pl.push(row);
        i += 1;
    }
    let get_col = |x: f64, y: f64| {
        let v = (noise(x * 0.02, y * 0.02, yoff) * 55.0 + 200.0).floor();
        rgba(v, 1.0)
    };
    for row in &pl {
        let l = row[row.len() - 1];
        fill(c, row.clone(), get_col(l[0], l[1]));
        for t in triangulate(row, 100.0, true, false) {
            let m = mid_pt(&t);
            let co = get_col(m[0], m[1]);
            poly(c, t, Some(co), Some(co), 1.0);
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn rock(c: &mut Canv, xoff: f64, yoff: f64, seed: f64, hei: f64, wid: f64, tex: usize, sha: usize) {
    let reso = [10usize, 50usize];
    let mut pl: Vec<Vec<P>> = Vec::new();
    for i in 0..reso[0] {
        let mut ns: Vec<f64> = (0..reso[1]).map(|j| noise(i as f64, j as f64 * 0.2, seed)).collect();
        loop_noise(&mut ns);
        let row = (0..reso[1])
            .map(|j| {
                let a = j as f64 / reso[1] as f64 * PI * 2.0 - PI / 2.0;
                let mut l = wid * hei / ((hei * a.cos()).powi(2) + (wid * a.sin()).powi(2)).sqrt();
                l *= 0.7 + 0.3 * ns[j];
                let p = 1.0 - i as f64 / reso[0] as f64;
                let nx = a.cos() * l * p;
                let mut ny = -a.sin() * l * p;
                if PI < a || a < 0.0 {
                    ny *= 0.2;
                }
                ny += hei * (i as f64 / reso[0] as f64) * 0.2;
                [nx, ny]
            })
            .collect();
        pl.push(row);
    }
    fill(c, offset(&concat(&pl[0], &[[0.0, 0.0]]), xoff, yoff), WHITE);
    stroke(c, &offset(&pl[0], xoff, yoff), StrokeArgs { col: ink(0.3), noi: 1.0, wid: 3.0, ..Default::default() });
    texture(
        c,
        &pl,
        TexArgs {
            xof: xoff,
            yof: yoff,
            tex,
            wid: 3.0,
            sha,
            col: &|_| rgba(180.0, round3(0.3 + rand() * 0.3)),
            dis: &|| if rand() > 0.5 { 0.15 + 0.15 * rand() } else { 0.85 - 0.15 * rand() },
            ..Default::default()
        },
    );
}
