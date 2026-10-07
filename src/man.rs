//! Man.* from the original: little figures in robes and hats.

use crate::draw::*;
use crate::geom::*;
use crate::noise::noise;
use crate::rng::*;
use std::f64::consts::PI;

fn expand(pts: &[P], wfun: &dyn Fn(f64) -> f64) -> (Vec<P>, Vec<P>) {
    let n = pts.len();
    let (mut v0, mut v1) = (Vec::new(), Vec::new());
    for i in 1..n - 1 {
        let w = wfun(i as f64 / n as f64);
        let a1 = (pts[i][1] - pts[i - 1][1]).atan2(pts[i][0] - pts[i - 1][0]);
        let a2 = (pts[i][1] - pts[i + 1][1]).atan2(pts[i][0] - pts[i + 1][0]);
        let mut a = (a1 + a2) / 2.0;
        if a < a2 {
            a += PI;
        }
        v0.push([pts[i][0] + w * a.cos(), pts[i][1] + w * a.sin()]);
        v1.push([pts[i][0] - w * a.cos(), pts[i][1] - w * a.sin()]);
    }
    let l = n - 1;
    let a0 = (pts[1][1] - pts[0][1]).atan2(pts[1][0] - pts[0][0]) - PI / 2.0;
    let a1 = (pts[l][1] - pts[l - 1][1]).atan2(pts[l][0] - pts[l - 1][0]) - PI / 2.0;
    let (w0, w1) = (wfun(0.0), wfun(1.0));
    v0.insert(0, [pts[0][0] + w0 * a0.cos(), pts[0][1] + w0 * a0.sin()]);
    v1.insert(0, [pts[0][0] - w0 * a0.cos(), pts[0][1] - w0 * a0.sin()]);
    v0.push([pts[l][0] + w1 * a1.cos(), pts[l][1] + w1 * a1.sin()]);
    v1.push([pts[l][0] - w1 * a1.cos(), pts[l][1] - w1 * a1.sin()]);
    (v0, v1)
}

fn tranpoly(p0: P, p1: P, pts: &[P]) -> Vec<P> {
    let ang = (p1[1] - p0[1]).atan2(p1[0] - p0[0]) - PI / 2.0;
    let scl = distance(p0, p1);
    pts.iter()
        .map(|v| {
            let v = [-v[0], v[1]];
            let d = distance(v, [0.0, 0.0]);
            let a = v[1].atan2(v[0]);
            [p0[0] + d * scl * (ang + a).cos(), p0[1] + d * scl * (ang + a).sin()]
        })
        .collect()
}

fn flipped(pts: &[P], fli: bool) -> Vec<P> {
    pts.iter().map(|v| if fli { [-v[0], v[1]] } else { *v }).collect()
}

fn hat01(c: &mut Canv, p0: P, p1: P, fli: bool) {
    let seed = rand();
    let shape = [[-0.3, 0.5], [0.3, 0.8], [0.2, 1.0], [0.0, 1.1], [-0.3, 1.15], [-0.55, 1.0], [-0.65, 0.5]];
    fill(c, tranpoly(p0, p1, &flipped(&shape, fli)), ink(0.8));
    let q: Vec<P> = (0..10)
        .map(|i| [-0.3 - noise(i as f64 * 0.2, seed, 0.0) * i as f64 * 0.1, 0.5 - i as f64 * 0.3])
        .collect();
    line(c, tranpoly(p0, p1, &flipped(&q, fli)), ink(0.8), 1.0);
}

fn hat02(c: &mut Canv, p0: P, p1: P, fli: bool) {
    let _seed = rand();
    let shape = [
        [-0.3, 0.5],
        [-1.1, 0.5],
        [-1.2, 0.6],
        [-1.1, 0.7],
        [-0.3, 0.8],
        [0.3, 0.8],
        [1.0, 0.7],
        [1.3, 0.6],
        [1.2, 0.5],
        [0.3, 0.5],
    ];
    fill(c, tranpoly(p0, p1, &flipped(&shape, fli)), ink(0.8));
}

fn stick01(c: &mut Canv, p0: P, p1: P, fli: bool) {
    let seed = rand();
    let l = 12;
    let q: Vec<P> = (0..l)
        .map(|i| {
            let fi = i as f64;
            [-noise(fi * 0.1, seed, 0.0) * 0.1 * (fi / l as f64 * PI).sin() * 5.0, fi * 0.3]
        })
        .collect();
    line(c, tranpoly(p0, p1, &flipped(&q, fli)), ink(0.5), 1.0);
}

