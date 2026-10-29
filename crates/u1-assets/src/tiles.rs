//! Tile graphics decoders (`???TILES.BIN`, `???TOWN.BIN`, `???MOND.BIN`,
//! `???FIGHT.BIN`, `???SPACE.BIN`).
//!
//! Formats per docs/SPEC.md §3 (ModdingWiki "Ultima I Tile Graphic Format",
//! Ultima Stones/mithyr.com EGATILES.BIN / EGATOWN.BIN pages). None of the
//! files has a header and none is compressed; tile size and count are fixed
//! by the executable, so we carry them in [`TileFileSpec`].

use crate::error::{AssetError, Result};

/// Pixel encoding of a tile file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileEncoding {
    /// EGA row-planar: for each pixel row, `w/8` bytes of the blue plane,
    /// then `w/8` green, `w/8` red, `w/8` intensity. Within a byte the most
    /// significant bit is the leftmost pixel. Pixel value = I<<3|R<<2|G<<1|B.
    EgaRowPlanar,
    /// CGA linear: 4 pixels per byte, bits 7-6 are the leftmost pixel.
    CgaLinear,
    /// Tandy (T1K) linear: 2 pixels per byte, high nibble first.
    TandyLinear,
    /// Monochrome linear: 8 pixels per byte, MSB first; set bits are drawn
    /// in white (value 15), clear bits black (0).
    MonoLinear,
}

impl TileEncoding {
    /// Bytes required to store one `w`x`h` tile.
    pub fn bytes_per_tile(self, w: usize, h: usize) -> Result<usize> {
        let (align, bytes_per_row) = match self {
            TileEncoding::EgaRowPlanar => (8, w / 8 * 4),
            TileEncoding::CgaLinear => (4, w / 4),
            TileEncoding::TandyLinear => (2, w / 2),
            TileEncoding::MonoLinear => (8, w / 8),
        };
        if w == 0 || h == 0 || w % align != 0 {
            return Err(AssetError::Invalid(format!(
                "tile width {w} not a positive multiple of {align} for {self:?}"
            )));
        }
        Ok(bytes_per_row * h)
    }
}

/// Static description of one documented tile file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TileFileSpec {
    pub file_name: &'static str,
    pub encoding: TileEncoding,
    pub width: usize,
    pub height: usize,
    /// Number of tiles stored in the file (including any pre-shifted
    /// duplicates in FIGHT/SPACE files).
    pub count: usize,
}

impl TileFileSpec {
    pub fn expected_size(&self) -> usize {
        self.encoding
            .bytes_per_tile(self.width, self.height)
            .expect("static spec is valid")
            * self.count
    }
}

macro_rules! spec {
    ($name:ident, $file:expr, $enc:ident, $w:expr, $h:expr, $n:expr) => {
        pub const $name: TileFileSpec = TileFileSpec {
            file_name: $file,
            encoding: TileEncoding::$enc,
            width: $w,
            height: $h,
            count: $n,
        };
    };
}

// World (overworld) tiles: 52 x 16x16.
spec!(EGA_TILES, "EGATILES.BIN", EgaRowPlanar, 16, 16, 52);
spec!(CGA_TILES, "CGATILES.BIN", CgaLinear, 16, 16, 52);
spec!(T1K_TILES, "T1KTILES.BIN", TandyLinear, 16, 16, 52);
// Town/castle tiles: 51 x 8x8.
spec!(EGA_TOWN, "EGATOWN.BIN", EgaRowPlanar, 8, 8, 51);
spec!(CGA_TOWN, "CGATOWN.BIN", CgaLinear, 8, 8, 51);
spec!(T1K_TOWN, "T1KTOWN.BIN", TandyLinear, 8, 8, 51);
// Mondain final battle tiles: 19 x 16x16.
spec!(EGA_MOND, "EGAMOND.BIN", EgaRowPlanar, 16, 16, 19);
spec!(CGA_MOND, "CGAMOND.BIN", CgaLinear, 16, 16, 19);
spec!(T1K_MOND, "T1KMOND.BIN", TandyLinear, 16, 16, 19);
// Space combat fighters: 24x19, pre-shifted copies.
spec!(EGA_FIGHT, "EGAFIGHT.BIN", MonoLinear, 24, 19, 56);
spec!(CGA_FIGHT, "CGAFIGHT.BIN", CgaLinear, 24, 19, 28);
spec!(T1K_FIGHT, "T1KFIGHT.BIN", TandyLinear, 24, 19, 14);
// Space docking ships: 32x19, pre-shifted copies.
spec!(EGA_SPACE, "EGASPACE.BIN", MonoLinear, 32, 19, 96);
spec!(CGA_SPACE, "CGASPACE.BIN", CgaLinear, 32, 19, 48);
spec!(T1K_SPACE, "T1KSPACE.BIN", TandyLinear, 32, 19, 24);

/// All documented tile files.
pub const ALL_TILE_FILES: [TileFileSpec; 15] = [
    EGA_TILES, CGA_TILES, T1K_TILES, EGA_TOWN, CGA_TOWN, T1K_TOWN, EGA_MOND, CGA_MOND,
    T1K_MOND, EGA_FIGHT, CGA_FIGHT, T1K_FIGHT, EGA_SPACE, CGA_SPACE, T1K_SPACE,
];

