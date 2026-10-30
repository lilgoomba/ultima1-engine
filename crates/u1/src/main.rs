//! `u1`: play Ultima I (overworld, towns and castles) with your own game
//! files.
//!
//!   u1 --game-dir <path> [--player N | --save FILE | --pos X,Y] [--mode ega|t1k|cga]
//!      [--scale N] [--save-to FILE]
//!   u1 --game-dir <path> ... --screenshot out.png [--keys TEXT] [--counter N]
//!   u1 --game-dir <path> ... --video enhanced [--pack DIR]
//!   u1 --game-dir <path> ... --sound original|enhanced|mute
//!   u1 --sound-demo out.wav [--pack DIR]
//!
//! F1 opens the options menu (display and sound, switched live); the
//! choice is kept in ~/.config/ultima1-engine/config.toml, which the
//! command-line options override.
//!
//! Game files are only read. Quit (Q, outdoors) writes the 820-byte player
//! record to `--save-to` (default `./PLAYER1.U1` in the current directory,
//! which must not be the game directory). Tables, names and messages come
//! from the user's OUT.EXE at start-up.

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, Instant};

use u1_assets::config::{self, Config, VideoChoice};
use u1_assets::enhanced::{self, Pack};
use u1_assets::options::{MenuKey, OptionsMenu};
use u1_assets::exe::Exe;
use u1_assets::game::{Assets, Game, VideoSet};
use u1_assets::gamedir::{GameDir, VideoMode};
use u1_assets::io::{keys, Halt, Io, IoResult, ScriptIo};
use u1_assets::locations::LocationTable;
use u1_assets::palette::{cga_as_16, Rgb, CGA_PALETTE_1_LOW, EGA_PALETTE};
use u1_assets::rules::ExeTables;
use u1_assets::save::PlayerRecord;
use u1_assets::sound::{self, Sfx};
use u1_assets::soundpack::{SoundMode, SoundPack};
use std::sync::Arc;
use u1_assets::tcd::TownCastleMaps;
use u1_assets::term::{Term, H, W};
use u1_assets::gen::{Gen, GenEnd, GenFiles};
use u1_assets::title::{self, TitleFiles};

/// One keyboard/animation poll of the original main loop, measured in
/// DOSBox (130 ms at 20000 cycles).
const POLL: Duration = Duration::from_micros(130_000);
/// One tick of the PC timer.
const TICK: Duration = Duration::from_micros(54_925);
/// Start of a new character (verified from GEN.EXE).
const DEFAULT_POS: (u16, u16) = (49, 40);

const USAGE: &str = "usage: u1 --game-dir <path> [--player N | --save FILE | --pos X,Y]
          [--mode ega|t1k|cga | C] [--font FILE] [--intro | --gen] [--roster DIR] [--scale N] [--save-to FILE]
          [--video original|enhanced] [--sound original|enhanced|mute] [--pack DIR]
          [--screenshot out.png [--keys TEXT] [--counter N] [--options]]

