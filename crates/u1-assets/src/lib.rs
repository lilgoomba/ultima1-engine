//! Asset loaders and game rules for Ultima I (the 1986 Origin DOS remake).
//!
//! Everything here is written fresh from the plain-language specification in
//! `docs/SPEC.md`, which is built from public documentation, observation of
//! the original game, and (for items marked "verified from original
//! executable") reference study of the original program. No original code
//! or game data is embedded; tests use synthetic data built from the spec.

pub mod config;
pub mod demo;
pub mod dgame;
pub mod dungeon;
pub mod dview;
pub mod enhanced;
pub mod error;
pub mod exe;
pub mod explore;
pub mod font;
pub mod front;
pub mod game;
pub mod gamedir;
pub mod gen;
pub mod images;
pub mod input;
pub mod io;
pub mod keymap;
pub mod locations;
pub mod map;
pub mod mondain;
pub mod msglog;
pub mod music;
pub mod options;
pub mod overworld;
pub mod palette;
pub mod portable;
pub mod qol;
pub mod render;
pub mod rng;
pub mod rules;
pub mod save;
pub mod shops;
pub mod slots;
pub mod sound;
pub mod soundpack;
pub mod space;
pub mod tcd;
pub mod term;
pub mod tiles;
pub mod title;
pub mod town;
pub mod views;

pub use error::{AssetError, Result};
