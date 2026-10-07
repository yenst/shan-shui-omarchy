//! Arch.* from the original: huts, pavilions, pagodas, boats, towers.

use crate::draw::*;
use crate::geom::*;
use crate::man;
use crate::noise::noise;
use crate::rng::*;
use std::f64::consts::PI;

fn hut(c: &mut Canv, xoff: f64, yoff: f64, hei: f64, wid: f64, tex: usize) {
    let reso = [10usize, 10usize];
    let mut pl: Vec<Vec<P>> = Vec::new();
    for i in 0..reso[0] {
        let heir = hei + hei * 0.2 * rand();
        let row = (0..reso[1])
            .map(|j| {
                let fj = j as f64 / (reso[1] - 1) as f64;
                [wid * (i as f64 / (reso[0] - 1) as f64 - 0.5) * fj.powf(0.7), heir * fj]
            })
            .collect();
        pl.push(row);
    }
    let n = pl.len();
    let outline = concat(&pl[0][..reso[1] - 1], &reversed(&pl[n - 1][..reso[1] - 1]));
    fill(c, offset(&outline, xoff, yoff), WHITE);
    line(c, offset(&pl[0], xoff, yoff), ink(0.3), 2.0);
    line(c, offset(&pl[n - 1], xoff, yoff), ink(0.3), 2.0);
    texture(
        c,
        &pl,
        TexArgs {
            xof: xoff,
            yof: yoff,
            tex,
            wid: 1.0,
            len: 0.25,
            col: &|_| rgba(120.0, round3(0.3 + rand() * 0.3)),
            dis: &|| wtrand(|a| a * a),
            noi: &|_| 5.0,
            ..Default::default()
        },
    );
}

type Deco<'a> = &'a dyn Fn(P, P, P, P) -> Vec<Vec<P>>;

fn no_deco(_: P, _: P, _: P, _: P) -> Vec<Vec<P>> {
    Vec::new()
}

struct BoxArgs<'a> {
    hei: f64,
    wid: f64,
    rot: f64,
    per: f64,
    tra: bool,
    bot: bool,
    wei: f64,
    dec: Deco<'a>,
}

impl Default for BoxArgs<'_> {
    fn default() -> Self {
        BoxArgs { hei: 20.0, wid: 120.0, rot: 0.7, per: 4.0, tra: true, bot: true, wei: 3.0, dec: &no_deco }
    }
}