Keys are the original's (arrows move; A B C D E F G H I K N O Q R S T U V X Z;
Space passes). --keys is typed text; {up} {down} {left} {right} {enter}
{esc} {bs} {space} name special keys and ~ ends a timed wait.
--intro starts with the title sequence and the character menu, --gen with
the menu; characters are kept in --roster DIR (default: the --save-to
directory, never the game directory) and the chosen one is saved back there.
--video enhanced draws the art of an asset pack (default: assets/enhanced in
the source tree) over the original tiles at a higher resolution; tiles the
pack lacks are the original scaled up. The game itself is unchanged.
--sound enhanced plays the pack's sounds (sounds/*.wav) instead of the
PC-speaker effects, the original effect for any it lacks; the default is the
original beeps (--mute, or --sound mute, plays nothing). Timing is the
original's either way. --sound-demo FILE writes every enhanced sound, each
after its original effect, to a WAV file.
F1 (a key the original never reads) pauses the game and opens the options
menu: the display (Enhanced, EGA, CGA, Tandy) and the sound (original beeps,
enhanced, mute), switched without restarting. The choice is saved to
~/.config/ultima1-engine/config.toml ($XDG_CONFIG_HOME if set); --video,
--mode and --sound override the file for one run. Screenshots ignore the
file; --options draws the menu over a screenshot.";

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
    font: Option<PathBuf>,
    intro: bool,
    gen: bool,
    roster: Option<PathBuf>,
    sound: SoundMode,
    sound_demo: Option<PathBuf>,
    enhanced: bool,
    pack: Option<PathBuf>,
    /// The display and sound options given on the command line (they win
    /// over the config file).
    cli: config::Cli,
    /// Screenshots: draw the options menu over the last screen.
    options_shot: bool,
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
        font: std::env::var_os("U1_FONT").map(PathBuf::from),
        intro: false,
        gen: false,
        roster: None,
        sound: SoundMode::Original,
        sound_demo: None,
        enhanced: false,
        pack: None,
        cli: config::Cli::default(),
        options_shot: false,
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
            "--mode" => {
                a.mode = Some(VideoMode::parse(&v()?).ok_or("bad --mode")?);
                a.cli.mode = a.mode;
            }
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
            "--font" => a.font = Some(v()?.into()),
            // Start with the title sequence (ULTIMA.EXE) like the original.
            "--intro" => a.intro = true,
            "--gen" => a.gen = true,
            "--roster" => a.roster = Some(v()?.into()),
            // No speaker output; effects still take their original time.
            "--mute" => {
                a.sound = SoundMode::Mute;
                a.cli.sound = Some(a.sound);
            }
            "--sound" => {
                a.sound = SoundMode::parse(&v()?).ok_or("--sound wants original, enhanced or mute")?;
                a.cli.sound = Some(a.sound);
            }
            "--sound-demo" => {
                a.sound_demo = Some(v()?.into());
                have_dir = true;
            }
            "--video" => {
                a.enhanced = match v()?.as_str() {
                    "original" => false,
                    "enhanced" => true,
                    _ => return Err("--video wants original or enhanced".into()),
                };
                a.cli.enhanced = Some(a.enhanced);
            }
            "--options" => a.options_shot = true,
            "--pack" => a.pack = Some(v()?.into()),
            "--counter" => a.counter = v()?.parse().map_err(|_| "bad --counter")?,
            "-h" | "--help" => return Err(USAGE.into()),
            // The original's only video choice: a first argument starting
            // with C forces CGA (`ULTIMA C`).
            c if !c.starts_with('-') && c.to_ascii_uppercase().starts_with('C') => {
                a.mode = Some(VideoMode::Cga);
                a.cli.mode = a.mode;
            }
            other => return Err(format!("unknown argument {other}\n{USAGE}")),
        }
    }
    if !have_dir {
        return Err(USAGE.into());
    }
    a.scale = a.scale.clamp(1, 6);
    if a.pack.is_some() && !a.enhanced && a.sound != SoundMode::Enhanced && a.sound_demo.is_none() {
        return Err("--pack needs --video enhanced or --sound enhanced".into());
    }
    Ok(a)
}

