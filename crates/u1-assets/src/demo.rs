//! Procedurally generated placeholder assets for `--demo` mode.
//!
//! These are *original* placeholder graphics and a synthetic map, generated
//! at runtime from code. They are deliberately not copies of Origin's art.
//! The demo tiles are encoded into the documented EGA row-planar format and
//! the map into the documented MAP.BIN nibble format, so `--demo` exercises
//! the same parsers used for real game files.

use crate::map::{BYTES_PER_ROW, WORLD_HEIGHT, WORLD_WIDTH};
use crate::tiles::{encode_tile, TileEncoding, EGA_TILES};

const T: usize = 16;

fn hash(x: u32, y: u32, seed: u32) -> u32 {
    let mut h = x.wrapping_mul(0x8DA6_B343) ^ y.wrapping_mul(0xD816_3841) ^ seed.wrapping_mul(0xCB1A_B31F);
    h ^= h >> 13;
    h = h.wrapping_mul(0x5bd1_e995);
    h ^ (h >> 15)
}

struct Canvas(Vec<u8>);

impl Canvas {
    fn new(c: u8) -> Self {
        Canvas(vec![c; T * T])
    }
    fn set(&mut self, x: i32, y: i32, c: u8) {
        if (0..T as i32).contains(&x) && (0..T as i32).contains(&y) {
            self.0[y as usize * T + x as usize] = c;
        }
    }
    fn rect(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, c: u8) {
        for y in y0..=y1 {
            for x in x0..=x1 {
                self.set(x, y, c);
            }
        }
    }
    fn disc(&mut self, cx: i32, cy: i32, r: i32, c: u8) {
        for y in -r..=r {
            for x in -r..=r {
                if x * x + y * y <= r * r + r / 2 {
                    self.set(cx + x, cy + y, c);
                }
            }
        }
    }
    fn speckle(&mut self, seed: u32, one_in: u32, c: u8) {
        for y in 0..T as u32 {
            for x in 0..T as u32 {
                if hash(x, y, seed) % one_in == 0 {
                    self.set(x as i32, y as i32, c);
                }
            }
        }
    }
}

fn grass() -> Canvas {
    let mut c = Canvas::new(2);
    c.speckle(11, 7, 10);
    c
}

fn tile(index: usize) -> Vec<u8> {
    let c = match index {
        0 => {
            let mut c = Canvas::new(1);
            for y in (1..T as i32).step_by(4) {
                let off = (y * 5) % 8;
                for x in 0..4 {
                    c.set(off + x, y, 9);
                    c.set((off + x + 9) % T as i32, y + 2, 9);
                }
            }
            c
        }
        1 => grass(),
        2 => {
            let mut c = grass();
            for (cx, cy) in [(4, 5), (11, 4), (8, 11)] {
                c.rect(cx, cy + 2, cx, cy + 4, 6);
                c.disc(cx, cy, 3, 10);
                c.set(cx - 1, cy - 1, 2);
                c.set(cx + 1, cy + 1, 2);
            }
            c
        }
        3 => {
            let mut c = grass();
            for y in 2..15 {
                let half = (y - 2) / 2 + 1;
                for x in 8 - half..=7 + half {
                    let col = if y < 6 { 15 } else if x < 8 { 7 } else { 8 };
                    c.set(x, y, col);
                }
            }
            c
        }
        4 | 5 => {
            let mut c = grass();
            c.rect(2, 6, 13, 14, 7);
            for x in (2..14).step_by(3) {
                c.rect(x, 4, x + 1, 5, 7);
            }
            c.rect(6, 10, 9, 14, 0);
            c.rect(7, 0, 7, 4, 8);
            let flag = if index == 4 { 12 } else { 14 };
            c.rect(8, 0, 10, 1, flag);
            c
        }
        6 => {
            let mut c = grass();
            c.rect(7, 6, 8, 15, 6);
            c.rect(3, 3, 12, 7, 14);
            c.rect(4, 5, 11, 5, 6);
            c
        }
        7 | 8 => {
            let mut c = grass();
            for (x0, y0) in [(1, 3), (8, 2), (4, 9)] {
                c.rect(x0, y0 + 2, x0 + 5, y0 + 5, 7);
                c.rect(x0, y0, x0 + 5, y0 + 1, 4);
                let win = if index == 7 { 14 } else { 0 };
                c.set(x0 + 2, y0 + 3, win);
                c.set(x0 + 3, y0 + 3, win);
            }
            c
        }
        9 => {
            let mut c = grass();
            c.disc(8, 9, 6, 8);
            c.disc(8, 10, 3, 0);
            c.rect(5, 11, 11, 15, 0);
            c.rect(2, 14, 14, 15, 8);
            c.rect(6, 12, 10, 15, 0);
            c
        }
        n => {
            // Generic placeholder figure for vehicles / monsters.
            let mut c = Canvas::new(0);
            let col = (n % 6 + 9) as u8;
            c.disc(8, 4, 2, col);
            c.rect(6, 7, 10, 11, col);
            c.rect(6, 12, 7, 15, col);
            c.rect(9, 12, 10, 15, col);
            c.rect(4, 7, 5, 10, col);
            c.rect(11, 7, 12, 10, col);
            c
        }
    };
    c.0
}

