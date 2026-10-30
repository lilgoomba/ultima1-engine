//! `u1`: play Ultima I (overworld, towns and castles) with your own game
//! files.
//!
//!   u1 --game-dir <path> [--player N | --save FILE | --pos X,Y] [--mode ega|t1k|cga]
//!      [--scale N] [--save-to FILE]
//!   u1 --game-dir <path> ... --screenshot out.png [--keys TEXT] [--counter N]
//!
//! Game files are only read. Quit (Q, outdoors) writes the 820-byte player
//! record to `--save-to` (default `./PLAYER1.U1` in the current directory,
//! which must not be the game directory). Tables, names and messages come
//! from the user's OUT.EXE at start-up.

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, Instant};

use u1_assets::exe::Exe;
use u1_assets::game::{Assets, Game};
use u1_assets::gamedir::{GameDir, VideoMode};
use u1_assets::io::{keys, Halt, Io, IoResult, ScriptIo};
use u1_assets::locations::LocationTable;
use u1_assets::palette::{cga_as_16, Rgb, CGA_PALETTE_1_LOW, EGA_PALETTE};
use u1_assets::rules::ExeTables;
use u1_assets::save::PlayerRecord;
use u1_assets::tcd::TownCastleMaps;
use u1_assets::term::{Term, H, W};

/// One keyboard/animation poll of the original main loop, measured in
/// DOSBox (130 ms at 20000 cycles).
const POLL: Duration = Duration::from_micros(130_000);
/// One tick of the PC timer.
const TICK: Duration = Duration::from_micros(54_925);
/// Start of a new character (verified from GEN.EXE).
const DEFAULT_POS: (u16, u16) = (49, 40);

const USAGE: &str = "usage: u1 --game-dir <path> [--player N | --save FILE | --pos X,Y]
          [--mode ega|t1k|cga] [--scale N] [--save-to FILE]
          [--screenshot out.png [--keys TEXT] [--counter N]]

Keys are the original's (arrows move; A B C D E F G H I K N O Q R S T U V X Z;
Space passes). --keys is typed text; {up} {down} {left} {right} {enter}
{esc} {bs} {space} name special keys and ~ ends a timed wait.";

struct Args {
    game_dir: PathBuf,
    player: Option<u8>,
    save: Option<PathBuf>,
    pos: Option<(u16, u16)>,
    mode: Option<VideoMode>,
    scale: u32,
    seed: Option<u16>,
    times: Vec<u16>,
    save_to: Option<PathBuf>,
    screenshot: Option<PathBuf>,
    keys: String,
    counter: u32,
}

fn parse_args() -> Result<Args, String> {
    let mut a = Args {
        game_dir: PathBuf::new(),
        player: None,
        save: None,
        pos: None,
        mode: None,
        scale: 3,
        seed: None,
        times: Vec::new(),
        save_to: None,
        screenshot: None,
        keys: String::new(),
        counter: 0,
    };
    let mut have_dir = false;
    let mut it = std::env::args().skip(1);
    while let Some(k) = it.next() {
        let mut v = || it.next().ok_or(format!("{k} needs a value"));
        match k.as_str() {
            "--game-dir" => {
                a.game_dir = v()?.into();
                have_dir = true;
            }
            "--player" => a.player = Some(v()?.parse().map_err(|_| "bad --player")?),
            "--save" => a.save = Some(v()?.into()),
            "--pos" => {
                let s = v()?;
                let (x, y) = s.split_once(',').ok_or("--pos wants X,Y")?;
                a.pos = Some((x.trim().parse().map_err(|_| "bad X")?, y.trim().parse().map_err(|_| "bad Y")?));
            }
            "--mode" => a.mode = Some(VideoMode::parse(&v()?).ok_or("bad --mode")?),
            "--scale" => a.scale = v()?.parse().map_err(|_| "bad --scale")?,
            // Hidden: a fixed RNG seed and clock value for deterministic runs.
            "--seed" => a.seed = Some(v()?.parse().map_err(|_| "bad --seed")?),
            // Hidden: clock values for successive time seeds (with --seed).
            "--times" => {
                for t in v()?.split(',') {
                    a.times.push(t.trim().parse().map_err(|_| "bad --times")?);
                }
            }
            "--save-to" => a.save_to = Some(v()?.into()),
            "--screenshot" => a.screenshot = Some(v()?.into()),
            "--keys" => a.keys = v()?,
            "--counter" => a.counter = v()?.parse().map_err(|_| "bad --counter")?,
            "-h" | "--help" => return Err(USAGE.into()),
            other => return Err(format!("unknown argument {other}\n{USAGE}")),
        }
    }
    if !have_dir {
        return Err(USAGE.into());
    }
    a.scale = a.scale.clamp(1, 6);
    Ok(a)
}

