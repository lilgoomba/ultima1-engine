//! The 8x8 text font.
//!
//! The original prints all text through the PC BIOS (int 10h teletype), so
//! its glyphs are the machine's ROM font, which neither the game files nor
//! this repository contain. The font is loaded at run time instead
//! ([`Font::parse`]): a user-supplied file (a raw dump of 8-byte glyphs, a
//! PSF1/PSF2 font with 8x8 glyphs, or a Unifont-style `.hex` file), else the
//! public-domain unscii-8 lookalike in `fonts/` ([`Font::fallback`]).
//! Glyphs are indexed by code page 437 byte; bit 7 is the leftmost pixel.

use std::sync::OnceLock;

/// A 256-glyph 8x8 font.
#[derive(Clone, PartialEq, Eq)]
pub struct Font {
    pub glyphs: Box<[[u8; 8]; 256]>,
}

/// Unicode for the CP437 bytes 0x01..=0x1F (used when reading `.hex`).
const CP437_LOW: [u32; 31] = [
    0x263A, 0x263B, 0x2665, 0x2666, 0x2663, 0x2660, 0x2022, 0x25D8, 0x25CB, 0x25D9, 0x2642, 0x2640, 0x266A, 0x266B, 0x263C, 0x25BA, 0x25C4, 0x2195, 0x203C, 0x00B6, 0x00A7, 0x25AC, 0x21A8, 0x2191,
    0x2193, 0x2192, 0x2190, 0x221F, 0x2194, 0x25B2, 0x25BC,
];

const FALLBACK_HEX: &str = include_str!("../fonts/unscii-8-subset.hex");

impl Font {
    fn empty() -> Self {
        Font { glyphs: Box::new([[0; 8]; 256]) }
    }

    /// Read a font file's bytes: PSF1, PSF2, `.hex` text, or a raw dump of
    /// 8-byte glyphs (128 or more; a longer dump is cut at 256).
    pub fn parse(b: &[u8]) -> Result<Font, String> {
        let mut f = Font::empty();
        let raw: &[u8];
        if b.len() >= 4 && b[0] == 0x36 && b[1] == 0x04 {
            if b[3] != 8 {
                return Err(format!("PSF1 font is 8x{}, need 8x8", b[3]));
            }
            raw = &b[4..];
        } else if b.len() >= 32 && b[..4] == [0x72, 0xB5, 0x4A, 0x86] {
            let u = |i: usize| u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]) as usize;
            let (hdr, n, size, h, w) = (u(8), u(16), u(20), u(24), u(28));
            if h != 8 || w != 8 || size != 8 {
                return Err(format!("PSF2 font is {w}x{h}, need 8x8"));
            }
            raw = b.get(hdr..hdr + n.min(256) * 8).ok_or("short PSF2 font")?;
        } else if let Some(t) = std::str::from_utf8(b).ok().filter(|t| t.contains(':')) {
            return Self::parse_hex(t);
        } else {
            raw = b;
        }
        if raw.len() < 128 * 8 {
            return Err(format!("font has {} bytes, need at least 1024 (128 8x8 glyphs)", raw.len()));
        }
        for (i, g) in raw.chunks_exact(8).take(256).enumerate() {
            f.glyphs[i].copy_from_slice(g);
        }
        Ok(f)
    }

    /// Unifont `.hex` lines `CODEPOINT:16HEXDIGITS`, placed by CP437 byte.
    fn parse_hex(t: &str) -> Result<Font, String> {
        let mut f = Font::empty();
        let mut n = 0;
        for line in t.lines() {
            let Some((cp, hex)) = line.trim().split_once(':') else { continue };
            let (Ok(cp), 16) = (u32::from_str_radix(cp, 16), hex.len()) else { continue };
            let byte = match cp {
                0x20..=0x7E => cp as usize,
                _ => match CP437_LOW.iter().position(|&u| u == cp) {
                    Some(i) => i + 1,
                    None => continue,
                },
            };
            for r in 0..8 {
                f.glyphs[byte][r] = u8::from_str_radix(&hex[2 * r..2 * r + 2], 16).map_err(|e| e.to_string())?;
            }
            n += 1;
        }
        if n < 95 {
            return Err(format!("hex font has only {n} usable glyphs"));
        }
        Ok(f)
    }

    /// The bundled public-domain unscii-8 glyphs (see `fonts/README.md`).
    pub fn fallback() -> Font {
        Self::parse_hex(FALLBACK_HEX).expect("bundled font")
    }
}

static FONT: OnceLock<Font> = OnceLock::new();

/// Make `f` the font for all text; only the first call has an effect.
pub fn install(f: Font) {
    let _ = FONT.set(f);
}

/// The font in use (the fallback unless [`install`] ran first).
pub fn font() -> &'static Font {
    FONT.get_or_init(Font::fallback)
}

/// The CP437 byte the game would print for `c`.
pub fn cp437(c: char) -> u8 {
    match c as u32 {
        n @ 0..=0xFF => n as u8,
        n => CP437_LOW.iter().position(|&u| u == n).map_or(b'?', |i| i as u8 + 1),
    }
}

/// The 8 rows of an 8x8 cell for `c`. Bit 7 is the leftmost pixel.
pub fn glyph(c: char) -> [u8; 8] {
    font().glyphs[cp437(c) as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_has_distinct_printable_glyphs() {
        let f = Font::fallback();
        assert_eq!(f.glyphs[b' ' as usize], [0; 8]);
        for a in 0x21..0x7F {
            assert!(f.glyphs[a].iter().any(|&r| r != 0), "{a:#x}");
            for b in a + 1..0x7F {
                assert_ne!(f.glyphs[a], f.glyphs[b], "{a:#x} {b:#x}");
            }
        }
        assert!(f.glyphs[0x10].iter().any(|&r| r != 0));
    }

    #[test]
    fn raw_and_psf_loaders() {
        let mut raw = vec![0u8; 2048];
        raw[b'A' as usize * 8] = 0x81;
        assert_eq!(Font::parse(&raw).unwrap().glyphs[b'A' as usize][0], 0x81);
        let mut psf = vec![0x36, 0x04, 0, 8];
        psf.extend(&raw[..2048]);
        assert_eq!(Font::parse(&psf).unwrap().glyphs[b'A' as usize][0], 0x81);
        assert!(Font::parse(&raw[..100]).is_err());
        assert_eq!(cp437('►'), 0x10);
    }
}
