//! u1view: scrollable overworld viewer for the u1 clean-room engine.
//!
//! Usage:
//!   u1view --game-dir <path> [--mode ega|t1k|cga]
//!   u1view --demo [--seed N]
//! Common options: --scale N, --cols N, --rows N, --start X,Y,
//!   --screenshot <file.png> (render one frame, save it, exit)

use sdl2::event::Event;
use sdl2::keyboard::{Keycode, Mod};
use sdl2::pixels::PixelFormatEnum;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{Duration, Instant};
use u1_assets::demo;
use u1_assets::gamedir::{GameDir, VideoMode};
use u1_assets::map::WorldMap;
use u1_assets::palette::{cga_as_16, Rgb, CGA_PALETTE_1_LOW, EGA_PALETTE};
use u1_assets::render::render_world;
use u1_assets::tiles::{TileSet, EGA_TILES};

struct Args {
    game_dir: Option<PathBuf>,
    demo: bool,
    mode: Option<VideoMode>,
    seed: u32,
    scale: u32,
    cols: usize,
    rows: usize,
    start: Option<(isize, isize)>,
    screenshot: Option<PathBuf>,
}

const USAGE: &str = "\
u1view - Ultima I overworld viewer (clean-room engine)

USAGE:
    u1view --game-dir <path> [options]
    u1view --demo [options]

OPTIONS:
    --game-dir <path>     Folder of your legally owned Ultima I (DOS) install
    --demo                Use procedurally generated placeholder assets
    --mode <ega|t1k|cga>  Tile set to use (default: best available)
    --seed <n>            Demo map seed (default 1)
    --scale <n>           Pixel scale factor (default 3)
    --cols <n>            Visible columns (default 21)
    --rows <n>            Visible rows (default 13)
    --start <x,y>         Initial top-left cell
    --screenshot <file>   Render one frame to a PNG and exit
    -h, --help            Show this help

KEYS: arrows scroll (Shift = 8 cells), Home recentres, Esc quits.";

fn parse_args() -> Result<Args, String> {
    let mut a = Args {
        game_dir: None,
        demo: false,
        mode: None,
        seed: 1,
        scale: 3,
        cols: 21,
        rows: 13,
        start: None,
        screenshot: None,
    };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        let mut val = |name: &str| it.next().ok_or_else(|| format!("{name} needs a value"));
        match arg.as_str() {
            "--game-dir" => a.game_dir = Some(val("--game-dir")?.into()),
            "--demo" => a.demo = true,
            "--mode" => {
                let v = val("--mode")?;
                a.mode = Some(VideoMode::parse(&v).ok_or(format!("unknown mode {v}"))?)
            }
            "--seed" => a.seed = val("--seed")?.parse().map_err(|e| format!("--seed: {e}"))?,
            "--scale" => a.scale = val("--scale")?.parse().map_err(|e| format!("--scale: {e}"))?,
            "--cols" => a.cols = val("--cols")?.parse().map_err(|e| format!("--cols: {e}"))?,
            "--rows" => a.rows = val("--rows")?.parse().map_err(|e| format!("--rows: {e}"))?,
            "--start" => {
                let v = val("--start")?;
                let (x, y) = v.split_once(',').ok_or("--start expects X,Y")?;
                a.start = Some((
                    x.trim().parse().map_err(|_| "bad X")?,
                    y.trim().parse().map_err(|_| "bad Y")?,
                ));
            }
            "--screenshot" => a.screenshot = Some(val("--screenshot")?.into()),
            "-h" | "--help" => {
                println!("{USAGE}");
                std::process::exit(0);
            }
            other => return Err(format!("unknown argument: {other}\n\n{USAGE}")),
        }
    }
    if a.demo == a.game_dir.is_some() {
        return Err(format!("specify exactly one of --game-dir or --demo\n\n{USAGE}"));
    }
    if a.scale == 0 || a.cols == 0 || a.rows == 0 {
        return Err("--scale/--cols/--rows must be positive".into());
    }
    Ok(a)
}

struct World {
    map: WorldMap,
    tiles: TileSet,
    palette: [Rgb; 16],
    label: String,
}

