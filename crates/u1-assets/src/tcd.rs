//! Town and castle maps (`TCD.BIN`). See docs/SPEC.md §5.
//!
//! 10 maps of 38x18 one-byte cells, 6,840 bytes total. Each map is stored
//! column-major ("top to bottom, left to right"): 18 bytes for column 0,
//! then 18 bytes for column 1, ... Maps 0-1 are castles, 2-9 towns
//! (per Ultima Stones). Cell values index `???TOWN.BIN` tiles.
//! Column-major order verified against original data (SPEC §5).

use crate::error::{AssetError, Result};

pub const TCD_MAPS: usize = 10;
pub const TCD_WIDTH: usize = 38;
pub const TCD_HEIGHT: usize = 18;
pub const TCD_MAP_SIZE: usize = TCD_WIDTH * TCD_HEIGHT;
pub const TCD_BIN_SIZE: usize = TCD_MAP_SIZE * TCD_MAPS;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalMap {
    /// Row-major cells (converted from the file's column-major order).
    pub cells: Vec<u8>,
}

impl LocalMap {
    pub fn cell(&self, x: usize, y: usize) -> u8 {
        self.cells[y * TCD_WIDTH + x]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TownCastleMaps {
    pub maps: Vec<LocalMap>,
}

impl TownCastleMaps {
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() != TCD_BIN_SIZE {
            return Err(AssetError::BadSize {
                what: "TCD.BIN".into(),
                expected: TCD_BIN_SIZE.to_string(),
                got: data.len(),
            });
        }
        let maps = data
            .chunks_exact(TCD_MAP_SIZE)
            .map(|m| {
                let mut cells = vec![0u8; TCD_MAP_SIZE];
                for x in 0..TCD_WIDTH {
                    for y in 0..TCD_HEIGHT {
                        cells[y * TCD_WIDTH + x] = m[x * TCD_HEIGHT + y];
                    }
                }
                LocalMap { cells }
            })
            .collect();
        Ok(TownCastleMaps { maps })
    }

    /// Tile to draw for a cell value. Values 0..=50 are `???TOWN.BIN`
    /// indices; 51..=63 are floor cells that also carry a region code
    /// (shop counters, pub, prison, door gaps; SPEC §5) and draw as floor
    /// (tile 1). Verified against original data.
    pub fn display_tile(value: u8) -> usize {
        if value <= 50 {
            value as usize
        } else {
            1
        }
    }

    /// Region code carried by a floor cell (51..=63), if any.
    pub fn region_code(value: u8) -> Option<u8> {
        (51..=63).contains(&value).then_some(value)
    }

    pub fn is_castle(index: usize) -> bool {
        index < 2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn column_major_layout() {
        let mut data = vec![0u8; TCD_BIN_SIZE];
        // Map 3, x = 2, y = 5 -> offset 3*684 + 2*18 + 5
        data[3 * TCD_MAP_SIZE + 2 * TCD_HEIGHT + 5] = 0x17;
        data[TCD_HEIGHT - 1] = 0x01; // map 0, x=0, y=17
        let t = TownCastleMaps::parse(&data).unwrap();
        assert_eq!(t.maps.len(), 10);
        assert_eq!(t.maps[3].cell(2, 5), 0x17);
        assert_eq!(t.maps[0].cell(0, 17), 0x01);
        assert_eq!(t.maps[0].cell(1, 0), 0x00);
        assert!(TownCastleMaps::is_castle(1) && !TownCastleMaps::is_castle(2));
    }

    #[test]
    fn region_values_draw_as_floor() {
        assert_eq!(TownCastleMaps::display_tile(0), 0);
        assert_eq!(TownCastleMaps::display_tile(50), 50);
        for v in 51..=63 {
            assert_eq!(TownCastleMaps::display_tile(v), 1);
            assert_eq!(TownCastleMaps::region_code(v), Some(v));
        }
        assert_eq!(TownCastleMaps::region_code(7), None);
    }

    #[test]
    fn rejects_bad_size() {
        assert!(TownCastleMaps::parse(&[0u8; 6839]).is_err());
    }
}