/// 52 placeholder 16x16 tiles encoded as a synthetic EGA row-planar file
/// (6,656 bytes, same layout as EGATILES.BIN).
pub fn demo_egatiles_bin() -> Vec<u8> {
    (0..EGA_TILES.count)
        .flat_map(|i| encode_tile(&tile(i), TileEncoding::EgaRowPlanar, T, T).expect("valid tile"))
        .collect()
}

fn value_noise(x: f32, y: f32, cell: f32, seed: u32) -> f32 {
    let gx = (x / cell).floor();
    let gy = (y / cell).floor();
    let fx = x / cell - gx;
    let fy = y / cell - gy;
    let r = |ix: f32, iy: f32| (hash(ix as i32 as u32, iy as i32 as u32, seed) % 1000) as f32 / 999.0;
    let s = |t: f32| t * t * (3.0 - 2.0 * t);
    let (sx, sy) = (s(fx), s(fy));
    let a = r(gx, gy) + (r(gx + 1.0, gy) - r(gx, gy)) * sx;
    let b = r(gx, gy + 1.0) + (r(gx + 1.0, gy + 1.0) - r(gx, gy + 1.0)) * sx;
    a + (b - a) * sy
}

/// A synthetic 168x156 overworld with four continents (one per quadrant,
/// echoing the four lands of the game), packed into MAP.BIN format.
pub fn demo_map_bin(seed: u32) -> Vec<u8> {
    let (w, h) = (WORLD_WIDTH, WORLD_HEIGHT);
    let mut cells = vec![0u8; w * h];
    for y in 0..h {
        for x in 0..w {
            let (fx, fy) = (x as f32, y as f32);
            let n = 0.55 * value_noise(fx, fy, 24.0, seed)
                + 0.3 * value_noise(fx, fy, 9.0, seed ^ 0xA5A5)
                + 0.15 * value_noise(fx, fy, 4.0, seed ^ 0x5A5A);
            // Distance to the centre of this quadrant -> island falloff.
            let qx = (x % (w / 2)) as f32 / (w / 2) as f32 - 0.5;
            let qy = (y % (h / 2)) as f32 / (h / 2) as f32 - 0.5;
            let d = (qx * qx + qy * qy).sqrt() * 2.0;
            let e = n + 0.32 - d * 0.85;
            cells[y * w + x] = if e < 0.3 {
                0
            } else if e < 0.5 {
                1
            } else if e < 0.64 {
                2
            } else {
                3
            };
        }
    }
    // Sprinkle landmarks on grass.
    for (i, kind) in [(0u32, 4u8), (1, 6), (2, 7), (3, 5)] {
        let mut placed = 0;
        let mut k = 0u32;
        while placed < 24 && k < 20_000 {
            let x = (hash(k, i, seed) % w as u32) as usize;
            let y = (hash(i, k, seed ^ 77) % h as u32) as usize;
            k += 1;
            if cells[y * w + x] == 1 {
                cells[y * w + x] = kind;
                placed += 1;
            }
        }
    }
    let bytes: Vec<u8> = cells.chunks_exact(2).map(|p| (p[0] << 4) | p[1]).collect();
    debug_assert_eq!(bytes.len(), BYTES_PER_ROW * h);
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::{Terrain, WorldMap};
    use crate::tiles::TileSet;

    #[test]
    fn demo_assets_parse_with_real_parsers() {
        let tiles = TileSet::decode_spec(&demo_egatiles_bin(), &EGA_TILES).unwrap();
        assert_eq!(tiles.len(), 52);
        assert_eq!(tiles.tiles[0], tile(0));
        let map = WorldMap::parse(&demo_map_bin(1)).unwrap();
        let count = |t: Terrain| (0..map.height)
            .flat_map(|y| (0..map.width).map(move |x| (x, y)))
            .filter(|&(x, y)| map.terrain(x, y) == t)
            .count();
        assert!(count(Terrain::Water) > 1000);
        assert!(count(Terrain::Grass) > 500);
        assert!(count(Terrain::Town) > 0);
    }
}
