//! 山水 — an endlessly scrolling Chinese landscape painting, as a screensaver.
//!
//! A Rust port of Lingdong Huang's shan-shui-inf
//! (https://github.com/LingDong-/shan-shui-inf, MIT), tinted with the active
//! Omarchy theme.

mod arch;
mod draw;
mod geom;
mod man;
mod mount;
mod noise;
mod render;
mod rng;
mod scene;
mod theme;
mod tree;

use rayon::prelude::*;
use render::{Paper, TILE_W, VIEW_H, View, pack, render_tile};
use scene::{CHUNK, MARGIN, World};
use std::collections::{HashMap, HashSet};
use std::num::NonZeroU32;
use std::process::Command;
use std::rc::Rc;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};
use theme::Palette;
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalPosition};
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::platform::wayland::WindowAttributesExtWayland;
use winit::window::{Fullscreen, Window, WindowId};

const SCREENSAVER_CLASS: &str = "org.omarchy.screensaver";
/// Tiles rendered ahead of the right screen edge.
const LOOKAHEAD: i64 = 2;
/// Input is ignored this long after start, so the key or click that launched
/// us doesn't immediately close the window.
const GRACE: Duration = Duration::from_millis(1000);
const FADE: f64 = 0.8;

const USAGE: &str = "\
shan-shui — endless shan-shui landscape screensaver

USAGE: shan-shui [OPTIONS]

  --seed N          world seed (default: time-based)
  --speed F         scroll speed in painting units per second (default 18)
  --fps F           frame rate cap (default 30)
  --zoom F          scale relative to fitting the painting's height (default 1)
  --windowed        run in a normal window instead of fullscreen
  --app-id ID       Wayland app id; as org.omarchy.screensaver it behaves as
                    the Omarchy screensaver (quits all screensaver windows on
                    input or focus loss)
  --bg HEX --fg HEX override paper and ink colors
  --paper           use the original's paper and ink instead of the theme
  --grain F         paper grain strength 0..1 (default 0.5)
  --png PATH        render one frame to PATH and exit
  --size WxH        frame size for --png (default 2880x1920)
  --at X            painting x position for --png (default 0)
";

#[derive(Clone)]
struct Args {
    seed: u64,
    speed: f64,
    fps: f64,
    zoom: f64,
    windowed: bool,
    app_id: String,
    pal: Palette,
    grain: f32,
    png: Option<String>,
    size: (u32, u32),
    at: f64,
}

fn parse_args() -> Result<Args, String> {
    let mut a = Args {
        seed: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(1),
        speed: 18.0,
        fps: 30.0,
        zoom: 1.0,
        windowed: false,
        app_id: "shan-shui".into(),
        pal: theme::load().unwrap_or(theme::PAPER),
        grain: 0.5,
        png: None,
        size: (2880, 1920),
        at: 0.0,
    };
    let mut it = std::env::args().skip(1);
    while let Some(k) = it.next() {
        let mut val = || it.next().ok_or(format!("{k} needs a value"));
        let num = |s: String| s.parse::<f64>().map_err(|_| format!("bad number: {s}"));
        match k.as_str() {
            "--seed" => {
                let s = val()?;
                a.seed = s.parse().unwrap_or_else(|_| s.bytes().fold(1469598103934665603u64, |h, b| (h ^ b as u64).wrapping_mul(1099511628211)));
            }
            "--speed" => a.speed = num(val()?)?,
            "--fps" => a.fps = num(val()?)?.max(1.0),
            "--zoom" => a.zoom = num(val()?)?.max(0.1),
            "--windowed" => a.windowed = true,
            "--app-id" => a.app_id = val()?,
            "--bg" => a.pal.bg = theme::parse_hex(&val()?).ok_or("bad --bg color")?,
            "--fg" => a.pal.fg = theme::parse_hex(&val()?).ok_or("bad --fg color")?,
            "--paper" => a.pal = theme::PAPER,
            "--grain" => a.grain = num(val()?)?.clamp(0.0, 1.0) as f32,
            "--png" => a.png = Some(val()?),
            "--size" => {
                let s = val()?;
                let (w, h) = s.split_once('x').ok_or("--size wants WxH")?;
                a.size = (w.parse().map_err(|_| "bad width")?, h.parse().map_err(|_| "bad height")?);
            }
            "--at" => a.at = num(val()?)?,
            "-h" | "--help" => {
                print!("{USAGE}");
                std::process::exit(0);
            }
            _ => return Err(format!("unknown option {k}")),
        }
    }
    Ok(a)
}