fn same_dir(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(x), Ok(y)) => x == y,
        _ => false,
    }
}

fn load(a: &Args) -> Result<(Game, [Rgb; 16]), String> {
    let e = |e: u1_assets::error::AssetError| e.to_string();
    let g = GameDir::new(&a.game_dir).map_err(e)?;
    let mode = a.mode.or_else(|| g.detect_video_mode()).ok_or("no tile files found in the game directory")?;
    let exe_bytes = g.read("OUT.EXE").map_err(e)?;
    let assets = Assets {
        t: ExeTables::from_out_exe(&exe_bytes).map_err(e)?,
        locs: LocationTable::from_out_exe(&exe_bytes).map_err(e)?,
        exe: Exe::new(exe_bytes).map_err(e)?,
        map: g.load_world_map().map_err(e)?,
        world_tiles: g.load_world_tiles(mode).map_err(e)?,
        town_tiles: g.load_town_tiles(mode).map_err(e)?,
        tcd: TownCastleMaps::parse(&g.read("TCD.BIN").map_err(e)?).map_err(e)?,
    };
    let palette = if mode == VideoMode::Cga { cga_as_16(&CGA_PALETTE_1_LOW) } else { EGA_PALETTE };
    let save_bytes = match (&a.save, a.player) {
        (Some(p), _) => Some(std::fs::read(p).map_err(|e| format!("{}: {e}", p.display()))?),
        (None, Some(n)) => Some(g.read(&format!("PLAYER{n}.U1")).map_err(e)?),
        _ => None,
    };
    let mut rec = match save_bytes {
        Some(b) => PlayerRecord::parse(&b).map_err(e)?,
        None => Game::default_record("Hero", DEFAULT_POS.0, DEFAULT_POS.1),
    };
    if let Some((x, y)) = a.pos {
        rec.set_u16(u1_assets::save::off::POS_X, x);
        rec.set_u16(u1_assets::save::off::POS_Y, y);
        let mut o = rec.objects()[0];
        o.x = x;
        o.y = y;
        rec.set_object(0, &o);
    }
    let mut game = match a.seed {
        Some(seed) => {
            let mut g = Game::new(assets, rec, seed);
            g.fixed_times = a.times.iter().copied().collect();
            g
        }
        None => {
            // As the original: the generator starts at the save's seed word
            // plus the time-of-day value modulo 999, and later time seeds
            // read the clock again.
            let ts = u1_assets::dgame::time_seed_from(local_time());
            let seed = rec.u16_at(0x32).wrapping_add(ts % 999);
            let mut g = Game::new(assets, rec, seed);
            g.clock = Some(local_time);
            g
        }
    };
    game.counter = a.counter;
    let save_to = a.save_to.clone().unwrap_or_else(|| PathBuf::from("PLAYER1.U1"));
    let dir = save_to.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    if same_dir(dir, &a.game_dir) {
        return Err("--save-to must not point into the game directory (originals are never overwritten)".into());
    }
    game.save_to = Some(save_to);
    Ok((game, palette))
}

fn write_png(path: &Path, w: u32, h: u32, rgb: &[u8]) -> Result<(), String> {
    let f = std::fs::File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(f), w, h);
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    let mut wr = enc.write_header().map_err(|e| e.to_string())?;
    wr.write_image_data(rgb).map_err(|e| e.to_string())
}

fn screenshot(mut g: Game, palette: &[Rgb; 16], a: &Args, out: &Path) -> Result<(), String> {
    let mut io = ScriptIo::from_text(&a.keys);
    let _ = g.run(&mut io);
    let scale = 2; // the size of a 640x400 DOSBox capture
    write_png(out, (W * scale) as u32, (H * scale) as u32, &g.term.to_rgb(palette, scale))?;
    println!("{}: mode={} loc={} at ({},{}) town ({},{})", out.display(), g.mode, g.loc, g.ow.x, g.ow.y, g.px, g.py);
    for l in g.term.messages() {
        println!("| {l}");
    }
    if let Some(s) = g.stopped {
        println!("stopped: {s}");
    }
    Ok(())
}

/// The window: every poll shows the screen and waits out the rest of the
/// original's 130 ms.
struct SdlIo {
    canvas: sdl2::render::WindowCanvas,
    tex: sdl2::render::Texture<'static>,
    events: sdl2::EventPump,
    palette: [Rgb; 16],
    next: Instant,
    pending: std::collections::VecDeque<u8>,
}

