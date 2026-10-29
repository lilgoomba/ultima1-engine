//! Overworld map (`MAP.BIN`). See docs/SPEC.md §4.
//!
//! 4 bits per cell, two cells per byte, **high nibble first**, rows stored
//! left-to-right, top-to-bottom; 84 bytes per 168-cell row. The documented
//! file size is 13,104 bytes = 156 rows. (ModdingWiki states 168x168, which
//! would be 14,112 bytes; we infer the height from the file length so both
//! claims can be checked against a real copy.)

use crate::error::{AssetError, Result};

pub const WORLD_WIDTH: usize = 168;
pub const WORLD_HEIGHT: usize = 156;
pub const BYTES_PER_ROW: usize = WORLD_WIDTH / 2;
pub const MAP_BIN_SIZE: usize = BYTES_PER_ROW * WORLD_HEIGHT; // 13,104

/// Terrain class of a map cell value, per ModdingWiki's table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Terrain {
    Water,
    Grass,
    Forest,
    Mountain,
    Castle,
    Signpost,
    Town,
    DungeonEntrance,
    /// Values 8..=15 are not documented. UNKNOWN.
    Unknown(u8),
}

impl Terrain {
    pub fn from_cell(v: u8) -> Terrain {
        match v {
            0 => Terrain::Water,
            1 => Terrain::Grass,
            2 => Terrain::Forest,
            3 => Terrain::Mountain,
            4 => Terrain::Castle,
            5 => Terrain::Signpost,
            6 => Terrain::Town,
            7 => Terrain::DungeonEntrance,
            n => Terrain::Unknown(n),
        }
    }

    /// Base index of this terrain's graphic in `???TILES.BIN`, and the
    /// number of animation frames (consecutive tiles) documented for it.
    pub fn tile(self) -> (usize, usize) {
        match self {
            Terrain::Water => (0, 1),
            Terrain::Grass => (1, 1),
            Terrain::Forest => (2, 1),
            Terrain::Mountain => (3, 1),
            Terrain::Castle => (4, 2),
            Terrain::Signpost => (6, 1),
            Terrain::Town => (7, 2),
            Terrain::DungeonEntrance => (9, 1),
            Terrain::Unknown(n) => (n as usize, 1),
        }
    }

    /// Tile index for a given animation frame counter.
    pub fn tile_for_frame(self, frame: usize) -> usize {
        let (base, frames) = self.tile();
        base + frame % frames
    }
}

/// The decoded overworld: one value (0..=15) per cell, row-major.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldMap {
    pub width: usize,
    pub height: usize,
    pub cells: Vec<u8>,
}

impl WorldMap {
    /// Parse `MAP.BIN`, requiring the documented 13,104-byte size.
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() != MAP_BIN_SIZE {
            return Err(AssetError::BadSize {
                what: "MAP.BIN".into(),
                expected: MAP_BIN_SIZE.to_string(),
                got: data.len(),
            });
        }
        Self::parse_lenient(data)
    }

    /// Parse any whole number of 84-byte rows (height inferred).
    pub fn parse_lenient(data: &[u8]) -> Result<Self> {
        if data.is_empty() || data.len() % BYTES_PER_ROW != 0 {
            return Err(AssetError::BadSize {
                what: "MAP.BIN".into(),
                expected: format!("a non-zero multiple of {BYTES_PER_ROW}"),
                got: data.len(),
            });
        }
        let mut cells = Vec::with_capacity(data.len() * 2);
        for &b in data {
            cells.push(b >> 4);
            cells.push(b & 0x0F);
        }
        Ok(WorldMap { width: WORLD_WIDTH, height: data.len() / BYTES_PER_ROW, cells })
    }

    /// Re-pack into the on-disk format.
    pub fn to_bytes(&self) -> Vec<u8> {
        self.cells.chunks_exact(2).map(|p| (p[0] << 4) | (p[1] & 0x0F)).collect()
    }

    pub fn cell(&self, x: usize, y: usize) -> u8 {
        self.cells[y * self.width + x]
    }

    pub fn terrain(&self, x: usize, y: usize) -> Terrain {
        Terrain::from_cell(self.cell(x, y))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn high_nibble_is_first_cell() {
        // Johnny Wood's example: byte 0x10 = grass (1) then water (0).
        let mut data = vec![0u8; MAP_BIN_SIZE];
        data[0] = 0x10;
        let m = WorldMap::parse(&data).unwrap();
        assert_eq!((m.width, m.height), (168, 156));
        assert_eq!(m.terrain(0, 0), Terrain::Grass);
        assert_eq!(m.terrain(1, 0), Terrain::Water);
    }

    #[test]
    fn row_stride_is_84_bytes() {
        let mut data = vec![0u8; MAP_BIN_SIZE];
        data[84 * 10 + 5] = 0x73; // row 10, cells 10 and 11
        let m = WorldMap::parse(&data).unwrap();
        assert_eq!(m.terrain(10, 10), Terrain::DungeonEntrance);
        assert_eq!(m.terrain(11, 10), Terrain::Mountain);
        assert_eq!(m.terrain(167, 155), Terrain::Water);
    }

    #[test]
    fn round_trip_and_tiles() {
        let data: Vec<u8> = (0..MAP_BIN_SIZE).map(|i| (i * 31 % 256) as u8 & 0x77).collect();
        let m = WorldMap::parse(&data).unwrap();
        assert_eq!(m.to_bytes(), data);
        assert_eq!(Terrain::from_cell(5).tile(), (6, 1));
        assert_eq!(Terrain::Town.tile_for_frame(1), 8);
        assert_eq!(Terrain::Castle.tile_for_frame(2), 4);
        assert_eq!(Terrain::from_cell(12), Terrain::Unknown(12));
    }

    #[test]
    fn size_checks() {
        assert!(WorldMap::parse(&[0u8; 13103]).is_err());
        assert!(WorldMap::parse(&[0u8; 84 * 168]).is_err());
        let m = WorldMap::parse_lenient(&[0u8; 84 * 168]).unwrap();
        assert_eq!(m.height, 168);
        assert!(WorldMap::parse_lenient(&[0u8; 85]).is_err());
    }
}