fn box_(c: &mut Canv, xoff: f64, yoff: f64, a: BoxArgs) {
    let (hei, wid, per) = (a.hei, a.wid, a.per);
    let mid = -wid * 0.5 + wid * a.rot;
    let bmid = -wid * 0.5 + wid * (1.0 - a.rot);
    let mut pl: Vec<Vec<P>> = Vec::new();
    pl.push(div(&[[-wid * 0.5, -hei], [-wid * 0.5, 0.0]], 5.0));
    pl.push(div(&[[wid * 0.5, -hei], [wid * 0.5, 0.0]], 5.0));
    if a.bot {
        pl.push(div(&[[-wid * 0.5, 0.0], [mid, per]], 5.0));
        pl.push(div(&[[wid * 0.5, 0.0], [mid, per]], 5.0));
    }
    pl.push(div(&[[mid, -hei], [mid, per]], 5.0));
    if a.tra {
        if a.bot {
            pl.push(div(&[[-wid * 0.5, 0.0], [bmid, -per]], 5.0));
            pl.push(div(&[[wid * 0.5, 0.0], [bmid, -per]], 5.0));
        }
        pl.push(div(&[[bmid, -hei], [bmid, -per]], 5.0));
    }
    let surf = if a.rot < 0.5 { 1.0 } else { -1.0 };
    pl.extend((a.dec)([surf * wid * 0.5, -hei], [mid, -hei + per], [surf * wid * 0.5, 0.0], [mid, per]));

    if !a.tra {
        let polist = [[-wid * 0.5, -hei], [wid * 0.5, -hei], [wid * 0.5, 0.0], [mid, per], [-wid * 0.5, 0.0]];
        fill(c, offset(&polist, xoff, yoff), WHITE);
    }
    for p in &pl {
        stroke(
            c,
            &offset(p, xoff, yoff),
            StrokeArgs { col: ink(0.4), noi: 1.0, wid: a.wei, fun: &one, ..Default::default() },
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn deco(style: usize, pul: P, pur: P, pdl: P, pdr: P, hsp: [usize; 2], vsp: [usize; 2]) -> Vec<Vec<P>> {
    let mut out = Vec::new();
    let dl = div(&[pul, pdl], vsp[1] as f64);
    let dr = div(&[pur, pdr], vsp[1] as f64);
    let du = div(&[pul, pur], hsp[1] as f64);
    let dd = div(&[pdl, pdr], hsp[1] as f64);
    let v1 = vsp[1] as f64;

    if style == 1 || style == 3 {
        let mlu = du[hsp[0]];
        let mru = du[du.len() - 1 - hsp[0]];
        let mld = dd[hsp[0]];
        let mrd = dd[du.len() - 1 - hsp[0]];
        let mut i = vsp[0];
        while i < dl.len() - vsp[0] {
            let mml = div(&[mlu, mld], v1)[i];
            let mmr = div(&[mru, mrd], v1)[i];
            if style == 1 {
                out.push(div(&[mml, dl[i]], 5.0));
                out.push(div(&[mmr, dr[i]], 5.0));
            } else {
                let mmu = div(&[mlu, mru], v1)[i];
                let mmd = div(&[mld, mrd], v1)[i];
                out.push(div(&[mml, mmr], 5.0));
                out.push(div(&[mmu, mmd], 5.0));
            }
            i += vsp[0];
        }
        out.push(div(&[mlu, mld], 5.0));
        out.push(div(&[mru, mrd], 5.0));
    } else if style == 2 {
        let mut i = hsp[0];
        while i < du.len() - hsp[0] {
            out.push(div(&[du[i], dd[i]], 5.0));
            i += hsp[0];
        }
    }
    out
}

struct RailArgs {
    hei: f64,
    wid: f64,
    rot: f64,
    per: f64,
    seg: usize,
    wei: f64,
    tra: bool,
    fro: bool,
}

impl Default for RailArgs {
    fn default() -> Self {
        RailArgs { hei: 20.0, wid: 180.0, rot: 0.7, per: 4.0, seg: 4, wei: 1.0, tra: true, fro: true }
    }
}

fn rail(c: &mut Canv, xoff: f64, yoff: f64, seed: f64, a: RailArgs) {
    let (hei, wid, per, seg) = (a.hei, a.wid, a.per, a.seg as f64);
    let mid = -wid * 0.5 + wid * a.rot;
    let bmid = -wid * 0.5 + wid * (1.0 - a.rot);
    let mut pl: Vec<Vec<P>> = Vec::new();
    if a.fro {
        pl.push(div(&[[-wid * 0.5, 0.0], [mid, per]], seg));
        pl.push(div(&[[mid, per], [wid * 0.5, 0.0]], seg));
    }
    if a.tra {
        pl.push(div(&[[-wid * 0.5, 0.0], [bmid, -per]], seg));
        pl.push(div(&[[bmid, -per], [wid * 0.5, 0.0]], seg));
    }
    if a.fro {
        pl.push(div(&[[-wid * 0.5, -hei], [mid, -hei + per]], seg));
        pl.push(div(&[[mid, -hei + per], [wid * 0.5, -hei]], seg));
    }
    if a.tra {
        pl.push(div(&[[-wid * 0.5, -hei], [bmid, -hei - per]], seg));
        pl.push(div(&[[bmid, -hei - per], [wid * 0.5, -hei]], seg));
    }
    if a.tra {
        // the original trims the same segment twice
        let open = (rand() * pl.len() as f64) as usize;
        pl[open].pop();
        pl[open].pop();
    }
    let n = pl.len();
    for i in 0..n / 2 {
        let k = (n / 2 + i) % n;
        let mut j = 0;
        while j < pl[i].len() {
            if pl[k].is_empty() {
                break;
            }
            pl[i][j][1] += (noise(i as f64, j as f64 * 0.5, seed) - 0.5) * hei;
            let m = j % pl[k].len();
            pl[k][m][1] += (noise(i as f64 + 0.5, j as f64 * 0.5, seed) - 0.5) * hei;
            let mut ln = div(&[pl[i][j], pl[k][m]], 2.0);
            ln[0][0] += (rand() - 0.5) * hei * 0.5;
            line(c, offset(&ln, xoff, yoff), ink(0.5), 2.0);
            j += 1;
        }
    }
    for p in &pl {
        stroke(
            c,
            &offset(p, xoff, yoff),
            StrokeArgs { col: ink(0.5), noi: 0.5, wid: a.wei, fun: &one, ..Default::default() },
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn roof(c: &mut Canv, xoff: f64, yoff: f64, hei: f64, wid: f64, rot: f64, per: f64, cor: f64, wei: f64) {
    let opf = |mut v: Vec<P>| {
        if rot < 0.5 {
            for p in v.iter_mut() {
                p[0] = -p[0];
            }
        }
        v
    };
    let rrot = if rot < 0.5 { 1.0 - rot } else { rot };
    let mid = -wid * 0.5 + wid * rrot;
    let quat = (mid + wid * 0.5) * 0.5 - mid;

    let pl = [
        div(
            &opf(vec![[-wid * 0.5 + quat, -hei - per / 2.0], [-wid * 0.5 + quat * 0.5, -hei / 2.0 - per / 4.0], [-wid * 0.5 - cor, 0.0]]),
            5.0,
        ),
        div(&opf(vec![[mid + quat, -hei], [(mid + quat + wid * 0.5) / 2.0, -hei / 2.0], [wid * 0.5 + cor, 0.0]]), 5.0),
        div(&opf(vec![[mid + quat, -hei], [mid + quat / 2.0, -hei / 2.0 + per / 2.0], [mid + cor, per]]), 5.0),
        div(&opf(vec![[-wid * 0.5 - cor, 0.0], [mid + cor, per]]), 5.0),
        div(&opf(vec![[wid * 0.5 + cor, 0.0], [mid + cor, per]]), 5.0),
        div(&opf(vec![[-wid * 0.5 + quat, -hei - per / 2.0], [mid + quat, -hei]]), 5.0),
    ];
    let polist = opf(vec![[-wid * 0.5, 0.0], [-wid * 0.5 + quat, -hei - per / 2.0], [mid + quat, -hei], [wid * 0.5, 0.0], [mid, per]]);
    fill(c, offset(&polist, xoff, yoff), WHITE);
    for p in &pl {
        stroke(
            c,
            &offset(p, xoff, yoff),
            StrokeArgs { col: ink(0.4), noi: 1.0, wid: wei, fun: &one, ..Default::default() },
        );
    }
}

fn pagroof(c: &mut Canv, xoff: f64, yoff: f64, hei: f64, wid: f64, per: f64, wei: f64) {
    let (cor, sid) = (10.0, 4usize);
    let mut pl: Vec<Vec<P>> = Vec::new();
    let mut polist: Vec<P> = vec![[0.0, -hei]];
    for i in 0..sid {
        let t = i as f64 / (sid - 1) as f64 - 0.5;
        let fx = wid * t;
        let fy = per * (1.0 - t.abs() * 2.0);
        let fxx = (wid + cor) * t;
        if i > 0 {
            let prev = pl[pl.len() - 1][2];
            pl.push(vec![prev, [fxx, fy]]);
        }
        pl.push(vec![[0.0, -hei], [fx * 0.5, (-hei + fy) * 0.5], [fxx, fy]]);
        polist.push([fxx, fy]);
    }
    fill(c, offset(&polist, xoff, yoff), WHITE);
    for p in &pl {
        stroke(
            c,
            &offset(&div(p, 5.0), xoff, yoff),
            StrokeArgs { col: ink(0.4), noi: 1.0, wid: wei, fun: &one, ..Default::default() },
        );
    }
}

/// Pavilion with people, on flat mounts.
pub fn arch01(c: &mut Canv, xoff: f64, yoff: f64, seed: f64, hei: f64, wid: f64, per: f64) {
    let p = 0.4 + rand() * 0.2;
    let h0 = hei * p;
    let h1 = hei * (1.0 - p);
    hut(c, xoff, yoff - hei, h0, wid, 300);
    box_(c, xoff, yoff, BoxArgs { hei: h1, wid: wid * 2.0 / 3.0, per, bot: false, ..Default::default() });
    rail(
        c,
        xoff,
        yoff,
        seed,
        RailArgs { tra: true, fro: false, hei: 10.0, wid, per: per * 2.0, seg: (3.0 + rand() * 3.0) as usize, ..Default::default() },
    );
    match choice(&[0, 1, 1, 2]) {
        1 => man::man(
            c,
            xoff + norm_rand(-wid / 3.0, wid / 3.0),
            yoff,
            man::ManArgs { fli: choice(&[true, false]), sca: 0.42, ..Default::default() },
        ),
        2 => {
            man::man(c, xoff + norm_rand(-wid / 4.0, -wid / 5.0), yoff, man::ManArgs { fli: false, sca: 0.42, ..Default::default() });
            man::man(c, xoff + norm_rand(wid / 5.0, wid / 4.0), yoff, man::ManArgs { fli: true, sca: 0.42, ..Default::default() });
        }
        _ => {}
    }
    rail(
        c,
        xoff,
        yoff,
        seed,
        RailArgs { tra: false, fro: true, hei: 10.0, wid, per: per * 2.0, seg: (3.0 + rand() * 3.0) as usize, ..Default::default() },
    );
}

pub struct Arch02 {
    pub hei: f64,
    pub wid: f64,
    pub rot: f64,
    pub per: f64,
    pub sto: usize,
    pub sty: usize,
}

impl Default for Arch02 {
    fn default() -> Self {
        Arch02 { hei: 10.0, wid: 50.0, rot: 0.3, per: 5.0, sto: 3, sty: 1 }
    }
}

/// Small houses at the foot of mountains.
pub fn arch02(c: &mut Canv, xoff: f64, yoff: f64, _seed: f64, a: Arch02) {
    let hsp = [[0, 0], [1, 5], [1, 5], [1, 4]][a.sty];
    let vsp = [[0, 0], [1, 2], [1, 2], [1, 3]][a.sty];
    let sty = a.sty;
    let mut hoff = 0.0;
    for i in 0..a.sto {
        let dec = |pul, pur, pdl, pdr| deco(sty, pul, pur, pdl, pdr, hsp, vsp);
        box_(
            c,
            xoff,
            yoff - hoff,
            BoxArgs { tra: false, hei: a.hei, wid: a.wid * 0.85f64.powi(i as i32), rot: a.rot, wei: 1.5, per: a.per, dec: &dec, ..Default::default() },
        );
        roof(c, xoff, yoff - hoff - a.hei, a.hei, a.wid * 0.9f64.powi(i as i32), a.rot, a.per, 5.0, 1.5);
        hoff += a.hei * 1.5;
    }
}

/// Tall pagoda on mountain tops.
pub fn arch03(c: &mut Canv, xoff: f64, yoff: f64, _seed: f64, sto: usize, wid: f64) {
    let (hei, rot, per) = (10.0, 0.7, 5.0);
    let mut hoff = 0.0;
    for i in 0..sto {
        let dec = |pul, pur, pdl, pdr| deco(1, pul, pur, pdl, pdr, [1, 4], [1, 2]);
        box_(
            c,
            xoff,
            yoff - hoff,
            BoxArgs { tra: false, hei, wid: wid * 0.85f64.powi(i as i32), rot, wei: 1.5, per: per / 2.0, dec: &dec, ..Default::default() },
        );
        rail(
            c,
            xoff,
            yoff - hoff,
            i as f64 * 0.2,
            RailArgs {
                seg: 5,
                wid: wid * 0.85f64.powi(i as i32) * 1.1,
                hei: hei / 2.0,
                per: per / 2.0,
                rot,
                wei: 0.5,
                tra: false,
                ..Default::default()
            },
        );
        pagroof(c, xoff, yoff - hoff - hei, hei * 1.5, wid * 0.9f64.powi(i as i32), per, 1.5);
        hoff += hei * 1.5;
    }
}

/// Open pavilion.
pub fn arch04(c: &mut Canv, xoff: f64, yoff: f64, _seed: f64, sto: usize) {
    let (hei, wid, rot, per) = (15.0, 30.0, 0.7, 5.0);
    let mut hoff = 0.0;
    for i in 0..sto {
        box_(
            c,
            xoff,
            yoff - hoff,
            BoxArgs { tra: true, hei, wid: wid * 0.85f64.powi(i as i32), rot, wei: 1.5, per: per / 2.0, ..Default::default() },
        );
        rail(
            c,
            xoff,
            yoff - hoff,
            i as f64 * 0.2,
            RailArgs {
                seg: 3,
                wid: wid * 0.85f64.powi(i as i32) * 1.2,
                hei: hei / 3.0,
                per: per / 2.0,
                rot,
                wei: 0.5,
                tra: true,
                ..Default::default()
            },
        );
        pagroof(c, xoff, yoff - hoff - hei, hei, wid * 0.9f64.powi(i as i32), per, 1.5);
        hoff += hei * 1.2;
    }
}

pub fn boat01(c: &mut Canv, xoff: f64, yoff: f64, sca: f64, fli: bool) {
    let len = 120.0;
    let dir = if fli { -1.0 } else { 1.0 };
    man::man(
        c,
        xoff + 20.0 * sca * dir,
        yoff,
        man::ManArgs {
            stick: true,
            hat: man::Hat::H02,
            sca: 0.5 * sca,
            fli: !fli,
            len: [0.0, 30.0, 20.0, 30.0, 10.0, 30.0, 30.0, 30.0, 30.0],
        },
    );
    let (mut p1, mut p2) = (Vec::new(), Vec::new());
    let f1 = |x: f64| (x * PI).sin().powf(0.5) * 7.0 * sca;
    let f2 = |x: f64| (x * PI).sin().powf(0.5) * 10.0 * sca;
    let mut i = 0.0;
    while i < len * sca {
        p1.push([i * dir, f1(i / len)]);
        p2.push([i * dir, f2(i / len)]);
        i += 5.0 * sca;
    }
    let pl = concat(&p1, &reversed(&p2));
    fill(c, offset(&pl, xoff, yoff), WHITE);
    stroke(
        c,
        &offset(&pl, xoff, yoff),
        StrokeArgs { wid: 1.0, fun: &|x| (x * PI * 2.0).sin(), col: ink(0.4), ..Default::default() },
    );
}

pub fn transmission_tower01(c: &mut Canv, xoff: f64, yoff: f64) {
    let (hei, wid) = (100.0, 20.0);
    let mut qs = |pl: &[P]| {
        stroke(
            c,
            &offset(&div(pl, 5.0), xoff, yoff),
            StrokeArgs { wid: 1.0, fun: &|_| 0.5, col: ink(0.4), ..Default::default() },
        );
    };
    let p00 = [-wid * 0.05, -hei];
    let p01 = [wid * 0.05, -hei];
    let p10 = [-wid * 0.1, -hei * 0.9];
    let p11 = [wid * 0.1, -hei * 0.9];
    let p20 = [-wid * 0.2, -hei * 0.5];
    let p21 = [wid * 0.2, -hei * 0.5];
    let p30 = [-wid * 0.5, 0.0];
    let p31 = [wid * 0.5, 0.0];

    for b in [[0.7, -0.85], [1.0, -0.675], [0.7, -0.5]] {
        qs(&[[-b[0] * wid, b[1] * hei], [b[0] * wid, b[1] * hei]]);
        qs(&[[-b[0] * wid, b[1] * hei], [0.0, (b[1] - 0.05) * hei]]);
        qs(&[[b[0] * wid, b[1] * hei], [0.0, (b[1] - 0.05) * hei]]);
        qs(&[[-b[0] * wid, b[1] * hei], [-b[0] * wid, (b[1] + 0.1) * hei]]);
        qs(&[[b[0] * wid, b[1] * hei], [b[0] * wid, (b[1] + 0.1) * hei]]);
    }
    let l10 = div(&[p00, p10, p20, p30], 5.0);
    let l11 = div(&[p01, p11, p21, p31], 5.0);
    for i in 0..l10.len() - 1 {
        qs(&[l10[i], l11[i + 1]]);
        qs(&[l11[i], l10[i + 1]]);
    }
    qs(&[p00, p01]);
    qs(&[p10, p11]);
    qs(&[p20, p21]);
    qs(&[p00, p10, p20, p30]);
    qs(&[p01, p11, p21, p31]);
}
