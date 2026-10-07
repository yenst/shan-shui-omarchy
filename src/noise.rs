//! Perlin noise, ported from p5.js (as used by the original).

use crate::rng::Rng;
use std::sync::OnceLock;

const YWRAPB: i64 = 4;
const YWRAP: i64 = 1 << YWRAPB;
const ZWRAPB: i64 = 8;
const ZWRAP: i64 = 1 << ZWRAPB;
const SIZE: i64 = 4095;
const OCTAVES: usize = 4;
const AMP_FALLOFF: f64 = 0.5;

static PERLIN: OnceLock<Vec<f64>> = OnceLock::new();

pub fn init(seed: u64) {
    let mut r = Rng(seed ^ 0xA076_1D64_78BD_642F);
    let table = (0..=SIZE).map(|_| r.f64()).collect();
    let _ = PERLIN.set(table);
}

fn scaled_cosine(i: f64) -> f64 {
    0.5 * (1.0 - (i * std::f64::consts::PI).cos())
}

pub fn noise(x: f64, y: f64, z: f64) -> f64 {
    let p = PERLIN.get().expect("noise::init not called");
    let at = |i: i64| p[(i & SIZE) as usize];

    let (x, y, z) = (x.abs(), y.abs(), z.abs());
    let (mut xi, mut yi, mut zi) = (x.floor() as i64, y.floor() as i64, z.floor() as i64);
    let (mut xf, mut yf, mut zf) = (x - xi as f64, y - yi as f64, z - zi as f64);

    let mut r = 0.0;
    let mut ampl = 0.5;
    for _ in 0..OCTAVES {
        let mut of = xi + (yi << YWRAPB) + (zi << ZWRAPB);
        let rxf = scaled_cosine(xf);
        let ryf = scaled_cosine(yf);

        let mut n1 = at(of);
        n1 += rxf * (at(of + 1) - n1);
        let mut n2 = at(of + YWRAP);
        n2 += rxf * (at(of + YWRAP + 1) - n2);
        n1 += ryf * (n2 - n1);

        of += ZWRAP;
        n2 = at(of);
        n2 += rxf * (at(of + 1) - n2);
        let mut n3 = at(of + YWRAP);
        n3 += rxf * (at(of + YWRAP + 1) - n3);
        n2 += ryf * (n3 - n2);

        n1 += scaled_cosine(zf) * (n2 - n1);

        r += n1 * ampl;
        ampl *= AMP_FALLOFF;
        xi <<= 1;
        xf *= 2.0;
        yi <<= 1;
        yf *= 2.0;
        zi <<= 1;
        zf *= 2.0;
        if xf >= 1.0 {
            xi += 1;
            xf -= 1.0;
        }
        if yf >= 1.0 {
            yi += 1;
            yf -= 1.0;
        }
        if zf >= 1.0 {
            zi += 1;
            zf -= 1.0;
        }
    }
    r
}
