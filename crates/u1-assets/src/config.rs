//! The player's display and sound choices, kept between runs in a small
//! TOML file (`~/.config/ultima1-engine/config.toml`, or under
//! `$XDG_CONFIG_HOME`). The in-game options menu (F1) writes it; command-line
//! options override it for one run. Nothing here changes the game itself.
//!
//! Only its own keys are read (the display, the sound and the optional
//! quality-of-life settings of SPEC §22, each written only when it differs
//! from the faithful default), as plain `key = "value"` lines; anything else
//! (comments, unknown keys, other tables) is ignored, so a newer or
//! hand-edited file never stops the game from starting, and is written
//! back unchanged when the menu saves.

use crate::gamedir::VideoMode;
use crate::keymap::{KeyMap, PadMap};
use crate::qol::{Qol, Scaling, CONFIRM_VALUES, MESSAGE_DELAYS, REPEAT_DELAYS, REPEAT_RATES};
use crate::soundpack::SoundMode;
use std::path::{Path, PathBuf};

/// The display choices the options menu offers: the enhanced art (drawn
/// over the EGA screen), or one of the original's three displays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoChoice {
    Enhanced,
    Ega,
    Cga,
    Tandy,
}

impl VideoChoice {
    pub const ALL: [VideoChoice; 4] = [VideoChoice::Enhanced, VideoChoice::Ega, VideoChoice::Cga, VideoChoice::Tandy];

    pub fn parse(s: &str) -> Option<VideoChoice> {
        match s.trim().to_ascii_lowercase().as_str() {
            "enhanced" => Some(VideoChoice::Enhanced),
            "ega" => Some(VideoChoice::Ega),
            "cga" => Some(VideoChoice::Cga),
            "tandy" | "t1k" => Some(VideoChoice::Tandy),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            VideoChoice::Enhanced => "enhanced",
            VideoChoice::Ega => "ega",
            VideoChoice::Cga => "cga",
            VideoChoice::Tandy => "tandy",
        }
    }

    /// The menu's label.
    pub fn label(self) -> &'static str {
        match self {
            VideoChoice::Enhanced => "Enhanced",
            VideoChoice::Ega => "EGA",
            VideoChoice::Cga => "CGA",
            VideoChoice::Tandy => "Tandy",
        }
    }

    /// Whether the enhanced art is drawn.
    pub fn enhanced(self) -> bool {
        self == VideoChoice::Enhanced
    }

    /// The original display underneath (the enhanced art sits on EGA).
    pub fn mode(self) -> VideoMode {
        match self {
            VideoChoice::Enhanced | VideoChoice::Ega => VideoMode::Ega,
            VideoChoice::Cga => VideoMode::Cga,
            VideoChoice::Tandy => VideoMode::Tandy,
        }
    }

    /// The choice that shows `mode`, with or without the enhanced art.
    pub fn from_parts(enhanced: bool, mode: VideoMode) -> VideoChoice {
        match (enhanced, mode) {
            (true, _) => VideoChoice::Enhanced,
            (false, VideoMode::Ega) => VideoChoice::Ega,
            (false, VideoMode::Cga) => VideoChoice::Cga,
            (false, VideoMode::Tandy) => VideoChoice::Tandy,
        }
    }
}

/// The saved choices; `None` is "not set" (the default applies).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Config {
    pub video: Option<VideoChoice>,
    pub sound: Option<SoundMode>,
    /// The optional engine features; the default is all off.
    pub qol: Qol,
    /// Extra keys for the commands and screens (SPEC §23.3).
    pub keys: KeyMap,
    /// Gamepad buttons.
    pub pad: PadMap,
    /// The window's last place and size (x, y, width, height), restored
    /// at start; fullscreen is kept in `qol`.
    pub window: Option<(i32, i32, u32, u32)>,
    /// Lines that could not be used (reported, never fatal).
    pub warnings: Vec<String>,
    /// The file's other lines, kept as they were for saving: top-level
    /// ones (comments, unknown keys) and everything from the first table.
    pub kept: Kept,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Kept {
    pub top: Vec<String>,
    pub tables: Vec<String>,
}

const HEADER: [&str; 2] = ["# ultima1-engine options, written by the in-game options menu (F1).", "# Command-line options override these for one run."];