/// The enhanced mode's pack: `--pack`, else `assets/enhanced` in the
/// current directory or the source tree; with none, no art (everything
/// is the original, scaled up).
fn pack_dir(a: &Args) -> Option<PathBuf> {
    match &a.pack {
        Some(d) => Some(d.clone()),
        None => [PathBuf::from("assets/enhanced"), Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/enhanced")]
            .into_iter()
            .find(|d| d.join("manifest.toml").is_file()),
    }
}

/// The enhanced sound mode's sounds (`sounds/` in the pack folder), at
/// the output rate.
fn load_sounds(a: &Args, rate: u32) -> SoundPack {
    let p = match pack_dir(a) {
        Some(d) => SoundPack::load(&d.join("sounds"), rate),
        None => SoundPack { rate, ..Default::default() },
    };
    for w in &p.warnings {
        eprintln!("sounds: {w}");
    }
    if p.clips.is_empty() {
        eprintln!("no enhanced sounds found; playing the original effects");
    }
    p
}

/// `--sound-demo`: each event's original effect, then its enhanced sound.
fn sound_demo(a: &Args, out: &Path) -> Result<(), String> {
    let p = load_sounds(a, 44_100);
    let (wav, order) = u1_assets::soundpack::demo_reel(&p);
    std::fs::write(out, u1_assets::soundpack::encode_wav(p.rate, &wav)).map_err(|e| e.to_string())?;
    for l in order {
        println!("{l}");
    }
    Ok(())
}

fn load_pack(a: &Args) -> Result<Option<Pack>, String> {
    if !a.enhanced {
        return Ok(None);
    }
    open_pack(pack_dir(a).as_deref()).map(Some)
}

fn open_pack(dir: Option<&Path>) -> Result<Pack, String> {
    let pack = match dir {
        Some(d) => Pack::load(d)?,
        None => {
            eprintln!("no asset pack found (assets/enhanced); showing the original art scaled up");
            Pack::empty(32)
        }
    };
    for w in &pack.warnings {
        eprintln!("pack: {w}");
    }
    Ok(pack)
}

/// The screen as RGB at the output resolution: the original, or composed
/// with the pack (`scale` is the original's factor).
fn frame_rgb(t: &Term, palette: &[Rgb; 16], pack: Option<&Pack>, scale: usize, ms: u64) -> (usize, usize, Vec<u8>) {
    match pack {
        Some(p) => (W * p.scale(), H * p.scale(), enhanced::compose(t, palette, p, ms)),
        None => (W * scale, H * scale, t.to_rgb(palette, scale)),
    }
}

fn same_dir(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(x), Ok(y)) => x == y,
        _ => false,
    }
}

fn install_font(a: &Args) -> Result<(), String> {
    if let Some(p) = &a.font {
        let b = std::fs::read(p).map_err(|e| format!("{}: {e}", p.display()))?;
        u1_assets::font::install(u1_assets::font::Font::parse(&b).map_err(|e| format!("{}: {e}", p.display()))?);
    }
    Ok(())
}

fn video_mode(a: &Args) -> Result<VideoMode, String> {
    let g = GameDir::new(&a.game_dir).map_err(|e| e.to_string())?;
    a.mode.or_else(|| g.detect_video_mode()).ok_or_else(|| "no tile files found in the game directory".into())
}

fn palette_for(mode: VideoMode) -> [Rgb; 16] {
    if mode == VideoMode::Cga { cga_as_16(&CGA_PALETTE_1_LOW) } else { EGA_PALETTE }
}

/// The directory the character program keeps its roster in.
fn roster_dir(a: &Args) -> Result<PathBuf, String> {
    let dir = match (&a.roster, &a.save_to) {
        (Some(d), _) => d.clone(),
        (None, Some(p)) => p.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new(".")).to_path_buf(),
        (None, None) => PathBuf::from("."),
    };
    if same_dir(&dir, &a.game_dir) {
        return Err("--roster must not be the game directory (originals are never overwritten)".into());
    }
    Ok(dir)
}

/// What the start-up programs decided.
enum Front {
    /// No --intro / --gen: straight into the game.
    Skip,
    /// The script ran out on one of their screens.
    Halted(Term),
    /// Play this roster file.
    Play(PathBuf),
}