fn main() {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("shan-shui: {e}\n\n{USAGE}");
            std::process::exit(2);
        }
    };
    noise::init(args.seed);
    if let Some(path) = &args.png {
        render_png(&args, path);
        return;
    }
    let event_loop = EventLoop::new().expect("event loop");
    let mut app = App::new(args);
    if let Err(e) = event_loop.run_app(&mut app) {
        eprintln!("shan-shui: {e}");
    }
    app.stop();
}

fn scale_for(height: u32, zoom: f64) -> f64 {
    height as f64 / VIEW_H * zoom
}

fn render_png(args: &Args, path: &str) {
    let (w, h) = args.size;
    let scale = scale_for(h, args.zoom);
    let view = View { scale, height: h, pal: args.pal, grain: args.grain };
    let paper = Paper::new(args.seed, scale);
    let ox = (args.at * scale).floor() as i64;
    let tw = TILE_W as i64;
    let (t0, t1) = (ox.div_euclid(tw), (ox + w as i64 - 1).div_euclid(tw));

    let t = Instant::now();
    let mut world = World::new(args.seed, -2.0 * CHUNK);
    world.generate_until((t1 + 1) as f64 * TILE_W as f64 / scale + MARGIN);
    let prims: usize = world.items.iter().map(|i| i.prims.len()).sum();
    eprintln!("generated {} items, {} prims in {:?}", world.items.len(), prims, t.elapsed());

    let t = Instant::now();
    let tiles: HashMap<i64, Vec<u32>> =
        (t0..=t1).into_par_iter().map(|i| (i, render_tile(&world.items, i, &view, &paper))).collect();
    eprintln!("rendered {} tiles in {:?}", tiles.len(), t.elapsed());

    let mut pm = tiny_skia::Pixmap::new(w, h).expect("size");
    let px = pm.pixels_mut();
    for y in 0..h as i64 {
        for x in 0..w as i64 {
            let g = ox + x;
            let v = tiles[&g.div_euclid(tw)][(y * tw + g.rem_euclid(tw)) as usize];
            px[(y * w as i64 + x) as usize] =
                tiny_skia::PremultipliedColorU8::from_rgba((v >> 16) as u8, (v >> 8) as u8, v as u8, 255).unwrap();
        }
    }
    pm.save_png(path).expect("write png");
}

/// What the main thread wants the renderer to produce.
#[derive(Clone, Default)]
struct Request {
    epoch: u64,
    scale: f64,
    width: u32,
    height: u32,
    cursor_px: f64,
    quit: bool,
}

#[derive(Default)]
struct Shared {
    req: Mutex<Request>,
    cv: Condvar,
}

struct TileMsg {
    epoch: u64,
    idx: i64,
    data: Vec<u32>,
}

/// Background thread: grows the world and renders tiles around the view.
fn producer(shared: Arc<Shared>, tx: Sender<TileMsg>, seed: u64, pal: Palette, grain: f32) {
    let batch = rayon::current_num_threads().max(2);
    let mut world = World::new(seed, -2.0 * CHUNK);
    let mut done: HashSet<i64> = HashSet::new();
    let mut epoch = u64::MAX;
    let mut view = None;
    let mut paper = None;
    loop {
        let req = {
            let g = shared.req.lock().unwrap();
            if g.quit {
                return;
            }
            g.clone()
        };
        if req.height == 0 {
            let g = shared.req.lock().unwrap();
            let _ = shared.cv.wait_timeout(g, Duration::from_millis(100));
            continue;
        }
        if req.epoch != epoch {
            epoch = req.epoch;
            done.clear();
            view = Some(View { scale: req.scale, height: req.height, pal, grain });
            paper = Some(Paper::new(seed, req.scale));
        }
        let (view, paper) = (view.as_ref().unwrap(), paper.as_ref().unwrap());
        let tw = TILE_W as f64;
        let first = (req.cursor_px / tw).floor() as i64;
        let last = ((req.cursor_px + req.width as f64) / tw).floor() as i64 + LOOKAHEAD;
        done.retain(|&i| i >= first);
        let missing: Vec<i64> = (first..=last).filter(|i| !done.contains(i)).take(batch).collect();
        if missing.is_empty() {
            world.prune(first as f64 * tw / req.scale);
            let g = shared.req.lock().unwrap();
            if g.epoch == epoch && ((g.cursor_px / tw).floor() as i64) == first && !g.quit {
                let _ = shared.cv.wait_timeout(g, Duration::from_millis(250));
            }
            continue;
        }
        let edge = (missing[missing.len() - 1] + 1) as f64 * tw / req.scale + MARGIN;
        world.generate_until(edge);
        let tiles: Vec<(i64, Vec<u32>)> =
            missing.par_iter().map(|&i| (i, render_tile(&world.items, i, view, paper))).collect();
        for (idx, data) in tiles {
            done.insert(idx);
            if tx.send(TileMsg { epoch, idx, data }).is_err() {
                return;
            }
        }
    }
}

