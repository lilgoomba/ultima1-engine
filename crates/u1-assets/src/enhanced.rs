//! The optional enhanced video mode (`--video enhanced`): an asset pack of
//! PNG pictures replaces original tiles and screens at a higher internal
//! resolution. The game itself is untouched: it still draws the original
//! 320x200 screen, and [`compose`] paints the pack's art over every region
//! that still shows a known original picture ([`crate::term::ArtOp`]).
//! Anything the pack lacks is the original screen scaled up
//! nearest-neighbour. See SPEC §20.
//!
//! A pack is a directory with `manifest.toml` (a small TOML subset):
//!
//! ```toml
//! name = "16-bit"
//! tile_size = 32          # pixels per 16-pixel original tile (16, 32, 48, 64)
//! license = "CC-BY-SA-4.0"
//!
//! [palette]               # free-form notes per video mode (not used)
//! ega = "Pack art ignores the 16-colour limit."
//!
//! [world.0]               # overworld tile 0: water
//! frames = ["world/water0.png", "world/water1.png"]
//! scroll = true           # follows the original's water scroll
//!
//! [world.1]               # grass: one of these per map cell, picked
//! variants = ["world/grass0.png", "world/grass1.png", "world/grass2.png"]
//!
//! [world.4]
//! file = "world/castle.png"
//! fps = 4                 # extra animation of a `frames` list, by clock
//!
//! [screen.nif]
//! file = "screens/nif.png"
//! ```
//!
//! Sections are `world.N`, `town.N`, `mond.N` (decimal or `0x` tile
//! numbers as in SPEC §3.2-3.4) and `screen.NAME`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::term::{ArtId, Term, H, W};

/// An RGBA picture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rgba {
    pub w: usize,
    pub h: usize,
    pub px: Vec<u8>,
}

impl Rgba {
    pub fn new(w: usize, h: usize) -> Self {
        Rgba { w, h, px: vec![0; w * h * 4] }
    }

    /// Decode a PNG (any colour type and depth the `png` crate expands to
    /// 8-bit gray/RGB/RGBA).
    pub fn from_png(data: &[u8]) -> Result<Rgba, String> {
        let mut dec = png::Decoder::new(std::io::Cursor::new(data));
        dec.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
        let mut r = dec.read_info().map_err(|e| e.to_string())?;
        let mut buf = vec![0; r.output_buffer_size()];
        let info = r.next_frame(&mut buf).map_err(|e| e.to_string())?;
        let (w, h) = (info.width as usize, info.height as usize);
        let src = &buf[..info.buffer_size()];
        let mut out = Rgba::new(w, h);
        let ch = match info.color_type {
            png::ColorType::Grayscale => 1,
            png::ColorType::GrayscaleAlpha => 2,
            png::ColorType::Rgb => 3,
            png::ColorType::Rgba => 4,
            png::ColorType::Indexed => return Err("indexed PNG not expanded".into()),
        };
        for i in 0..w * h {
            let s = &src[i * ch..i * ch + ch];
            let p = match ch {
                1 => [s[0], s[0], s[0], 255],
                2 => [s[0], s[0], s[0], s[1]],
                3 => [s[0], s[1], s[2], 255],
                _ => [s[0], s[1], s[2], s[3]],
            };
            out.px[i * 4..i * 4 + 4].copy_from_slice(&p);
        }
        Ok(out)
    }

    /// Nearest-neighbour resize (packs should supply exact sizes; this only
    /// keeps a wrong-sized file usable).
    pub fn resized(&self, w: usize, h: usize) -> Rgba {
        if (w, h) == (self.w, self.h) {
            return self.clone();
        }
        let mut o = Rgba::new(w, h);
        for y in 0..h {
            for x in 0..w {
                let s = ((y * self.h / h) * self.w + x * self.w / w) * 4;
                o.px[(y * w + x) * 4..(y * w + x) * 4 + 4].copy_from_slice(&self.px[s..s + 4]);
            }
        }
        o
    }
}

/// A parsed manifest value.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Str(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    List(Vec<Value>),
}

/// `manifest.toml` as tables of keys (the top level is table `""`).
pub type Tables = Vec<(String, Vec<(String, Value)>)>;

