//! `u1`: play the Ultima I overworld with your own game files.
//!
//!   u1 --game-dir <path> [--player N | --save FILE | --pos X,Y] [--mode ega|t1k|cga]
//!      [--scale N] [--seed N]
//!   u1 --game-dir <path> ... --screenshot out.png [--keys k1,k2,...] [--counter N]
//!
//! The save file is only read, never written. Tables (monster, food and
//! spawn tables, names, messages) come from the user's OUT.EXE at start-up.

mod screen;

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{Duration, Instant};

use u1_assets::gamedir::{GameDir, VideoMode};
use u1_assets::locations::{kind_of, LocationTable};
use u1_assets::map::WorldMap;
use u1_assets::overworld::Overworld;
use u1_assets::palette::{cga_as_16, Rgb, CGA_PALETTE_1_LOW, EGA_PALETTE};
use u1_assets::rules::{Dir, ExeTables};
use u1_assets::save::PlayerRecord;
use u1_assets::tiles::TileSet;

use screen::{Screen, H, W};

/// One animation/keyboard poll of the original: a 3-tick wait on the
/// 18.2 Hz timer (2..3 ticks depending on phase) plus drawing time. Measured
/// in DOSBox as 130 ms per poll at 20000 cycles (157 ms at 3000 cycles).
const POLL: Duration = Duration::from_micros(130_000);
/// Polls without a key before the game passes a turn by itself.
const IDLE_POLLS: u32 = 1500;
/// Start of a new character (verified from GEN.EXE).
const DEFAULT_POS: (i32, i32) = (49, 40);

const USAGE: &str = "usage: u1 --game-dir <path> [--player N | --save FILE | --pos X,Y]
          [--mode ega|t1k|cga] [--scale N] [--seed N]
          [--screenshot out.png [--keys k1,k2,...] [--counter N]]

Keys: arrows or keypad 8/2/4/6 move, Space passes, B boards, X exits a
vehicle, E enters (stub), Esc quits. --keys accepts up, down, left, right,
space, b, x, e.";

struct Args {
    game_dir: PathBuf,
    player: Option<u8>,
    save: Option<PathBuf>,
    pos: Option<(i32, i32)>,
    mode: Option<VideoMode>,
    scale: u32,
    seed: u16,
    screenshot: Option<PathBuf>,
    keys: Vec<String>,
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
        seed: 0x1234,
        screenshot: None,
        keys: vec![],
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
            "--seed" => a.seed = v()?.parse().map_err(|_| "bad --seed")?,
            "--screenshot" => a.screenshot = Some(v()?.into()),
            "--keys" => a.keys = v()?.split(',').map(|s| s.trim().to_ascii_lowercase()).filter(|s| !s.is_empty()).collect(),
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

struct Game {
    map: WorldMap,
    tiles: TileSet,
    palette: [Rgb; 16],
    tables: ExeTables,
    locs: LocationTable,
    state: Overworld,
}

fn load(a: &Args) -> Result<Game, String> {
    let g = GameDir::new(&a.game_dir).map_err(|e| e.to_string())?;
    let mode = a.mode.or_else(|| g.detect_video_mode()).ok_or("no tile files found in the game directory")?;
    let tiles = g.load_world_tiles(mode).map_err(|e| e.to_string())?;
    let map = g.load_world_map().map_err(|e| e.to_string())?;
    let exe = g.read("OUT.EXE").map_err(|e| e.to_string())?;
    let tables = ExeTables::from_out_exe(&exe).map_err(|e| e.to_string())?;
    let locs = LocationTable::from_out_exe(&exe).map_err(|e| e.to_string())?;
    let palette = if mode == VideoMode::Cga { cga_as_16(&CGA_PALETTE_1_LOW) } else { EGA_PALETTE };
    let save_bytes = match (&a.save, a.player) {
        (Some(p), _) => Some(std::fs::read(p).map_err(|e| format!("{}: {e}", p.display()))?),
        (None, Some(n)) => Some(g.read(&format!("PLAYER{n}.U1")).map_err(|e| e.to_string())?),
        _ => None,
    };
    let mut state = match save_bytes {
        Some(b) => Overworld::from_save(&PlayerRecord::parse(&b).map_err(|e| e.to_string())?, a.seed),
        None => {
            let (x, y) = DEFAULT_POS;
            Overworld::new_at(x, y, a.seed)
        }
    };
    if let Some((x, y)) = a.pos {
        let (x, y) = u1_assets::rules::wrap(x, y);
        state.x = x;
        state.y = y;
    }
    Ok(Game { map, tiles, palette, tables, locs, state })
}

enum Cmd {
    Move(Dir),
    Pass,
    Board,
    Exit,
    Enter,
}

fn cmd_from_name(s: &str) -> Option<Cmd> {
    Some(match s {
        "up" | "n" | "north" | "8" => Cmd::Move(Dir::North),
        "down" | "s" | "south" | "2" => Cmd::Move(Dir::South),
        "left" | "w" | "west" | "4" => Cmd::Move(Dir::West),
        "right" | "east" | "6" => Cmd::Move(Dir::East),
        "space" | "pass" => Cmd::Pass,
        "b" | "board" => Cmd::Board,
        "x" | "exit" => Cmd::Exit,
        "e" | "enter" => Cmd::Enter,
        _ => return None,
    })
}

impl Game {
    fn run_cmd(&mut self, c: Cmd) {
        let s = &mut self.state;
        match c {
            Cmd::Move(d) => s.step(d, &self.map, &self.tables, &self.locs),
            Cmd::Pass => s.pass(&self.map, &self.tables),
            Cmd::Board => s.board(),
            Cmd::Exit => s.exit(),
            Cmd::Enter => {
                s.say(">Enter");
                match s.location_here(&self.locs).map(|(id, n)| (id, n.to_string())) {
                    Some((id, name)) => {
                        s.say(format!("Entering {name}..."));
                        let what = match kind_of(id) {
                            Some(u1_assets::locations::LocationKind::Town) => "Towns",
                            Some(u1_assets::locations::LocationKind::Castle) => "Castles",
                            Some(u1_assets::locations::LocationKind::Landmark) => "Landmarks",
                            _ => "Dungeons",
                        };
                        s.say(format!("({what} not yet in u1)"));
                    }
                    None => s.say("Nothing to enter here"),
                }
            }
        }
    }
}

fn write_png(path: &PathBuf, w: u32, h: u32, rgb: &[u8]) -> Result<(), String> {
    let f = std::fs::File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(f), w, h);
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    let mut wr = enc.write_header().map_err(|e| e.to_string())?;
    wr.write_image_data(rgb).map_err(|e| e.to_string())
}