impl Config {
    pub fn parse(text: &str) -> Config {
        let mut c = Config::default();
        let mut table = String::new();
        let mut in_tables = false;
        for (n, line) in text.lines().enumerate() {
            let l = strip_comment(line).trim();
            if l.starts_with('[') {
                in_tables = true;
                table = l.trim_matches(|ch| ch == '[' || ch == ']').trim().to_string();
            }
            if in_tables {
                c.kept.tables.push(line.to_string());
                continue;
            }
            if HEADER.contains(&line.trim_end()) {
                continue;
            }
            if l.is_empty() || !table.is_empty() {
                c.kept.top.push(line.to_string());
                continue;
            }
            let Some((k, v)) = l.split_once('=') else {
                c.warnings.push(format!("line {}: not `key = value`", n + 1));
                c.kept.top.push(line.to_string());
                continue;
            };
            let v = unquote(v.trim());
            match k.trim() {
                "video" => match VideoChoice::parse(&v) {
                    Some(x) => c.video = Some(x),
                    None => c.warnings.push(format!("line {}: video wants enhanced, ega, cga or tandy", n + 1)),
                },
                "sound" => match SoundMode::parse(&v.to_ascii_lowercase()) {
                    Some(x) => c.sound = Some(x),
                    None => c.warnings.push(format!("line {}: sound wants original, enhanced or mute", n + 1)),
                },
                "window" => match parse_window(&v) {
                    Some(w) => c.window = Some(w),
                    None => c.warnings.push(format!("line {}: window wants \"x,y,width,height\"", n + 1)),
                },
                // The older name of key_screenshot.
                "screenshot_key" if !v.is_empty() => {
                    let _ = c.keys.config_key("key_screenshot", &v);
                }
                k if k.starts_with("key_") => match c.keys.config_key(k, &v) {
                    Some(Ok(())) => {}
                    Some(Err(e)) => c.warnings.push(format!("line {}: {e}", n + 1)),
                    None => c.kept.top.push(line.to_string()),
                },
                k if k.starts_with("pad_") => match c.pad.config_key(k, &v) {
                    Some(Ok(())) => {}
                    Some(Err(e)) => c.warnings.push(format!("line {}: {e}", n + 1)),
                    None => c.kept.top.push(line.to_string()),
                },
                k => match qol_key(&mut c.qol, k, &v) {
                    Some(Ok(())) => {}
                    Some(Err(want)) => c.warnings.push(format!("line {}: {k} wants {want}", n + 1)),
                    None => c.kept.top.push(line.to_string()),
                },
            }
        }
        c
    }

