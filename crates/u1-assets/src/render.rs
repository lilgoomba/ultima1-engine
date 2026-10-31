//! Software renderer: composes an RGB24 frame from a map and a tile set.
//! Kept free of SDL so it can be unit-tested and used for screenshots.

use crate::map::{water_scroll, Terrain, WorldMap};
use crate::palette::Rgb;
use crate::tiles::TileSet;

/// Render a `cols` x `rows` cell window whose top-left cell is
/// (`left`, `top`) into an RGB24 buffer of
/// `(cols * tiles.width) x (rows * tiles.height)` pixels.
/// Cells outside the map are drawn black. `frame` is the animation tick
/// (about 0.165 s each in the original): it scrolls water and selects the
/// castle/town frames (SPEC §3.5).
#[allow(clippy::too_many_arguments)]
pub fn render_world(map: &WorldMap, tiles: &TileSet, palette: &[Rgb; 16], left: isize, top: isize, cols: usize, rows: usize, frame: usize) -> Vec<u8> {
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
            let terrain = Terrain::from_cell(map.cell(mx as usize, my as usize));
            let t = terrain.tile_for_frame(frame);
            let shift = if terrain == Terrain::Water { water_scroll(frame, tw) } else { 0 };
            if t >= tiles.len() {
                continue;
            }
            let tile = &tiles.tiles[t];
            for py in 0..th {
                let row = (cy * th + py) * pitch + cx * tw * 3;
                for px in 0..tw {
                    let sx = (px + shift) % tw;
                    let c = palette[(tile[py * tw + sx] & 0x0F) as usize];
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
        let tiles = TileSet { width: 2, height: 2, tiles: (0..10).map(|i| vec![i as u8; 4]).collect() };
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

    #[test]
    fn water_scrolls_left_four_pixels_per_tick() {
        // 16x1 water tile whose pixel value equals its column.
        let mut tiles: Vec<Vec<u8>> = (0..10).map(|_| vec![0u8; 16]).collect();
        tiles[0] = (0..16u8).collect();
        let tiles = TileSet { width: 16, height: 1, tiles };
        let map = WorldMap::parse_lenient(&[0u8; 84 * 2]).unwrap(); // all water
        for tick in 0..8 {
            let fb = render_world(&map, &tiles, &EGA_PALETTE, 0, 0, 1, 1, tick);
            for x in 0..16 {
                let want = EGA_PALETTE[(x + 4 * (tick % 4)) % 16];
                assert_eq!(&fb[x * 3..x * 3 + 3], &want, "tick {tick} x {x}");
            }
        }
    }

    #[test]
    fn castle_and_town_cadence() {
        use crate::map::Terrain;
        let castle: Vec<usize> = (0..12).map(|t| Terrain::Castle.tile_for_frame(t)).collect();
        assert_eq!(castle, [4, 4, 4, 5, 5, 5, 4, 4, 4, 5, 5, 5]);
        let town: Vec<usize> = (0..8).map(|t| Terrain::Town.tile_for_frame(t)).collect();
        assert_eq!(town, [7, 8, 8, 8, 7, 8, 8, 8]);
        assert_eq!(Terrain::Grass.tile_for_frame(5), 1);
    }
}
