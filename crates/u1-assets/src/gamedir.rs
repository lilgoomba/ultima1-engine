//! Locating data files in a user-supplied game directory.
//!
//! DOS file names are case-insensitive; GOG installs may use any case, so we
//! match names case-insensitively. We also look in a few common
//! sub-directories used by GOG/DOSBox packaging (UNKNOWN / needs
//! verification against a real GOG install).

use crate::error::{AssetError, Result};
use crate::map::WorldMap;
use crate::tiles::{TileFileSpec, TileSet, CGA_TILES, CGA_TOWN, EGA_TILES, EGA_TOWN, T1K_TILES, T1K_TOWN};
use std::path::{Path, PathBuf};

/// Graphics modes the 1986 DOS release ships data for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoMode {
    Ega,
    Tandy,
    Cga,
}

impl VideoMode {
    pub fn world_tiles(self) -> TileFileSpec {
        match self {
            VideoMode::Ega => EGA_TILES,
            VideoMode::Tandy => T1K_TILES,
            VideoMode::Cga => CGA_TILES,
        }
    }

    /// The 8x8 town and castle tiles of this mode.
    pub fn town_tiles(self) -> TileFileSpec {
        match self {
            VideoMode::Ega => EGA_TOWN,
            VideoMode::Tandy => T1K_TOWN,
            VideoMode::Cga => CGA_TOWN,
        }
    }

    pub fn parse(s: &str) -> Option<VideoMode> {
        match s.to_ascii_lowercase().as_str() {
            "ega" => Some(VideoMode::Ega),
            "t1k" | "tandy" => Some(VideoMode::Tandy),
            "cga" => Some(VideoMode::Cga),
            _ => None,
        }
    }
}

const SUBDIRS: [&str; 4] = ["", "ULTIMA1", "ultima1", "U1"];

pub struct GameDir {
    root: PathBuf,
}

impl GameDir {
    pub fn new(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        if !root.is_dir() {
            return Err(AssetError::NotFound(format!("{} (not a directory)", root.display())));
        }
        Ok(GameDir { root })
    }

    /// Find `name` case-insensitively in the root or a known sub-directory.
    pub fn find(&self, name: &str) -> Option<PathBuf> {
        for sub in SUBDIRS {
            let dir = self.root.join(sub);
            let Ok(rd) = std::fs::read_dir(&dir) else { continue };
            for e in rd.flatten() {
                if e.file_name().to_string_lossy().eq_ignore_ascii_case(name) {
                    return Some(e.path());
                }
            }
        }
        None
    }

    pub fn read(&self, name: &str) -> Result<Vec<u8>> {
        let p = self.find(name).ok_or_else(|| AssetError::NotFound(name.to_string()))?;
        Ok(std::fs::read(p)?)
    }

    /// Best available world tile set, preferring EGA, then Tandy, then CGA.
    pub fn detect_video_mode(&self) -> Option<VideoMode> {
        [VideoMode::Ega, VideoMode::Tandy, VideoMode::Cga].into_iter().find(|m| self.find(m.world_tiles().file_name).is_some())
    }

    pub fn load_world_tiles(&self, mode: VideoMode) -> Result<TileSet> {
        let spec = mode.world_tiles();
        TileSet::decode_spec(&self.read(spec.file_name)?, &spec)
    }

    pub fn load_town_tiles(&self, mode: VideoMode) -> Result<TileSet> {
        let spec = mode.town_tiles();
        TileSet::decode_spec(&self.read(spec.file_name)?, &spec)
    }

    /// Load MAP.BIN. Accepts the documented 13,104 bytes, or any whole
    /// number of 84-byte rows (to tolerate the conflicting 168x168 claim).
    pub fn load_world_map(&self) -> Result<WorldMap> {
        WorldMap::parse_lenient(&self.read("MAP.BIN")?)
    }
}

/// Files the engine cannot start without: the game, the title sequence and
/// the character menu. A tile set (world and town tiles of one display) is
/// needed as well; see [`GameDir::missing`].
pub const REQUIRED: [&str; 5] = ["OUT.EXE", "MAP.BIN", "TCD.BIN", "ULTIMA.EXE", "GEN.EXE"];

impl GameDir {
    /// The required files this folder lacks (empty: it can be played).
    /// `front` includes the title sequence's and character menu's programs.
    pub fn missing(&self, front: bool) -> Vec<String> {
        let mut m: Vec<String> = REQUIRED.iter().filter(|f| (front || !f.ends_with(".EXE") || **f == "OUT.EXE") && self.find(f).is_none()).map(|f| f.to_string()).collect();
        match self.detect_video_mode() {
            None => m.push("EGATILES.BIN + EGATOWN.BIN (or the T1K or CGA pair)".into()),
            Some(mode) => {
                let town = mode.town_tiles().file_name;
                if self.find(town).is_none() {
                    m.push(town.to_string());
                }
            }
        }
        m
    }
}

