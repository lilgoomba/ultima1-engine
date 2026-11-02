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

use std::sync::Arc;
use u1_assets::config::{self, Config, VideoChoice};
use u1_assets::enhanced::{self, Pack};
use u1_assets::exe::Exe;
use u1_assets::explore::Explore;
use u1_assets::game::{Assets, Game, VideoSet};
use u1_assets::gamedir::{self, GameDir, VideoMode};
use u1_assets::gen::GenEnd;
use u1_assets::input::{PadAction, PadEvent, PadInput, Repeater};
use u1_assets::io::{keys, Halt, Io, IoResult, ScriptIo};
use u1_assets::keymap::{KeyMap, RebindKey, RebindScreen};
use u1_assets::locations::LocationTable;
use u1_assets::msglog::{self, LogView};
use u1_assets::music::{self, Track};
use u1_assets::options::{self as opts, MenuKey, OptionsMenu, PANEL};
use u1_assets::palette::{cga_as_16, Rgb, CGA_PALETTE_1_LOW, EGA_PALETTE};
use u1_assets::qol::{self, Qol, Wait};
use u1_assets::rules::ExeTables;
use u1_assets::save::PlayerRecord;
use u1_assets::slots::{SaveReq, Slot, SlotKey, SlotScreen};
use u1_assets::sound::{self, Sfx};
use u1_assets::soundpack::{SoundMode, SoundPack};
use u1_assets::tcd::TownCastleMaps;
use u1_assets::term::{Term, H, W};
use u1_assets::touch;

/// A picture and the rectangle of it drawn over the screen.
type Overlay<'a> = (&'a Term, (usize, usize, usize, usize));

/// One keyboard/animation poll of the original main loop, measured in
/// DOSBox (130 ms at 20000 cycles).
const POLL: Duration = Duration::from_micros(130_000);
/// One tick of the PC timer.
const TICK: Duration = Duration::from_micros(54_925);
/// Start of a new character (verified from GEN.EXE).
const DEFAULT_POS: (u16, u16) = (49, 40);

const USAGE: &str = "usage: u1 [--game-dir <path>] [--player N | --save FILE | --pos X,Y]
          [--mode ega|t1k|cga | C] [--font FILE] [--intro | --gen] [--roster DIR] [--scale N] [--save-to FILE]
          [--video original|enhanced] [--sound original|enhanced|mute] [--pack DIR]
          [--fullscreen] [--crt] [--smoothing off|linear|pixel-art]
          [--screenshot out.png [--keys TEXT] [--counter N] [--options | --slots | --log] [--crt]]

Without --game-dir the game files are looked for in $U1_GAME_DIR, the
folder holding the AppImage (or this program), the current folder (each
also in an ULTIMA1 folder inside it) and ~/.local/share/ultima1-engine/game
($XDG_DATA_HOME if set). With no start option (--player, --save, --pos,
--intro, --gen) the game starts with the title sequence, as the original.
Without --save-to and --roster, characters and saves are kept in
~/.local/share/ultima1-engine/saves, never in the game folder.

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
windows (--keys may hold {saveN} / {loadN}, N 1-5 or q).
More optional features (SPEC §23), also off by default: a dungeon automap
and a world mini-map (Tab), a character screen (Ztats, Z, shows it), a command help screen
(F3), short hints at command prompts, a confirmation before selling or
dropping things worth a set amount, extra keys for the commands (F1, Keys),
a gamepad and the engine's own arrow repeat. --features LIST turns some on
for one run (automap, minimap, character, help, hints, confirm=N, repeat,
gamepad); in screenshots --keys may hold {map} {help}, and --rebind
draws the key window.";

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
    music_demo: Option<PathBuf>,
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
    /// `--features`: the §23 features on for this run.
    features: Qol,
    /// Screenshots: the key rebinding window.
    rebind_shot: bool,
    /// Screenshots: the options menu's chosen row.
    menu_row: usize,
}

fn parse_args(args: Vec<String>) -> Result<Args, String> {
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
        music_demo: None,
        enhanced: false,
        pack: None,
        cli: config::Cli::default(),
        options_shot: false,
        slots_shot: false,
        log_shot: false,
        crt: false,
        fullscreen: false,
        features: Qol::default(),
        rebind_shot: false,
        menu_row: 0,
    };
    let mut have_dir = false;
    let mut it = args.into_iter().skip(1);
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
            "--music-demo" => {
                a.music_demo = Some(v()?.into());
                have_dir = true;
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
            "--smoothing" => a.features.smoothing = qol::Smoothing::parse(&v()?).ok_or("--smoothing wants off, linear or pixel-art")?,
            "--fullscreen" => a.fullscreen = true,
            "--features" => features(&mut a.features, &v()?)?,
            "--rebind" => a.rebind_shot = true,
            "--menu-row" => a.menu_row = v()?.parse().map_err(|_| "bad --menu-row")?,
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
    let interactive = a.screenshot.is_none() && a.sound_demo.is_none() && a.music_demo.is_none();
    let start_given = a.player.is_some() || a.save.is_some() || a.pos.is_some() || a.intro || a.gen;
    if interactive && !start_given {
        a.intro = true;
    }
    if a.sound_demo.is_none() && a.music_demo.is_none() {
        let front = a.intro || a.gen;
        a.game_dir = if have_dir { gamedir::locate(std::slice::from_ref(&a.game_dir), front)? } else { gamedir::locate(&game_dir_candidates(), front)? };
    }
    if interactive && a.save_to.is_none() && a.roster.is_none() {
        if let Some(d) = gamedir::data_dir().map(|d| d.join("saves")) {
            let new = !d.is_dir();
            std::fs::create_dir_all(&d).map_err(|e| format!("{}: {e}", d.display()))?;
            // The first time, the characters that came with the game join
            // the roster as copies (the originals are only read).
            if new {
                if let Ok(g) = GameDir::new(&a.game_dir) {
                    for n in 1..=4 {
                        let f = format!("PLAYER{n}.U1");
                        if let Some(src) = g.find(&f) {
                            if let Ok(b) = std::fs::read(src) {
                                let _ = std::fs::write(d.join(&f), b);
                            }
                        }
                    }
                }
            }
            a.save_to = Some(d.join("PLAYER1.U1"));
        }
    }
    a.scale = a.scale.clamp(1, 6);
    if a.pack.is_some() && !a.enhanced && a.sound != SoundMode::Enhanced && a.sound_demo.is_none() {
        return Err("--pack needs --video enhanced or --sound enhanced".into());
    }
    Ok(a)
}

/// Where to look for the game files without `--game-dir`, in order.
fn game_dir_candidates() -> Vec<PathBuf> {
    let mut v = Vec::new();
    if let Some(d) = std::env::var_os("U1_GAME_DIR").filter(|d| !d.is_empty()) {
        v.push(PathBuf::from(d));
    }
    // The AppImage's own path (the program inside runs from a temporary mount).
    if let Some(d) = std::env::var_os("APPIMAGE").map(PathBuf::from).and_then(|p| p.parent().map(Path::to_path_buf)) {
        v.push(d);
    }
    if std::env::var_os("APPDIR").is_none() {
        if let Some(d) = std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf)) {
            v.push(d);
        }
    }
    if let Ok(d) = std::env::current_dir() {
        v.push(d);
    }
    if let Some(d) = gamedir::data_dir() {
        v.push(d.join("game"));
    }
    v
}

