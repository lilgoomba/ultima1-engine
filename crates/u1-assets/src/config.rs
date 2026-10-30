//! The player's display and sound choices, kept between runs in a small
//! TOML file (`~/.config/ultima1-engine/config.toml`, or under
//! `$XDG_CONFIG_HOME`). The in-game options menu (F1) writes it; command-line
//! options override it for one run. Nothing here changes the game itself.
//!
//! Only two keys are read, as plain `key = "value"` lines; anything else
//! (comments, unknown keys, other tables) is ignored, so a newer or
//! hand-edited file never stops the game from starting.

use crate::gamedir::VideoMode;
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
    /// Lines that could not be used (reported, never fatal).
    pub warnings: Vec<String>,
}

impl Config {
    pub fn parse(text: &str) -> Config {
        let mut c = Config::default();
        let mut table = String::new();
        for (n, line) in text.lines().enumerate() {
            let l = strip_comment(line).trim();
            if l.is_empty() {
                continue;
            }
            if l.starts_with('[') {
                table = l.trim_matches(|ch| ch == '[' || ch == ']').trim().to_string();
                continue;
            }
            if !table.is_empty() {
                continue;
            }
            let Some((k, v)) = l.split_once('=') else {
                c.warnings.push(format!("line {}: not `key = value`", n + 1));
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
                _ => {}
            }
        }
        c
    }

    pub fn to_toml(&self) -> String {
        let mut s = String::from("# ultima1-engine options, written by the in-game options menu (F1).\n# Command-line options override these for one run.\n");
        if let Some(v) = self.video {
            s += &format!("video = \"{}\"   # enhanced | ega | cga | tandy\n", v.as_str());
        }
        if let Some(v) = self.sound {
            s += &format!("sound = \"{}\"   # original | enhanced | mute\n", v.as_str());
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

    /// Write the file (and its folder), replacing it whole.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(d) = path.parent() {
            std::fs::create_dir_all(d)?;
        }
        let tmp = path.with_extension("toml.tmp");
        std::fs::write(&tmp, self.to_toml())?;
        std::fs::rename(&tmp, path)
    }
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
                let c = Config { video: Some(v), sound: Some(s), warnings: vec![] };
                assert_eq!(Config::parse(&c.to_toml()), c);
            }
        }
        assert_eq!(Config::parse(&Config::default().to_toml()), Config::default());
    }

    #[test]
    fn tolerant_parse() {
        let c = Config::parse(
            "# hand edited\n\nvideo='CGA'  # four colours\nsound = \"enhanced\"\nfuture = 3\n[keys]\nvideo = \"ega\"\n",
        );
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
        let c = Config { video: Some(VideoChoice::Enhanced), sound: Some(SoundMode::Mute), warnings: vec![] };
        c.save(&p).unwrap();
        assert_eq!(Config::load(&p), c);
        let c2 = Config { video: Some(VideoChoice::Tandy), ..c.clone() };
        c2.save(&p).unwrap();
        assert_eq!(Config::load(&p), c2);
        assert!(!p.with_extension("toml.tmp").exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
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
        let cfg = Config { video: Some(VideoChoice::Enhanced), sound: Some(SoundMode::Enhanced), warnings: vec![] };
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
    fn choices() {
        for v in VideoChoice::ALL {
            assert_eq!(VideoChoice::parse(v.as_str()), Some(v));
            assert_eq!(VideoChoice::from_parts(v.enhanced(), v.mode()), v);
        }
    }
}
