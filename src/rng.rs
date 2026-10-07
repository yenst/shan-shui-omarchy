//! Random numbers. The original replaces `Math.random` globally; we keep a
//! thread-local generator so generation code can call `rand()` the same way,
//! and reseed it per scene item so items can be generated in parallel.

use std::cell::Cell;

#[derive(Clone, Copy)]
pub struct Rng(pub u64);

impl Rng {
    pub fn next_u64(&mut self) -> u64 {
        // splitmix64
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    pub fn f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
}

thread_local! {
    static STATE: Cell<u64> = const { Cell::new(0x5EED) };
}

pub fn seed(s: u64) {
    STATE.with(|c| c.set(s));
}

pub fn state() -> u64 {
    STATE.with(|c| c.get())
}

pub fn next_u64() -> u64 {
    STATE.with(|c| {
        let mut r = Rng(c.get());
        let v = r.next_u64();
        c.set(r.0);
        v
    })
}

/// `Math.random()`: uniform in [0, 1).
pub fn rand() -> f64 {
    (next_u64() >> 11) as f64 / (1u64 << 53) as f64
}

pub fn norm_rand(m: f64, mx: f64) -> f64 {
    m + (mx - m) * rand()
}

pub fn choice<T: Copy>(arr: &[T]) -> T {
    arr[((arr.len() as f64) * rand()) as usize]
}

/// `randChoice([-1, 1])`
pub fn sign() -> f64 {
    choice(&[-1.0, 1.0])
}

pub fn wtrand(f: impl Fn(f64) -> f64) -> f64 {
    loop {
        let x = rand();
        let y = rand();
        if y < f(x) {
            return x;
        }
    }
}

pub fn gaussian() -> f64 {
    wtrand(|x| (-24.0 * (x - 0.5).powi(2)).exp()) * 2.0 - 1.0
}