fn load(a: &Args) -> Result<World, String> {
    if a.demo {
        let tiles = TileSet::decode_spec(&demo::demo_egatiles_bin(), &EGA_TILES).map_err(|e| e.to_string())?;
        let map = WorldMap::parse(&demo::demo_map_bin(a.seed)).map_err(|e| e.to_string())?;
        return Ok(World { map, tiles, palette: EGA_PALETTE, label: "DEMO (placeholder assets)".into() });
    }
    let dir = a.game_dir.as_ref().expect("checked");
    let g = GameDir::new(dir).map_err(|e| e.to_string())?;
    let mode = a
        .mode
        .or_else(|| g.detect_video_mode())
        .ok_or("no EGATILES.BIN / T1KTILES.BIN / CGATILES.BIN found in game directory")?;
    let tiles = g.load_world_tiles(mode).map_err(|e| e.to_string())?;
    let map = g.load_world_map().map_err(|e| e.to_string())?;
    if map.height != u1_assets::map::WORLD_HEIGHT {
        eprintln!(
            "warning: MAP.BIN has {} rows (spec expects {}); please report this",
            map.height,
            u1_assets::map::WORLD_HEIGHT
        );
    }
    let palette = if mode == VideoMode::Cga { cga_as_16(&CGA_PALETTE_1_LOW) } else { EGA_PALETTE };
    Ok(World { map, tiles, palette, label: format!("{mode:?}") })
}

fn write_png(path: &PathBuf, w: u32, h: u32, rgb: &[u8]) -> Result<(), String> {
    let f = std::fs::File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(f), w, h);
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    let mut wr = enc.write_header().map_err(|e| e.to_string())?;
    wr.write_image_data(rgb).map_err(|e| e.to_string())
}

fn run(a: Args) -> Result<(), String> {
    let world = load(&a)?;
    let (tw, th) = (world.tiles.width, world.tiles.height);
    let (fb_w, fb_h) = ((a.cols * tw) as u32, (a.rows * th) as u32);
    let max_x = world.map.width as isize - a.cols as isize;
    let max_y = world.map.height as isize - a.rows as isize;
    let clamp = |x: isize, y: isize| (x.clamp(0, max_x.max(0)), y.clamp(0, max_y.max(0)));
    let centre = clamp(
        world.map.width as isize / 4 - a.cols as isize / 2,
        world.map.height as isize / 4 - a.rows as isize / 2,
    );
    let (mut vx, mut vy) = a.start.map(|(x, y)| clamp(x, y)).unwrap_or(centre);

    let sdl = sdl2::init()?;
    let video = sdl.video()?;
    let window = video
        .window("u1view", fb_w * a.scale, fb_h * a.scale)
        .position_centered()
        .build()
        .map_err(|e| e.to_string())?;
    let mut canvas = match window.into_canvas().accelerated().present_vsync().build() {
        Ok(c) => c,
        Err(_) => video
            .window("u1view", fb_w * a.scale, fb_h * a.scale)
            .build()
            .map_err(|e| e.to_string())?
            .into_canvas()
            .software()
            .build()
            .map_err(|e| e.to_string())?,
    };
    let tc = canvas.texture_creator();
    let mut tex = tc
        .create_texture_streaming(PixelFormatEnum::RGB24, fb_w, fb_h)
        .map_err(|e| e.to_string())?;
    let mut events = sdl.event_pump()?;
    let t0 = Instant::now();

    loop {
        let frame = (t0.elapsed().as_millis() / 500) as usize;
        let fb = render_world(&world.map, &world.tiles, &world.palette, vx, vy, a.cols, a.rows, frame);
        tex.update(None, &fb, fb_w as usize * 3).map_err(|e| e.to_string())?;
        canvas.clear();
        canvas.copy(&tex, None, None)?;
        if let Some(path) = &a.screenshot {
            // Read back what SDL actually rendered.
            let (w, h) = canvas.output_size()?;
            let pixels = canvas
                .read_pixels(None, PixelFormatEnum::RGB24)
                .unwrap_or_else(|_| Vec::new());
            if pixels.len() == (w * h * 3) as usize {
                write_png(path, w, h, &pixels)?;
            } else {
                eprintln!("read_pixels unavailable, saving unscaled framebuffer");
                write_png(path, fb_w, fb_h, &fb)?;
            }
            println!("saved screenshot to {}", path.display());
            return Ok(());
        }
        canvas.present();
        canvas
            .window_mut()
            .set_title(&format!("u1view [{}] - view ({vx},{vy}) - arrows scroll, Esc quits", world.label))
            .map_err(|e| e.to_string())?;

        for ev in events.poll_iter() {
            match ev {
                Event::Quit { .. } | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => return Ok(()),
                Event::KeyDown { keycode: Some(k), keymod, .. } => {
                    let step = if keymod.intersects(Mod::LSHIFTMOD | Mod::RSHIFTMOD) { 8 } else { 1 };
                    let (dx, dy) = match k {
                        Keycode::Left => (-step, 0),
                        Keycode::Right => (step, 0),
                        Keycode::Up => (0, -step),
                        Keycode::Down => (0, step),
                        Keycode::Home => {
                            (vx, vy) = centre;
                            (0, 0)
                        }
                        _ => (0, 0),
                    };
                    (vx, vy) = clamp(vx + dx, vy + dy);
                }
                _ => {}
            }
        }
        std::thread::sleep(Duration::from_millis(16));
    }
}

fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
    match run(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
