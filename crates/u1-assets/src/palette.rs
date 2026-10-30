//! Display palettes.
//!
//! The EGA palette below is the standard IBM EGA/CGA 16-colour RGBI default
//! palette (public hardware documentation), not data taken from the game.

/// An RGB triple.
pub type Rgb = [u8; 3];

/// Default 16-colour EGA palette, indexed by IRGB value
/// (bit0 = blue, bit1 = green, bit2 = red, bit3 = intensity).
pub const EGA_PALETTE: [Rgb; 16] = [
    [0x00, 0x00, 0x00], // 0 black
    [0x00, 0x00, 0xAA], // 1 blue
    [0x00, 0xAA, 0x00], // 2 green
    [0x00, 0xAA, 0xAA], // 3 cyan
    [0xAA, 0x00, 0x00], // 4 red
    [0xAA, 0x00, 0xAA], // 5 magenta
    [0xAA, 0x55, 0x00], // 6 brown
    [0xAA, 0xAA, 0xAA], // 7 light grey
    [0x55, 0x55, 0x55], // 8 dark grey
    [0x55, 0x55, 0xFF], // 9 light blue
    [0x55, 0xFF, 0x55], // 10 light green
    [0x55, 0xFF, 0xFF], // 11 light cyan
    [0xFF, 0x55, 0x55], // 12 light red
    [0xFF, 0x55, 0xFF], // 13 light magenta
    [0xFF, 0xFF, 0x55], // 14 yellow
    [0xFF, 0xFF, 0xFF], // 15 white
];

/// CGA mode 4 palette 1, low intensity (black, cyan, magenta, light grey).
///
/// UNKNOWN / needs verification: which CGA palette/intensity the game
/// actually selects. ModdingWiki's tile viewer description uses this one.
pub const CGA_PALETTE_1_LOW: [Rgb; 4] = [
    [0x00, 0x00, 0x00],
    [0x00, 0xAA, 0xAA],
    [0xAA, 0x00, 0xAA],
    [0xAA, 0xAA, 0xAA],
];

/// Expand a 4-entry CGA palette into a 16-entry table so CGA tile sets
/// (pixel values 0..=3) can share the same renderer.
/// MONDAIN.EXE's CGA flash (`4031`): background white, palette 0
/// (green, red, brown).
pub const CGA_FLASH: [Rgb; 4] = [
    [0xFF, 0xFF, 0xFF],
    [0x00, 0xAA, 0x00],
    [0xAA, 0x00, 0x00],
    [0xAA, 0x55, 0x00],
];

pub fn cga_as_16(p: &[Rgb; 4]) -> [Rgb; 16] {
    let mut out = [[0u8; 3]; 16];
    out[..4].copy_from_slice(p);
    out
}
