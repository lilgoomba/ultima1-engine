//! Asset loaders and game rules for Ultima I (the 1986 Origin DOS remake).
//!
//! Everything here is written fresh from the plain-language specification in
//! `docs/SPEC.md`, which is built from public documentation, observation of
//! the original game, and (for items marked "verified from original
//! executable") reference study of the original program. No original code
//! or game data is embedded; tests use synthetic data built from the spec.

pub mod demo;
pub mod dungeon;
pub mod error;
pub mod gamedir;
pub mod images;
pub mod locations;
pub mod map;
pub mod overworld;
pub mod palette;
pub mod render;
pub mod rng;
pub mod rules;
pub mod save;
pub mod tcd;
pub mod tiles;

pub use error::{AssetError, Result};
