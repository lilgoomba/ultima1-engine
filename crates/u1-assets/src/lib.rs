//! Clean-room asset loaders for Ultima I (the 1986 Origin DOS remake).
//!
//! Every format implemented here is derived from *public* community
//! documentation, cited in `docs/SPEC.md`. Nothing in this crate was derived
//! from disassembly or decompilation of the original executables, and no
//! original game data is embedded. Tests use synthetic data built from the
//! spec.

pub mod demo;
pub mod error;
pub mod gamedir;
pub mod map;
pub mod palette;
pub mod render;
pub mod save;
pub mod tcd;
pub mod tiles;

pub use error::{AssetError, Result};