    pub fn to_toml(&self) -> String {
        let mut s = String::new();
        for h in HEADER {
            s += h;
            s.push('\n');
        }
        if let Some(v) = self.video {
            s += &format!("video = \"{}\"   # enhanced | ega | cga | tandy\n", v.as_str());
        }
        if let Some(v) = self.sound {
            s += &format!("sound = \"{}\"   # original | enhanced | mute\n", v.as_str());
        }
        let (q, d) = (&self.qol, Qol::default());
        let mut flag = |on: bool, def: bool, k: &str, note: &str| {
            if on != def {
                s += &format!("{k} = {on}   # {note}\n");
            }
        };
        flag(q.fullscreen, d.fullscreen, "fullscreen", "also Alt+Enter");
        flag(q.crt, d.crt, "crt", "CRT scanline filter");
        flag(q.slots, d.slots, "save_slots", "F2 slots, F5 quicksave, F9 quickload");
        flag(q.log, d.log, "message_log", "PgUp shows the messages that scrolled away");
        flag(q.turbo, d.turbo, "turbo", "also F4");
        flag(q.fast_space, d.fast_space, "fast_space", "space flight and combat wait half as long");
        flag(q.steady, d.steady, "steady_timing", "speaker effects no longer hold the game");
        flag(q.automap, d.automap, "automap", "Tab in a dungeon");
        flag(q.minimap, d.minimap, "minimap", "Tab outdoors");
        flag(q.charsheet, d.charsheet, "character_screen", "Z shows it");
        flag(q.help, d.help, "help_screen", "F3");
        flag(q.hints, d.hints, "hints", "a line on what a command asks for");
        flag(q.repeat, d.repeat, "arrow_repeat", "our own repeat for held arrows");
        flag(q.gamepad, d.gamepad, "gamepad", "SDL game controllers");
        flag(q.footsteps, d.footsteps, "footsteps", "overworld footsteps in enhanced sound");
        flag(q.vehicle_facing, d.vehicle_facing, "vehicle_facing", "enhanced video: vehicles face left or right");
        flag(q.music, d.music, "music", "our own area music (F1)");
        flag(q.confirm_quit, d.confirm_quit, "confirm_quit", "ask before closing with unsaved progress");
        flag(q.sign_wait, d.sign_wait, "wait_on_signs", "\"Press any key\" after a sign's words");
        flag(q.docking_fee, d.docking_fee, "docking_fee", "the 500 coin fee to refuel at a space station");
        flag(q.steady_aim, d.steady_aim, "steady_aim", "space sights stop when the arrow is let go");
        if q.food_use != d.food_use {
            s += &format!("food_use = \"{}\"   # original | half | off\n", q.food_use.as_str());
        }
        if q.music_volume != d.music_volume {
            s += &format!("music_volume = {}   # per cent: 10 .. 100\n", q.music_volume);
        }
        if q.master_volume != d.master_volume {
            s += &format!("master_volume = {}   # per cent: 0 .. 100\n", q.master_volume);
        }
        if q.sfx_volume != d.sfx_volume {
            s += &format!("sfx_volume = {}   # per cent: 0 .. 100\n", q.sfx_volume);
        }
        if q.smoothing != d.smoothing {
            s += &format!("smoothing = \"{}\"   # off | linear | pixel-art\n", q.smoothing.as_str());
        }
        if q.scaling != d.scaling {
            s += &format!("scaling = \"{}\"   # whole | fill\n", q.scaling.as_str());
        }
        if q.message_delay != d.message_delay {
            s += &format!("message_delay = {}   # per cent of the original: 100 | 50 | 25 | 0\n", q.message_delay);
        }
        if q.confirm != d.confirm {
            s += &format!("confirm_value = {}   # ask before selling or dropping things worth this much (0 never)\n", q.confirm);
        }
        if q.repeat_delay != d.repeat_delay {
            s += &format!("repeat_delay = {}   # ms: 250 | 400 | 600 | 800\n", q.repeat_delay);
        }
        if q.repeat_rate != d.repeat_rate {
            s += &format!("repeat_rate = {}   # steps a second: 0 (the game's pace) | 4 | 8 | 12 | 16\n", q.repeat_rate);
        }
        if let Some((x, y, w, h)) = self.window {
            s += &format!("window = \"{x},{y},{w},{h}\"   # x, y, width, height (remembered)\n");
        }
        for l in self.keys.config_lines().into_iter().chain(self.pad.config_lines()) {
            s += &l;
            s.push('\n');
        }
        for l in self.kept.top.iter().chain(&self.kept.tables) {
            s += l;
            s.push('\n');
        }
        s
    }

    /// The file's choices; a missing file is the defaults.
    pub fn load(path: &Path) -> Config {
        match std::fs::read_to_string(path) {
            Ok(t) => Config::parse(&t),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Config::default(),
            Err(e) => Config { warnings: vec![format!("{}: {e}", path.display())], ..Default::default() },
        }
    }

    /// Write the file (and its folder): the choices, then the lines kept
    /// from the file they were read from.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(d) = path.parent() {
            std::fs::create_dir_all(d)?;
        }
        let tmp = path.with_extension("toml.tmp");
        std::fs::write(&tmp, self.to_toml())?;
        std::fs::rename(&tmp, path)
    }
}