/// A decoded set of equally sized tiles. Each tile is `width * height`
/// palette indices (0..=15 for EGA/Tandy/mono, 0..=3 for CGA), row-major.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TileSet {
    pub width: usize,
    pub height: usize,
    pub tiles: Vec<Vec<u8>>,
}

impl TileSet {
    /// Decode a headerless tile file. The data length must be an exact
    /// multiple of the per-tile size.
    pub fn decode(data: &[u8], encoding: TileEncoding, width: usize, height: usize) -> Result<Self> {
        let bpt = encoding.bytes_per_tile(width, height)?;
        if data.is_empty() || data.len() % bpt != 0 {
            return Err(AssetError::BadSize {
                what: format!("{encoding:?} {width}x{height} tile data"),
                expected: format!("a non-zero multiple of {bpt}"),
                got: data.len(),
            });
        }
        let tiles = data
            .chunks_exact(bpt)
            .map(|chunk| decode_tile(chunk, encoding, width, height))
            .collect();
        Ok(TileSet { width, height, tiles })
    }

    /// Decode a file according to a documented spec, enforcing its exact size.
    pub fn decode_spec(data: &[u8], spec: &TileFileSpec) -> Result<Self> {
        if data.len() != spec.expected_size() {
            return Err(AssetError::BadSize {
                what: spec.file_name.to_string(),
                expected: spec.expected_size().to_string(),
                got: data.len(),
            });
        }
        Self::decode(data, spec.encoding, spec.width, spec.height)
    }

    pub fn len(&self) -> usize {
        self.tiles.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tiles.is_empty()
    }

    /// Pixel value at (x, y) of tile `idx`.
    pub fn pixel(&self, idx: usize, x: usize, y: usize) -> u8 {
        self.tiles[idx][y * self.width + x]
    }
}

fn decode_tile(chunk: &[u8], enc: TileEncoding, w: usize, h: usize) -> Vec<u8> {
    let mut out = vec![0u8; w * h];
    match enc {
        TileEncoding::EgaRowPlanar => {
            let pb = w / 8; // bytes per plane per row
            for y in 0..h {
                let row = &chunk[y * pb * 4..(y + 1) * pb * 4];
                for x in 0..w {
                    let byte = x / 8;
                    let bit = 7 - (x % 8);
                    let mut v = 0u8;
                    for plane in 0..4 {
                        // plane 0 = blue (bit0), 1 = green, 2 = red, 3 = intensity
                        if row[plane * pb + byte] >> bit & 1 == 1 {
                            v |= 1 << plane;
                        }
                    }
                    out[y * w + x] = v;
                }
            }
        }
        TileEncoding::CgaLinear => {
            for (i, px) in out.iter_mut().enumerate() {
                let b = chunk[i / 4];
                *px = (b >> (6 - 2 * (i % 4))) & 0x03;
            }
        }
        TileEncoding::TandyLinear => {
            for (i, px) in out.iter_mut().enumerate() {
                let b = chunk[i / 2];
                *px = if i % 2 == 0 { b >> 4 } else { b & 0x0F };
            }
        }
        TileEncoding::MonoLinear => {
            for (i, px) in out.iter_mut().enumerate() {
                let b = chunk[i / 8];
                *px = if (b >> (7 - i % 8)) & 1 == 1 { 15 } else { 0 };
            }
        }
    }
    out
}