fn parse_value(s: &str, line: usize) -> Result<Value, String> {
    let s = s.trim();
    let err = || format!("manifest line {line}: bad value `{s}`");
    if let Some(r) = s.strip_prefix('"') {
        let r = r.strip_suffix('"').ok_or_else(err)?;
        if r.contains('"') {
            return Err(err());
        }
        return Ok(Value::Str(r.replace("\\\\", "\\")));
    }
    if let Some(r) = s.strip_prefix('[') {
        let r = r.strip_suffix(']').ok_or_else(err)?.trim();
        let mut items = Vec::new();
        for part in split_list(r) {
            let part = part.trim();
            if !part.is_empty() {
                items.push(parse_value(part, line)?);
            }
        }
        return Ok(Value::List(items));
    }
    match s {
        "true" => return Ok(Value::Bool(true)),
        "false" => return Ok(Value::Bool(false)),
        _ => {}
    }
    if let Some(h) = s.strip_prefix("0x") {
        return i64::from_str_radix(h, 16).map(Value::Int).map_err(|_| err());
    }
    if let Ok(i) = s.replace('_', "").parse::<i64>() {
        return Ok(Value::Int(i));
    }
    s.parse::<f64>().map(Value::Float).map_err(|_| err())
}

/// Split a list body at commas outside quotes.
fn split_list(s: &str) -> Vec<&str> {
    let (mut out, mut start, mut q) = (Vec::new(), 0, false);
    for (i, c) in s.char_indices() {
        match c {
            '"' => q = !q,
            ',' if !q => {
                out.push(&s[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    out.push(&s[start..]);
    out
}

/// Strip a `#` comment that is not inside quotes.
fn strip_comment(l: &str) -> &str {
    let mut q = false;
    for (i, c) in l.char_indices() {
        match c {
            '"' => q = !q,
            '#' if !q => return &l[..i],
            _ => {}
        }
    }
    l
}

/// Parse the TOML subset the manifest uses: `[a.b]` headers and one-line
/// `key = value` pairs (strings, integers, floats, booleans, flat lists).
pub fn parse_tables(text: &str) -> Result<Tables, String> {
    let mut t: Tables = vec![(String::new(), Vec::new())];
    for (n, raw) in text.lines().enumerate() {
        let line = n + 1;
        let l = strip_comment(raw).trim();
        if l.is_empty() {
            continue;
        }
        if let Some(h) = l.strip_prefix('[') {
            let h = h.strip_suffix(']').ok_or(format!("manifest line {line}: bad header"))?.trim();
            if h.is_empty() || t.iter().any(|(k, _)| k == h) {
                return Err(format!("manifest line {line}: empty or repeated table [{h}]"));
            }
            t.push((h.replace(['"', ' '], ""), Vec::new()));
            continue;
        }
        let (k, v) = l.split_once('=').ok_or(format!("manifest line {line}: expected key = value"))?;
        let k = k.trim().trim_matches('"').to_string();
        if k.is_empty() {
            return Err(format!("manifest line {line}: empty key"));
        }
        let v = parse_value(v, line)?;
        let cur = &mut t.last_mut().expect("top table").1;
        if cur.iter().any(|(e, _)| *e == k) {
            return Err(format!("manifest line {line}: repeated key {k}"));
        }
        cur.push((k, v));
    }
    Ok(t)
}

/// Section name to picture id.
pub fn parse_id(section: &str) -> Option<ArtId> {
    let (kind, n) = section.split_once('.')?;
    let num = || -> Option<u16> {
        match n.strip_prefix("0x") {
            Some(h) => u16::from_str_radix(h, 16).ok(),
            None => n.parse().ok(),
        }
    };
    match kind {
        "world" => num().map(ArtId::World),
        "town" => num().map(ArtId::Town),
        "mond" => num().map(ArtId::Mond),
        "screen" => match n {
            "nif" => Some(ArtId::Screen("nif")),
            _ => None,
        },
        _ => None,
    }
}

/// One picture of the pack with its animation.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub frames: Vec<Rgba>,
    /// Frames follow the original tile's horizontal scroll (water), and
    /// the picture scrolls with it.
    pub scroll: bool,
    /// Frames per second of a clock-driven animation (0: frame 0 only,
    /// unless `scroll`).
    pub fps: f64,
    /// Alternative pictures (`variants`), one chosen per map cell so large
    /// areas do not repeat; empty if none. `frames[0]` is the first.
    pub variants: Vec<Rgba>,
}

/// Which of `n` variants a map cell shows: a fixed hash of the cell, so a
/// cell always looks the same and neighbours differ irregularly.
pub fn variant_index(pos: (u16, u16), n: usize) -> usize {
    if n <= 1 {
        return 0;
    }
    let mut h = u32::from(pos.0).wrapping_mul(0x9E37_79B1) ^ u32::from(pos.1).wrapping_mul(0x85EB_CA77);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297A_2D39);
    h ^= h >> 15;
    (h % n as u32) as usize
}

/// A loaded asset pack.
#[derive(Debug, Clone, PartialEq)]
pub struct Pack {
    pub name: String,
    pub license: String,
    /// Pixels per original 16-pixel tile edge.
    pub tile_size: usize,
    pub notes: Vec<(String, String)>,
    pub entries: HashMap<ArtId, Entry>,
    /// Problems that did not stop loading (unknown sections, resized art).
    pub warnings: Vec<String>,
}

impl Pack {
    /// No art at all: everything falls back to the original, scaled.
    pub fn empty(tile_size: usize) -> Pack {
        Pack { name: "none".into(), license: String::new(), tile_size, notes: Vec::new(), entries: HashMap::new(), warnings: Vec::new() }
    }

    /// The internal resolution factor over 320x200.
    pub fn scale(&self) -> usize {
        self.tile_size / 16
    }

    /// Read `dir/manifest.toml` and its PNGs.
    pub fn load(dir: &Path) -> Result<Pack, String> {
        let m = dir.join("manifest.toml");
        let text = std::fs::read_to_string(&m).map_err(|e| format!("{}: {e}", m.display()))?;
        Pack::parse(&text, |f| {
            let p: PathBuf = dir.join(f);
            let d = std::fs::read(&p).map_err(|e| format!("{}: {e}", p.display()))?;
            Rgba::from_png(&d).map_err(|e| format!("{}: {e}", p.display()))
        })
    }

    /// Parse a manifest, getting each named picture from `load`.
    pub fn parse(text: &str, mut load: impl FnMut(&str) -> Result<Rgba, String>) -> Result<Pack, String> {
        let tables = parse_tables(text)?;
        let mut pack = Pack::empty(32);
        for (k, v) in &tables[0].1 {
            match (k.as_str(), v) {
                ("name", Value::Str(s)) => pack.name = s.clone(),
                ("license", Value::Str(s)) => pack.license = s.clone(),
                ("tile_size", Value::Int(n)) => {
                    if *n < 16 || n % 16 != 0 || *n > 128 {
                        return Err(format!("tile_size {n}: must be 16, 32, 48 ... 128"));
                    }
                    pack.tile_size = *n as usize;
                }
                ("tile_size", _) => return Err("tile_size must be an integer".into()),
                (_, _) => {} // author, description, version: informational
            }
        }
        let s = pack.scale();
        for (sec, keys) in &tables[1..] {
            if sec == "palette" {
                for (k, v) in keys {
                    if let Value::Str(t) = v {
                        pack.notes.push((k.clone(), t.clone()));
                    }
                }
                continue;
            }
            let Some(id) = parse_id(sec) else {
                pack.warnings.push(format!("unknown section [{sec}] ignored"));
                continue;
            };
            let (ow, oh) = match id {
                ArtId::World(_) | ArtId::Mond(_) => (16, 16),
                ArtId::Town(_) => (8, 8),
                ArtId::Screen(_) => (W, 168),
            };
            let mut files = Vec::new();
            let mut vfiles = Vec::new();
            let (mut scroll, mut fps) = (false, 0.0);
            for (k, v) in keys {
                match (k.as_str(), v) {
                    ("file", Value::Str(f)) => files.push(f.clone()),
                    ("frames", Value::List(l)) => {
                        for f in l {
                            match f {
                                Value::Str(f) => files.push(f.clone()),
                                _ => return Err(format!("[{sec}] frames: strings expected")),
                            }
                        }
                    }
                    ("variants", Value::List(l)) => {
                        for f in l {
                            match f {
                                Value::Str(f) => vfiles.push(f.clone()),
                                _ => return Err(format!("[{sec}] variants: strings expected")),
                            }
                        }
                    }
                    ("scroll", Value::Bool(b)) => scroll = *b,
                    ("fps", Value::Int(n)) => fps = *n as f64,
                    ("fps", Value::Float(f)) => fps = *f,
                    ("name" | "notes", _) => {}
                    _ => return Err(format!("[{sec}] bad key {k}")),
                }
            }
            if files.len() > 1 && !vfiles.is_empty() {
                return Err(format!("[{sec}] variants cannot be combined with several frames"));
            }
            files.extend(vfiles.iter().cloned());
            if files.is_empty() {
                return Err(format!("[{sec}] needs file, frames or variants"));
            }
            let mut frames = Vec::new();
            for f in &files {
                let img = load(f)?;
                if (img.w, img.h) != (ow * s, oh * s) {
                    pack.warnings.push(format!("{f}: {}x{}, resized to {}x{}", img.w, img.h, ow * s, oh * s));
                }
                frames.push(img.resized(ow * s, oh * s));
            }
            let variants = if vfiles.is_empty() { Vec::new() } else { std::mem::take(&mut frames) };
            if frames.is_empty() {
                frames.push(variants[0].clone());
            }
            pack.entries.insert(id, Entry { frames, scroll, fps, variants });
        }
        Ok(pack)
    }

    /// The picture of `e` for a tile drawn at map cell `pos` (variants)
    /// with `shift` (0..ow) at clock `ms`.
    fn pick(e: &Entry, pos: Option<(u16, u16)>, shift: usize, ow: usize, ms: u64) -> &Rgba {
        match pos {
            Some(p) if !e.variants.is_empty() => &e.variants[variant_index(p, e.variants.len())],
            _ => Pack::frame(e, shift, ow, ms),
        }
    }

    /// The frame of `e` to show for a tile drawn with `shift` (0..ow) at
    /// clock `ms`.
    fn frame(e: &Entry, shift: usize, ow: usize, ms: u64) -> &Rgba {
        let n = e.frames.len();
        let i = if n == 1 {
            0
        } else if e.scroll {
            shift * n / ow.max(1) % n
        } else if e.fps > 0.0 {
            ((ms as f64 * e.fps / 1000.0) as usize) % n
        } else {
            0
        };
        &e.frames[i]
    }
}

/// The screen at `pack.scale()` times 320x200 as RGB: the original scaled
/// up nearest-neighbour, with the pack's art over every intact region
/// whose picture it has. `ms` drives clock animations.
pub fn compose(t: &Term, palette: &[[u8; 3]; 16], pack: &Pack, ms: u64) -> Vec<u8> {
    let s = pack.scale();
    let mut out = t.to_rgb(palette, s);
    // Palette tricks (Mondain's flash, the start-up castle's registers)
    // stay original.
    if t.inverted || t.remap.is_some() {
        return out;
    }
    let ow = W * s;
    let put = |out: &mut Vec<u8>, img: &Rgba, x0: usize, y0: usize, sx: usize| {
        for y in 0..img.h {
            for x in 0..img.w {
                let src = (y * img.w + (x + sx) % img.w) * 4;
                let a = u32::from(img.px[src + 3]);
                if a == 0 {
                    continue;
                }
                let (dx, dy) = (x0 + x, y0 + y);
                if dx >= ow || dy >= H * s {
                    continue;
                }
                let d = (dy * ow + dx) * 3;
                for c in 0..3 {
                    let v = u32::from(img.px[src + c]);
                    out[d + c] = ((v * a + u32::from(out[d + c]) * (255 - a)) / 255) as u8;
                }
            }
        }
    };
    for op in &t.art {
        let Some(e) = pack.entries.get(&op.id) else { continue };
        if !op.intact(t) {
            continue;
        }
        let (x0, y0) = (op.x * s, op.y * s);
        let f = Pack::pick(e, op.pos, op.shift as usize, op.w, ms);
        if f.px.chunks(4).any(|p| p[3] < 255) {
            // A sprite: the terrain behind it, else black (the original
            // tiles have black backgrounds).
            match op.under.and_then(|u| pack.entries.get(&u)) {
                Some(u) => put(&mut out, Pack::pick(u, op.pos, 0, op.w, ms), x0, y0, 0),
                None => {
                    for y in y0..y0 + op.h * s {
                        for x in x0..(x0 + op.w * s).min(ow) {
                            out[(y * ow + x) * 3..(y * ow + x) * 3 + 3].fill(0);
                        }
                    }
                }
            }
        }
        let sx = if e.scroll { op.shift as usize * s } else { 0 };
        put(&mut out, f, x0, y0, sx);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::palette::EGA_PALETTE;

    fn solid(w: usize, h: usize, c: [u8; 4]) -> Rgba {
        let mut i = Rgba::new(w, h);
        for p in i.px.chunks_mut(4) {
            p.copy_from_slice(&c);
        }
        i
    }

    const MANIFEST: &str = r#"
# a test pack
name = "test"   # trailing comment
license = "CC0"
tile_size = 32

[palette]
ega = "ignores the 16 colours # not a comment"

[world.0]
frames = ["w0.png", "w1.png"]
scroll = true

[world.0x0A]
file = "player.png"

[world.4]
frames = ["c0.png", "c1.png"]
fps = 2.5

[town.1]
file = "floor.png"

[sound.1]
file = "x.png"
"#;

    fn load(f: &str) -> Result<Rgba, String> {
        Ok(match f {
            "w0.png" => solid(32, 32, [0, 0, 200, 255]),
            "w1.png" => solid(32, 32, [0, 0, 100, 255]),
            // A sprite: transparent but for a red middle.
            "player.png" => {
                let mut i = solid(32, 32, [0, 0, 0, 0]);
                for y in 12..20 {
                    for x in 12..20 {
                        i.px[(y * 32 + x) * 4..(y * 32 + x) * 4 + 4].copy_from_slice(&[255, 0, 0, 255]);
                    }
                }
                i
            }
            "floor.png" => solid(8, 8, [9, 9, 9, 255]),
            _ => solid(32, 32, [1, 2, 3, 255]),
        })
    }

    #[test]
    fn manifest_parses() {
        let p = Pack::parse(MANIFEST, load).unwrap();
        assert_eq!((p.name.as_str(), p.license.as_str(), p.tile_size, p.scale()), ("test", "CC0", 32, 2));
        assert_eq!(p.notes, vec![("ega".to_string(), "ignores the 16 colours # not a comment".to_string())]);
        assert_eq!(p.entries.len(), 4);
        let w = &p.entries[&ArtId::World(0)];
        assert!(w.scroll && w.frames.len() == 2);
        assert_eq!(p.entries[&ArtId::World(4)].fps, 2.5);
        assert!(p.entries.contains_key(&ArtId::World(10)));
        // The 8x8 town tile was given at 8x8 and is blown up to 16x16.
        assert_eq!((p.entries[&ArtId::Town(1)].frames[0].w, p.entries[&ArtId::Town(1)].frames[0].h), (16, 16));
        assert_eq!(p.warnings.len(), 2, "{:?}", p.warnings);
    }

    #[test]
    fn manifest_errors() {
        assert!(Pack::parse("tile_size = 20", load).is_err());
        assert!(Pack::parse("tile_size = \"32\"", load).is_err());
        assert!(Pack::parse("[world.1]\nscroll = true", load).is_err());
        assert!(Pack::parse("[world.1]\nfile = 3", load).is_err());
        assert!(Pack::parse("[world.1]\nfile = \"a.png\"\n[world.1]\nfile = \"b.png\"", load).is_err());
        assert!(Pack::parse("x = [1, 2", load).is_err());
        assert!(Pack::parse("[world.1]\nfile = \"missing.png\"", |_| Err("nope".into())).is_err());
        assert_eq!(parse_value("[\"a,b\", \"c\"]", 1).unwrap(), Value::List(vec![Value::Str("a,b".into()), Value::Str("c".into())]));
    }

    /// A screen with one water tile, one player tile and one tile the
    /// pack lacks.
    fn screen() -> Term {
        let mut t = Term::new();
        t.blit_art(ArtId::World(0), None, 8, 8, 16, 16, &[1; 256], 4);
        t.blit_art(ArtId::World(10), Some(ArtId::World(0)), 24, 8, 16, 16, &[15; 256], 0);
        t.blit_art(ArtId::World(2), None, 40, 8, 16, 16, &[2; 256], 0);
        t
    }

    fn px(rgb: &[u8], s: usize, x: usize, y: usize) -> [u8; 3] {
        let o = (y * W * s + x) * 3;
        [rgb[o], rgb[o + 1], rgb[o + 2]]
    }

    #[test]
    fn empty_pack_is_the_original_scaled() {
        let t = screen();
        for ts in [16, 32, 64] {
            let s = ts / 16;
            assert_eq!(compose(&t, &EGA_PALETTE, &Pack::empty(ts), 0), t.to_rgb(&EGA_PALETTE, s));
        }
    }

    #[test]
    fn art_replaces_known_tiles_and_falls_back_for_the_rest() {
        let t = screen();
        let p = Pack::parse(MANIFEST, load).unwrap();
        let rgb = compose(&t, &EGA_PALETTE, &p, 0);
        // Water at shift 4 of 16: frame 0 of 2.
        assert_eq!(px(&rgb, 2, 16, 16), [0, 0, 200]);
        // The player sprite over the pack's water, red in the middle.
        assert_eq!(px(&rgb, 2, 48, 16), [0, 0, 200]);
        assert_eq!(px(&rgb, 2, 48 + 16, 16 + 16), [255, 0, 0]);
        // Forest is not in the pack: original green, scaled.
        assert_eq!(px(&rgb, 2, 80, 16), EGA_PALETTE[2]);
        // Text and frame: original.
        assert_eq!(px(&rgb, 2, 6, 200), EGA_PALETTE[1]);
    }

    #[test]
    fn overdrawn_tiles_fall_back() {
        let mut t = screen();
        let p = Pack::parse(MANIFEST, load).unwrap();
        // Something else drawn over the water tile (a missile, a line).
        t.plot(10, 10, 12);
        let rgb = compose(&t, &EGA_PALETTE, &p, 0);
        assert_eq!(px(&rgb, 2, 16, 16), EGA_PALETTE[1]);
        assert_eq!(px(&rgb, 2, 20, 20), EGA_PALETTE[12]);
        // A tile drawn over an older one replaces its note.
        t.blit_art(ArtId::World(0), None, 8, 8, 16, 16, &[1; 256], 12);
        assert_eq!(t.art.iter().filter(|o| o.x == 8 && o.y == 8).count(), 1);
        let rgb = compose(&t, &EGA_PALETTE, &p, 0);
        // Shift 12: frame 1.
        assert_eq!(px(&rgb, 2, 16, 16), [0, 0, 100]);
    }

    #[test]
    fn clock_animation_and_palette_tricks() {
        let mut t = Term::new();
        t.blit_art(ArtId::World(4), None, 8, 8, 16, 16, &[7; 256], 0);
        let p = Pack::parse(MANIFEST, load).unwrap();
        assert_eq!(Pack::frame(&p.entries[&ArtId::World(4)], 0, 16, 0), &p.entries[&ArtId::World(4)].frames[0]);
        assert_eq!(Pack::frame(&p.entries[&ArtId::World(4)], 0, 16, 400), &p.entries[&ArtId::World(4)].frames[1]);
        t.inverted = true;
        assert_eq!(compose(&t, &EGA_PALETTE, &p, 0), t.to_rgb(&EGA_PALETTE, 2));
    }

    #[test]
    fn variants_follow_the_map_cell() {
        let m = "[world.1]\nvariants = [\"g0.png\", \"g1.png\", \"g2.png\", \"g3.png\"]\n[world.0x0A]\nfile = \"player.png\"";
        let ld = |f: &str| -> Result<Rgba, String> {
            Ok(match f {
                "g0.png" => solid(32, 32, [0, 10, 0, 255]),
                "g1.png" => solid(32, 32, [0, 11, 0, 255]),
                "g2.png" => solid(32, 32, [0, 12, 0, 255]),
                "g3.png" => solid(32, 32, [0, 13, 0, 255]),
                other => load(other)?,
            })
        };
        let p = Pack::parse(m, ld).unwrap();
        let e = &p.entries[&ArtId::World(1)];
        assert_eq!((e.variants.len(), e.frames.len()), (4, 1));
        // Deterministic, and all variants turn up over a field.
        let mut seen = [0usize; 4];
        for y in 0..16u16 {
            for x in 0..16u16 {
                let i = variant_index((x, y), 4);
                assert_eq!(i, variant_index((x, y), 4));
                seen[i] += 1;
            }
        }
        assert!(seen.iter().all(|&n| n > 30), "{seen:?}");
        // Not one value along a row or a column.
        assert!((0..8u16).map(|x| variant_index((x, 3), 4)).collect::<std::collections::HashSet<_>>().len() > 1);
        assert!((0..8u16).map(|y| variant_index((3, y), 4)).collect::<std::collections::HashSet<_>>().len() > 1);
        // The compositor shows the cell's variant, also under a sprite.
        let mut t = Term::new();
        t.blit_art(ArtId::World(1), None, 8, 8, 16, 16, &[2; 256], 0);
        t.art_pos((5, 7));
        t.blit_art(ArtId::World(10), Some(ArtId::World(1)), 24, 8, 16, 16, &[15; 256], 0);
        t.art_pos((6, 7));
        let rgb = compose(&t, &EGA_PALETTE, &p, 0);
        assert_eq!(px(&rgb, 2, 16, 16), [0, 10 + variant_index((5, 7), 4) as u8, 0]);
        assert_eq!(px(&rgb, 2, 50, 18), [0, 10 + variant_index((6, 7), 4) as u8, 0]);
        // Without a cell (a tile drawn outside the map): the first one.
        t.blit_art(ArtId::World(1), None, 40, 8, 16, 16, &[2; 256], 0);
        assert_eq!(px(&compose(&t, &EGA_PALETTE, &p, 0), 2, 82, 18), [0, 10, 0]);
        // A file plus variants: the file is variant 0.
        let p2 = Pack::parse("[world.1]\nfile = \"g0.png\"\nvariants = [\"g1.png\"]", ld).unwrap();
        assert_eq!(p2.entries[&ArtId::World(1)].variants.len(), 2);
        // Variants of an animation are refused.
        assert!(Pack::parse("[world.1]\nframes = [\"g0.png\", \"g1.png\"]\nvariants = [\"g2.png\"]", ld).is_err());
    }

    #[test]
    fn png_round_trip() {
        let mut buf = Vec::new();
        {
            let mut enc = png::Encoder::new(&mut buf, 2, 1);
            enc.set_color(png::ColorType::Rgba);
            enc.set_depth(png::BitDepth::Eight);
            enc.write_header().unwrap().write_image_data(&[1, 2, 3, 4, 5, 6, 7, 8]).unwrap();
        }
        let i = Rgba::from_png(&buf).unwrap();
        assert_eq!((i.w, i.h, i.px.clone()), (2, 1, vec![1, 2, 3, 4, 5, 6, 7, 8]));
    }

    #[test]
    fn shipped_pack_loads_cleanly() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/enhanced");
        let p = Pack::load(&dir).expect("assets/enhanced loads");
        assert!(p.warnings.is_empty(), "{:?}", p.warnings);
        assert_eq!(p.tile_size, 32);
        for (id, n) in [(1u16, 4usize), (3, 3)] {
            assert_eq!(p.entries[&ArtId::World(id)].variants.len(), n, "world.{id}");
        }
        for (id, e) in &p.entries {
            if !matches!(id, ArtId::World(_)) {
                continue;
            }
            for f in e.frames.iter().chain(&e.variants) {
                assert_eq!((f.w, f.h), (p.tile_size, p.tile_size));
            }
        }
    }
}