/// Reads one quality-of-life key into `q`: `None` if `k` is not one,
/// `Some(Err(what it wants))` if the value is wrong.
fn qol_key(q: &mut Qol, k: &str, v: &str) -> Option<Result<(), &'static str>> {
    let flag = |v: &str| match v.to_ascii_lowercase().as_str() {
        "true" | "on" | "yes" | "1" => Ok(true),
        "false" | "off" | "no" | "0" => Ok(false),
        _ => Err("true or false"),
    };
    let r = match k {
        "fullscreen" => flag(v).map(|b| q.fullscreen = b),
        "crt" => flag(v).map(|b| q.crt = b),
        "save_slots" => flag(v).map(|b| q.slots = b),
        "message_log" => flag(v).map(|b| q.log = b),
        "turbo" => flag(v).map(|b| q.turbo = b),
        "fast_space" => flag(v).map(|b| q.fast_space = b),
        "steady_timing" => flag(v).map(|b| q.steady = b),
        "automap" => flag(v).map(|b| q.automap = b),
        "minimap" => flag(v).map(|b| q.minimap = b),
        "character_screen" => flag(v).map(|b| q.charsheet = b),
        "help_screen" => flag(v).map(|b| q.help = b),
        "hints" => flag(v).map(|b| q.hints = b),
        "arrow_repeat" => flag(v).map(|b| q.repeat = b),
        "gamepad" => flag(v).map(|b| q.gamepad = b),
        "footsteps" => flag(v).map(|b| q.footsteps = b),
        "vehicle_facing" => flag(v).map(|b| q.vehicle_facing = b),
        "music" => flag(v).map(|b| q.music = b),
        "music_volume" => match v.trim_end_matches('%').parse::<u8>() {
            Ok(d) if crate::qol::MUSIC_VOLUMES.contains(&d) => {
                q.music_volume = d;
                Ok(())
            }
            _ => Err("10, 20 .. 100"),
        },
        "master_volume" | "sfx_volume" => match v.trim_end_matches('%').parse::<u8>() {
            Ok(d) if crate::qol::VOLUMES.contains(&d) => {
                *if k == "master_volume" { &mut q.master_volume } else { &mut q.sfx_volume } = d;
                Ok(())
            }
            _ => Err("0, 10 .. 100"),
        },
        "confirm_quit" => flag(v).map(|b| q.confirm_quit = b),
        "wait_on_signs" => flag(v).map(|b| q.sign_wait = b),
        "docking_fee" => flag(v).map(|b| q.docking_fee = b),
        "steady_aim" => flag(v).map(|b| q.steady_aim = b),
        "smoothing" => crate::qol::Smoothing::parse(v).map(|x| q.smoothing = x).ok_or("off, linear or pixel-art"),
        "confirm_value" => match v.parse::<u16>() {
            Ok(d) if CONFIRM_VALUES.contains(&d) => {
                q.confirm = d;
                Ok(())
            }
            _ => Err("0, 10, 50, 100, 250 or 500"),
        },
        "repeat_delay" => match v.parse::<u16>() {
            Ok(d) if REPEAT_DELAYS.contains(&d) => {
                q.repeat_delay = d;
                Ok(())
            }
            _ => Err("250, 400, 600 or 800"),
        },
        "repeat_rate" => match v.parse::<u8>() {
            Ok(d) if REPEAT_RATES.contains(&d) => {
                q.repeat_rate = d;
                Ok(())
            }
            _ => Err("0, 4, 8, 12 or 16"),
        },
        "food_use" => crate::qol::FoodUse::parse(v).map(|x| q.food_use = x).ok_or("original, half or off"),
        "scaling" => Scaling::parse(v).map(|x| q.scaling = x).ok_or("whole or fill"),
        "message_delay" => match v.trim_end_matches('%').parse::<u8>() {
            Ok(d) if MESSAGE_DELAYS.contains(&d) => {
                q.message_delay = d;
                Ok(())
            }
            _ => Err("100, 50, 25 or 0"),
        },
        _ => return None,
    };
    Some(r)
}

/// `"x,y,width,height"`; sizes of at least 64 by 40.
fn parse_window(v: &str) -> Option<(i32, i32, u32, u32)> {
    let p: Vec<&str> = v.split(',').map(str::trim).collect();
    let [x, y, w, h] = p[..] else { return None };
    let (w, h) = (w.parse::<u32>().ok()?, h.parse::<u32>().ok()?);
    (w >= 64 && h >= 40 && w <= 16384 && h <= 16384).then_some((x.parse().ok()?, y.parse().ok()?, w, h))
}

fn strip_comment(l: &str) -> &str {
    // A `#` outside quotes starts a comment.
    let mut q = false;
    for (i, ch) in l.char_indices() {
        match ch {
            '"' => q = !q,
            '#' if !q => return &l[..i],
            _ => {}
        }
    }
    l
}

fn unquote(v: &str) -> String {
    let v = v.trim();
    if v.len() >= 2 && ((v.starts_with('"') && v.ends_with('"')) || (v.starts_with('\'') && v.ends_with('\''))) {
        v[1..v.len() - 1].to_string()
    } else {
        v.to_string()
    }
}

/// `$XDG_CONFIG_HOME/ultima1-engine/config.toml`, else
/// `$HOME/.config/ultima1-engine/config.toml`.
pub fn default_path() -> Option<PathBuf> {
    path_from(std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from), std::env::var_os("HOME").map(PathBuf::from))
}

