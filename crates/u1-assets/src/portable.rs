//! Portable mode: everything the engine writes lives beside the program.
//!
//! The Windows portable ZIP ships a `portable.txt` marker next to `u1.exe`
//! (`--portable` does the same for one run). With it, the configuration is
//! `<program folder>\config.toml`, characters and saves are in
//! `<program folder>\saves`, the asset pack is `<program folder>\assets\enhanced`
//! and the game files are looked for only in the program folder (or its
//! `ULTIMA1` folder) unless `--game-dir` or `U1_GAME_DIR` names one. Nothing
//! depends on the current folder, and without the marker every path is the
//! usual one. The game files themselves are only ever read.

use std::path::{Path, PathBuf};

/// The marker file that turns portable mode on beside the program.
pub const MARKER: &str = "portable.txt";

/// Where a portable copy keeps its files, all inside `root`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Portable {
    pub root: PathBuf,
}

impl Portable {
    /// Portable mode for a program in `exe_dir`: on when `forced`
    /// (`--portable`) or when the marker sits beside the program.
    pub fn detect(exe_dir: Option<&Path>, forced: bool) -> Option<Portable> {
        let root = exe_dir?;
        (forced || root.join(MARKER).is_file()).then(|| Portable { root: root.to_path_buf() })
    }

    /// [`Portable::detect`] for this process (the folder of its executable,
    /// never the current folder).
    pub fn for_this_program(forced: bool) -> Option<Portable> {
        let exe = std::env::current_exe().ok()?;
        let exe = exe.canonicalize().unwrap_or(exe);
        Portable::detect(exe.parent(), forced)
    }

    pub fn config_path(&self) -> PathBuf {
        self.root.join("config.toml")
    }

    pub fn saves_dir(&self) -> PathBuf {
        self.root.join("saves")
    }

    pub fn pack_dir(&self) -> PathBuf {
        self.root.join("assets").join("enhanced")
    }

    /// Where to look for the game files: `U1_GAME_DIR` if set, then the
    /// program folder (each also with its `ULTIMA1` subfolder).
    pub fn game_dir_candidates(&self, env_dir: Option<PathBuf>) -> Vec<PathBuf> {
        env_dir.into_iter().chain(std::iter::once(self.root.clone())).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("u1-portable-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn marker_or_flag_turns_it_on_and_paths_stay_beside_the_program() {
        let d = tmp("on");
        assert_eq!(Portable::detect(Some(&d), false), None, "no marker: normal paths");
        let forced = Portable::detect(Some(&d), true).unwrap();
        std::fs::write(d.join(MARKER), b"").unwrap();
        let p = Portable::detect(Some(&d), false).unwrap();
        assert_eq!(p, forced);
        assert_eq!(p.config_path(), d.join("config.toml"));
        assert_eq!(p.saves_dir(), d.join("saves"));
        assert_eq!(p.pack_dir(), d.join("assets").join("enhanced"));
        for f in [p.config_path(), p.saves_dir(), p.pack_dir()] {
            assert!(f.starts_with(&d), "{} outside the program folder", f.display());
        }
        assert_eq!(p.game_dir_candidates(None), vec![d.clone()]);
        let env = d.join("elsewhere");
        assert_eq!(p.game_dir_candidates(Some(env.clone())), vec![env, d.clone()]);
        assert_eq!(Portable::detect(None, true), None);
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn a_marker_in_the_current_folder_does_not_count() {
        let exe = tmp("exe");
        let cwd = tmp("cwd");
        std::fs::write(cwd.join(MARKER), b"").unwrap();
        assert_eq!(Portable::detect(Some(&exe), false), None);
        std::fs::remove_dir_all(&exe).unwrap();
        std::fs::remove_dir_all(&cwd).unwrap();
    }
}