struct Tile {
    data: Vec<u32>,
    born: Instant,
}

struct App {
    args: Args,
    window: Option<Rc<Window>>,
    surface: Option<softbuffer::Surface<Rc<Window>, Rc<Window>>>,
    shared: Arc<Shared>,
    rx: Receiver<TileMsg>,
    thread: Option<std::thread::JoinHandle<()>>,
    tiles: HashMap<i64, Tile>,
    epoch: u64,
    size: (u32, u32),
    scale: f64,
    cursor_px: f64,
    scrolling: bool,
    started: Instant,
    last: Instant,
    next_frame: Instant,
    pointer: Option<PhysicalPosition<f64>>,
}

impl App {
    fn new(args: Args) -> Self {
        let shared = Arc::new(Shared::default());
        let (tx, rx) = channel();
        let thread = {
            let (shared, seed, pal, grain) = (shared.clone(), args.seed, args.pal, args.grain);
            std::thread::spawn(move || producer(shared, tx, seed, pal, grain))
        };
        let now = Instant::now();
        App {
            args,
            window: None,
            surface: None,
            shared,
            rx,
            thread: Some(thread),
            tiles: HashMap::new(),
            epoch: 0,
            size: (0, 0),
            scale: 1.0,
            cursor_px: 0.0,
            scrolling: false,
            started: now,
            last: now,
            next_frame: now,
            pointer: None,
        }
    }

    fn screensaver(&self) -> bool {
        self.args.app_id == SCREENSAVER_CLASS
    }

    fn publish(&self) {
        let mut g = self.shared.req.lock().unwrap();
        g.epoch = self.epoch;
        g.scale = self.scale;
        g.width = self.size.0;
        g.height = self.size.1;
        g.cursor_px = self.cursor_px;
        self.shared.cv.notify_all();
    }

    fn stop(&mut self) {
        self.shared.req.lock().unwrap().quit = true;
        self.shared.cv.notify_all();
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }

    fn quit(&mut self, el: &ActiveEventLoop) {
        if self.screensaver() {
            // close the screensaver on every monitor, like omarchy-screensaver does
            let _ = Command::new("pkill").args(["-f", "[o]rg.omarchy.screensaver"]).spawn();
        }
        el.exit();
    }

    fn resize(&mut self, w: u32, h: u32) {
        if (w, h) == self.size || w == 0 || h == 0 {
            return;
        }
        if let Some(s) = self.surface.as_mut() {
            let _ = s.resize(NonZeroU32::new(w).unwrap(), NonZeroU32::new(h).unwrap());
        }
        // keep the same painting x at the left edge
        let new_scale = scale_for(h, self.args.zoom);
        self.cursor_px = self.cursor_px / self.scale * new_scale;
        self.scale = new_scale;
        self.size = (w, h);
        self.epoch += 1;
        self.tiles.clear();
        self.scrolling = false;
        self.publish();
    }

    fn redraw(&mut self) {
        let now = Instant::now();
        while let Ok(m) = self.rx.try_recv() {
            if m.epoch == self.epoch {
                self.tiles.insert(m.idx, Tile { data: m.data, born: now });
            }
        }
        let (w, h) = self.size;
        if w == 0 || h == 0 || self.surface.is_none() {
            return;
        }
        let dt = (now - self.last).as_secs_f64().min(0.25);
        self.last = now;

        let tw = TILE_W as i64;
        let mut ox = self.cursor_px.floor() as i64;
        let (t0, t1) = (ox.div_euclid(tw), (ox + w as i64 - 1).div_euclid(tw));
        if !self.scrolling && (t0..=t1).all(|t| self.tiles.contains_key(&t)) {
            self.scrolling = true;
        }
        if self.scrolling {
            let before = ox.div_euclid(tw);
            self.cursor_px += self.args.speed * self.scale * dt;
            ox = self.cursor_px.floor() as i64;
            let after = ox.div_euclid(tw);
            if after != before {
                self.tiles.retain(|&t, _| t >= after);
                self.publish();
            }
        }

        let (Some(window), Some(surface)) = (self.window.as_ref(), self.surface.as_mut()) else { return };
        let Ok(mut buf) = surface.buffer_mut() else { return };
        let bg = pack(self.args.pal.bg);
        let (wu, hu) = (w as usize, h as usize);
        let (t0, t1) = (ox.div_euclid(tw), (ox + w as i64 - 1).div_euclid(tw));
        for t in t0..=t1 {
            let sx = t * tw - ox;
            let x0 = sx.max(0) as usize;
            let x1 = (sx + tw).min(w as i64) as usize;
            let off = (x0 as i64 - sx) as usize;
            let tile = self.tiles.get(&t).filter(|tl| tl.data.len() == TILE_W as usize * hu);
            match tile {
                None => {
                    for y in 0..hu {
                        buf[y * wu + x0..y * wu + x1].fill(bg);
                    }
                }
                Some(tl) => {
                    let f = ((now - tl.born).as_secs_f64() / FADE).min(1.0);
                    for y in 0..hu {
                        let src = &tl.data[y * TILE_W as usize + off..y * TILE_W as usize + off + (x1 - x0)];
                        let dst = &mut buf[y * wu + x0..y * wu + x1];
                        if f >= 1.0 {
                            dst.copy_from_slice(src);
                        } else {
                            let k = (f * f * (3.0 - 2.0 * f)) as f32;
                            for (d, &s) in dst.iter_mut().zip(src) {
                                *d = mix(bg, s, k);
                            }
                        }
                    }
                }
            }
        }
        window.pre_present_notify();
        let _ = buf.present();
    }
}

