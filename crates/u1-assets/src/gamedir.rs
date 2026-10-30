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
        [VideoMode::Ega, VideoMode::Tandy, VideoMode::Cga]
            .into_iter()
            .find(|m| self.find(m.world_tiles().file_name).is_some())
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

#[cfg(test)]
mod tests {
    use super::*;

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
