//! Overworld locations (towns, castles, landmarks, dungeons). See SPEC §4.1.
//!
//! The coordinate and name tables live in the original `OUT.EXE`; nothing is
//! embedded here. [`LocationTable::from_out_exe`] reads them at run time from
//! the user's own copy, using offsets documented in the spec for the
//! 69,858-byte `OUT.EXE` of the DOS release, and refuses other builds.

use crate::error::{AssetError, Result};

pub const LOCATION_COUNT: usize = 84;
pub const OUT_EXE_SIZE: usize = 69_858;
const X_TABLE: usize = 0xEF48;
const Y_TABLE: usize = 0xEF9C;
const NAME_PTRS: usize = 0x106FE;
const DATA_BASE: usize = 0xD680;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocationKind {
    Town,
    Castle,
    Landmark,
    Dungeon,
}

/// Kind of 1-based location id 1..=84.
pub fn kind_of(id: usize) -> Option<LocationKind> {
    match id {
        1..=32 => Some(LocationKind::Town),
        33..=40 => Some(LocationKind::Castle),
        41..=48 => Some(LocationKind::Landmark),
        49..=84 => Some(LocationKind::Dungeon),
        _ => None,
    }
}

/// `TCD.BIN` map used for a town (maps 2..=9) or castle (maps 0..=1).
pub fn tcd_map_for(id: usize) -> Option<usize> {
    match kind_of(id)? {
        LocationKind::Town => Some((id - 1) % 8 + 2),
        LocationKind::Castle => Some((id - 1) % 2),
        _ => None,
    }
}

/// Overworld terrain nibble expected under each kind of location.
pub fn terrain_for(kind: LocationKind) -> u8 {
    match kind {
        LocationKind::Castle => 4,
        LocationKind::Landmark => 5,
        LocationKind::Town => 6,
        LocationKind::Dungeon => 7,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Location {
    pub id: usize,
    pub x: u8,
    pub y: u8,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocationTable {
    pub locations: Vec<Location>,
}

impl LocationTable {
    /// Parse from separate X and Y byte arrays and a name list.
    pub fn from_parts(xs: &[u8], ys: &[u8], names: Vec<String>) -> Result<Self> {
        if xs.len() != LOCATION_COUNT || ys.len() != LOCATION_COUNT || names.len() != LOCATION_COUNT {
            return Err(AssetError::Invalid("location table needs 84 entries".into()));
        }
        let locations = names
            .into_iter()
            .enumerate()
            .map(|(i, name)| Location { id: i + 1, x: xs[i], y: ys[i], name })
            .collect();
        Ok(LocationTable { locations })
    }

    /// Read the tables from the user's `OUT.EXE` (see module docs).
    pub fn from_out_exe(exe: &[u8]) -> Result<Self> {
        if exe.len() != OUT_EXE_SIZE {
            return Err(AssetError::BadSize { what: "OUT.EXE".into(), expected: OUT_EXE_SIZE.to_string(), got: exe.len() });
        }
        let xs = &exe[X_TABLE..X_TABLE + LOCATION_COUNT];
        let ys = &exe[Y_TABLE..Y_TABLE + LOCATION_COUNT];
        let mut names = Vec::with_capacity(LOCATION_COUNT);
        for i in 0..LOCATION_COUNT {
            let p = NAME_PTRS + 2 * i;
            let off = DATA_BASE + u16::from_le_bytes([exe[p], exe[p + 1]]) as usize;
            let end = exe[off..].iter().position(|&b| b == 0).map(|e| off + e)
                .ok_or_else(|| AssetError::Invalid("unterminated location name".into()))?;
            names.push(String::from_utf8_lossy(&exe[off..end]).into_owned());
        }
        Self::from_parts(xs, ys, names)
    }

    /// The game's lookup: first entry whose coordinates match, as a 1-based
    /// id; 0 when nothing is there.
    pub fn id_at(&self, x: u8, y: u8) -> usize {
        self.locations.iter().find(|l| l.x == x && l.y == y).map_or(0, |l| l.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_and_maps() {
        assert_eq!(kind_of(1), Some(LocationKind::Town));
        assert_eq!(kind_of(33), Some(LocationKind::Castle));
        assert_eq!(kind_of(48), Some(LocationKind::Landmark));
        assert_eq!(kind_of(84), Some(LocationKind::Dungeon));
        assert_eq!(kind_of(0), None);
        assert_eq!(tcd_map_for(1), Some(2));
        assert_eq!(tcd_map_for(9), Some(2));
        assert_eq!(tcd_map_for(8), Some(9));
        assert_eq!(tcd_map_for(33), Some(0));
        assert_eq!(tcd_map_for(34), Some(1));
        assert_eq!(tcd_map_for(50), None);
    }

    #[test]
    fn synthetic_exe_roundtrip() {
        let mut exe = vec![0u8; OUT_EXE_SIZE];
        let str_area = 0x2000usize; // DS-relative
        for i in 0..LOCATION_COUNT {
            exe[X_TABLE + i] = i as u8;
            exe[Y_TABLE + i] = 200 - i as u8;
            let name = format!("Loc{i}");
            let ds = str_area + i * 8;
            exe[DATA_BASE + ds..DATA_BASE + ds + name.len()].copy_from_slice(name.as_bytes());
            exe[NAME_PTRS + 2 * i..NAME_PTRS + 2 * i + 2].copy_from_slice(&(ds as u16).to_le_bytes());
        }
        let t = LocationTable::from_out_exe(&exe).unwrap();
        assert_eq!(t.locations[5].name, "Loc5");
        assert_eq!(t.id_at(5, 195), 6);
        assert_eq!(t.id_at(5, 5), 0);
        assert!(LocationTable::from_out_exe(&exe[1..]).is_err());
    }
}