/// The title sequence (with --intro) and the character program.
fn front(a: &Args, mode: VideoMode, io: &mut dyn Io) -> Result<Front, String> {
    if !a.intro && !a.gen {
        return Ok(Front::Skip);
    }
    let g = GameDir::new(&a.game_dir).map_err(|e| e.to_string())?;
    if a.intro {
        let tf = TitleFiles::load(&g, mode).map_err(|e| e.to_string())?;
        let mut t = title::blank_term(&tf);
        if title::run_intro(&tf, &mut t, io).is_err() {
            return Ok(Front::Halted(t));
        }
    }
    let gf = GenFiles::load(&g, mode).map_err(|e| e.to_string())?;
    let dir = roster_dir(a)?;
    let fixed = a.seed.is_some();
    let clock: Box<dyn FnMut() -> [u8; 4]> = if fixed { Box::new(|| [0, 0, 0, 0]) } else { Box::new(local_time) };
    let mut gen = Gen::new(&gf, &dir, clock);
    let mut t = Term::with_cga(mode == VideoMode::Cga);
    match gen.run(&mut t, io) {
        Ok(GenEnd::Play { path, .. }) => {
            // The "please wait" screen stays up while the game loads.
            if io.pause(&t, 9).is_err() {
                return Ok(Front::Halted(t));
            }
            Ok(Front::Play(path))
        }
        Err(Halt) => Ok(Front::Halted(t)),
    }
}