fn screenshot(mut g: Game, a: &Args, out: &PathBuf) -> Result<(), String> {
    for k in &a.keys {
        let c = cmd_from_name(k).ok_or(format!("unknown key {k}"))?;
        g.run_cmd(c);
    }
    let mut s = Screen::new();
    s.compose(&g.state, &g.map, &g.tiles, a.counter, false);
    let scale = 2; // same size as a 640x400 DOSBox capture
    write_png(out, (W * scale) as u32, (H * scale) as u32, &s.to_rgb(&g.palette, scale))?;
    println!("{} at ({},{}) moves={} food={}", out.display(), g.state.x, g.state.y, g.state.moves, g.state.food);
    Ok(())
}

fn play(mut g: Game, a: &Args) -> Result<(), String> {
    use sdl2::event::Event;
    use sdl2::keyboard::Keycode;
    use sdl2::pixels::PixelFormatEnum;

    let sdl = sdl2::init()?;
    let video = sdl.video()?;
    let window = video
        .window("u1 - Ultima I overworld", W as u32 * a.scale, H as u32 * a.scale)
        .position_centered()
        .build()
        .map_err(|e| e.to_string())?;
    let mut canvas = window.into_canvas().build().map_err(|e| e.to_string())?;
    let tc = canvas.texture_creator();
    let mut tex = tc.create_texture_streaming(PixelFormatEnum::RGB24, W as u32, H as u32).map_err(|e| e.to_string())?;
    let mut events = sdl.event_pump()?;
    let mut screen = Screen::new();
    let mut counter: u32 = a.counter;
    let mut idle: u32 = 0;
    let mut next_poll = Instant::now();
    let start = Instant::now();
    'main: loop {
        for ev in events.poll_iter() {
            let cmd = match ev {
                Event::Quit { .. } => break 'main,
                Event::KeyDown { keycode: Some(k), .. } => match k {
                    Keycode::Escape => break 'main,
                    Keycode::Up | Keycode::Kp8 => Some(Cmd::Move(Dir::North)),
                    Keycode::Down | Keycode::Kp2 => Some(Cmd::Move(Dir::South)),
                    Keycode::Left | Keycode::Kp4 => Some(Cmd::Move(Dir::West)),
                    Keycode::Right | Keycode::Kp6 => Some(Cmd::Move(Dir::East)),
                    Keycode::Space => Some(Cmd::Pass),
                    Keycode::B => Some(Cmd::Board),
                    Keycode::X => Some(Cmd::Exit),
                    Keycode::E => Some(Cmd::Enter),
                    _ => None,
                },
                _ => None,
            };
            if let Some(c) = cmd {
                idle = 0;
                g.run_cmd(c);
            }
        }
        let now = Instant::now();
        while now >= next_poll {
            next_poll += POLL;
            counter = counter.wrapping_add(1);
            idle += 1;
            if idle >= IDLE_POLLS {
                idle = 0;
                g.run_cmd(Cmd::Pass);
            }
        }
        let blink = (start.elapsed().as_millis() / 300) % 2 == 0;
        screen.compose(&g.state, &g.map, &g.tiles, counter, blink);
        let rgb = screen.to_rgb(&g.palette, 1);
        tex.update(None, &rgb, W * 3).map_err(|e| e.to_string())?;
        canvas.copy(&tex, None, None)?;
        canvas.present();
        std::thread::sleep(Duration::from_millis(15));
    }
    Ok(())
}

fn main() -> ExitCode {
    let res = parse_args().and_then(|a| {
        let g = load(&a)?;
        match &a.screenshot {
            Some(out) => screenshot(g, &a, out),
            None => play(g, &a),
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
