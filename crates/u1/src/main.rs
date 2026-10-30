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
//! F1 opens the options menu (display and sound, switched live, and the
//! optional engine features of SPEC §22, all off by default); the choice is
//! kept in ~/.config/ultima1-engine/config.toml, which the command-line
//! options override. Alt+Enter toggles fullscreen and F4 turbo; with save
//! slots on, F2 opens the slots, F5 quicksaves and F9 quickloads; with the
//! message log on, PgUp opens it.
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
use u1_assets::msglog::{self, LogView};
use u1_assets::options::{self as opts, MenuKey, OptionsMenu, PANEL};
use u1_assets::qol::{self, Qol, Scaling, Wait};
use u1_assets::slots::{SaveReq, Slot, SlotKey, SlotScreen};
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
use u1_assets::gen::GenEnd;

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
          [--fullscreen] [--crt]
          [--screenshot out.png [--keys TEXT] [--counter N] [--options | --slots | --log] [--crt]]

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
file; --options draws the menu over a screenshot.
The menu's optional features are all off by default and change nothing when
off: fullscreen (also Alt+Enter), whole-pixel or fill scaling, a CRT
scanline filter, save slots (F2 slots, F5 quicksave, F9 quickload; slot 1 is
the character's own file, every slot an original 820-byte record), a
message log (PgUp, PgDn), turbo (F4), shorter message delays, faster space
flight and steady timing. --fullscreen and --crt turn those on for one run;
in screenshots --crt applies the filter, --slots and --log draw those
windows (--keys may hold {saveN} / {loadN}, N 1-5 or q).";

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
    /// Screenshots: the save-slot screen, the message log.
    slots_shot: bool,
    log_shot: bool,
    /// `--crt`, `--fullscreen`: on for this run.
    crt: bool,
    fullscreen: bool,
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
        slots_shot: false,
        log_shot: false,
        crt: false,
        fullscreen: false,
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
            "--slots" => a.slots_shot = true,
            "--log" => a.log_shot = true,
            "--crt" => a.crt = true,
            "--fullscreen" => a.fullscreen = true,
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
    // A display chosen on these screens starts the program again in it.
    let mut mode = mode;
    if a.intro {
        if let Some(t) = u1_assets::front::intro(&g, &mut mode, io).map_err(|e| e.to_string())? {
            return Ok(Front::Halted(t));
        }
    }
    let dir = roster_dir(a)?;
    let fixed = a.seed.is_some();
    let mut clock = || -> Box<dyn FnMut() -> [u8; 4]> { if fixed { Box::new(|| [0, 0, 0, 0]) } else { Box::new(local_time) } };
    let (t, r) = u1_assets::front::character(&g, &dir, &mut mode, &mut clock, io).map_err(|e| e.to_string())?;
    match r {
        Ok(GenEnd::Play { path, .. }) => {
            // The "please wait" screen stays up while the game loads (a
            // display chosen now is the game's).
            if io.pause(&t, 9).is_err() && io.display_restart().is_none() {
                return Ok(Front::Halted(t));
            }
            Ok(Front::Play(path))
        }
        Err(Halt) => Ok(Front::Halted(t)),
    }
}