/// Encode one tile (row-major palette indices) into the given encoding.
/// Used to build synthetic test fixtures and the `--demo` tile set; it is
/// the exact inverse of the decoder.
pub fn encode_tile(pixels: &[u8], enc: TileEncoding, w: usize, h: usize) -> Result<Vec<u8>> {
    let bpt = enc.bytes_per_tile(w, h)?;
    if pixels.len() != w * h {
        return Err(AssetError::Invalid(format!(
            "expected {} pixels, got {}",
            w * h,
            pixels.len()
        )));
    }
    let mut out = vec![0u8; bpt];
    match enc {
        TileEncoding::EgaRowPlanar => {
            let pb = w / 8;
            for y in 0..h {
                for x in 0..w {
                    let v = pixels[y * w + x];
                    for plane in 0..4 {
                        if v >> plane & 1 == 1 {
                            out[y * pb * 4 + plane * pb + x / 8] |= 1 << (7 - x % 8);
                        }
                    }
                }
            }
        }
        TileEncoding::CgaLinear => {
            for (i, &v) in pixels.iter().enumerate() {
                out[i / 4] |= (v & 3) << (6 - 2 * (i % 4));
            }
        }
        TileEncoding::TandyLinear => {
            for (i, &v) in pixels.iter().enumerate() {
                out[i / 2] |= if i % 2 == 0 { (v & 0xF) << 4 } else { v & 0xF };
            }
        }
        TileEncoding::MonoLinear => {
            for (i, &v) in pixels.iter().enumerate() {
                if v != 0 {
                    out[i / 8] |= 1 << (7 - i % 8);
                }
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn documented_file_sizes_match_spec() {
        // Sizes from the public file list (gigi.nullneuron.net "Game Files").
        let expected = [
            ("EGATILES.BIN", 6656),
            ("CGATILES.BIN", 3328),
            ("T1KTILES.BIN", 6656),
            ("EGATOWN.BIN", 1632),
            ("CGATOWN.BIN", 816),
            ("T1KTOWN.BIN", 1632),
            ("EGAMOND.BIN", 2432),
            ("CGAMOND.BIN", 1216),
            ("T1KMOND.BIN", 2432),
            ("EGAFIGHT.BIN", 3192),
            ("CGAFIGHT.BIN", 3192),
            ("T1KFIGHT.BIN", 3192),
            ("EGASPACE.BIN", 7296),
            ("CGASPACE.BIN", 7296),
            ("T1KSPACE.BIN", 7296),
        ];
        for (name, size) in expected {
            let spec = ALL_TILE_FILES.iter().find(|s| s.file_name == name).unwrap();
            assert_eq!(spec.expected_size(), size, "{name}");
        }
    }

    #[test]
    fn ega_row_planar_handcrafted_row() {
        // One 16x1 "tile": BB GG RR II.
        // Blue plane: 0b1000_0000 0b0000_0001 -> pixel 0 and 15 have blue.
        // Green plane: 0b1100_0000 0 -> pixels 0,1 green.
        // Red plane:   0 0
        // Intensity:   0b1111_1111 0b0000_0000 -> pixels 0..8 bright.
        let data = [0x80, 0x01, 0xC0, 0x00, 0x00, 0x00, 0xFF, 0x00];
        let ts = TileSet::decode(&data, TileEncoding::EgaRowPlanar, 16, 1).unwrap();
        let t = &ts.tiles[0];
        assert_eq!(t[0], 0b1011); // I + G + B = 11 light cyan
        assert_eq!(t[1], 0b1010); // I + G = 10 light green
        assert_eq!(t[2], 0b1000); // I only = 8 dark grey
        assert_eq!(t[8], 0);
        assert_eq!(t[15], 0b0001); // blue
    }

    #[test]
    fn ega_town_8px_layout_is_bgri_bytes() {
        // 8x1: one byte per plane, B G R I.
        let data = [0b0000_0001, 0b0000_0001, 0b0000_0001, 0b0000_0001];
        let ts = TileSet::decode(&data, TileEncoding::EgaRowPlanar, 8, 1).unwrap();
        assert_eq!(ts.tiles[0][7], 15);
        assert!(ts.tiles[0][..7].iter().all(|&p| p == 0));
    }

    #[test]
    fn cga_linear_bit_order() {
        // 0b11_10_01_00 -> pixels 3,2,1,0
        let ts = TileSet::decode(&[0b1110_0100], TileEncoding::CgaLinear, 4, 1).unwrap();
        assert_eq!(ts.tiles[0], vec![3, 2, 1, 0]);
    }

    #[test]
    fn tandy_linear_high_nibble_first() {
        let ts = TileSet::decode(&[0xA5, 0x0F], TileEncoding::TandyLinear, 4, 1).unwrap();
        assert_eq!(ts.tiles[0], vec![0xA, 0x5, 0x0, 0xF]);
    }

    #[test]
    fn mono_linear_msb_first_white() {
        let ts = TileSet::decode(&[0b1000_0001], TileEncoding::MonoLinear, 8, 1).unwrap();
        assert_eq!(ts.tiles[0], vec![15, 0, 0, 0, 0, 0, 0, 15]);
    }

    #[test]
    fn round_trip_all_encodings() {
        for spec in ALL_TILE_FILES {
            let max = match spec.encoding {
                TileEncoding::CgaLinear => 4,
                TileEncoding::MonoLinear => 2,
                _ => 16,
            };
            let mut file = Vec::new();
            let mut originals = Vec::new();
            for t in 0..spec.count {
                let px: Vec<u8> = (0..spec.width * spec.height)
                    .map(|i| {
                        let v = ((i * 7 + t * 13) % max) as u8;
                        if spec.encoding == TileEncoding::MonoLinear && v == 1 { 15 } else { v }
                    })
                    .collect();
                file.extend(encode_tile(&px, spec.encoding, spec.width, spec.height).unwrap());
                originals.push(px);
            }
            assert_eq!(file.len(), spec.expected_size(), "{}", spec.file_name);
            let ts = TileSet::decode_spec(&file, &spec).unwrap();
            assert_eq!(ts.len(), spec.count);
            assert_eq!(ts.tiles, originals, "{}", spec.file_name);
        }
    }

    #[test]
    fn rejects_wrong_sizes() {
        assert!(TileSet::decode(&[0u8; 127], TileEncoding::EgaRowPlanar, 16, 16).is_err());
        assert!(TileSet::decode(&[], TileEncoding::EgaRowPlanar, 16, 16).is_err());
        assert!(TileSet::decode(&[0u8; 8], TileEncoding::EgaRowPlanar, 12, 1).is_err());
        assert!(TileSet::decode_spec(&[0u8; 6655], &EGA_TILES).is_err());
    }
}