fn mix(a: u32, b: u32, k: f32) -> u32 {
    let ch = |s: u32| {
        let (x, y) = (((a >> s) & 255) as f32, ((b >> s) & 255) as f32);
        ((x + (y - x) * k) as u32) << s
    };
    ch(16) | ch(8) | ch(0)
}

/// Is a screensaver window still the active window? Focus may legitimately
/// move between screensaver windows on different monitors.
fn screensaver_focused() -> bool {
    let Ok(out) = Command::new("hyprctl").args(["activewindow", "-j"]).output() else { return true };
    let s = String::from_utf8_lossy(&out.stdout);
    s.contains(&format!("\"class\": \"{SCREENSAVER_CLASS}\""))
}

impl ApplicationHandler for App {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let mut attrs = Window::default_attributes()
            .with_title("Shan Shui")
            .with_name(self.args.app_id.clone(), self.args.app_id.clone());
        attrs = if self.args.windowed {
            attrs.with_inner_size(LogicalSize::new(1400.0, 800.0))
        } else {
            attrs.with_fullscreen(Some(Fullscreen::Borderless(None)))
        };
        let window = match el.create_window(attrs) {
            Ok(w) => Rc::new(w),
            Err(e) => {
                eprintln!("shan-shui: cannot create window: {e}");
                el.exit();
                return;
            }
        };
        if !self.args.windowed {
            window.set_cursor_visible(false);
        }
        let ctx = softbuffer::Context::new(window.clone()).expect("softbuffer context");
        self.surface = Some(softbuffer::Surface::new(&ctx, window.clone()).expect("softbuffer surface"));
        let size = window.inner_size();
        self.window = Some(window);
        self.started = Instant::now();
        self.resize(size.width, size.height);
    }

    fn window_event(&mut self, el: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        let armed = self.started.elapsed() > GRACE && !self.args.windowed;
        match event {
            WindowEvent::CloseRequested => self.quit(el),
            WindowEvent::Resized(s) => self.resize(s.width, s.height),
            WindowEvent::RedrawRequested => self.redraw(),
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed && (armed || self.args.windowed) {
                    self.quit(el);
                }
            }
            WindowEvent::MouseInput { state: ElementState::Pressed, .. } | WindowEvent::MouseWheel { .. }
                if armed =>
            {
                self.quit(el)
            }
            WindowEvent::CursorMoved { position, .. } => {
                // the compositor reports where the pointer sits when we map;
                // only real movement after the grace period counts
                match self.pointer {
                    Some(p) if armed && (p.x - position.x).hypot(p.y - position.y) > 10.0 => self.quit(el),
                    Some(_) if armed => {}
                    _ => self.pointer = Some(position),
                }
            }
            WindowEvent::Focused(false) if self.screensaver() => {
                std::thread::spawn(|| {
                    std::thread::sleep(Duration::from_millis(400));
                    if !screensaver_focused() {
                        let _ = Command::new("pkill").args(["-f", "[o]rg.omarchy.screensaver"]).status();
                        std::process::exit(0);
                    }
                });
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, el: &ActiveEventLoop) {
        let now = Instant::now();
        if now >= self.next_frame {
            if let Some(w) = &self.window {
                w.request_redraw();
            }
            let dt = Duration::from_secs_f64(1.0 / self.args.fps);
            self.next_frame = (self.next_frame + dt).max(now);
        }
        el.set_control_flow(ControlFlow::WaitUntil(self.next_frame));
    }
}