pub fn path_from(xdg: Option<PathBuf>, home: Option<PathBuf>) -> Option<PathBuf> {
    let base = match xdg.filter(|p| p.is_absolute()) {
        Some(x) => x,
        None => home.filter(|h| !h.as_os_str().is_empty())?.join(".config"),
    };
    Some(base.join("ultima1-engine").join("config.toml"))
}

/// What the command line said (each `None` when not given).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Cli {
    /// `--video original|enhanced`.
    pub enhanced: Option<bool>,
    /// `--mode ega|t1k|cga`.
    pub mode: Option<VideoMode>,
    /// `--sound ...` or `--mute`.
    pub sound: Option<SoundMode>,
}

/// The run's settings: each command-line option wins over the file, the
/// file over the defaults (no enhanced art, the original effects, and the
/// display the game files suggest, `detected`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Settings {
    pub enhanced: bool,
    pub mode: VideoMode,
    pub sound: SoundMode,
}

pub fn resolve(cli: Cli, cfg: &Config, detected: VideoMode) -> Settings {
    let enhanced = cli.enhanced.unwrap_or_else(|| cfg.video.is_some_and(VideoChoice::enhanced));
    let mode = cli.mode.unwrap_or_else(|| match cfg.video {
        // The enhanced art sits on EGA when the file chose it; with
        // `--video original` given, the file's EGA base still counts.
        Some(v) => v.mode(),
        None => detected,
    });
    let sound = cli.sound.or(cfg.sound).unwrap_or(SoundMode::Original);
    Settings { enhanced, mode, sound }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        for v in VideoChoice::ALL {
            for s in [SoundMode::Original, SoundMode::Enhanced, SoundMode::Mute] {
                let c = Config { video: Some(v), sound: Some(s), ..Default::default() };
                assert_eq!(Config::parse(&c.to_toml()), c);
            }
        }
        assert_eq!(Config::parse(&Config::default().to_toml()), Config::default());
    }

    #[test]
    fn polish_keys_round_trip() {
        let mut c = Config { window: Some((-5, 40, 1280, 800)), ..Default::default() };
        assert_eq!(c.keys.config_key("key_screenshot", "f11"), Some(Ok(())));
        assert_eq!(Config::parse("screenshot_key = \"f11\"\n").keys.screenshot(), "f11", "the older name is read");
        c.qol.master_volume = 70;
        c.qol.sfx_volume = 0;
        c.qol.confirm_quit = false;
        c.qol.sign_wait = false;
        c.qol.docking_fee = false;
        let t = c.to_toml();
        assert!(t.contains("window = \"-5,40,1280,800\"") && t.contains("sfx_volume = 0"), "{t}");
        assert_eq!(Config::parse(&t), c);
        assert!(!Config::default().to_toml().contains("volume"), "defaults are not written");
        let bad = Config::parse("window = \"1,2,3\"\nsfx_volume = 15\nwindow = \"0,0,10,10\"\n");
        assert_eq!((bad.window, bad.qol.sfx_volume, bad.warnings.len()), (None, 100, 3));
    }

    #[test]
    fn save_slots_default_on_and_old_configs() {
        // An old config that never mentions slots now gets them; one that
        // turned them on keeps them; one that says off stays off.
        assert!(Config::parse("video = \"ega\"\n").qol.slots);
        assert!(Config::parse("save_slots = true\n").qol.slots);
        let off = Config::parse("save_slots = false\n");
        assert!(!off.qol.slots && off.to_toml().contains("save_slots = false"));
        assert!(!Config::default().to_toml().contains("save_slots"));
    }

    #[test]
    fn tolerant_parse() {
        let c = Config::parse("# hand edited\n\nvideo='CGA'  # four colours\nsound = \"enhanced\"\nfuture = 3\n[keys]\nvideo = \"ega\"\n");
        assert_eq!(c.video, Some(VideoChoice::Cga));
        assert_eq!(c.sound, Some(SoundMode::Enhanced));
        assert!(c.warnings.is_empty(), "{:?}", c.warnings);
        let c = Config::parse("video = \"vga\"\nsound = loud\nnonsense\nvideo = t1k\n");
        assert_eq!(c.video, Some(VideoChoice::Tandy));
        assert_eq!(c.sound, None);
        assert_eq!(c.warnings.len(), 3);
    }

    #[test]
    fn file_save_load() {
        let dir = std::env::temp_dir().join(format!("u1cfg-{}", std::process::id()));
        let p = dir.join("ultima1-engine").join("config.toml");
        assert_eq!(Config::load(&p), Config::default(), "missing file is the defaults");
        let c = Config { video: Some(VideoChoice::Enhanced), sound: Some(SoundMode::Mute), ..Default::default() };
        c.save(&p).unwrap();
        assert_eq!(Config::load(&p), c);
        let c2 = Config { video: Some(VideoChoice::Tandy), ..c.clone() };
        c2.save(&p).unwrap();
        assert_eq!(Config::load(&p), c2);
        assert!(!p.with_extension("toml.tmp").exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn unknown_lines_survive_a_save() {
        let text = "# mine\nvideo = \"cga\"\nfuture = 3   # later\n\nsound = \"mute\"\n[keys]\nvideo = \"ega\"\nf1 = \"menu\"\n";
        let mut c = Config::parse(text);
        assert_eq!(c.video, Some(VideoChoice::Cga));
        c.video = Some(VideoChoice::Enhanced);
        c.sound = Some(SoundMode::Original);
        let out = c.to_toml();
        for l in ["# mine", "future = 3   # later", "[keys]", "video = \"ega\"", "f1 = \"menu\""] {
            assert!(out.lines().any(|x| x == l), "{l:?} kept in\n{out}");
        }
        // Keys before tables, the table's lines last and in order.
        let t = out.find("[keys]").unwrap();
        assert!(out.find("future").unwrap() < t && out.find("video = \"enhanced\"").unwrap() < t);
        assert!(out[t..].starts_with("[keys]\nvideo = \"ega\"\nf1 = \"menu\"\n"));
        let back = Config::parse(&out);
        assert_eq!((back.video, back.sound), (Some(VideoChoice::Enhanced), Some(SoundMode::Original)));
        // Saving again changes nothing (no duplicated header or keys).
        assert_eq!(back.to_toml(), out);
        assert_eq!(out.matches("video =").count(), 2);
    }

    #[test]
    #[cfg(unix)] // "/x/cfg" is not an absolute path on Windows.
    fn paths() {
        let p = path_from(Some("/x/cfg".into()), Some("/home/u".into())).unwrap();
        assert_eq!(p, PathBuf::from("/x/cfg/ultima1-engine/config.toml"));
        let p = path_from(Some("relative".into()), Some("/home/u".into())).unwrap();
        assert_eq!(p, PathBuf::from("/home/u/.config/ultima1-engine/config.toml"));
        assert_eq!(path_from(None, None), None);
    }

    #[test]
    fn command_line_wins() {
        let none = Config::default();
        let s = resolve(Cli::default(), &none, VideoMode::Tandy);
        assert_eq!(s, Settings { enhanced: false, mode: VideoMode::Tandy, sound: SoundMode::Original }, "defaults");
        let cfg = Config { video: Some(VideoChoice::Enhanced), sound: Some(SoundMode::Enhanced), ..Default::default() };
        let s = resolve(Cli::default(), &cfg, VideoMode::Tandy);
        assert_eq!(s, Settings { enhanced: true, mode: VideoMode::Ega, sound: SoundMode::Enhanced }, "file");
        let cli = Cli { enhanced: Some(false), mode: Some(VideoMode::Cga), sound: Some(SoundMode::Mute) };
        assert_eq!(resolve(cli, &cfg, VideoMode::Ega), Settings { enhanced: false, mode: VideoMode::Cga, sound: SoundMode::Mute }, "all overridden");
        let cli = Cli { sound: Some(SoundMode::Original), ..Default::default() };
        let s = resolve(cli, &cfg, VideoMode::Ega);
        assert!(s.enhanced && s.sound == SoundMode::Original, "only the given option is overridden");
        let cfg = Config { video: Some(VideoChoice::Cga), ..Default::default() };
        let cli = Cli { enhanced: Some(true), ..Default::default() };
        assert_eq!(resolve(cli, &cfg, VideoMode::Ega).mode, VideoMode::Cga, "--video enhanced keeps the file's display");
    }

    #[test]
    fn second_group_and_keys_round_trip() {
        assert!(Config::default().keys.config_lines().is_empty() && Config::default().pad.config_lines().is_empty());
        let q = Qol { automap: true, minimap: true, charsheet: true, help: true, hints: true, confirm: 100, repeat: true, repeat_delay: 250, repeat_rate: 16, gamepad: true, ..Default::default() };
        let mut c = Config { qol: q, ..Default::default() };
        c.keys.bind(5, "j").unwrap();
        c.keys.bind(26, "\\").unwrap();
        c.keys.bind(25, "m").unwrap();
        c.pad.config_key("pad_y", "z").unwrap().unwrap();
        let text = c.to_toml();
        assert!(text.contains("key_attack = \"j\"") && !text.contains("key_character") && text.contains("pad_y = \"z\""), "{text}");
        let back = Config::parse(&text);
        assert!(back.warnings.is_empty(), "{:?}", back.warnings);
        assert_eq!(back, c);
        assert_eq!(back.keys.code_for("j"), Some(b'A'));
        assert_eq!(back.keys.code_for("\\"), Some(crate::io::keys::HELP));
        assert_eq!(back.keys.code_for("f6"), None);
        let bad = Config::parse("key_attack = \"k\"\nconfirm_value = 7\nrepeat_rate = 99\npad_a = \"jump\"\n");
        assert_eq!(bad.warnings.len(), 4, "{:?}", bad.warnings);
        assert_eq!(bad.keys, KeyMap::default());
    }

    #[test]
    fn qol_keys() {
        assert_eq!(Config::default().qol, Qol::default(), "all off without a file");
        assert!(!Config::default().to_toml().contains("turbo"), "defaults are not written");
        let q = Qol { fullscreen: true, scaling: Scaling::Fill, crt: true, slots: true, log: true, turbo: true, message_delay: 25, fast_space: true, steady: true, ..Default::default() };
        let c = Config { qol: q, ..Default::default() };
        let back = Config::parse(&c.to_toml());
        assert_eq!(back, c);
        for one in [Qol { crt: true, ..Default::default() }, Qol { message_delay: 0, ..Default::default() }, Qol { scaling: Scaling::Fill, ..Default::default() }] {
            let c = Config { qol: one, ..Default::default() };
            assert_eq!(Config::parse(&c.to_toml()), c);
        }
        let c = Config::parse("turbo = on\ncrt = maybe\nmessage_delay = 30\nmessage_delay = \"50%\"\nscaling = huge\n");
        assert!(c.qol.turbo);
        assert_eq!(c.qol.message_delay, 50);
        assert_eq!(c.warnings.len(), 3, "{:?}", c.warnings);
        assert!(c.kept.top.is_empty(), "known keys are not kept twice");
    }

    #[test]
    fn choices() {
        for v in VideoChoice::ALL {
            assert_eq!(VideoChoice::parse(v.as_str()), Some(v));
            assert_eq!(VideoChoice::from_parts(v.enhanced(), v.mode()), v);
        }
    }

    #[test]
    fn food_use_and_footsteps_round_trip() {
        use crate::qol::FoodUse;
        assert_eq!(Config::default().qol.food_use, FoodUse::Original);
        assert!(Config::default().qol.footsteps);
        // The defaults write nothing.
        assert!(!Config::default().to_toml().contains("food_use"));
        for f in FoodUse::ALL {
            let mut c = Config::default();
            c.qol.food_use = f;
            c.qol.footsteps = false;
            let back = Config::parse(&c.to_toml());
            assert_eq!(back.qol.food_use, f);
            assert!(!back.qol.footsteps);
        }
        assert_eq!(Config::parse("food_use = \"half\"\n").qol.food_use, FoodUse::Half);
    }

    #[test]
    fn music_and_smoothing_round_trip() {
        use crate::qol::Smoothing;
        let d = Config::default();
        assert!(!d.qol.music && d.qol.smoothing == Smoothing::Off);
        assert!(!d.to_toml().contains("music") && !d.to_toml().contains("smoothing"));
        for sm in Smoothing::ALL {
            let mut c = Config::default();
            c.qol.music = true;
            c.qol.music_volume = 80;
            c.qol.smoothing = sm;
            let back = Config::parse(&c.to_toml());
            assert_eq!((back.qol.music, back.qol.music_volume, back.qol.smoothing), (true, 80, sm));
        }
        let c = Config::parse("smoothing = \"pixel-art\"\nmusic_volume = 55\n");
        assert_eq!((c.qol.smoothing, c.qol.music_volume), (Smoothing::PixelArt, 50));
        assert!(!c.warnings.is_empty());
    }
}
