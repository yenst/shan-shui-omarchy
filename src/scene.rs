//! The infinite scroll: the original's mountplanner/chunkloader, producing
//! render-ready items sorted back to front.

use crate::arch;
use crate::draw::*;
use crate::mount;
use crate::noise::noise;
use crate::rng::{self, choice, rand};
use rayon::prelude::*;
use std::collections::HashMap;
use std::f64::consts::PI;
use std::sync::Arc;

/// World units per planner chunk (`MEM.cwid`).
pub const CHUNK: f64 = 512.0;
/// How far left of its planner chunk an item may reach. A tile is final once
/// the world is generated this far past its right edge.
pub const MARGIN: f64 = 2000.0;

fn water(c: &mut Canv, xoff: f64, yoff: f64) {
    let (hei, len, clu) = (2.0, 800.0, 10);
    let mut rows: Vec<Vec<[f64; 2]>> = Vec::new();
    let mut yk = 0.0;
    for _ in 0..clu {
        let xk = (rand() - 0.5) * (len / 8.0);
        yk += rand() * 5.0;
        let lk = len / 4.0 + rand() * (len / 4.0);
        let mut row = Vec::new();
        let mut j = -lk;
        while j < lk {
            row.push([j + xk, (j * 0.2).sin() * hei * noise(j * 0.1, 0.0, 0.0) - 20.0 + yk]);
            j += 5.0;
        }
        rows.push(row);
    }
    for row in rows.iter().skip(1) {
        stroke(
            c,
            &crate::geom::offset(row, xoff, yoff),
            StrokeArgs { col: ink(round3(0.3 + rand() * 0.3)), wid: 1.0, ..Default::default() },
        );
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Tag {
    Mount,
    DistMount,
    FlatMount,
    Boat,
}

struct Plan {
    tag: Tag,
    x: f64,
    y: f64,
}

struct PlanEntry {
    tag: Tag,
    x: f64,
    y: f64,
    idx: usize,
    seed: u64,
}

/// A primitive in item-local coordinates (x relative to `Item::base_x`).
pub struct RPrim {
    pub pts: Vec<[f32; 2]>,
    pub fill: Option<Col>,
    pub stroke: Option<(Col, f32)>,
    /// xmin, ymin, xmax, ymax (local), including stroke width
    pub bb: [f32; 4],
}

pub struct Item {
    pub y: f64,
    pub seq: u64,
    pub base_x: f64,
    /// absolute xmin, xmax
    pub xr: [f64; 2],
    pub prims: Vec<RPrim>,
}

fn finalize(canv: Canv, y: f64, base_x: f64) -> Item {
    let mut xr = [f64::INFINITY, f64::NEG_INFINITY];
    let prims = canv
        .into_iter()
        .map(|p| {
            let pad = p.stroke.map_or(0.0, |s| s.1 as f32) + 1.0;
            let mut bb = [f32::INFINITY, f32::INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY];
            let pts: Vec<[f32; 2]> = p
                .pts
                .iter()
                .map(|q| {
                    let v = [(q[0] - base_x) as f32, q[1] as f32];
                    bb = [bb[0].min(v[0]), bb[1].min(v[1]), bb[2].max(v[0]), bb[3].max(v[1])];
                    v
                })
                .collect();
            let bb = [bb[0] - pad, bb[1] - pad, bb[2] + pad, bb[3] + pad];
            xr = [xr[0].min(base_x + bb[0] as f64), xr[1].max(base_x + bb[2] as f64)];
            RPrim { pts, fill: p.fill, stroke: p.stroke.map(|(c, w)| (c, w as f32)), bb }
        })
        .collect();
    Item { y, seq: 0, base_x, xr, prims }
}

fn generate(e: &PlanEntry) -> Vec<Item> {
    rng::seed(e.seed);
    let mut c = Canv::new();
    match e.tag {
        Tag::Mount => {
            let seed = e.idx as f64 * 2.0 * rand();
            let hei = 100.0 + rand() * 400.0;
            let wid = 400.0 + rand() * 200.0;
            mount::mountain(&mut c, e.x, e.y, seed, hei, wid, 200, true);
            let mut w = Canv::new();
            water(&mut w, e.x, e.y);
            return vec![finalize(c, e.y, e.x), finalize(w, e.y - 10000.0, e.x)];
        }
        Tag::FlatMount => {
            let seed = 2.0 * rand() * PI;
            let wid = 600.0 + rand() * 400.0;
            let cho = 0.5 + rand() * 0.2;
            mount::flat_mount(&mut c, e.x, e.y, seed, 100.0, wid, 80, cho);
        }
        Tag::DistMount => {
            let seed = rand() * 100.0;
            mount::dist_mount(&mut c, e.x, e.y, seed, 150.0, choice(&[500.0, 1000.0, 1500.0]), 5);
        }
        Tag::Boat => {
            arch::boat01(&mut c, e.x, e.y, e.y / 800.0, choice(&[true, false]));
        }
    }
    vec![finalize(c, e.y, e.x)]
}

pub struct World {
    /// The world is planned and generated up to here (world units).
    pub xmax: f64,
    planmtx: HashMap<i64, i32>,
    rng: u64,
    seq: u64,
    pub items: Vec<Arc<Item>>,
}

impl World {
    pub fn new(seed: u64, start: f64) -> Self {
        World { xmax: start, planmtx: HashMap::new(), rng: seed, seq: 0, items: Vec::new() }
    }

    pub fn generate_until(&mut self, x: f64) {
        while self.xmax < x {
            self.chunk();
        }
    }

    /// Drop items entirely left of `x`; the view only scrolls right.
    pub fn prune(&mut self, x: f64) {
        self.items.retain(|it| it.xr[1] >= x);
        self.planmtx.retain(|&k, _| k as f64 * 5.0 >= x - MARGIN);
    }

    fn chunk(&mut self) {
        let (a, b) = (self.xmax, self.xmax + CHUNK);
        rng::seed(self.rng);
        let plan = self.mountplanner(a, b);
        let entries: Vec<PlanEntry> = plan
            .iter()
            .enumerate()
            .map(|(idx, p)| PlanEntry { tag: p.tag, x: p.x, y: p.y, idx, seed: rng::next_u64() })
            .collect();
        self.rng = rng::state();
        self.xmax = b;

        let made: Vec<Vec<Item>> = entries.par_iter().map(generate).collect();
        for (e, items) in entries.iter().zip(made) {
            for mut it in items {
                if it.prims.is_empty() {
                    continue;
                }
                if it.xr[0] < a - MARGIN {
                    eprintln!("shan-shui: {:?} at x={:.0} reaches {:.0} past the margin", e.tag, e.x, a - MARGIN - it.xr[0]);
                }
                it.seq = self.seq;
                self.seq += 1;
                self.items.push(Arc::new(it));
            }
        }
    }

    fn mountplanner(&mut self, xmin: f64, xmax: f64) -> Vec<Plan> {
        let mut reg: Vec<Plan> = Vec::new();
        let samp = 0.03;
        let ns = |x: f64| (noise(x * samp, 0.0, 0.0) - 0.55).max(0.0) * 2.0;
        let yr = |x: f64| noise(x * 0.01, PI, 0.0);
        let locmax = |x: f64| {
            let z0 = ns(x);
            if z0 <= 0.3 {
                return false;
            }
            let mut i = x - 2.0;
            while i < x + 2.0 {
                if ns(i) > z0 {
                    return false;
                }
                i += 1.0;
            }
            true
        };
        let chadd = |reg: &mut Vec<Plan>, r: Plan, mind: f64| {
            if reg.iter().any(|k| (k.x - r.x).abs() < mind) {
                return false;
            }
            reg.push(r);
            true
        };
        let xstep = 5.0;
        let mwid = 200.0;
        let cell = |x: f64| (x / xstep).floor() as i64;

        let mut i = xmin;
        while i < xmax {
            self.planmtx.entry(cell(i)).or_insert(0);
            i += xstep;
        }

        let mut i = xmin;
        while i < xmax {
            let mut j = 0.0;
            while j < yr(i) * 480.0 {
                if locmax(i) {
                    let xof = i + 2.0 * (rand() - 0.5) * 500.0;
                    let yof = j + 300.0;
                    if chadd(&mut reg, Plan { tag: Tag::Mount, x: xof, y: yof }, 10.0) {
                        let mut k = cell(xof - mwid);
                        while (k as f64) < (xof + mwid) / xstep {
                            // cells outside the planned range become NaN in JS and reset later
                            if let Some(v) = self.planmtx.get_mut(&k) {
                                *v += 1;
                            }
                            k += 1;
                        }
                    }
                }
                j += 30.0;
            }
            if i.abs() % 1000.0 < (xstep - 1.0).max(1.0) {
                chadd(&mut reg, Plan { tag: Tag::DistMount, x: i, y: 280.0 - rand() * 50.0 }, 10.0);
            }
            i += xstep;
        }

        let mut i = xmin;
        while i < xmax {
            if self.planmtx.get(&cell(i)) == Some(&0) && rand() < 0.01 {
                let mut j = 0.0;
                while j < 4.0 * rand() {
                    let x = i + 2.0 * (rand() - 0.5) * 700.0;
                    chadd(&mut reg, Plan { tag: Tag::FlatMount, x, y: 700.0 - j * 50.0 }, 10.0);
                    j += 1.0;
                }
            }
            i += xstep;
        }

        let mut i = xmin;
        while i < xmax {
            if rand() < 0.2 {
                chadd(&mut reg, Plan { tag: Tag::Boat, x: i, y: 300.0 + rand() * 390.0 }, 400.0);
            }
            i += xstep;
        }
        reg
    }
}
