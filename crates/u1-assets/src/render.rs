//! Software renderer: composes an RGB24 frame from a map and a tile set.
//! Kept free of SDL so it can be unit-tested and used for screenshots.

use crate::map::{Terrain, WorldMap};
use crate::palette::Rgb;
use crate::tiles::TileSet;

/// Render a `cols` x `rows` cell window whose top-left cell is
/// (`left`, `top`) into an RGB24 buffer of
/// `(cols * tiles.width) x (rows * tiles.height)` pixels.
/// Cells outside the map are drawn black. `frame` selects the animation
/// frame for animated terrain (castle, town).
#[allow(clippy::too_many_arguments)]
pub fn render_world(
    map: &WorldMap,
    tiles: &TileSet,
    palette: &[Rgb; 16],
    left: isize,
    top: isize,
    cols: usize,
    rows: usize,
    frame: usize,
) -> Vec<u8> {
    let (tw, th) = (tiles.width, tiles.height);
    let pitch = cols * tw * 3;
    let mut out = vec![0u8; pitch * rows * th];
    for cy in 0..rows {
        for cx in 0..cols {
            let mx = left + cx as isize;
            let my = top + cy as isize;
            if mx < 0 || my < 0 || mx as usize >= map.width || my as usize >= map.height {
                continue;
            }
            let t = Terrain::from_cell(map.cell(mx as usize, my as usize)).tile_for_frame(frame);
            if t >= tiles.len() {
                continue;
            }
            let tile = &tiles.tiles[t];
            for py in 0..th {
                let row = (cy * th + py) * pitch + cx * tw * 3;
                for px in 0..tw {
                    let c = palette[(tile[py * tw + px] & 0x0F) as usize];
                    out[row + px * 3..row + px * 3 + 3].copy_from_slice(&c);
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::palette::EGA_PALETTE;

    #[test]
    fn renders_expected_colours() {
        let tiles = TileSet {
            width: 2,
            height: 2,
            tiles: (0..10).map(|i| vec![i as u8; 4]).collect(),
        };
        let mut bytes = vec![0u8; 84 * 2];
        bytes[0] = 0x31; // (0,0)=water? no: high nibble 3 = mountain, low 1 = grass
        let map = WorldMap::parse_lenient(&bytes).unwrap();
        let fb = render_world(&map, &tiles, &EGA_PALETTE, 0, 0, 3, 1, 0);
        assert_eq!(fb.len(), 3 * 2 * 3 * 2);
        assert_eq!(&fb[0..3], &EGA_PALETTE[3]); // mountain -> tile 3
        assert_eq!(&fb[6..9], &EGA_PALETTE[1]); // grass -> tile 1
        // Off-map rendering stays black.
        let fb = render_world(&map, &tiles, &EGA_PALETTE, -1, 0, 1, 1, 0);
        assert!(fb.iter().all(|&b| b == 0));
    }
}
