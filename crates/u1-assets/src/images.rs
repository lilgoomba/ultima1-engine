//! Full-screen images (SPEC §8). Encodings verified against original data
//! by pixel comparison with the game running in DOSBox.

use crate::error::{AssetError, Result};

pub const SCREEN_W: usize = 320;
pub const SCREEN_H: usize = 200;

/// A decoded indexed-colour image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexedImage {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u8>,
}

fn check(what: &str, data: &[u8], want: usize) -> Result<()> {
    if data.len() != want {
        return Err(AssetError::BadSize { what: what.into(), expected: want.to_string(), got: data.len() });
    }
    Ok(())
}

/// `CASTLE.16`: 320x200, packed 4 bits per pixel, high nibble = left pixel.
/// Values are *not* EGA colours; map them through [`TITLE_EGA_REMAP`].
pub fn decode_castle16(data: &[u8]) -> Result<IndexedImage> {
    check("CASTLE.16", data, SCREEN_W * SCREEN_H / 2)?;
    let pixels = data.iter().flat_map(|&b| [b >> 4, b & 0x0F]).collect();
    Ok(IndexedImage { width: SCREEN_W, height: SCREEN_H, pixels })
}

/// EGA colour shown for each `CASTLE.16` pixel value (observed in DOSBox).
pub const TITLE_EGA_REMAP: [u8; 16] = [0, 8, 7, 15, 15, 2, 10, 6, 12, 7, 13, 11, 1, 9, 7, 15];

/// `CASTLE.4`: a CGA video-memory dump. Bank 0 (offset 0) holds even
/// scanlines, bank 1 (offset 8192) odd scanlines; 80 bytes per line,
/// 2 bits per pixel, leftmost pixel in bits 7-6. Each bank is 8,000 bytes
/// followed by 192 bytes of padding.
pub fn decode_castle4(data: &[u8]) -> Result<IndexedImage> {
    check("CASTLE.4", data, 16_384)?;
    let mut pixels = vec![0u8; SCREEN_W * SCREEN_H];
    for y in 0..SCREEN_H {
        let base = (y & 1) * 8192 + (y >> 1) * 80;
        for xb in 0..80 {
            let b = data[base + xb];
            for k in 0..4 {
                pixels[y * SCREEN_W + xb * 4 + k] = (b >> (6 - 2 * k)) & 3;
            }
        }
    }
    Ok(IndexedImage { width: SCREEN_W, height: SCREEN_H, pixels })
}

/// `NIF.BIN`: 320x168, 1 bit per pixel, row-major, most significant bit
/// leftmost. Output values are 0 or 1.
pub fn decode_nif(data: &[u8]) -> Result<IndexedImage> {
    check("NIF.BIN", data, 40 * 168)?;
    let pixels = data.iter().flat_map(|&b| (0..8).map(move |k| (b >> (7 - k)) & 1)).collect();
    Ok(IndexedImage { width: SCREEN_W, height: 168, pixels })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn castle16_nibbles() {
        let mut d = vec![0u8; 32_000];
        d[0] = 0xA5;
        d[160] = 0x0F; // row 1, pixels 0..1
        let img = decode_castle16(&d).unwrap();
        assert_eq!(&img.pixels[0..2], &[0xA, 0x5]);
        assert_eq!(&img.pixels[320..322], &[0x0, 0xF]);
        assert!(decode_castle16(&d[1..]).is_err());
        assert_eq!(TITLE_EGA_REMAP[0], 0);
    }

    #[test]
    fn castle4_even_bank_first() {
        let mut d = vec![0u8; 16_384];
        d[0] = 0b1100_0001; // row 0: pixel0 = 3, pixel3 = 1
        d[8192] = 0b0010_0000; // row 1: pixel1 = 2
        d[80] = 0b0100_0000; // row 2: pixel0 = 1
        let img = decode_castle4(&d).unwrap();
        assert_eq!(&img.pixels[0..4], &[3, 0, 0, 1]);
        assert_eq!(img.pixels[320 + 1], 2);
        assert_eq!(img.pixels[640], 1);
    }

    #[test]
    fn nif_msb_left() {
        let mut d = vec![0u8; 6720];
        d[0] = 0x81;
        d[40] = 0x40;
        let img = decode_nif(&d).unwrap();
        assert_eq!(&img.pixels[0..8], &[1, 0, 0, 0, 0, 0, 0, 1]);
        assert_eq!(img.pixels[321], 1);
        assert_eq!(img.height, 168);
    }
}