#[derive(Clone, Copy)]
pub enum Hat {
    H01,
    H02,
}

pub struct ManArgs {
    pub sca: f64,
    pub hat: Hat,
    pub stick: bool,
    pub fli: bool,
    pub len: [f64; 9],
}

impl Default for ManArgs {
    fn default() -> Self {
        ManArgs {
            sca: 0.5,
            hat: Hat::H01,
            stick: false,
            fli: true,
            len: [0.0, 30.0, 20.0, 30.0, 30.0, 30.0, 30.0, 30.0, 30.0],
        }
    }
}

//      2
//    1/
// 7/  | \_ 6
// 8| 0 \ 5
//      /3
//     4
const PATHS: [&[usize]; 9] = [
    &[0],
    &[0, 1],
    &[0, 1, 2],
    &[0, 3],
    &[0, 3, 4],
    &[0, 1, 5],
    &[0, 1, 5, 6],
    &[0, 1, 7],
    &[0, 1, 7, 8],
];

pub fn man(c: &mut Canv, xoff: f64, yoff: f64, a: ManArgs) {
    let sca = a.sca;
    let ang = [
        0.0,
        -PI / 2.0,
        norm_rand(0.0, 0.0),
        PI / 4.0 * rand(),
        PI * 3.0 / 4.0 * rand(),
        PI * 3.0 / 4.0,
        -PI / 4.0,
        -PI * 3.0 / 4.0 - PI / 4.0 * rand(),
        -PI / 4.0,
    ];
    let len = a.len.map(|v| v * sca);

    let grot = |ind: usize| PATHS[ind].iter().map(|&p| ang[p]).sum::<f64>();
    let gpos = |ind: usize| {
        PATHS[ind].iter().fold([0.0, 0.0], |acc, &p| {
            let r = grot(p);
            [acc[0] + len[p] * r.cos(), acc[1] + len[p] * r.sin()]
        })
    };
    let pts: Vec<P> = (0..9).map(gpos).collect();
    let yoff = yoff - pts[4][1];
    let fli = a.fli;
    let to_global = |v: &P| [if fli { -v[0] } else { v[0] } + xoff, v[1] + yoff];
    let glob = |v: &[P]| v.iter().map(to_global).collect::<Vec<P>>();

    let cloth = |c: &mut Canv, pl: &[P], fun: &dyn Fn(f64) -> f64| {
        let t = bezmh(pl, 2.0);
        let (t1, mut t2) = expand(&t, fun);
        t2.reverse();
        fill(c, glob(&concat(&t1, &t2)), WHITE);
        stroke(c, &glob(&t1), StrokeArgs { wid: 1.0, col: ink(0.5), ..Default::default() });
        stroke(c, &glob(&t2), StrokeArgs { wid: 1.0, col: ink(0.6), ..Default::default() });
    };

    let shape = |x: f64, k: f64| (0.5 * x * PI).sin() * (x * PI).sin().powf(0.1) + (1.0 - x) * k;
    let fsleeve = |x: f64| sca * 8.0 * shape(x, 0.4);
    let fbody = |x: f64| sca * 11.0 * shape(x, 0.5);
    let fhead = |x: f64| sca * 7.0 * (0.25 - (x - 0.5).powi(2)).powf(0.3);

    if a.stick {
        stick01(c, to_global(&pts[8]), to_global(&pts[6]), fli);
    }

    cloth(c, &[pts[1], pts[7], pts[8]], &fsleeve);
    cloth(c, &[pts[1], pts[0], pts[3], pts[4]], &fbody);
    cloth(c, &[pts[1], pts[5], pts[6]], &fsleeve);
    cloth(c, &[pts[1], pts[2]], &fhead);

    let hl = bezmh(&[pts[1], pts[2]], 2.0);
    let (mut h1, mut h2) = expand(&hl, &fhead);
    let k1 = (h1.len() as f64 * 0.1).floor() as usize;
    h1.drain(0..k1);
    let k2 = (h2.len() as f64 * 0.95).floor() as usize;
    h2.drain(0..k2);
    fill(c, glob(&concat(&h1, &reversed(&h2))), ink(0.6));

    match a.hat {
        Hat::H01 => hat01(c, to_global(&pts[1]), to_global(&pts[2]), fli),
        Hat::H02 => hat02(c, to_global(&pts[1]), to_global(&pts[2]), fli),
    }
}