impl SdlIo {
    fn present(&mut self, t: &Term) -> IoResult<()> {
        let rgb = t.to_rgb(&self.palette, 1);
        self.tex.update(None, &rgb, W * 3).map_err(|_| Halt)?;
        self.canvas.copy(&self.tex, None, None).map_err(|_| Halt)?;
        self.canvas.present();
        Ok(())
    }

    fn pump(&mut self) -> IoResult<()> {
        use sdl2::event::Event;
        use sdl2::keyboard::{Keycode, Mod};
        for ev in self.events.poll_iter() {
            match ev {
                Event::Quit { .. } => return Err(Halt),
                Event::KeyDown { keycode: Some(k), keymod, .. } => {
                    let shift = keymod.intersects(Mod::LSHIFTMOD | Mod::RSHIFTMOD | Mod::CAPSMOD);
                    let c = match k {
                        Keycode::Left | Keycode::Kp4 => Some(keys::LEFT),
                        Keycode::Right | Keycode::Kp6 => Some(keys::RIGHT),
                        Keycode::Up | Keycode::Kp8 => Some(keys::UP),
                        Keycode::Down | Keycode::Kp2 => Some(keys::DOWN),
                        Keycode::Return | Keycode::KpEnter => Some(keys::ENTER),
                        Keycode::Escape => Some(keys::ESC),
                        Keycode::Backspace => Some(keys::BACKSPACE),
                        _ => {
                            let n = k.into_i32();
                            if (0x20..0x7F).contains(&n) {
                                let c = n as u8;
                                Some(if shift { c.to_ascii_uppercase() } else { c })
                            } else {
                                None
                            }
                        }
                    };
                    if let Some(c) = c {
                        self.pending.push_back(c);
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
}

impl Io for SdlIo {
    fn poll(&mut self, t: &Term) -> IoResult<Option<u8>> {
        self.present(t)?;
        loop {
            self.pump()?;
            if let Some(k) = self.pending.pop_front() {
                self.next = Instant::now() + POLL;
                return Ok(Some(k));
            }
            let now = Instant::now();
            if now >= self.next {
                self.next = (self.next + POLL).max(now);
                return Ok(None);
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    fn pause(&mut self, t: &Term, ticks: u32) -> IoResult<()> {
        self.present(t)?;
        let end = Instant::now() + TICK * ticks;
        while Instant::now() < end {
            self.pump()?;
            std::thread::sleep(Duration::from_millis(5));
        }
        Ok(())
    }

    fn wait_or_key(&mut self, t: &Term) -> IoResult<()> {
        // About 1.5 s, or until a key (which is swallowed).
        for _ in 0..12 {
            if self.poll(t)?.is_some() {
                break;
            }
        }
        Ok(())
    }
}

fn play(mut g: Game, palette: [Rgb; 16], a: &Args) -> Result<(), String> {
    use sdl2::pixels::PixelFormatEnum;
    let sdl = sdl2::init()?;
    let video = sdl.video()?;
    let window = video
        .window("u1 - Ultima I", W as u32 * a.scale, H as u32 * a.scale)
        .position_centered()
        .build()
        .map_err(|e| e.to_string())?;
    let canvas = window.into_canvas().build().map_err(|e| e.to_string())?;
    // The texture creator lives for the whole program.
    let tc: &'static _ = Box::leak(Box::new(canvas.texture_creator()));
    let tex = tc.create_texture_streaming(PixelFormatEnum::RGB24, W as u32, H as u32).map_err(|e| e.to_string())?;
    let events = sdl.event_pump()?;
    let mut io = SdlIo { canvas, tex, events, palette, next: Instant::now(), pending: Default::default() };
    let _ = g.run(&mut io);
    if let Some(s) = g.stopped {
        eprintln!("stopped: {s}");
    }
    Ok(())
}

fn main() -> ExitCode {
    let res = parse_args().and_then(|a| {
        let (g, pal) = load(&a)?;
        match &a.screenshot {
            Some(out) => screenshot(g, &pal, &a, out),
            None => play(g, pal, &a),
        }
    });
    match res {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

/// Local wall-clock time as DOS reports it: hour, minute, second, hundredths.
fn local_time() -> [u8; 4] {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let t = now.as_secs() as libc::time_t;
    // SAFETY: localtime_r writes only into the zeroed `tm` we pass.
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    let ok = unsafe { !libc::localtime_r(&t, &mut tm).is_null() };
    let (h, m, s) = if ok {
        (tm.tm_hour as u8, tm.tm_min as u8, tm.tm_sec as u8)
    } else {
        let d = now.as_secs() % 86400;
        ((d / 3600) as u8, (d / 60 % 60) as u8, (d % 60) as u8)
    };
    [h, m, s, (now.subsec_millis() / 10) as u8]
}