/// The game for the record in `chosen` (a roster file), `--save` or
/// `--player`; `reload` (a save slot loaded) wins over all of them, and
/// `--pos` is not applied to it.
fn load(a: &Args, mode: VideoMode, chosen: Option<&Path>, reload: Option<PlayerRecord>) -> Result<(Game, [Rgb; 16]), String> {
    let reloaded = reload.is_some();
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
        _ if reloaded => None,
        (Some(p), _, _) => Some(std::fs::read(p).map_err(|e| format!("{}: {e}", p.display()))?),
        (None, Some(p), _) => Some(std::fs::read(p).map_err(|e| format!("{}: {e}", p.display()))?),
        (None, None, Some(n)) => Some(g.read(&format!("PLAYER{n}.U1")).map_err(e)?),
        _ => None,
    };
    let mut rec = match (save_bytes, reload) {
        (_, Some(r)) => r,
        (Some(b), None) => PlayerRecord::parse(&b).map_err(e)?,
        (None, None) => Game::default_record("Hero", DEFAULT_POS.0, DEFAULT_POS.1),
    };
    if let Some((x, y)) = a.pos.filter(|_| !reloaded) {
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
    io.log = a.log_shot;
    let scale = 2; // the size of a 640x400 DOSBox capture
    let mode = video_mode(a)?;
    let pack = load_pack(a)?;
    // --crt: the filter on the frame at its own size (as in the window).
    let finish = |w: usize, h: usize, rgb: Vec<u8>| -> Result<(), String> {
        if a.crt {
            let k = qol::crt_factor(w);
            write_png(out, (w * k) as u32, (h * k) as u32, &qol::crt(&rgb, w, h, k))
        } else {
            write_png(out, w as u32, h as u32, &rgb)
        }
    };
    let shot = |t: &Term, pal: &[Rgb; 16]| -> Result<(), String> {
        let (w, h, rgb) = frame_rgb(t, pal, pack.as_ref(), if a.crt { 1 } else { scale }, 0);
        finish(w, h, rgb)
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
    let (mut g, palette) = load(a, mode, chosen.as_deref(), None)?;
    let palette = &palette;
    let _ = g.run(&mut io);
    for d in &io.done {
        println!("slots: {d}");
    }
    if let Some(r) = g.reload.take() {
        // A loaded slot: the game as the window would start it again.
        let (g2, _) = load(a, g.a.video, chosen.as_deref(), Some(r))?;
        g = g2;
    }
    let window = if a.options_shot {
        let avail = available(a);
        let mut c = g.term.clone();
        let q = Qol { crt: a.crt, fullscreen: a.fullscreen, ..Default::default() };
        OptionsMenu::new(VideoChoice::from_parts(a.enhanced, mode), a.sound, avail).with_qol(q).draw(&mut c);
        Some(c)
    } else if a.slots_shot {
        let mut c = g.term.clone();
        SlotScreen::new(g.save_to.as_deref().unwrap_or(Path::new("PLAYER1.U1"))).draw(&mut c);
        Some(c)
    } else if a.log_shot {
        let mut c = g.term.clone();
        LogView::new(&g.term).draw(&mut c);
        Some(c)
    } else {
        None
    };
    if let Some(c) = window {
        // As in the window: the art stays on what the box leaves uncovered.
        match &pack {
            Some(p) => {
                let rgb = enhanced::compose_boxed(&g.term, &c, palette, p, 0, PANEL);
                finish(W * p.scale(), H * p.scale(), rgb)?;
            }
            None => shot(&c, palette)?,
        }
    } else {
        shot(&g.term, palette)?;
    }
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
    /// F1 (F2, PgUp) was pressed: that window opens at the next screen
    /// update.
    modal_wanted: Option<Modal>,
    /// Keys go to the open window (`menu_keys`), not the game.
    in_menu: bool,
    menu_keys: std::collections::VecDeque<u8>,
    cfg_path: Option<PathBuf>,
    /// On the title or character screens (until the game starts).
    front: bool,
    /// A display was chosen there: they stop and start again in it.
    restart: bool,
    /// The optional engine features (SPEC §22).
    qol: Qol,
    /// In space flight (for the fast space setting).
    in_space: bool,
    /// The CRT filter's larger texture (its size, and the texture).
    crt_tex: Option<(usize, usize, sdl2::render::Texture<'static>)>,
    /// The character's save file (slot 1), once the game runs.
    save_base: Option<PathBuf>,
    /// A slot action for the game to take at its next key poll, and how
    /// many polls it has waited.
    save_req: Option<(SaveReq, u8)>,
    /// A short note over the top of the map (slot results, F4).
    toast: Option<(String, Instant)>,
}

/// The engine's own windows, run with the game paused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Modal {
    Options,
    Slots,
    Log,
}

/// How long a note shows, and where.
const TOAST: Duration = Duration::from_millis(2500);
const TOAST_BOX: (usize, usize, usize, usize) = (8, 8, 311, 15);

impl SdlIo {
    fn present(&mut self, t: &Term) -> IoResult<()> {
        match self.toast.as_ref().filter(|(_, at)| at.elapsed() < TOAST) {
            Some((msg, _)) => {
                let mut c = t.clone();
                c.fill(TOAST_BOX.0, TOAST_BOX.1, TOAST_BOX.2, TOAST_BOX.3, u1_assets::term::BLACK);
                opts::center(&mut c, 1, &msg.clone(), u1_assets::term::WARN);
                self.present_over(t, Some((&c, TOAST_BOX)))
            }
            None => {
                self.toast = None;
                self.present_over(t, None)
            }
        }
    }

    /// The screen, or (with `over`) the screen with another picture's
    /// rectangle over it (a window, a note); the enhanced art stays on
    /// what the rectangle leaves uncovered. Then the CRT filter, if on.
    fn present_over(&mut self, t: &Term, over: Option<(&Term, (usize, usize, usize, usize))>) -> IoResult<()> {
        let ms = self.start.elapsed().as_millis() as u64;
        let pack = if self.enhanced { self.pack.as_ref() } else { None };
        let (w, h, rgb) = match (pack, over) {
            (Some(p), Some((m, r))) => (W * p.scale(), H * p.scale(), enhanced::compose_boxed(t, m, &self.palette, p, ms, r)),
            (_, o) => frame_rgb(o.map_or(t, |o| o.0), &self.palette, pack, 1, ms),
        };
        if self.qol.crt {
            let k = qol::crt_factor(w);
            let (cw, ch) = (w * k, h * k);
            if self.crt_tex.as_ref().map_or(true, |(x, y, _)| (*x, *y) != (cw, ch)) {
                let tex = self.tc.create_texture_streaming(sdl2::pixels::PixelFormatEnum::RGB24, cw as u32, ch as u32).map_err(|_| Halt)?;
                self.crt_tex = Some((cw, ch, tex));
            }
            let out = qol::crt(&rgb, w, h, k);
            let (_, _, tex) = self.crt_tex.as_mut().ok_or(Halt)?;
            tex.update(None, &out, cw * 3).map_err(|_| Halt)?;
            self.canvas.copy(tex, None, None).map_err(|_| Halt)?;
        } else {
            self.tex.update(None, &rgb, w * 3).map_err(|_| Halt)?;
            self.canvas.copy(&self.tex, None, None).map_err(|_| Halt)?;
        }
        self.canvas.present();
        Ok(())
    }

    /// `d` under the speed settings.
    fn scaled(&self, d: Duration, w: Wait) -> Duration {
        d.mul_f64(self.qol.wait_factor(w, self.in_space))
    }

    fn note(&mut self, msg: &str) {
        self.toast = Some((msg.to_string(), Instant::now()));
    }

    /// Open one of the engine's windows (a second press closes it).
    fn open(&mut self, m: Modal) {
        if self.in_menu {
            self.menu_keys.push_back(keys::ESC);
        } else {
            self.modal_wanted = Some(m);
            self.in_menu = true;
        }
    }

    /// A slot action for the game's next key poll (only the main loop's
    /// poll takes it).
    fn request(&mut self, r: SaveReq) {
        if self.front || self.save_base.is_none() {
            self.note("No game to save yet");
            return;
        }
        self.save_req = Some((r, 0));
    }

    /// A request the game did not take within two polls (in a prompt,
    /// space or a scene) is dropped.
    fn age_request(&mut self) {
        if let Some((_, age)) = &mut self.save_req {
            *age += 1;
            if *age > 2 {
                self.save_req = None;
                self.note("Not now (only at the command prompt)");
            }
        }
    }

    fn pump(&mut self) -> IoResult<()> {
        use sdl2::event::Event;
        use sdl2::keyboard::{Keycode, Mod};
        let evs: Vec<Event> = self.events.poll_iter().collect();
        for ev in evs {
            match ev {
                Event::Quit { .. } => return Err(Halt),
                Event::KeyDown { keycode: Some(k), keymod, .. } => {
                    let shift = keymod.intersects(Mod::LSHIFTMOD | Mod::RSHIFTMOD | Mod::CAPSMOD);
                    let alt = keymod.intersects(Mod::LALTMOD | Mod::RALTMOD);
                    let c = match k {
                        // Fullscreen: never passed to the game.
                        Keycode::Return | Keycode::KpEnter if alt => {
                            let q = Qol { fullscreen: !self.qol.fullscreen, ..self.qol };
                            self.apply_qol(q);
                            self.save_config(false);
                            None
                        }
                        Keycode::Left | Keycode::Kp4 => Some(keys::LEFT),
                        Keycode::Right | Keycode::Kp6 => Some(keys::RIGHT),
                        Keycode::Up | Keycode::Kp8 => Some(keys::UP),
                        Keycode::Down | Keycode::Kp2 => Some(keys::DOWN),
                        Keycode::Return | Keycode::KpEnter => Some(keys::ENTER),
                        Keycode::Escape => Some(keys::ESC),
                        Keycode::Backspace => Some(keys::BACKSPACE),
                        // The engine's own keys: none of them the original's.
                        Keycode::F1 => {
                            self.open(Modal::Options);
                            None
                        }
                        Keycode::F2 if self.qol.slots && (self.in_menu || !self.front) => {
                            self.open(Modal::Slots);
                            None
                        }
                        Keycode::F5 if self.qol.slots && !self.in_menu => {
                            self.request(SaveReq::Save(Slot::Quick));
                            None
                        }
                        Keycode::F9 if self.qol.slots && !self.in_menu => {
                            self.request(SaveReq::Load(Slot::Quick));
                            None
                        }
                        Keycode::F4 if !self.in_menu => {
                            let q = Qol { turbo: !self.qol.turbo, ..self.qol };
                            self.apply_qol(q);
                            self.note(if q.turbo { "Turbo on (F4)" } else { "Turbo off (F4)" });
                            self.save_config(false);
                            None
                        }
                        Keycode::PageUp if self.in_menu => Some(msglog::keys::PGUP),
                        Keycode::PageDown if self.in_menu => Some(msglog::keys::PGDN),
                        Keycode::Home if self.in_menu => Some(msglog::keys::HOME),
                        Keycode::End if self.in_menu => Some(msglog::keys::END),
                        Keycode::PageUp if self.qol.log && !self.front => {
                            self.open(Modal::Log);
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
                        // Keys typed while a window is up are the window's.
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

    /// Wait `d`, with the windows and restarts handled (time in a window
    /// does not count).
    fn wait(&mut self, t: &Term, d: Duration) -> IoResult<()> {
        self.present(t)?;
        let mut end = Instant::now() + d;
        while Instant::now() < end {
            self.pump()?;
            end += self.menu(t)?;
            self.restarting()?;
            std::thread::sleep(Duration::from_millis(2).min(d));
        }
        Ok(())
    }

    /// Hold the game for a speaker effect of `len` (the original's CPU
    /// was busy playing it), unless the speed settings say otherwise.
    fn hold(&mut self, len: Duration) {
        let end = Instant::now() + self.scaled(len, Wait::Sound);
        while Instant::now() < end {
            let _ = self.pump();
            std::thread::sleep(Duration::from_millis(1));
        }
    }
}

impl Io for SdlIo {
    fn poll(&mut self, t: &Term) -> IoResult<Option<u8>> {
        self.age_request();
        self.present(t)?;
        loop {
            self.pump()?;
            self.menu(t)?;
            self.restarting()?;
            let poll = self.scaled(POLL, Wait::Poll);
            if let Some(k) = self.pending.pop_front() {
                self.next = Instant::now() + poll;
                return Ok(Some(k));
            }
            let now = Instant::now();
            if now >= self.next {
                self.next = (self.next + poll).max(now);
                return Ok(None);
            }
            if self.toast.is_some() {
                self.present(t)?;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    fn pause(&mut self, t: &Term, ticks: u32) -> IoResult<()> {
        self.wait(t, self.scaled(TICK * ticks, Wait::Poll))
    }

    fn message_pause(&mut self, t: &Term, ticks: u32) -> IoResult<()> {
        self.wait(t, self.scaled(TICK * ticks, Wait::Message))
    }

    fn poll_now(&mut self, t: &Term) -> IoResult<Option<u8>> {
        self.age_request();
        self.present(t)?;
        self.pump()?;
        self.menu(t)?;
        self.restarting()?;
        Ok(self.pending.pop_front())
    }

    fn retrace(&mut self, t: &Term, n: u32) -> IoResult<()> {
        self.wait(t, self.scaled(Duration::from_micros(14_286) * n, Wait::Poll))
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
        self.hold(len);
    }

    fn sound_distant(&mut self, n: u8) {
        if self.sound == SoundMode::Enhanced {
            // Unheard, but the original's time passes all the same.
            self.hold(Duration::from_secs_f64(sound::effect(n).millis() / 1000.0));
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

    fn display_restart(&mut self) -> Option<VideoMode> {
        if !std::mem::take(&mut self.restart) {
            return None;
        }
        self.pending.clear();
        let m = self.wanted;
        self.video_shown(m);
        Some(m)
    }

    fn video_shown(&mut self, mode: VideoMode) {
        self.shown = mode;
        self.palette = palette_for(mode);
    }

    fn wait_or_key(&mut self, t: &Term) -> IoResult<()> {
        // About 1.5 s (12 polls; fewer with a shorter message delay), or
        // until a key (which is swallowed).
        let n = (12 * u32::from(self.qol.message_delay.min(100)) + 50) / 100;
        for _ in 0..n {
            if self.poll(t)?.is_some() {
                break;
            }
        }
        Ok(())
    }

    fn in_space(&mut self, on: bool) {
        self.in_space = on;
    }

    fn save_request(&mut self) -> Option<SaveReq> {
        self.save_req.take().map(|(r, _)| r)
    }

    fn save_done(&mut self, msg: &str) {
        self.note(msg);
    }

    fn wants_log(&self) -> bool {
        self.qol.log
    }
}

impl SdlIo {
    /// The window F1, F2 or PgUp asked for, if any: drawn over a copy of
    /// `t` and run with the game paused until it closes. Returns how long
    /// it was open (timed waits resume where they were).
    fn menu(&mut self, t: &Term) -> IoResult<Duration> {
        let Some(which) = self.modal_wanted.take() else { return Ok(Duration::ZERO) };
        let opened = Instant::now();
        let r = match which {
            Modal::Options => {
                let video = VideoChoice::from_parts(self.enhanced, self.wanted);
                let mut m = OptionsMenu::new(video, self.sound, self.avail).with_qol(self.qol);
                self.run_modal(t, &mut m, |m, c| m.draw(c), |io, m, k| match m.key(k) {
                    MenuKey::Changed => {
                        io.choose(m.video, m.sound);
                        io.apply_qol(m.qol);
                        io.save_config(true);
                        true
                    }
                    MenuKey::Close => false,
                    MenuKey::Stay => true,
                })
            }
            Modal::Slots => {
                let base = self.save_base.clone().unwrap_or_else(|| PathBuf::from("PLAYER1.U1"));
                let mut s = SlotScreen::new(&base);
                self.run_modal(t, &mut s, |s, c| s.draw(c), |io, s, k| match s.key(k) {
                    SlotKey::Do(r) => {
                        io.request(r);
                        false
                    }
                    SlotKey::Close => false,
                    SlotKey::Stay => true,
                })
            }
            Modal::Log => {
                let mut v = LogView::new(t);
                self.run_modal(t, &mut v, |v, c| v.draw(c), |_, v, k| v.key(k))
            }
        };
        self.in_menu = false;
        self.menu_keys.clear();
        r?;
        let paused = opened.elapsed();
        self.next = Instant::now() + POLL;
        self.present(t)?;
        Ok(paused)
    }

    /// Run one window until `key` says it closes (false).
    fn run_modal<S>(&mut self, t: &Term, st: &mut S, draw: fn(&S, &mut Term), key: fn(&mut SdlIo, &mut S, u8) -> bool) -> IoResult<()> {
        loop {
            let mut c = t.clone();
            draw(st, &mut c);
            self.present_over(t, Some((&c, PANEL)))?;
            self.pump()?;
            while let Some(k) = self.menu_keys.pop_front() {
                if !key(self, st, k) {
                    return Ok(());
                }
            }
            std::thread::sleep(Duration::from_millis(15));
        }
    }

    /// The title or character program stops here for a display chosen on
    /// its screen ([`Io::display_restart`]).
    fn restarting(&self) -> IoResult<()> {
        if self.restart { Err(Halt) } else { Ok(()) }
    }

    /// Apply the menu's choice now (the display's pictures at the game's
    /// next key poll; the title and character programs start again in it
    /// when the menu closes).
    fn choose(&mut self, video: VideoChoice, sound: SoundMode) {
        self.sound = sound;
        self.wanted = video.mode();
        self.restart = self.front && self.wanted != self.shown;
        if video.enhanced() != self.enhanced {
            self.set_enhanced(video.enhanced());
        }
    }

    /// Apply the optional features (the window's at once, the rest as
    /// they are next used).
    fn apply_qol(&mut self, q: Qol) {
        let old = std::mem::replace(&mut self.qol, q);
        if old.fullscreen != q.fullscreen || old.scaling != q.scaling {
            self.apply_window();
        }
        if !q.crt {
            self.crt_tex = None;
        }
        if !q.slots {
            self.save_req = None;
        }
    }

    fn apply_window(&mut self) {
        use sdl2::video::FullscreenType;
        let _ = self.canvas.set_integer_scale(self.qol.scaling == Scaling::Whole);
        let want = if self.qol.fullscreen { FullscreenType::Desktop } else { FullscreenType::Off };
        if self.canvas.window().fullscreen_state() != want {
            if let Err(e) = self.canvas.window_mut().set_fullscreen(want) {
                eprintln!("fullscreen: {e}");
            }
        }
    }

    /// Save the choices (`all`: the display and sound as well as the
    /// optional features; the keys F4 and Alt+Enter save only those).
    fn save_config(&self, all: bool) {
        let Some(p) = &self.cfg_path else { return };
        // The file's other lines are kept; one that cannot be read is left
        // alone.
        let mut c = Config::load(p);
        if p.exists() && std::fs::read_to_string(p).is_err() {
            eprintln!("{}: not saved (the file cannot be read)", p.display());
            return;
        }
        if all {
            c.video = Some(VideoChoice::from_parts(self.enhanced, self.wanted));
            c.sound = Some(self.sound);
        }
        c.qol = self.qol;
        c.warnings.clear();
        if let Err(e) = c.save(p) {
            eprintln!("{}: {e}", p.display());
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
        modal_wanted: None,
        in_menu: false,
        menu_keys: Default::default(),
        cfg_path,
        front: true,
        restart: false,
        qol: Qol::default(),
        in_space: false,
        crt_tex: None,
        save_base: None,
        save_req: None,
        toast: None,
    };
    // The optional features: the file's, and --crt / --fullscreen.
    let mut q = cfg.qol;
    q.crt |= a.crt;
    q.fullscreen |= a.fullscreen;
    io.apply_qol(q);
    if q != Qol::default() {
        // apply_qol only touches the window for a change.
        io.apply_window();
    }
    let chosen = match front(a, mode, &mut io)? {
        Front::Halted(_) => return Ok(()),
        Front::Play(p) => Some(p),
        Front::Skip => None,
    };
    // A display chosen during the title or the character menu starts the
    // game.
    io.front = false;
    io.restart = false;
    let mut reload = None;
    loop {
        let mode = io.wanted;
        io.video_shown(mode);
        let (mut g, _) = load(a, mode, chosen.as_deref(), reload.take())?;
        io.save_base = g.save_to.clone();
        io.in_space = false;
        let _ = g.run(&mut io);
        // A save slot was loaded: the game starts again from it.
        if let Some(r) = g.reload.take() {
            reload = Some(r);
            continue;
        }
        if let Some(s) = g.stopped {
            eprintln!("stopped: {s}");
        }
        return Ok(());
    }
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