/// The first of `candidates` holding every required file (with `front`, the
/// title and character programs too). Otherwise an error naming each place
/// tried and what it lacks.
pub fn locate(candidates: &[PathBuf], front: bool) -> std::result::Result<PathBuf, String> {
    let mut tried = Vec::new();
    for c in candidates {
        let why = match GameDir::new(c) {
            Err(_) => "no such folder".to_string(),
            Ok(g) => {
                let m = g.missing(front);
                if m.is_empty() {
                    return Ok(c.clone());
                }
                format!("missing {}", m.join(", "))
            }
        };
        let line = format!("  {} ({why})", c.display());
        if !tried.contains(&line) {
            tried.push(line);
        }
    }
    Err(format!(
        "The Ultima I game files were not found.\n\nLooked in:\n{}\n\nNeeded: {}, and one display's tiles \
         (EGATILES.BIN + EGATOWN.BIN, T1KTILES.BIN + T1KTOWN.BIN or CGATILES.BIN + CGATOWN.BIN). \
         The other files of the game (SPACE.EXE, MONDAIN.EXE, NIF.BIN, CASTLE.16, the *FIGHT, *SPACE and *MOND tiles) are used when present.\n\n\
         Copy the files of your own copy of Ultima I into one of the folders above, put this program next to them \
         (or next to their ULTIMA1 folder), or start it with --game-dir <folder>.",
        tried.join("\n"),
        REQUIRED.join(", ")
    ))
}

/// The folder the engine keeps its own data in: `$XDG_DATA_HOME/ultima1-engine`,
/// else `$HOME/.local/share/ultima1-engine` (`%APPDATA%\ultima1-engine` on
/// Windows when neither is set).
pub fn data_dir_from(xdg: Option<PathBuf>, home: Option<PathBuf>, appdata: Option<PathBuf>) -> Option<PathBuf> {
    let xdg = xdg.filter(|p| p.is_absolute());
    let base = xdg.or_else(|| home.map(|h| h.join(".local/share"))).or(appdata)?;
    Some(base.join("ultima1-engine"))
}

/// [`data_dir_from`] for this process's environment.
pub fn data_dir() -> Option<PathBuf> {
    let var = |k| std::env::var_os(k).filter(|v| !v.is_empty()).map(PathBuf::from);
    data_dir_from(var("XDG_DATA_HOME"), var("HOME"), var("APPDATA"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locate_reports_each_place_and_what_it_lacks() {
        let root = std::env::temp_dir().join(format!("u1-locate-{}", std::process::id()));
        let empty = root.join("empty");
        let full = root.join("full");
        std::fs::create_dir_all(&empty).unwrap();
        std::fs::create_dir_all(full.join("ULTIMA1")).unwrap();
        for f in REQUIRED.iter().chain(&["egatiles.bin", "EGATOWN.BIN"]) {
            std::fs::write(full.join("ULTIMA1").join(f), b"x").unwrap();
        }
        let gone = root.join("gone");
        let e = locate(&[gone.clone(), empty.clone()], true).unwrap_err();
        assert!(e.contains(&format!("{} (no such folder)", gone.display())), "{e}");
        assert!(e.contains("missing OUT.EXE, MAP.BIN, TCD.BIN, ULTIMA.EXE, GEN.EXE, EGATILES.BIN"), "{e}");
        // The files may sit in an ULTIMA1 folder beside the program.
        assert_eq!(locate(&[empty.clone(), full.clone()], true).unwrap(), full);
        std::fs::remove_file(full.join("ULTIMA1/GEN.EXE")).unwrap();
        assert!(locate(std::slice::from_ref(&full), true).unwrap_err().contains("missing GEN.EXE"));
        assert_eq!(locate(std::slice::from_ref(&full), false).unwrap(), full);
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    #[cfg(unix)] // "/x/data" is not an absolute path on Windows.
    fn data_dir_follows_xdg() {
        let p = |s: &str| Some(PathBuf::from(s));
        assert_eq!(data_dir_from(p("/x/data"), p("/home/u"), None), p("/x/data/ultima1-engine"));
        assert_eq!(data_dir_from(p("rel"), p("/home/u"), None), p("/home/u/.local/share/ultima1-engine"));
        assert_eq!(data_dir_from(None, None, p("C:/AppData")), p("C:/AppData/ultima1-engine"));
        assert_eq!(data_dir_from(None, None, None), None);
    }

    #[test]
    fn finds_files_case_insensitively() {
        let dir = std::env::temp_dir().join(format!("u1-gamedir-test-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("ULTIMA1")).unwrap();
        // Synthetic placeholder data only; never real game files.
        std::fs::write(dir.join("ULTIMA1").join("map.bin"), vec![0x12u8; 13104]).unwrap();
        std::fs::write(dir.join("EgaTiles.Bin"), vec![0u8; 6656]).unwrap();
        let g = GameDir::new(&dir).unwrap();
        assert_eq!(g.detect_video_mode(), Some(VideoMode::Ega));
        let m = g.load_world_map().unwrap();
        assert_eq!(m.cell(0, 0), 1);
        assert_eq!(g.load_world_tiles(VideoMode::Ega).unwrap().len(), 52);
        assert!(g.load_world_tiles(VideoMode::Cga).is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