/// `--features LIST`: turn the named §23 features on in `q`.
fn features(q: &mut Qol, list: &str) -> Result<(), String> {
    for f in list.split(',').map(str::trim).filter(|f| !f.is_empty()) {
        match f {
            "automap" => q.automap = true,
            "minimap" => q.minimap = true,
            "character" => q.charsheet = true,
            "help" => q.help = true,
            "hints" => q.hints = true,
            "repeat" => q.repeat = true,
            "gamepad" => q.gamepad = true,
            "all" => {
                let c = q.confirm;
                *q = Qol { automap: true, minimap: true, charsheet: true, help: true, hints: true, ..*q };
                q.confirm = c;
            }
            c if c.starts_with("confirm=") => q.confirm = c[8..].parse().map_err(|_| "--features confirm=N wants a number")?,
            other => return Err(format!("unknown feature {other} (automap, minimap, character, help, hints, confirm=N, repeat, gamepad, all)")),
        }
    }
    Ok(())
}

/// The enhanced mode's pack: `--pack`, else `assets/enhanced` in the
/// current directory, the installed pack or the source tree; with none, no art (everything
/// is the original, scaled up).
fn pack_dir(a: &Args) -> Option<PathBuf> {
    match &a.pack {
        Some(d) => Some(d.clone()),
        None => {
            let mut c = vec![PathBuf::from("assets/enhanced")];
            // Installed (the AppImage): <prefix>/share/ultima1-engine/enhanced
            // beside <prefix>/bin.
            if let Some(dir) = std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf)) {
                c.push(dir.join("../share/ultima1-engine/enhanced"));
            }
            if let Some(d) = std::env::var_os("APPDIR") {
                c.push(Path::new(&d).join("usr/share/ultima1-engine/enhanced"));
            }
            c.push(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/enhanced"));
            c.into_iter().find(|d| d.join("manifest.toml").is_file())
        }
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

/// `--music-demo`: the first 20 seconds of each music track (the stings
/// whole), one after another, as a WAV file.
fn music_demo(out: &Path) -> Result<(), String> {
    let rate = 44_100;
    let mut wav = Vec::new();
    for t in Track::ALL {
        let s = music::render(t, rate);
        let n = if t.loops() { (20 * rate as usize).min(s.len()) } else { s.len() };
        let fade = (rate as usize).min(n);
        wav.extend(s[..n].iter().enumerate().map(|(i, &x)| if t.loops() && i + fade > n { (i32::from(x) * (n - i) as i32 / fade as i32) as i16 } else { x }));
        wav.extend(std::iter::repeat(0i16).take(rate as usize / 2));
        println!("{}", t.name());
    }
    std::fs::write(out, u1_assets::soundpack::encode_wav(rate, &wav)).map_err(|e| e.to_string())
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
    if mode == VideoMode::Cga {
        cga_as_16(&CGA_PALETTE_1_LOW)
    } else {
        EGA_PALETTE
    }
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
    let mut clock = || -> Box<dyn FnMut() -> [u8; 4]> {
        if fixed {
            Box::new(|| [0, 0, 0, 0])
        } else {
            Box::new(local_time)
        }
    };
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
    // The explored map beside the save (SPEC §23.1), if there is one.
    game.explore = Explore::load(&save_to);
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
    io.qol = a.features;
    let scale = 2; // the size of a 640x400 DOSBox capture
    let mode = video_mode(a)?;
    let pack = load_pack(a)?;
    // --crt: the filter on the frame at its own size (as in the window).
    let finish = |w: usize, h: usize, rgb: Vec<u8>| -> Result<(), String> {
        let (w, h, rgb) = if a.features.smoothing == qol::Smoothing::PixelArt { qol::pixel_art(&rgb, w, h) } else { (w, h, rgb) };
        if a.crt {
            let k = qol::crt_factor(w);
            write_png(out, (w * k) as u32, (h * k) as u32, &qol::crt(&rgb, w, h, k))
        } else {
            write_png(out, w as u32, h as u32, &rgb)
        }
    };
    let shot = |t: &Term, pal: &[Rgb; 16]| -> Result<(), String> {
        let pixel_art = a.features.smoothing == qol::Smoothing::PixelArt;
        let (w, h, rgb) = frame_rgb(t, pal, pack.as_ref(), if a.crt || pixel_art { 1 } else { scale }, 0);
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
        let carried = g.reload_explore.take();
        let (g2, _) = load(a, g.a.video, chosen.as_deref(), Some(r))?;
        g = g2;
        if let Some(e) = carried {
            g.explore = e;
        }
    }
    let window = if a.options_shot {
        let avail = available(a);
        let mut c = g.term.clone();
        let q = Qol { crt: a.crt, fullscreen: a.fullscreen, ..a.features };
        let mut m = OptionsMenu::new(VideoChoice::from_parts(a.enhanced, mode), a.sound, avail).with_qol(q);
        m.row = a.menu_row.min(opts::ROWS.len() - 1);
        m.draw(&mut c);
        Some(c)
    } else if a.rebind_shot {
        let mut c = g.term.clone();
        let mut r = RebindScreen::new(KeyMap::default());
        r.key(keys::DOWN, "down");
        r.key(keys::ENTER, "return");
        r.key(0, "j");
        r.draw(&mut c);
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
/// The on-screen controls' state (u1_assets::touch).
struct TouchUi {
    o: touch::Overlay,
    dpi: f32,
    /// The buttons as last laid out (for the window's size then).
    buttons: Vec<touch::Button>,
    /// Which button each finger (or the mouse, -1) pressed.
    down: Vec<(i64, touch::Act)>,
    text: sdl2::keyboard::TextInputUtil,
}

/// The id the arrow repeat knows a finger by (keys use their keycodes,
/// which are all far from these).
fn finger_tag(f: i64) -> i32 {
    -1000 - (f & 0xFFFF) as i32
}

struct SdlIo {
    canvas: sdl2::render::WindowCanvas,
    /// The touch controls (on Android, or with U1_TOUCH=1).
    touch: Option<TouchUi>,
    /// "Import again" was chosen: end, for the import screen.
    reimport: bool,
    tex: sdl2::render::Texture<'static>,
    events: sdl2::EventPump,
    palette: [Rgb; 16],
    next: Instant,
    /// Keys for the game: what each types anywhere, and what it means at
    /// the command prompt if that differs (an extra key, SPEC §23.4).
    pending: std::collections::VecDeque<(Option<u8>, Option<u8>)>,
    /// The game is at its command prompt (`Io::command_prompt`).
    at_prompt: bool,
    /// The key of the engine screen that is up, which closes it.
    view_key: Option<u8>,
    /// The extra keys and the gamepad's buttons.
    keymap: KeyMap,
    pad: PadInput,
    /// The engine's arrow repeat (SPEC §23.3).
    held: Repeater,
    /// The gamepad subsystem and the pads open.
    gc: Option<sdl2::GameControllerSubsystem>,
    pads: Vec<sdl2::controller::GameController>,
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
    /// The window's keys: the game's code (0 for none) and SDL's name.
    menu_keys: std::collections::VecDeque<(u8, String)>,
    cfg_path: Option<PathBuf>,
    /// On the title or character screens (until the game starts).
    front: bool,
    /// A display was chosen there: they stop and start again in it.
    restart: bool,
    /// The optional engine features (SPEC §22).
    qol: Qol,
    /// In space flight (for the fast space setting).
    in_space: bool,
    /// The area the game is in (its music, when music is on).
    area: Option<Track>,
    /// The tracks rendered so far (by a background thread).
    tracks: Arc<std::sync::Mutex<std::collections::HashMap<Track, Arc<Vec<i16>>>>>,
    rendering: bool,
    /// The CRT filter's larger texture (its size, and the texture).
    crt_tex: Option<(usize, usize, sdl2::render::Texture<'static>)>,
    /// The last frame's internal size and whether it went through the CRT
    /// texture, for [`SdlIo::redraw`].
    last_frame: Option<(u32, u32, bool)>,
    /// The character's save file (slot 1), once the game runs.
    save_base: Option<PathBuf>,
    /// A slot action for the game to take at its next key poll, and how
    /// many polls it has waited.
    save_req: Option<(SaveReq, u8)>,
    /// A short note over the top of the map (slot results, F4).
    toast: Option<(String, Instant)>,
    /// Tap actions (SPEC §24): what the game says a tap would find now,
    /// keys a tap still has to give (dropped at the next prompt), a key
    /// for arriving on a thing, the press under way (finger, picture
    /// point, since, auto-walking), the highlight, and where the picture
    /// was last drawn.
    scene: u1_assets::tap::Scene,
    tap_queue: std::collections::VecDeque<u8>,
    tap_then: Option<(u1_assets::tap::Thing, u8)>,
    tap_press: Option<(i64, i32, i32, Instant, bool)>,
    tap_flash: Option<((i32, i32, i32, i32), Instant)>,
    pic: sdl2::rect::Rect,
}

/// The engine's own windows, run with the game paused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Modal {
    Options,
    Slots,
    Log,
    Rebind,
}

/// How long a note shows, and where.
const TOAST: Duration = Duration::from_millis(2500);
/// Tap actions: how long the tapped cell is outlined, and a long press.
const TAP_FLASH: Duration = Duration::from_millis(250);
const LONG_PRESS: Duration = Duration::from_millis(500);
const TOAST_BOX: (usize, usize, usize, usize) = (8, 8, 311, 15);

impl SdlIo {
    /// Bring the music in line with the area and the settings: start the
    /// renderer the first time it is needed, then play, crossfade or stop.
    fn sync_music(&mut self) {
        let Some((dev, rate)) = &mut self.audio else { return };
        let want = self.area.filter(|_| self.qol.music && self.sound != SoundMode::Mute);
        let vol = f32::from(self.qol.music_volume) / 100.0;
        let have = dev.lock().music.current();
        if want.is_some() && !self.rendering {
            self.rendering = true;
            let (store, rate) = (self.tracks.clone(), *rate);
            // The current area first, then the rest.
            let first = want.unwrap_or(Track::Title);
            std::thread::spawn(move || {
                for t in std::iter::once(first).chain(Track::ALL.into_iter().filter(|&t| t != first)) {
                    let s = Arc::new(music::render(t, rate));
                    if let Ok(mut m) = store.lock() {
                        m.insert(t, s);
                    }
                }
            });
        }
        let mut m = dev.lock();
        m.music.volume = vol;
        match want {
            None if have.is_some() => m.music.stop(),
            Some(t) if have != Some(t) => {
                if let Some(s) = self.tracks.lock().ok().and_then(|c| c.get(&t).cloned()) {
                    m.music.start(t, s);
                }
            }
            _ => {}
        }
    }

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
    fn present_over(&mut self, t: &Term, over: Option<Overlay>) -> IoResult<()> {
        let ms = self.start.elapsed().as_millis() as u64;
        let pack = if self.enhanced { self.pack.as_ref() } else { None };
        let (w, h, rgb) = match (pack, over) {
            (Some(p), Some((m, r))) => (W * p.scale(), H * p.scale(), enhanced::compose_boxed(t, m, &self.palette, p, ms, r)),
            (_, o) => frame_rgb(o.map_or(t, |o| o.0), &self.palette, pack, 1, ms),
        };
        // Pixel-art smoothing enlarges the frame first, then the CRT
        // filter works on the result; either goes through its own texture.
        // The window fits the original's size (whole multiples of it).
        let pixel_art = self.qol.smoothing == qol::Smoothing::PixelArt;
        let post = self.qol.crt || pixel_art;
        if post {
            let (sw, sh, srgb) = if pixel_art { qol::pixel_art(&rgb, w, h) } else { (w, h, rgb) };
            let k = if self.qol.crt { qol::crt_factor(sw) } else { 1 };
            let (cw, ch) = (sw * k, sh * k);
            if self.crt_tex.as_ref().map_or(true, |(x, y, _)| (*x, *y) != (cw, ch)) {
                self.scale_hint();
                let tex = self.tc.create_texture_streaming(sdl2::pixels::PixelFormatEnum::RGB24, cw as u32, ch as u32).map_err(|_| Halt)?;
                self.crt_tex = Some((cw, ch, tex));
            }
            let out = if self.qol.crt { qol::crt(&srgb, sw, sh, k) } else { srgb };
            let (_, _, tex) = self.crt_tex.as_mut().ok_or(Halt)?;
            tex.update(None, &out, cw * 3).map_err(|_| Halt)?;
        } else {
            self.tex.update(None, &rgb, w * 3).map_err(|_| Halt)?;
        }
        self.last_frame = Some((w as u32, h as u32, post));
        self.redraw();
        Ok(())
    }

    /// How SDL stretches the textures made after this: smoothly with any
    /// smoothing (the pixel-art scaler's remaining stretch is small),
    /// square pixels without.
    fn scale_hint(&self) {
        sdl2::hint::set("SDL_RENDER_SCALE_QUALITY", if self.qol.smoothing == qol::Smoothing::Off { "0" } else { "1" });
    }

    /// Show the last frame again: the whole window cleared to black, the
    /// picture centred in it (whole multiples, or filled, as the Scaling
    /// option says). Also called when the window is resized or uncovered.
    fn redraw(&mut self) {
        let Some((w, h, crt)) = self.last_frame else { return };
        let (ww, wh) = self.canvas.output_size().unwrap_or((w, h));
        let (ax, ay, aw, ah) = match &self.touch {
            Some(t) => touch::game_area(ww, wh, t.dpi, t.o),
            None => (0, 0, ww, wh),
        };
        let (x, y, dw, dh) = qol::fit(aw, ah, w, h, self.qol.scaling);
        let dst = sdl2::rect::Rect::new(ax + x, ay + y, dw, dh);
        self.pic = dst;
        self.canvas.set_draw_color(sdl2::pixels::Color::RGB(0, 0, 0));
        self.canvas.clear();
        let tex = match (&self.crt_tex, crt) {
            (Some((_, _, t)), true) => t,
            _ => &self.tex,
        };
        let _ = self.canvas.copy(tex, None, dst);
        if let Some(((x, y, w, h), at)) = self.tap_flash {
            if at.elapsed() < TAP_FLASH {
                let (sx, sy) = (dst.width() as f32 / 320.0, dst.height() as f32 / 200.0);
                let r = sdl2::rect::Rect::new(dst.x() + (x as f32 * sx) as i32, dst.y() + (y as f32 * sy) as i32, (w as f32 * sx) as u32, (h as f32 * sy) as u32);
                self.canvas.set_blend_mode(sdl2::render::BlendMode::Blend);
                self.canvas.set_draw_color(sdl2::pixels::Color::RGBA(255, 255, 160, 150));
                let _ = self.canvas.draw_rect(r);
                let _ = self.canvas.draw_rect(sdl2::rect::Rect::new(r.x() + 1, r.y() + 1, r.width().saturating_sub(2), r.height().saturating_sub(2)));
                self.canvas.set_blend_mode(sdl2::render::BlendMode::None);
            } else {
                self.tap_flash = None;
            }
        }
        self.draw_touch(ww, wh);
        self.canvas.present();
    }

    /// The touch buttons, see-through, over whatever is under them.
    fn draw_touch(&mut self, ww: u32, wh: u32) {
        use sdl2::pixels::Color;
        use sdl2::rect::Rect;
        let tap = self.qol.tap;
        let Some(t) = &mut self.touch else { return };
        t.o.tap = tap;
        t.buttons = touch::layout(ww, wh, t.dpi, t.o);
        self.canvas.set_blend_mode(sdl2::render::BlendMode::Blend);
        for b in &t.buttons {
            let pressed = t.down.iter().any(|d| d.1 == b.act);
            let r = Rect::new(b.x, b.y, b.w as u32, b.h as u32);
            self.canvas.set_draw_color(if pressed { Color::RGBA(200, 200, 255, 150) } else { Color::RGBA(60, 60, 90, 90) });
            let _ = self.canvas.fill_rect(r);
            self.canvas.set_draw_color(Color::RGBA(220, 220, 255, 110));
            let _ = self.canvas.draw_rect(r);
            // The label in the engine's own font, as large as fits.
            let n = b.label.chars().count().max(1) as i32;
            let px = ((b.h * 5 / 10) / 8).min(b.w * 85 / 100 / (8 * n)).max(1);
            let (lx, ly) = (b.x + (b.w - 8 * px * n) / 2, b.y + (b.h - 8 * px) / 2);
            let mut dots = Vec::new();
            for (i, c) in b.label.chars().enumerate() {
                for (row, bits) in u1_assets::font::glyph(c).iter().enumerate() {
                    for col in 0..8 {
                        if bits & (0x80 >> col) != 0 {
                            dots.push(Rect::new(lx + (i as i32 * 8 + col) * px, ly + row as i32 * px, px as u32, px as u32));
                        }
                    }
                }
            }
            self.canvas.set_draw_color(Color::RGBA(255, 255, 255, 200));
            let _ = self.canvas.fill_rects(&dots);
        }
        self.canvas.set_blend_mode(sdl2::render::BlendMode::None);
    }

    /// A touch (or, with U1_TOUCH, a click) at window pixel (x, y).
    fn touch_down(&mut self, id: i64, x: i32, y: i32) {
        let act = {
            let Some(t) = &mut self.touch else { return };
            let Some(b) = touch::hit(&t.buttons, x, y) else {
                self.tap_start(id, x, y);
                return;
            };
            let act = b.act;
            t.down.retain(|d| d.0 != id);
            t.down.push((id, act));
            act
        };
        use touch::Act;
        match act {
            Act::Key(c) => {
                let name = match c {
                    keys::ENTER => "return".to_string(),
                    keys::ESC => "escape".to_string(),
                    keys::BACKSPACE => "backspace".to_string(),
                    b' ' => "space".to_string(),
                    c => (c as char).to_ascii_lowercase().to_string(),
                };
                self.pad_key(c, &name);
            }
            Act::Arrow(a) => {
                let name = match a {
                    keys::UP => "up",
                    keys::DOWN => "down",
                    keys::LEFT => "left",
                    _ => "right",
                };
                self.pad_key(a, name);
                if !self.in_menu {
                    // Held, it repeats (whatever the key-repeat option).
                    self.hold_arrow(a, finger_tag(id));
                }
            }
            Act::Options => self.open(Modal::Options),
            Act::Map if self.in_menu => {}
            Act::Map if self.wanted_code(keys::MAP) => self.pending.push_back((None, Some(keys::MAP))),
            Act::Map => self.note("Map: turn on Automap or Minimap (F1)"),
            Act::Log if self.qol.log && (self.in_menu || !self.front) => self.open(Modal::Log),
            Act::Log => self.note("Log: turn on the Message log (F1)"),
            Act::QuickSave | Act::QuickLoad if !self.qol.slots => self.note("Turn on Save slots (F1) first"),
            Act::QuickSave | Act::QuickLoad if self.in_menu => {}
            Act::QuickSave => self.request(SaveReq::Save(Slot::Quick)),
            Act::QuickLoad => self.request(SaveReq::Load(Slot::Quick)),
            Act::Keyboard | Act::Numbers | Act::More | Act::Hide | Act::Show => {
                let Some(t) = &mut self.touch else { return };
                match act {
                    Act::Keyboard if t.text.is_active() => t.text.stop(),
                    Act::Keyboard => t.text.start(),
                    Act::Numbers => t.o.numbers = !t.o.numbers,
                    Act::More => t.o.more = !t.o.more,
                    _ => t.o.hidden = !t.o.hidden,
                }
            }
        }
        self.redraw();
    }
    fn touch_up(&mut self, id: i64) {
        self.tap_end(id);
        let Some(t) = &mut self.touch else { return };
        if let Some(i) = t.down.iter().position(|d| d.0 == id) {
            t.down.remove(i);
            self.held.release(finger_tag(id));
            self.redraw();
        }
    }

    /// A finger sliding off its button lets go of it (an arrow stops).
    fn touch_move(&mut self, id: i64, x: i32, y: i32) {
        let Some(t) = &self.touch else { return };
        let Some(&(_, act)) = t.down.iter().find(|d| d.0 == id) else { return };
        if touch::hit(&t.buttons, x, y).map(|b| b.act) != Some(act) {
            self.touch_up(id);
        }
    }

    /// Tap actions (SPEC §24): a press on the game picture starts.
    fn tap_start(&mut self, id: i64, x: i32, y: i32) {
        if !self.qol.tap || self.in_menu || self.front || !self.pic.contains_point((x, y)) {
            return;
        }
        let (px, py) = ((x - self.pic.x()) * 320 / self.pic.width().max(1) as i32, (y - self.pic.y()) * 200 / self.pic.height().max(1) as i32);
        self.tap_press = Some((id, px, py, Instant::now(), false));
    }

    /// The press ends: a short one is a tap.
    fn tap_end(&mut self, id: i64) {
        if let Some((i, x, y, at, walking)) = self.tap_press {
            if i == id {
                self.tap_press = None;
                if !walking && at.elapsed() < LONG_PRESS {
                    self.tap_fire(x, y, false);
                }
            }
        }
    }

    /// A press held: on the player a long press (X-it, or what is
    /// underfoot in a dungeon); elsewhere it keeps walking that way, one
    /// normal turn per step, until let go or something other than a
    /// step would happen (a foe next to the way, a horse to board...).
    fn tap_held(&mut self) {
        let Some((id, x, y, at, walking)) = self.tap_press else { return };
        if at.elapsed() < LONG_PRESS || !self.at_prompt || self.in_menu || !self.tap_queue.is_empty() || !self.pending.is_empty() {
            return;
        }
        let a = u1_assets::tap::resolve(&self.scene, x, y, false);
        let own = matches!(&self.scene, u1_assets::tap::Scene::Grid(g) if a.cell == Some((g.px, g.py)));
        if !walking && (own || matches!(self.scene, u1_assets::tap::Scene::Dungeon(_))) {
            self.tap_press = None;
            self.tap_fire(x, y, true);
            return;
        }
        if !a.walk || own {
            self.tap_press = None;
            return;
        }
        self.tap_press = Some((id, x, y, at, true));
        self.tap_fire(x, y, false);
    }

    fn tap_fire(&mut self, x: i32, y: i32, long: bool) {
        if !self.qol.tap || !self.at_prompt || self.in_menu || self.front {
            return;
        }
        let a = u1_assets::tap::resolve(&self.scene, x, y, long);
        if let (Some((c, r)), u1_assets::tap::Scene::Grid(g)) = (a.cell, &self.scene) {
            self.tap_flash = Some((g.rect(c, r), Instant::now()));
        }
        self.tap_queue = a.keys.into_iter().collect();
        self.tap_then = a.then;
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
            self.menu_keys.push_back((keys::ESC, "escape".into()));
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
        if self.reimport {
            // Android's activity sees the marker as the program ends and
            // opens the import screen (android/.../U1Activity.java).
            if let Some(d) = self.game_dir.parent() {
                let _ = std::fs::write(d.join("reimport"), b"");
            }
            return Err(Halt);
        }
        self.sync_music();
        use sdl2::event::Event;
        use sdl2::keyboard::{Keycode, Mod};
        let evs: Vec<Event> = self.events.poll_iter().collect();
        for ev in evs {
            match ev {
                Event::Quit { .. } => return Err(Halt),
                Event::FingerDown { finger_id, x, y, .. } | Event::FingerUp { finger_id, x, y, .. } | Event::FingerMotion { finger_id, x, y, .. } if self.touch.is_some() => {
                    let (ww, wh) = self.canvas.output_size().unwrap_or((1, 1));
                    let (px, py) = ((x * ww as f32) as i32, (y * wh as f32) as i32);
                    match ev {
                        Event::FingerDown { .. } => self.touch_down(finger_id, px, py),
                        Event::FingerUp { .. } => self.touch_up(finger_id),
                        _ => self.touch_move(finger_id, px, py),
                    }
                }
                // The mouse stands in for a finger on the desktop (SDL's
                // copies of real touches are left out).
                Event::MouseButtonDown { which, x, y, .. } if self.touch.is_some() && which != u32::MAX => self.touch_down(-1, x, y),
                Event::MouseButtonUp { which, .. } if self.touch.is_some() && which != u32::MAX => self.touch_up(-1),
                // Without the touch controls, clicks on the picture are taps
                // when the option is on (right click: a long press).
                Event::MouseButtonDown { which, x, y, mouse_btn, .. } if self.qol.tap && which != u32::MAX => {
                    self.tap_start(-1, x, y);
                    if mouse_btn == sdl2::mouse::MouseButton::Right {
                        if let Some((_, px, py, _, _)) = self.tap_press.take() {
                            self.tap_fire(px, py, true);
                        }
                    }
                }
                Event::MouseButtonUp { which, .. } if self.qol.tap && which != u32::MAX => self.tap_end(-1),
                Event::Window { win_event, .. } => {
                    use sdl2::event::WindowEvent as We;
                    if matches!(win_event, We::SizeChanged(..) | We::Resized(..) | We::Exposed | We::Maximized | We::Restored | We::Shown) {
                        self.redraw();
                    }
                }
                Event::KeyDown { keycode: Some(k), keymod, repeat, .. } => {
                    let shift = keymod.intersects(Mod::LSHIFTMOD | Mod::RSHIFTMOD | Mod::CAPSMOD);
                    let alt = keymod.intersects(Mod::LALTMOD | Mod::RALTMOD);
                    let c = match k {
                        // Fullscreen: never passed to the game.
                        Keycode::Return | Keycode::KpEnter if alt => {
                            let q = Qol { fullscreen: !self.qol.fullscreen, ..self.qol };
                            self.apply_qol(q);
                            self.save_config(false);
                            continue;
                        }
                        Keycode::Left | Keycode::Kp4 => Some(keys::LEFT),
                        Keycode::Right | Keycode::Kp6 => Some(keys::RIGHT),
                        Keycode::Up | Keycode::Kp8 => Some(keys::UP),
                        Keycode::Down | Keycode::Kp2 => Some(keys::DOWN),
                        Keycode::Return | Keycode::KpEnter => Some(keys::ENTER),
                        // Android's Back is Esc.
                        Keycode::Escape | Keycode::AcBack => Some(keys::ESC),
                        Keycode::Backspace => Some(keys::BACKSPACE),
                        // The engine's own keys: none of them the original's.
                        Keycode::F1 => {
                            self.open(Modal::Options);
                            continue;
                        }
                        Keycode::F2 if self.qol.slots && (self.in_menu || !self.front) => {
                            self.open(Modal::Slots);
                            continue;
                        }
                        Keycode::F5 if self.qol.slots && !self.in_menu => {
                            self.request(SaveReq::Save(Slot::Quick));
                            continue;
                        }
                        Keycode::F9 if self.qol.slots && !self.in_menu => {
                            self.request(SaveReq::Load(Slot::Quick));
                            continue;
                        }
                        Keycode::F4 if !self.in_menu => {
                            let q = Qol { turbo: !self.qol.turbo, ..self.qol };
                            self.apply_qol(q);
                            self.note(if q.turbo { "Turbo on (F4)" } else { "Turbo off (F4)" });
                            self.save_config(false);
                            continue;
                        }
                        Keycode::PageUp if self.in_menu => Some(msglog::keys::PGUP),
                        Keycode::PageDown if self.in_menu => Some(msglog::keys::PGDN),
                        Keycode::Home if self.in_menu => Some(msglog::keys::HOME),
                        Keycode::End if self.in_menu => Some(msglog::keys::END),
                        Keycode::PageUp if self.qol.log && !self.front => {
                            self.open(Modal::Log);
                            continue;
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
                    let name = k.name().to_lowercase();
                    // Keys typed while a window is up are the window's.
                    if self.in_menu {
                        self.menu_keys.push_back((c.unwrap_or(0), name));
                        continue;
                    }
                    if let Some(a) = c.filter(|&c| is_arrow(c) && self.qol.repeat) {
                        // The engine's repeat replaces the system's.
                        if repeat {
                            continue;
                        }
                        self.hold_arrow(a, k.into_i32());
                    }
                    let extra = self.keymap.code_for(&name).filter(|&x| self.wanted_code(x));
                    if c.is_some() || extra.is_some() {
                        self.pending.push_back((c, extra));
                    }
                }
                Event::KeyUp { keycode: Some(k), .. } => {
                    self.held.release(k.into_i32());
                }
                Event::ControllerDeviceAdded { which, .. } if self.qol.gamepad => self.open_pad(which),
                Event::ControllerButtonDown { .. } | Event::ControllerButtonUp { .. } | Event::ControllerAxisMotion { .. } if self.qol.gamepad => {
                    if let Some(e) = pad_event(&ev) {
                        self.pad_event(e);
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// An engine code the game should get: a screen whose feature is off
    /// leaves the key as it always was.
    fn wanted_code(&self, code: u8) -> bool {
        match code {
            keys::MAP => self.qol.automap || self.qol.minimap,
            keys::HELP => self.qol.help,
            _ => true,
        }
    }

    /// Start the engine's repeat for arrow `a`, held by `by`.
    fn hold_arrow(&mut self, a: u8, by: i32) {
        let (delay, _) = self.qol.repeat_timing(self.scaled(POLL, Wait::Poll));
        self.held.hold(a, by, Instant::now(), delay);
    }

    /// A key from the pad: to the open window, or to the game.
    fn pad_key(&mut self, c: u8, name: &str) {
        if self.in_menu {
            self.menu_keys.push_back((c, name.to_string()));
        } else {
            self.pending.push_back((Some(c), None));
        }
    }

    /// Open the pads already plugged in (the gamepad setting turned on).
    fn open_pads(&mut self) {
        let n = self.gc.as_ref().and_then(|g| g.num_joysticks().ok()).unwrap_or(0);
        for i in 0..n {
            self.open_pad(i);
        }
    }

    fn open_pad(&mut self, which: u32) {
        let Some(gc) = &self.gc else { return };
        if !gc.is_game_controller(which) {
            return;
        }
        match gc.open(which) {
            Ok(p) => {
                if self.pads.iter().all(|q| q.instance_id() != p.instance_id()) {
                    self.note(&format!("Gamepad: {}", p.name()));
                    self.pads.push(p);
                }
            }
            Err(e) => eprintln!("gamepad: {e}"),
        }
    }

    /// A gamepad event, through the tested mapping layer (SPEC §23.3).
    fn pad_event(&mut self, e: PadEvent) {
        for act in self.pad.event(e, self.in_menu, self.qol.repeat) {
            match act {
                PadAction::Key(k, name) => self.pad_key(k, &name),
                PadAction::Command(c) if self.wanted_code(c) => self.pending.push_back((None, Some(c))),
                PadAction::Command(_) => {}
                PadAction::Options => self.open(Modal::Options),
                PadAction::Hold(a, by) => self.hold_arrow(a, by),
                PadAction::Release(by) => self.held.release(by),
            }
        }
    }

    /// The next key for the game: an extra key means its command only at
    /// the command prompt, and nothing elsewhere.
    fn next_key(&mut self) -> Option<u8> {
        while let Some((plain, cmd)) = self.pending.pop_front() {
            let k = pick_key(self.at_prompt, self.view_key, plain, cmd);
            if k.is_some() {
                return k;
            }
        }
        if !self.in_menu {
            if let Some(k) = self.tap_queue.pop_front() {
                return Some(k);
            }
        }
        None
    }

    /// A held arrow due to repeat: only at the command prompt, with no
    /// key waiting and no window open, and at most one per step.
    fn repeat_due(&mut self) {
        let ready = self.at_prompt && self.pending.is_empty() && !self.in_menu;
        let (_, step) = self.qol.repeat_timing(self.scaled(POLL, Wait::Poll));
        if let Some(a) = self.held.due(Instant::now(), ready, step) {
            self.pending.push_back((Some(a), None));
        }
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
            self.repeat_due();
            self.tap_held();
            if let Some(k) = self.next_key() {
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
        Ok(self.next_key())
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

    fn music(&mut self, t: Track) {
        if self.area != Some(t) {
            self.area = Some(t);
            self.sync_music();
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

    fn features(&self) -> Qol {
        self.qol
    }

    fn command_prompt(&mut self, on: bool) {
        if on && !self.at_prompt {
            // What a tap still had to type belonged to the last command.
            self.tap_queue.clear();
        }
        self.at_prompt = on;
    }

    fn tap_scene(&mut self, s: u1_assets::tap::Scene) {
        if let Some((want, k)) = self.tap_then.take() {
            if matches!(&s, u1_assets::tap::Scene::Grid(g) if g.here.thing == want) {
                self.tap_queue.push_back(k);
            }
        }
        self.scene = s;
    }

    fn view_open(&mut self, key: Option<u8>) {
        self.view_key = key;
    }
}

/// Which key the game gets from a press: an extra key's command at the
/// command prompt, or the open screen's own key (Tab, F3, Z) so it closes
/// that screen; elsewhere only the plain key.
fn pick_key(at_prompt: bool, view: Option<u8>, plain: Option<u8>, cmd: Option<u8>) -> Option<u8> {
    if at_prompt {
        cmd.or(plain)
    } else if view.is_some() && cmd == view {
        cmd
    } else {
        plain
    }
}

/// An SDL controller event as the mapping layer's plain value.
fn pad_event(ev: &sdl2::event::Event) -> Option<PadEvent<'static>> {
    use sdl2::controller::{Axis, Button};
    use sdl2::event::Event;
    fn name(b: Button) -> &'static str {
        match b {
            Button::A => "a",
            Button::B => "b",
            Button::X => "x",
            Button::Y => "y",
            Button::Back => "back",
            Button::Guide => "guide",
            Button::Start => "start",
            Button::LeftStick => "leftstick",
            Button::RightStick => "rightstick",
            Button::LeftShoulder => "leftshoulder",
            Button::RightShoulder => "rightshoulder",
            Button::DPadUp => "dpup",
            Button::DPadDown => "dpdown",
            Button::DPadLeft => "dpleft",
            Button::DPadRight => "dpright",
            _ => "",
        }
    }
    match *ev {
        Event::ControllerButtonDown { button, .. } => Some(PadEvent::Button(name(button), true)),
        Event::ControllerButtonUp { button, .. } => Some(PadEvent::Button(name(button), false)),
        Event::ControllerAxisMotion { axis: Axis::LeftX, value, .. } => Some(PadEvent::Stick(true, value)),
        Event::ControllerAxisMotion { axis: Axis::LeftY, value, .. } => Some(PadEvent::Stick(false, value)),
        _ => None,
    }
}

fn is_arrow(c: u8) -> bool {
    matches!(c, keys::UP | keys::DOWN | keys::LEFT | keys::RIGHT)
}

impl SdlIo {
    /// The window F1, F2 or PgUp asked for, if any: drawn over a copy of
    /// `t` and run with the game paused until it closes. Returns how long
    /// it was open (timed waits resume where they were).
    fn menu(&mut self, t: &Term) -> IoResult<Duration> {
        if self.modal_wanted.is_none() {
            return Ok(Duration::ZERO);
        }
        let opened = Instant::now();
        // One window can open the next (Options, then Keys).
        if let Some((dev, _)) = &mut self.audio {
            dev.lock().music.paused = true;
        }
        while let Some(which) = self.modal_wanted.take() {
            self.in_menu = true;
            let r = self.run_one(t, which);
            self.in_menu = false;
            self.menu_keys.clear();
            r?;
        }
        if let Some((dev, _)) = &mut self.audio {
            dev.lock().music.paused = false;
        }
        let paused = opened.elapsed();
        self.next = Instant::now() + POLL;
        self.held.clear();
        self.present(t)?;
        Ok(paused)
    }

    fn run_one(&mut self, t: &Term, which: Modal) -> IoResult<()> {
        match which {
            Modal::Options => {
                let video = VideoChoice::from_parts(self.enhanced, self.wanted);
                let mut m = OptionsMenu::new(video, self.sound, self.avail).with_qol(self.qol);
                m.import = cfg!(target_os = "android");
                self.run_modal(
                    t,
                    &mut m,
                    |m, c| m.draw(c),
                    |io, m, k, _| match m.key(k) {
                        MenuKey::Changed => {
                            io.choose(m.video, m.sound);
                            io.apply_qol(m.qol);
                            io.save_config(true);
                            true
                        }
                        MenuKey::Rebind => {
                            io.modal_wanted = Some(Modal::Rebind);
                            false
                        }
                        MenuKey::Import => {
                            io.reimport = true;
                            false
                        }
                        MenuKey::Close => false,
                        MenuKey::Stay => true,
                    },
                )
            }
            Modal::Rebind => {
                let mut r = RebindScreen::new(self.keymap.clone());
                self.run_modal(
                    t,
                    &mut r,
                    |r, c| r.draw(c),
                    |io, r, k, name| match r.key(k, name) {
                        RebindKey::Changed => {
                            io.keymap = r.map.clone();
                            io.save_config(false);
                            true
                        }
                        RebindKey::Close => false,
                        RebindKey::Stay => true,
                    },
                )
            }
            Modal::Slots => {
                let base = self.save_base.clone().unwrap_or_else(|| PathBuf::from("PLAYER1.U1"));
                let mut s = SlotScreen::new(&base);
                self.run_modal(
                    t,
                    &mut s,
                    |s, c| s.draw(c),
                    |io, s, k, _| match s.key(k) {
                        SlotKey::Do(r) => {
                            io.request(r);
                            false
                        }
                        SlotKey::Close => false,
                        SlotKey::Stay => true,
                    },
                )
            }
            Modal::Log => {
                let mut v = LogView::new(t);
                self.run_modal(t, &mut v, |v, c| v.draw(c), |_, v, k, _| v.key(k))
            }
        }
    }

    /// Run one window until `key` says it closes (false).
    fn run_modal<S>(&mut self, t: &Term, st: &mut S, draw: fn(&S, &mut Term), key: fn(&mut SdlIo, &mut S, u8, &str) -> bool) -> IoResult<()> {
        loop {
            let mut c = t.clone();
            draw(st, &mut c);
            self.present_over(t, Some((&c, PANEL)))?;
            self.pump()?;
            while let Some((k, name)) = self.menu_keys.pop_front() {
                if !key(self, st, k, &name) {
                    return Ok(());
                }
            }
            std::thread::sleep(Duration::from_millis(15));
        }
    }

    /// The title or character program stops here for a display chosen on
    /// its screen ([`Io::display_restart`]).
    fn restarting(&self) -> IoResult<()> {
        if self.restart {
            Err(Halt)
        } else {
            Ok(())
        }
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
        if old.smoothing != q.smoothing {
            // New textures with the new filtering.
            self.set_enhanced(self.enhanced);
        }
        if !q.slots {
            self.save_req = None;
        }
        if !q.repeat {
            self.held.clear();
        }
        if q.gamepad && !old.gamepad {
            self.open_pads();
        } else if !q.gamepad {
            self.pads.clear();
        }
    }

    fn apply_window(&mut self) {
        use sdl2::video::FullscreenType;
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
        c.keys = self.keymap.clone();
        c.pad = self.pad.map.clone();
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
        self.scale_hint();
        let tex = match self.tc.create_texture_streaming(sdl2::pixels::PixelFormatEnum::RGB24, iw, ih) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("{e}");
                return;
            }
        };
        self.tex = tex;
        // The window keeps the size the player gave it; the new picture
        // is only scaled into it.
        self.crt_tex = None;
        self.last_frame = None;
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
    /// The area music (its own layer and volume).
    music: music::Player,
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
            let mut acc = self.music.sample();
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
    let q = sub
        .open_playback(None, &want, |spec| Mixer { voices: Vec::new(), rate: spec.freq.max(1) as usize, music: music::Player::new(spec.freq.max(1) as u32) })
        .map_err(|e| eprintln!("no audio: {e}"))
        .ok()?;
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
    let window = video.window("u1 - Ultima I", iw * n, ih * n).position_centered().resizable().build().map_err(|e| e.to_string())?;
    let canvas = window.into_canvas().build().map_err(|e| e.to_string())?;
    // The texture creator lives for the whole program.
    let tc: &'static _ = Box::leak(Box::new(canvas.texture_creator()));
    let tex = tc.create_texture_streaming(PixelFormatEnum::RGB24, iw, ih).map_err(|e| e.to_string())?;
    let events = sdl.event_pump()?;
    // Opened even when muted, so the options can turn sound on.
    let audio = open_speaker(&sdl);
    let sound_src = pack_src.as_ref().map(|d| d.join("sounds"));
    let touch = (cfg!(target_os = "android") || std::env::var("U1_TOUCH").is_ok_and(|v| v == "1")).then(|| {
        let text = video.text_input();
        // SDL starts the system's text input with the window; the
        // keyboard button turns it on when wanted.
        text.stop();
        TouchUi { o: Default::default(), dpi: video.display_dpi(0).map_or(160.0, |d| d.0), buttons: Vec::new(), down: Vec::new(), text }
    });
    let mut io = SdlIo {
        canvas,
        touch,
        reimport: false,
        tex,
        events,
        palette,
        next: Instant::now(),
        pending: Default::default(),
        at_prompt: false,
        view_key: None,
        keymap: cfg.keys.clone(),
        pad: PadInput::new(cfg.pad.clone()),
        held: Repeater::default(),
        // Opened even with the setting off (it reads nothing until a pad
        // is opened), so the menu can turn the gamepad on.
        gc: sdl.game_controller().map_err(|e| eprintln!("no gamepad support: {e}")).ok(),
        pads: Vec::new(),
        audio,
        sound: set.sound,
        sounds: None,
        sound_src,
        pack,
        enhanced: set.enhanced,
        pack_src,
        start: Instant::now(),
        tc,
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
        area: None,
        tracks: Default::default(),
        rendering: false,
        crt_tex: None,
        last_frame: None,
        save_base: None,
        save_req: None,
        toast: None,
        scene: Default::default(),
        tap_queue: Default::default(),
        tap_then: None,
        tap_press: None,
        tap_flash: None,
        pic: sdl2::rect::Rect::new(0, 0, 1, 1),
    };
    // The optional features: the file's, and --crt / --fullscreen.
    let mut q = cfg.qol;
    q.crt |= a.crt;
    q.fullscreen |= a.fullscreen;
    let f = a.features;
    q.automap |= f.automap;
    q.minimap |= f.minimap;
    q.charsheet |= f.charsheet;
    q.help |= f.help;
    q.hints |= f.hints;
    q.repeat |= f.repeat;
    q.gamepad |= f.gamepad;
    if f.smoothing != qol::Smoothing::Off {
        q.smoothing = f.smoothing;
    }
    if f.confirm > 0 {
        q.confirm = f.confirm;
    }
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
    let mut carried: Option<Explore> = None;
    loop {
        let mode = io.wanted;
        io.video_shown(mode);
        let (mut g, _) = load(a, mode, chosen.as_deref(), reload.take())?;
        if let Some(e) = carried.take() {
            g.explore = e;
        }
        io.save_base = g.save_to.clone();
        io.in_space = false;
        io.at_prompt = false;
        let _ = g.run(&mut io);
        // A save slot was loaded: the game starts again from it.
        if let Some(r) = g.reload.take() {
            reload = Some(r);
            carried = g.reload_explore.take();
            continue;
        }
        if let Some(s) = g.stopped {
            eprintln!("stopped: {s}");
        }
        return Ok(());
    }
}

fn main() -> ExitCode {
    run(std::env::args().collect())
}

/// The program for these arguments (the first is the program's name).
/// Android's entry point (crates/u1-android) calls this too.
pub fn run(args: Vec<String>) -> ExitCode {
    let batch = args.iter().any(|a| a == "--screenshot" || a == "--sound-demo");
    let res = parse_args(args).and_then(|a| {
        if let Some(out) = &a.sound_demo {
            return sound_demo(&a, out);
        }
        if let Some(out) = &a.music_demo {
            return music_demo(out);
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
            // Started from a file manager there is no terminal to read the
            // message in: show it in a window too.
            use std::io::IsTerminal;
            if !std::io::stderr().is_terminal() && !batch && !e.starts_with("usage:") {
                let _ = sdl2::messagebox::show_simple_message_box(sdl2::messagebox::MessageBoxFlag::ERROR, "Ultima I engine", &wrap_text(&e, 80), None);
            }
            ExitCode::FAILURE
        }
    }
}

/// Wraps long lines at spaces for the error window, which (on X11) does not
/// wrap by itself. Leading indentation is kept on the continuation lines.
fn wrap_text(text: &str, width: usize) -> String {
    let mut out = String::new();
    for line in text.lines() {
        let indent: String = line.chars().take_while(|c| *c == ' ').collect();
        let mut cur = String::new();
        for word in line.split_whitespace() {
            if !cur.is_empty() && cur.chars().count() + 1 + word.chars().count() > width {
                out.push_str(&cur);
                out.push('\n');
                cur.clear();
            }
            if cur.is_empty() {
                cur.push_str(&indent);
            } else {
                cur.push(' ');
            }
            cur.push_str(word);
        }
        out.push_str(&cur);
        out.push('\n');
    }
    out.truncate(out.trim_end().len());
    out
}

/// Local wall-clock time as DOS reports it: hour, minute, second, hundredths.
fn local_time() -> [u8; 4] {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    let t = now.as_secs() as libc::time_t;
    // SAFETY: localtime_r / localtime_s write only into the zeroed `tm` we pass.
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    #[cfg(unix)]
    let ok = unsafe { !libc::localtime_r(&t, &mut tm).is_null() };
    #[cfg(windows)]
    let ok = unsafe { libc::localtime_s(&mut tm, &t) == 0 };
    #[cfg(not(any(unix, windows)))]
    let ok = false;
    let (h, m, s) = if ok {
        (tm.tm_hour as u8, tm.tm_min as u8, tm.tm_sec as u8)
    } else {
        let d = now.as_secs() % 86400;
        ((d / 3600) as u8, (d / 60 % 60) as u8, (d % 60) as u8)
    };
    [h, m, s, (now.subsec_millis() / 10) as u8]
}

#[cfg(test)]
mod tests {
    #[test]
    fn screen_key_closes_its_screen() {
        use super::pick_key;
        use u1_assets::io::keys;
        // Tab on the automap / mini-map, F3 on help, F6 on the character
        // screen: the key that opened it reaches it and closes it.
        for k in [keys::MAP, keys::HELP] {
            assert_eq!(pick_key(false, Some(k), None, Some(k)), Some(k));
        }
        // Other screens' keys and other prompts are unchanged.
        assert_eq!(pick_key(false, Some(keys::MAP), None, Some(keys::HELP)), None);
        assert_eq!(pick_key(false, None, None, Some(keys::MAP)), None);
        assert_eq!(pick_key(false, Some(keys::MAP), Some(b'a'), None), Some(b'a'));
        assert_eq!(pick_key(true, None, None, Some(keys::MAP)), Some(keys::MAP));
    }

    #[test]
    fn wrap_text_keeps_short_lines_and_indents() {
        assert_eq!(super::wrap_text("a b\n\n  c", 80), "a b\n\n  c");
        assert_eq!(super::wrap_text("aaa bbb ccc", 7), "aaa bbb\nccc");
        assert_eq!(super::wrap_text("  aa bb cc", 7), "  aa bb\n  cc");
    }

    use super::*;
    use sdl2::controller::{Axis, Button};
    use sdl2::event::Event;

    /// Synthetic SDL controller events through the same path the window
    /// takes (`pad_event`, then the mapping layer). A real controller is
    /// still to be tried (SPEC §23.3).
    #[test]
    fn synthetic_controller_events_map_to_keys() {
        let down = |b| Event::ControllerButtonDown { timestamp: 0, which: 0, button: b };
        let up = |b| Event::ControllerButtonUp { timestamp: 0, which: 0, button: b };
        let axis = |a, v| Event::ControllerAxisMotion { timestamp: 0, which: 0, axis: a, value: v };
        let mut pad = PadInput::default();
        let mut run = |e: Event| pad_event(&e).map(|p| pad.event(p, false, true)).unwrap_or_default();
        assert_eq!(run(down(Button::A)), vec![PadAction::Key(b' ', "space".into())]);
        assert_eq!(run(down(Button::Back)), vec![PadAction::Command(keys::MAP)]);
        assert_eq!(run(down(Button::Guide)), vec![PadAction::Command(keys::HELP)]);
        assert!(matches!(run(down(Button::LeftStick))[..], [PadAction::Key(b'Z', _)]));
        assert_eq!(run(down(Button::Start)), vec![PadAction::Options]);
        let v = run(down(Button::DPadUp));
        assert!(matches!(v[..], [PadAction::Key(keys::UP, _), PadAction::Hold(keys::UP, _)]), "{v:?}");
        assert!(matches!(run(up(Button::DPadUp))[..], [PadAction::Release(_)]));
        assert!(matches!(run(axis(Axis::LeftX, -30000))[..], [PadAction::Key(keys::LEFT, _), PadAction::Hold(..)]));
        assert!(matches!(run(axis(Axis::LeftX, 0))[..], [PadAction::Release(_)]));
        assert!(run(axis(Axis::TriggerLeft, 30000)).is_empty(), "triggers do nothing");
        assert!(pad_event(&Event::Quit { timestamp: 0 }).is_none());
    }

    /// Our button names are SDL's own (the config's `pad_<name>` keys).
    #[test]
    fn button_names_are_sdls() {
        for b in [
            Button::A,
            Button::B,
            Button::X,
            Button::Y,
            Button::Back,
            Button::Guide,
            Button::Start,
            Button::LeftStick,
            Button::RightStick,
            Button::LeftShoulder,
            Button::RightShoulder,
            Button::DPadUp,
            Button::DPadDown,
            Button::DPadLeft,
            Button::DPadRight,
        ] {
            let Some(PadEvent::Button(n, true)) = pad_event(&Event::ControllerButtonDown { timestamp: 0, which: 0, button: b }) else { panic!() };
            assert_eq!(n, b.string(), "{b:?}");
        }
    }
}