fn load(a: &Args, mode: VideoMode, chosen: Option<&Path>) -> Result<(Game, [Rgb; 16]), String> {
    let e = |e: u1_assets::error::AssetError| e.to_string();
    let g = GameDir::new(&a.game_dir).map_err(e)?;
    let exe_bytes = g.read("OUT.EXE").map_err(e)?;
    let assets = Assets {
        t: ExeTables::from_out_exe(&exe_bytes).map_err(e)?,
        locs: LocationTable::from_out_exe(&exe_bytes).map_err(e)?,
        exe: Exe::new(exe_bytes).map_err(e)?,
        map: g.load_world_map().map_err(e)?,
        world_tiles: g.load_world_tiles(mode).map_err(e)?,
        town_tiles: g.load_town_tiles(mode).map_err(e)?,
        tcd: TownCastleMaps::parse(&g.read("TCD.BIN").map_err(e)?).map_err(e)?,
        video: mode,
    };
    let palette = palette_for(mode);
    let save_bytes = match (chosen, &a.save, a.player) {
        (Some(p), _, _) => Some(std::fs::read(p).map_err(|e| format!("{}: {e}", p.display()))?),
        (None, Some(p), _) => Some(std::fs::read(p).map_err(|e| format!("{}: {e}", p.display()))?),
        (None, None, Some(n)) => Some(g.read(&format!("PLAYER{n}.U1")).map_err(e)?),
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
    // SPACE.EXE is optional: without it boarding a shuttle stops the run.
    game.space = u1_assets::space::SpaceFiles::load(&g, game.a.video).ok();
    game.mondain = u1_assets::mondain::MondainFiles::load(&g, game.a.video).ok();
    // A character chosen in the menu is saved back to its roster file.
    let save_to = match chosen {
        Some(p) => p.to_path_buf(),
        None => a.save_to.clone().unwrap_or_else(|| PathBuf::from("PLAYER1.U1")),
    };
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

/// The start-up program's files, when `--intro` asks for them.
fn screenshot(a: &Args, out: &Path) -> Result<(), String> {
    let mut io = ScriptIo::from_text(&a.keys);
    let scale = 2; // the size of a 640x400 DOSBox capture
    let mode = video_mode(a)?;
    let pack = load_pack(a)?;
    let shot = |t: &Term, pal: &[Rgb; 16]| -> Result<(), String> {
        let (w, h, rgb) = frame_rgb(t, pal, pack.as_ref(), scale, 0);
        write_png(out, w as u32, h as u32, &rgb)
    };
    let chosen = match front(a, mode, &mut io)? {
        Front::Halted(t) => {
            shot(&t, &palette_for(mode))?;
            println!("{}: start-up", out.display());
            return Ok(());
        }
        Front::Play(p) => Some(p),
        Front::Skip => None,
    };
    let (mut g, palette) = load(a, mode, chosen.as_deref())?;
    let palette = &palette;
    let _ = g.run(&mut io);
    if a.options_shot {
        let avail = available(a);
        OptionsMenu::new(VideoChoice::from_parts(a.enhanced, mode), a.sound, avail).draw(&mut g.term);
    }
    shot(&g.term, palette)?;
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
    /// The sound output (none without an audio device).
    audio: Option<(sdl2::audio::AudioDevice<Mixer>, u32)>,
    sound: SoundMode,
    /// The enhanced sounds, loaded on first use.
    sounds: Option<SoundPack>,
    sound_src: Option<PathBuf>,
    /// The enhanced mode's art (drawn when `enhanced`; kept loaded when
    /// the options menu turns it off).
    pack: Option<Pack>,
    enhanced: bool,
    pack_src: Option<PathBuf>,
    start: Instant,
    /// The texture creator, for a new texture when the resolution changes.
    tc: &'static sdl2::render::TextureCreator<sdl2::video::WindowContext>,
    /// `--scale`: the window is this multiple of 320x200.
    scale: u32,
    game_dir: PathBuf,
    /// The display the game draws in, and the one the menu asked for (the
    /// game switches at its next key poll, `video_switch`).
    shown: VideoMode,
    wanted: VideoMode,
    /// Which of the menu's displays this game directory has.
    avail: [bool; 4],
    /// F1 was pressed: the menu opens at the next screen update.
    menu_wanted: bool,
    in_menu: bool,
    menu_keys: std::collections::VecDeque<u8>,
    cfg_path: Option<PathBuf>,
}

impl SdlIo {
    fn present(&mut self, t: &Term) -> IoResult<()> {
        let ms = self.start.elapsed().as_millis() as u64;
        let pack = if self.enhanced { self.pack.as_ref() } else { None };
        let (w, _, rgb) = frame_rgb(t, &self.palette, pack, 1, ms);
        self.tex.update(None, &rgb, w * 3).map_err(|_| Halt)?;
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
                        // The options menu: no key of the original's.
                        Keycode::F1 => {
                            if self.in_menu {
                                self.menu_keys.push_back(keys::ESC);
                            } else {
                                self.menu_wanted = true;
                                self.in_menu = true;
                            }
                            None
                        }
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
                        // Keys typed while the menu is up are the menu's.
                        if self.in_menu {
                            self.menu_keys.push_back(c);
                        } else {
                            self.pending.push_back(c);
                        }
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
            self.menu(t)?;
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
        let mut end = Instant::now() + TICK * ticks;
        while Instant::now() < end {
            self.pump()?;
            end += self.menu(t)?;
            std::thread::sleep(Duration::from_millis(5));
        }
        Ok(())
    }

    fn poll_now(&mut self, t: &Term) -> IoResult<Option<u8>> {
        self.present(t)?;
        self.pump()?;
        self.menu(t)?;
        Ok(self.pending.pop_front())
    }

    fn retrace(&mut self, t: &Term, n: u32) -> IoResult<()> {
        self.present(t)?;
        let mut end = Instant::now() + Duration::from_micros(14_286) * n;
        while Instant::now() < end {
            self.pump()?;
            end += self.menu(t)?;
            std::thread::sleep(Duration::from_millis(2));
        }
        Ok(())
    }

    fn flush(&mut self) {
        self.pending.clear();
    }

    fn sound(&mut self, n: u8) {
        self.sound_ev(n, Sfx::default_for(n));
    }

    fn sound_ev(&mut self, n: u8, ev: Sfx) {
        // The original plays an effect with the CPU busy, so the game waits
        // for it to end; so does the engine, whatever is heard. An
        // enhanced sound plays on (mixed) past that wait.
        let e = sound::effect(n);
        let len = Duration::from_secs_f64(e.millis() / 1000.0);
        let clip = self.clip(ev);
        if let Some((dev, rate)) = &mut self.audio {
            match (self.sound, clip) {
                (SoundMode::Mute, _) => {}
                (SoundMode::Enhanced, Some(c)) => dev.lock().play(c),
                _ => dev.lock().play(Arc::new(sound::render(&e, *rate, sound::CPU_HZ, u1_assets::soundpack::BEEP_AMP))),
            }
        }
        let end = Instant::now() + len;
        while Instant::now() < end {
            let _ = self.pump();
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    fn sound_distant(&mut self, n: u8) {
        if self.sound == SoundMode::Enhanced {
            // Unheard, but the original's time passes all the same.
            let end = Instant::now() + Duration::from_secs_f64(sound::effect(n).millis() / 1000.0);
            while Instant::now() < end {
                let _ = self.pump();
                std::thread::sleep(Duration::from_millis(1));
            }
        } else {
            self.sound(n);
        }
    }

    fn cue(&mut self, ev: Sfx) {
        if let (Some(c), Some((dev, _))) = (self.clip(ev), &mut self.audio) {
            dev.lock().play(c);
        }
    }

    fn video_switch(&mut self) -> Option<VideoSet> {
        if self.wanted == self.shown {
            return None;
        }
        let mode = self.wanted;
        match load_video(&self.game_dir, mode) {
            Ok(v) => Some(v),
            Err(e) => {
                eprintln!("display {}: {e}", mode_name(mode));
                self.wanted = self.shown;
                None
            }
        }
    }

    fn video_shown(&mut self, mode: VideoMode) {
        self.shown = mode;
        self.palette = palette_for(mode);
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

impl SdlIo {
    /// The options menu, if F1 was pressed: drawn over a copy of `t` and
    /// run with the game paused until F1, Enter or Esc. Returns how long it
    /// was open (timed waits resume where they were).
    fn menu(&mut self, t: &Term) -> IoResult<Duration> {
        if !self.menu_wanted {
            return Ok(Duration::ZERO);
        }
        self.menu_wanted = false;
        let opened = Instant::now();
        let mut m = OptionsMenu::new(VideoChoice::from_parts(self.enhanced, self.wanted), self.sound, self.avail);
        let r = loop {
            let mut c = t.clone();
            m.draw(&mut c);
            if let Err(e) = self.present(&c) {
                break Err(e);
            }
            if let Err(e) = self.pump() {
                break Err(e);
            }
            let mut close = false;
            while let Some(k) = self.menu_keys.pop_front() {
                match m.key(k) {
                    MenuKey::Changed => self.choose(m.video, m.sound),
                    MenuKey::Close => close = true,
                    MenuKey::Stay => {}
                }
            }
            if close {
                break Ok(());
            }
            std::thread::sleep(Duration::from_millis(15));
        };
        self.in_menu = false;
        self.menu_keys.clear();
        r?;
        let paused = opened.elapsed();
        self.next = Instant::now() + POLL;
        self.present(t)?;
        Ok(paused)
    }

    /// Apply the menu's choice now (the display's pictures at the game's
    /// next key poll) and save it.
    fn choose(&mut self, video: VideoChoice, sound: SoundMode) {
        self.sound = sound;
        self.wanted = video.mode();
        if video.enhanced() != self.enhanced {
            self.set_enhanced(video.enhanced());
        }
        if let Some(p) = &self.cfg_path {
            let c = Config { video: Some(video), sound: Some(sound), warnings: Vec::new() };
            if let Err(e) = c.save(p) {
                eprintln!("{}: {e}", p.display());
            }
        }
    }

    /// Turn the enhanced art on or off: a new internal resolution, and the
    /// window resized to the same multiple of the original it started at.
    fn set_enhanced(&mut self, on: bool) {
        if on && self.pack.is_none() {
            match open_pack(self.pack_src.as_deref()) {
                Ok(p) => self.pack = Some(p),
                Err(e) => {
                    eprintln!("pack: {e}");
                    return;
                }
            }
        }
        self.enhanced = on;
        let k = if on { self.pack.as_ref().map_or(1, |p| p.scale()) as u32 } else { 1 };
        let (iw, ih) = (W as u32 * k, H as u32 * k);
        let tex = match self.tc.create_texture_streaming(sdl2::pixels::PixelFormatEnum::RGB24, iw, ih) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("{e}");
                return;
            }
        };
        self.tex = tex;
        let _ = self.canvas.set_logical_size(iw, ih);
        let n = (self.scale.max(k) / k).max(1);
        let w = self.canvas.window_mut();
        let full = !matches!(w.fullscreen_state(), sdl2::video::FullscreenType::Off);
        if !full && !w.is_maximized() {
            let _ = w.set_size(iw * n, ih * n);
        }
    }

    /// The enhanced sound for `ev`, if that mode is on and the pack has one.
    fn clip(&mut self, ev: Sfx) -> Option<Arc<Vec<i16>>> {
        if self.sound != SoundMode::Enhanced {
            return None;
        }
        let rate = self.audio.as_ref()?.1;
        if self.sounds.is_none() {
            let p = match &self.sound_src {
                Some(d) => SoundPack::load(d, rate),
                None => SoundPack { rate, ..Default::default() },
            };
            for w in &p.warnings {
                eprintln!("sounds: {w}");
            }
            self.sounds = Some(p);
        }
        self.sounds.as_ref()?.get(ev).cloned()
    }
}

/// Our own mixer: every sound playing is summed (beeps queue no longer
/// block enhanced sounds that outlast the original's wait).
struct Mixer {
    voices: Vec<(Arc<Vec<i16>>, usize)>,
    rate: usize,
}

impl Mixer {
    fn play(&mut self, s: Arc<Vec<i16>>) {
        // The same sound asked for again within 150 ms is one sound (the
        // original repeats some effects, e.g. three times at death); later
        // it restarts rather than stacking (footsteps, the star field).
        if self.voices.iter().any(|(v, p)| Arc::ptr_eq(v, &s) && *p < self.rate * 3 / 20) {
            return;
        }
        self.voices.retain(|(v, _)| !Arc::ptr_eq(v, &s));
        if self.voices.len() < 16 {
            self.voices.push((s, 0));
        }
    }
}

impl sdl2::audio::AudioCallback for Mixer {
    type Channel = i16;
    fn callback(&mut self, out: &mut [i16]) {
        for o in out.iter_mut() {
            let mut acc = 0i32;
            for (v, pos) in self.voices.iter_mut() {
                if let Some(&x) = v.get(*pos) {
                    acc += i32::from(x);
                    *pos += 1;
                }
            }
            *o = acc.clamp(-32768, 32767) as i16;
        }
        self.voices.retain(|(v, p)| *p < v.len());
    }
}

/// A mono 16-bit mixed output; `None` (silent play) if there is no audio
/// device.
fn open_speaker(sdl: &sdl2::Sdl) -> Option<(sdl2::audio::AudioDevice<Mixer>, u32)> {
    let sub = sdl.audio().map_err(|e| eprintln!("no audio: {e}")).ok()?;
    let want = sdl2::audio::AudioSpecDesired { freq: Some(44_100), channels: Some(1), samples: Some(512) };
    let q = sub.open_playback(None, &want, |spec| Mixer { voices: Vec::new(), rate: spec.freq.max(1) as usize }).map_err(|e| eprintln!("no audio: {e}")).ok()?;
    q.resume();
    let rate = q.spec().freq as u32;
    // The subsystem must outlive the queue.
    std::mem::forget(sub);
    Some((q, rate))
}

/// One display's pictures, read from the game directory.
fn load_video(dir: &Path, mode: VideoMode) -> Result<VideoSet, String> {
    let e = |e: u1_assets::error::AssetError| e.to_string();
    let g = GameDir::new(dir).map_err(e)?;
    Ok(VideoSet {
        mode,
        world_tiles: g.load_world_tiles(mode).map_err(e)?,
        town_tiles: g.load_town_tiles(mode).map_err(e)?,
        space: u1_assets::space::SpaceFiles::load(&g, mode).ok(),
        mondain: u1_assets::mondain::MondainFiles::load(&g, mode).ok(),
    })
}

fn mode_name(m: VideoMode) -> &'static str {
    VideoChoice::from_parts(false, m).label()
}

/// Which of the menu's displays ([`VideoChoice::ALL`]) the game directory
/// has (the enhanced art sits on EGA).
fn available(a: &Args) -> [bool; 4] {
    let Ok(g) = GameDir::new(&a.game_dir) else { return [false; 4] };
    VideoChoice::ALL.map(|v| g.load_world_tiles(v.mode()).is_ok() && g.load_town_tiles(v.mode()).is_ok())
}

fn play(a: &Args) -> Result<(), String> {
    // The command line, then the config file, then the defaults.
    let cfg_path = config::default_path();
    let cfg = cfg_path.as_deref().map(Config::load).unwrap_or_default();
    for w in &cfg.warnings {
        eprintln!("config: {w}");
    }
    let avail = available(a);
    let detected = video_mode(a)?;
    let mut set = config::resolve(a.cli, &cfg, detected);
    if !avail[VideoChoice::ALL.iter().position(|&v| v == VideoChoice::from_parts(false, set.mode)).unwrap_or(1)] {
        eprintln!("no {} files in the game directory; using {}", mode_name(set.mode), mode_name(detected));
        set.mode = detected;
    }
    let mode = set.mode;
    let palette = palette_for(mode);
    use sdl2::pixels::PixelFormatEnum;
    let sdl = sdl2::init()?;
    let video = sdl.video()?;
    let pack_src = pack_dir(a);
    let pack = if set.enhanced { Some(open_pack(pack_src.as_deref())?) } else { None };
    // Internal resolution: 320x200, or the pack's multiple of it; the
    // window is a whole multiple of that and stays integer-scaled.
    let k = pack.as_ref().map_or(1, |p| p.scale()) as u32;
    let (iw, ih) = (W as u32 * k, H as u32 * k);
    let n = (a.scale / k).max(1);
    let window = video
        .window("u1 - Ultima I", iw * n, ih * n)
        .position_centered()
        .resizable()
        .build()
        .map_err(|e| e.to_string())?;
    let mut canvas = window.into_canvas().build().map_err(|e| e.to_string())?;
    canvas.set_logical_size(iw, ih).map_err(|e| e.to_string())?;
    canvas.set_integer_scale(true).map_err(|e| e.to_string())?;
    // The texture creator lives for the whole program.
    let tc: &'static _ = Box::leak(Box::new(canvas.texture_creator()));
    let tex = tc.create_texture_streaming(PixelFormatEnum::RGB24, iw, ih).map_err(|e| e.to_string())?;
    let events = sdl.event_pump()?;
    // Opened even when muted, so the options can turn sound on.
    let audio = open_speaker(&sdl);
    let sound_src = pack_src.as_ref().map(|d| d.join("sounds"));
    let mut io = SdlIo {
        canvas,
        tex,
        events,
        palette,
        next: Instant::now(),
        pending: Default::default(),
        audio,
        sound: set.sound,
        sounds: None,
        sound_src,
        pack,
        enhanced: set.enhanced,
        pack_src,
        start: Instant::now(),
        tc,
        scale: a.scale,
        game_dir: a.game_dir.clone(),
        shown: mode,
        wanted: mode,
        avail,
        menu_wanted: false,
        in_menu: false,
        menu_keys: Default::default(),
        cfg_path,
    };
    let chosen = match front(a, mode, &mut io)? {
        Front::Halted(_) => return Ok(()),
        Front::Play(p) => Some(p),
        Front::Skip => None,
    };
    // A display chosen during the title or the character menu starts the
    // game.
    let mode = io.wanted;
    io.video_shown(mode);
    let (mut g, _) = load(a, mode, chosen.as_deref())?;
    let _ = g.run(&mut io);
    if let Some(s) = g.stopped {
        eprintln!("stopped: {s}");
    }
    Ok(())
}

fn main() -> ExitCode {
    let res = parse_args().and_then(|a| {
        if let Some(out) = &a.sound_demo {
            return sound_demo(&a, out);
        }
        install_font(&a)?;
        match &a.screenshot {
            Some(out) => screenshot(&a, out),
            None => play(&a),
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
