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
//! behind = true           # only under the original's black pixels: its
//!                         # lit pixels (text) stay, with a drop shadow
//!
//! [dungeon.wall]          # first-person view textures, any size
//! file = "dungeon/wall.png"
//!
//! [dmon.1]                # dungeon monster kind 1 (skeleton), any size
//! file = "dungeon/skeleton.png"
//! ```
//!
//! Sections are `world.N`, `town.N`, `mond.N` (decimal or `0x` tile
//! numbers as in SPEC §3.2-3.4), `screen.NAME`, `dungeon.PART`
//! ([`crate::term::DUN_PARTS`]), `dmon.N` (monster kinds 0-24) and
//! `space.PART` ([`crate::term::SPACE_PARTS`]; any size: the frame and the
//! backgrounds are stretched over theirs, ships are drawn nose up and
//! turned to their heading).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::term::{dun, space_part as sp, ArtId, Term, DUN_PARTS, H, SPACE_PARTS, W};

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
            "title" => Some(ArtId::Screen("title")),
            "intro" => Some(ArtId::Screen("intro")),
            "castle" => Some(ArtId::Screen("castle")),
            "menu" => Some(ArtId::Screen("menu")),
            "loading" => Some(ArtId::Screen("loading")),
            _ => None,
        },
        "dungeon" => DUN_PARTS.iter().position(|p| *p == n).map(|i| ArtId::Dun(i as u16)),
        "dmon" => num().filter(|&i| i < 25).map(ArtId::DMon),
        "space" => SPACE_PARTS.iter().position(|p| *p == n).map(|i| ArtId::Space(i as u16)),
        _ => None,
    }
}

/// The start-up castle picture's left flag is left of this column, its
/// right flag right of it (original pixels).
pub const CASTLE_SPLIT: usize = 160;

/// The original-pixel size of each `screen.NAME` region (the pictures
/// are this times the pack's scale).
pub fn screen_size(name: &str) -> Option<(usize, usize)> {
    match name {
        "nif" => Some((W, 168)),
        "title" | "intro" | "castle" => Some((W, H)),
        "menu" => Some((0x130, 0xb8)),
        "loading" => Some((0x130, 0x90)),
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
    /// The picture is a backdrop: it shows only where the original region
    /// is black, and the original's lit pixels stay on top with a dark
    /// shadow (screens of text, such as the ending).
    pub behind: bool,
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
            // Dungeon pictures are textures and sprites, stretched to the
            // original geometry, so any size will do.
            let want = match id {
                ArtId::World(_) | ArtId::Mond(_) => Some((16 * s, 16 * s)),
                ArtId::Town(_) => Some((8 * s, 8 * s)),
                ArtId::Screen(n) => screen_size(n).map(|(w, h)| (w * s, h * s)),
                ArtId::Dun(_) | ArtId::DMon(_) | ArtId::Space(_) => None,
            };
            let mut files = Vec::new();
            let mut vfiles = Vec::new();
            let (mut scroll, mut fps, mut behind) = (false, 0.0, false);
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
                    ("behind", Value::Bool(b)) => behind = *b,
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
                match want {
                    Some((w, h)) if (img.w, img.h) != (w, h) => {
                        pack.warnings.push(format!("{f}: {}x{}, resized to {w}x{h}", img.w, img.h));
                        frames.push(img.resized(w, h));
                    }
                    _ => frames.push(img),
                }
            }
            let variants = if vfiles.is_empty() { Vec::new() } else { std::mem::take(&mut frames) };
            if frames.is_empty() {
                frames.push(variants[0].clone());
            }
            pack.entries.insert(id, Entry { frames, scroll, fps, variants, behind });
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

/// Light at depth `p` (0 = the screen plane) in the dungeon.
fn light(p: f32) -> f32 {
    0.95 * 0.78f32.powf(p.max(0.0))
}

/// The pixel of `img` at texture coordinates `u`, `v` (wrapping).
fn texel(img: &Rgba, u: f32, v: f32) -> [u8; 4] {
    let x = ((u.rem_euclid(1.0) * img.w as f32) as usize).min(img.w - 1);
    let y = ((v.rem_euclid(1.0) * img.h as f32) as usize).min(img.h - 1);
    let i = (y * img.w + x) * 4;
    [img.px[i], img.px[i + 1], img.px[i + 2], img.px[i + 3]]
}

/// Blend `c` (alpha in `c[3]`) darkened by `k` into pixel `(x, y)`.
fn blend(out: &mut [u8], ow: usize, x: usize, y: usize, c: [u8; 4], k: f32) {
    let a = f32::from(c[3]) / 255.0;
    if a <= 0.0 {
        return;
    }
    let d = (y * ow + x) * 3;
    for i in 0..3 {
        let v = f32::from(c[i]) * k;
        out[d + i] = (v * a + f32::from(out[d + i]) * (1.0 - a)).clamp(0.0, 255.0) as u8;
    }
}

/// Redraw the first-person dungeon view from its scene with the pack's
/// textures and sprites, keeping the original geometry. Pieces the pack
/// has no picture for keep their original pixels from `orig`.
fn dungeon(out: &mut [u8], orig: &[u8], s: usize, sc: &crate::dview::DScene, pack: &Pack, ms: u64) {
    use crate::dview::{DPiece, GEOM};
    let ow = W * s;
    let (gx0, gy0, gx1, gy1) = GEOM;
    // The corridor's outer walls reach on to the edges of the view.
    let (vx0, vy0, vx1, vy1) = crate::term::VIEW;
    let (cx0, cy0, cx1, cy1) = (vx0 * s, vy0 * s, (vx1 + 1) * s, (vy1 + 1) * s);
    let sf = s as f32;
    let art = |id: ArtId| pack.entries.get(&id).map(|e| Pack::frame(e, 0, 1, ms));
    let (xs, ys) = (sc.x.map(|v| v as f32), sc.y.map(|v| v as f32));
    let mid = (gy0 + gy1) as f32 / 2.0 + 0.5;
    // Floor and ceiling, one picture per cell, placed by the corridor
    // tables and fading toward the horizon.
    let fl = art(ArtId::Dun(dun::FLOOR));
    let ce = art(ArtId::Dun(dun::CEILING));
    for oy in cy0..cy1 {
        let fy = (oy as f32 + 0.5) / sf;
        let floor = fy > mid;
        let Some(img) = (if floor { fl } else { ce }) else { continue };
        let edge = |i: usize| if floor { gy1 as f32 + 1.0 - ys[i] } else { gy0 as f32 + ys[i] };
        let dist = |e: f32| (e - mid).abs();
        let dy = (fy - mid).abs();
        // Plane depth of this row.
        let mut p = 6.0;
        for i in 0..5 {
            let (a, b) = (dist(edge(i)), dist(edge(i + 1)));
            if dy >= b || i == 0 && dy > a {
                p = i as f32 + (a - dy) / (a - b).max(0.01);
                break;
            }
        }
        if p >= 6.0 {
            let b = dist(edge(5));
            p = 5.0 + (b - dy) / b.max(0.01) * 4.0;
        }
        let pi = (p.floor() as usize).min(4);
        let f = (p - pi as f32).clamp(0.0, 1.0);
        let xp = xs[pi] + (xs[pi + 1] - xs[pi]) * f;
        let (l, r) = (gx0 as f32 + xp, gx1 as f32 + 1.0 - xp);
        let k = light(p) * if p > 5.0 { (1.0 - (p - 5.0) / 4.0).max(0.0) } else { 1.0 };
        for ox in cx0..cx1 {
            let fx = (ox as f32 + 0.5) / sf;
            let u = (fx - l) / (r - l).max(0.5);
            let c = texel(img, u, if floor { -p } else { p });
            out[(oy * ow + ox) * 3..(oy * ow + ox) * 3 + 3].fill(0);
            blend(out, ow, ox, oy, c, k);
        }
    }
    let copy = |out: &mut [u8], x0: f32, y0: f32, x1: f32, y1: f32, only_lit: bool| {
        let (a, b) = (((x0 * sf) as usize).max(cx0), ((x1 * sf) as usize).min(cx1));
        let (c, d) = (((y0 * sf) as usize).max(cy0), ((y1 * sf) as usize).min(cy1));
        for y in c..d {
            for x in a..b {
                let i = (y * ow + x) * 3;
                if !only_lit || orig[i..i + 3] != [0, 0, 0] {
                    out[i..i + 3].copy_from_slice(&orig[i..i + 3]);
                }
            }
        }
    };
    for piece in sc.ordered() {
        match piece {
            DPiece::Face(f) => {
                let id = match f.id {
                    ArtId::Dun(dun::SECRET) if !pack.entries.contains_key(&f.id) => ArtId::Dun(dun::WALL),
                    id => id,
                };
                let Some(img) = art(id) else {
                    let (t, b) = (f.top.0.min(f.top.1), f.bot.0.max(f.bot.1));
                    copy(out, f.x0 as f32, t as f32, f.x1 as f32 + 1.0, b as f32 + 1.0, false);
                    continue;
                };
                let (x0, x1) = (f.x0 as f32, f.x1 as f32 + 1.0);
                let h0 = (f.bot.0 - f.top.0 + 1).max(1) as f32;
                let h1 = (f.bot.1 - f.top.1 + 1).max(1) as f32;
                let ea = if f.x0 <= gx0 { vx0 as f32 } else { x0 };
                let eb = if f.x1 >= gx1 { vx1 as f32 + 1.0 } else { x1 };
                let (a, b) = (((ea * sf) as usize).max(cx0), ((eb * sf) as usize).min(cx1));
                for ox in a..b {
                    let fx = (ox as f32 + 0.5) / sf;
                    let t = (fx - x0) / (x1 - x0);
                    let tc = t.clamp(0.0, 1.0);
                    let u = (tc * h1 / ((1.0 - tc) * h0 + tc * h1)).clamp(0.0, 1.0) + (t - tc);
                    let yt = f.top.0 as f32 + t * (f.top.1 - f.top.0) as f32;
                    let yb = f.bot.0 as f32 + t * (f.bot.1 - f.bot.0) as f32 + 1.0;
                    let k = light(f.plane.0 as f32 + u * (f.plane.1 - f.plane.0) as f32);
                    let edge_x = ox < a + s.max(2) / 2 || ox + s.max(2) / 2 >= b;
                    let (c, d) = (((yt * sf) as usize).max(cy0), ((yb * sf) as usize).min(cy1));
                    for oy in c..d {
                        let v = ((oy as f32 + 0.5) / sf - yt) / (yb - yt).max(0.5);
                        let edge = edge_x || oy < c + s / 2 || oy + s / 2 >= d;
                        blend(out, ow, ox, oy, texel(img, u, v), if edge { k * 0.5 } else { k });
                    }
                }
            }
            DPiece::Obj(o) => {
                let (mut x0, y0, mut x1, y1) = (o.x0 as f32, o.y0 as f32, o.x1 as f32 + 1.0, o.y1 as f32 + 1.0);
                if matches!(o.id, ArtId::Dun(dun::LADDER_UP | dun::LADDER_DOWN)) {
                    // A ladder seen edge-on is a line: give it a rail's width.
                    let w = (y1 - y0) * 0.06;
                    if x1 - x0 < w {
                        let c = (x0 + x1) / 2.0;
                        (x0, x1) = (c - w / 2.0, c + w / 2.0);
                    }
                }
                let Some(img) = art(o.id) else {
                    copy(out, x0 - 1.0, y0 - 1.0, x1 + 1.0, y1 + 1.0, true);
                    continue;
                };
                // Monsters, chests and coffins keep their proportions and
                // stand on the box's bottom; ladders and holes stretch.
                // The original draws chests and coffins as low, wide boxes,
                // so their pictures fill the box's width and may rise up
                // to twice its height.
                let fit = !matches!(o.id, ArtId::Dun(d) if (dun::LADDER_UP..=dun::HOLE_DOWN).contains(&d)) && o.id != ArtId::DMon(9);
                let wide = matches!(o.id, ArtId::Dun(dun::CHEST | dun::COFFIN) | ArtId::DMon(11));
                let (bw, bh) = (x1 - x0, y1 - y0);
                let (w, h, l, t) = if fit {
                    let tall = if wide { 2.0 * bh } else { bh };
                    let z = (bw / img.w as f32).min(tall / img.h as f32);
                    let (w, h) = (img.w as f32 * z, img.h as f32 * z);
                    (w, h, x0 + (bw - w) / 2.0, y1 - h)
                } else {
                    (bw, bh, x0, y0)
                };
                let k = light(o.plane as f32 * 0.7);
                if let Some([tl, tr, br, bl]) = o.quad {
                    // A hole keeps the original's four-sided outline: each
                    // row runs between the slanted sides, and the picture's
                    // rows run from the top edge to the bottom edge.
                    let (ty, by) = (tl.1 as f32, br.1 as f32 + 1.0);
                    let (c, d) = (((ty * sf) as usize).max(cy0), ((by * sf) as usize).min(cy1));
                    for oy in c..d {
                        let v = (((oy as f32 + 0.5) / sf - ty) / (by - ty)).clamp(0.0, 0.999);
                        let xl = tl.0 as f32 + v * (bl.0 - tl.0) as f32;
                        let xr = tr.0 as f32 + 1.0 + v * (br.0 - tr.0) as f32;
                        let (a, b) = (((xl * sf) as usize).max(cx0), ((xr * sf) as usize).min(cx1));
                        for ox in a..b {
                            let u = (((ox as f32 + 0.5) / sf - xl) / (xr - xl).max(0.5)).clamp(0.0, 0.999);
                            blend(out, ow, ox, oy, texel(img, u, v), k);
                        }
                    }
                    continue;
                }
                let (a, b) = (((l * sf) as usize).max(cx0), (((l + w) * sf) as usize).min(cx1));
                let (c, d) = (((t * sf) as usize).max(cy0), (((t + h) * sf) as usize).min(cy1));
                for oy in c..d {
                    let v = ((oy as f32 + 0.5) / sf - t) / h;
                    for ox in a..b {
                        let u = ((ox as f32 + 0.5) / sf - l) / w;
                        blend(out, ow, ox, oy, texel(img, u.min(0.999), v.min(0.999)), k);
                    }
                }
            }
        }
    }
}

/// Draw `img` stretched over the screen rectangle `(x, y, w, h)` (original
/// pixels; `rot` quarter turns clockwise), only where `keep` allows.
#[allow(clippy::too_many_arguments)]
fn stretch(out: &mut [u8], s: usize, img: &Rgba, r: (f32, f32, f32, f32), rot: u8, keep: impl Fn(usize, usize) -> bool) {
    let ow = W * s;
    let sf = s as f32;
    let (x, y, w, h) = r;
    let (a, b) = (((x * sf).max(0.0)) as usize, (((x + w) * sf).min(ow as f32)) as usize);
    let (c, d) = (((y * sf).max(0.0)) as usize, (((y + h) * sf).min((H * s) as f32)) as usize);
    for oy in c..d {
        let v = ((oy as f32 + 0.5) / sf - y) / h;
        for ox in a..b {
            if !keep(ox / s, oy / s) {
                continue;
            }
            let u = ((ox as f32 + 0.5) / sf - x) / w;
            let (su, sv) = match rot & 3 {
                0 => (u, v),
                1 => (v, 1.0 - u),
                2 => (1.0 - u, 1.0 - v),
                _ => (1.0 - v, u),
            };
            blend(out, ow, ox, oy, texel(img, su.clamp(0.0, 0.999), sv.clamp(0.0, 0.999)), 1.0);
        }
    }
}

/// Repaint the space view from its scene: the frame, a background where
/// the original is black (and under the pictures it has), the pictures
/// turned and fitted to the boxes the original drew, and soft stars.
/// Shots, flames and anything unknown keep their original pixels.
fn space(out: &mut [u8], s: usize, t: &Term, sc: &crate::term::SScene, pack: &Pack, ms: u64) {
    let get = |p: u16| pack.entries.get(&ArtId::Space(p)).map(|e| Pack::pick(e, None, 0, 0, ms));
    let interior = |x: usize, y: usize| (16..=303).contains(&x) && (16..=143).contains(&y);
    let frame = if sc.fp { get(sp::COCKPIT).or_else(|| get(sp::FRAME)) } else { get(sp::FRAME) };
    if let Some(f) = frame {
        stretch(out, s, f, (8.0, 8.0, 304.0, 144.0), 0, |x, y| !interior(x, y));
    }
    let shown: Vec<_> = sc.objs.iter().filter(|o| o.shown(t) && pack.entries.contains_key(&ArtId::Space(o.part))).collect();
    let mut under = vec![false; W * H];
    for o in &shown {
        for &i in &o.lit {
            under[i as usize] = true;
        }
    }
    let back = if sc.fp { get(sp::DEEP).or_else(|| get(sp::STARFIELD)) } else { get(sp::STARFIELD) };
    if let Some(b) = back {
        stretch(out, s, b, (16.0, 16.0, 288.0, 128.0), 0, |x, y| interior(x, y) && (t.px[y * W + x] == 0 || under[y * W + x]));
        // Lone white pixels (the front view's stars) as soft points.
        let ow = W * s;
        for y in 17..143 {
            for x in 17..303 {
                let i = y * W + x;
                if t.px[i] != 15 || under[i] {
                    continue;
                }
                if [i - W - 1, i - W, i - W + 1, i - 1, i + 1, i + W - 1, i + W, i + W + 1].iter().any(|&j| t.px[j] != 0) {
                    continue;
                }
                let (cx, cy) = (x as f32 + 0.5, y as f32 + 0.5);
                for oy in (y - 1) * s..(y + 2) * s {
                    for ox in (x - 1) * s..(x + 2) * s {
                        let (dx, dy) = ((ox as f32 + 0.5) / s as f32 - cx, (oy as f32 + 0.5) / s as f32 - cy);
                        let a = (1.0 - (dx * dx + dy * dy).sqrt() / 1.3).clamp(0.0, 1.0);
                        blend(out, ow, ox, oy, [235, 240, 255, (a * 255.0) as u8], 1.0);
                    }
                }
            }
        }
    }
    for o in &shown {
        let Some(img) = get(o.part) else { continue };
        // The crosshair's picture is a little larger than its plus.
        let g = if o.part == sp::RETICLE { 3.0 } else { 0.0 };
        let (bw, bh) = (o.w as f32 + 2.0 * g, o.h as f32 + 2.0 * g);
        let (iw, ih) = if o.rot & 1 == 1 { (img.h as f32, img.w as f32) } else { (img.w as f32, img.h as f32) };
        // The fighter's flat original (an H) may be outgrown in height by
        // half again, so a pack's fighter is not shrunk to a sliver.
        let tall = if o.part == sp::FIGHTER { 1.5 } else { 1.0 };
        let k = (bw / iw).min(tall * bh / ih);
        let (w, h) = (iw * k, ih * k);
        let r = (o.x as f32 - g + (bw - w) / 2.0, o.y as f32 - g + (bh - h) / 2.0, w, h);
        stretch(out, s, img, r, o.rot, interior);
    }
}

/// Draw backdrop `img` over region `op` where the original is black; the
/// original's lit pixels stay, each with a shadow one original pixel down
/// and right on the backdrop.
fn backdrop(out: &mut [u8], s: usize, t: &Term, op: &crate::term::ArtOp, img: &Rgba) {
    let ow = W * s;
    let lit = |x: usize, y: usize| x < W && y < H && t.px[y * W + x] != 0;
    for y in 0..op.h * s {
        for x in 0..op.w * s {
            let (dx, dy) = (op.x * s + x, op.y * s + y);
            if dx >= ow || dy >= H * s {
                continue;
            }
            let (ox, oy) = (dx / s, dy / s);
            if lit(ox, oy) {
                continue;
            }
            let (ix, iy) = (x * img.w / (op.w * s), y * img.h / (op.h * s));
            let src = (iy * img.w + ix) * 4;
            let d = (dy * ow + dx) * 3;
            let a = u32::from(img.px[src + 3]);
            let shade = if ox > 0 && oy > 0 && lit(ox - 1, oy - 1) { 4 } else { 1 };
            for c in 0..3 {
                let v = (u32::from(img.px[src + c]) * a + u32::from(out[d + c]) * (255 - a)) / 255;
                out[d + c] = (v / shade) as u8;
            }
        }
    }
}

/// A box drawn over the screen (the options menu), `over` being `under`
/// with the box on it: `under` composed with the pack as usual, and the
/// box's rectangle (inclusive, in 320x200 pixels) from `over`, so tiles the
/// box only partly covers keep their art outside it.
pub fn compose_boxed(under: &Term, over: &Term, palette: &[[u8; 3]; 16], pack: &Pack, ms: u64, rect: (usize, usize, usize, usize)) -> Vec<u8> {
    let s = pack.scale();
    let mut out = compose(under, palette, pack, ms);
    let top = compose(over, palette, pack, ms);
    let (x0, y0, x1, y1) = rect;
    let ow = W * s;
    for y in y0 * s..((y1 + 1) * s).min(H * s) {
        let a = (y * ow + x0 * s) * 3;
        let b = (y * ow + ((x1 + 1) * s).min(ow)) * 3;
        out[a..b].copy_from_slice(&top[a..b]);
    }
    out
}

/// The screen at `pack.scale()` times 320x200 as RGB: the original scaled
/// up nearest-neighbour, with the pack's art over every intact region
/// whose picture it has. `ms` drives clock animations.
pub fn compose(t: &Term, palette: &[[u8; 3]; 16], pack: &Pack, ms: u64) -> Vec<u8> {
    let s = pack.scale();
    let mut out = t.to_rgb(palette, s);
    // Palette tricks (Mondain's flash, the start-up castle's registers)
    // stay original, except under the castle picture, which the pack
    // replaces whole (with its own flags).
    if t.inverted || (t.remap.is_some() && !t.art.iter().any(|o| o.id == ArtId::Screen("castle"))) {
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
    if let Some(sc) = &t.sscene {
        if sc.frame.intact(t) {
            space(&mut out, s, t, sc, pack, ms);
        }
    }
    if let Some((id, x, y, w, h)) = t.backdrop {
        if let Some(e) = pack.entries.get(&id) {
            let op = crate::term::ArtOp { id, under: None, shift: 0, pos: None, x, y, w, h, px: Vec::new() };
            backdrop(&mut out, s, t, &op, Pack::frame(e, 0, 1, ms));
        }
    }
    for op in &t.art {
        if op.id == ArtId::Screen("castle") {
            // The start-up castle: each flag shows the pack frame of the
            // original's flag frame there (`pos` = left, right).
            let Some(e) = pack.entries.get(&op.id) else { continue };
            if !op.intact(t) {
                continue;
            }
            let n = e.frames.len();
            let (l, r) = op.pos.map_or((0, 0), |(l, r)| (usize::from(l) % n, usize::from(r) % n));
            put(&mut out, &e.frames[l], 0, 0, 0);
            if r != l {
                let img = &e.frames[r];
                let x0 = img.w * CASTLE_SPLIT / W;
                for y in 0..img.h.min(H * s) {
                    for x in x0..img.w.min(ow) {
                        let src = (y * img.w + x) * 4;
                        out[(y * ow + x) * 3..(y * ow + x) * 3 + 3].copy_from_slice(&img.px[src..src + 3]);
                    }
                }
            }
            continue;
        }
        if op.id == ArtId::Screen("dungeon") {
            if let Some(sc) = &t.dscene {
                if pack.entries.contains_key(&ArtId::Dun(dun::WALL)) && op.intact(t) {
                    let orig = out.clone();
                    dungeon(&mut out, &orig, s, sc, pack, ms);
                }
            }
            continue;
        }
        let Some(e) = pack.entries.get(&op.id) else { continue };
        if !op.intact(t) {
            continue;
        }
        let (x0, y0) = (op.x * s, op.y * s);
        let f = Pack::pick(e, op.pos, op.shift as usize, op.w, ms);
        if e.behind {
            backdrop(&mut out, s, t, op, f);
            continue;
        }
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
    fn space_sections() {
        assert_eq!(parse_id("space.ship1"), Some(ArtId::Space(8)));
        assert_eq!(parse_id("space.reticle"), Some(ArtId::Space(sp::RETICLE)));
        assert_eq!(parse_id("space.planet"), None);
    }

    #[test]
    fn a_backdrop_shows_only_behind_the_original_text() {
        let mut t = Term::new();
        let mut pic = vec![0u8; W * 168];
        pic[10 * W + 10] = 15;
        t.blit_art(ArtId::Screen("nif"), None, 0, 0, W, 168, &pic, 0);
        let p = Pack::parse("tile_size = 32\n[screen.nif]\nfile = \"bg.png\"\nbehind = true", |_| Ok(solid(64, 64, [40, 80, 120, 255]))).unwrap();
        assert!(p.entries[&ArtId::Screen("nif")].behind);
        let rgb = compose(&t, &EGA_PALETTE, &p, 0);
        // Black: the backdrop; lit: the original; down-right: shadowed.
        assert_eq!(px(&rgb, 2, 0, 0), [40, 80, 120]);
        assert_eq!(px(&rgb, 2, 20, 20), EGA_PALETTE[15]);
        assert_eq!(px(&rgb, 2, 22, 22), [10, 20, 30]);
        // Below the screen region: original.
        assert_eq!(px(&rgb, 2, 0, 2 * 170), [0, 0, 0]);
    }

    #[test]
    fn the_castle_picture_replaces_the_remapped_original() {
        let mut t = Term::new();
        t.remap = Some([0; 16]);
        t.px.fill(3);
        let p = Pack::parse("tile_size = 32\n[screen.castle]\nfile = \"c.png\"", |_| Ok(solid(640, 400, [9, 8, 7, 255]))).unwrap();
        // Unmarked: the palette trick stays original.
        assert_ne!(px(&compose(&t, &EGA_PALETTE, &p, 0), 2, 5, 5), [9, 8, 7]);
        t.mark(ArtId::Screen("castle"), None, 0, 0, 0, W, H);
        assert_eq!(px(&compose(&t, &EGA_PALETTE, &p, 0), 2, 5, 5), [9, 8, 7]);
        // A flag drawn after the mark: original again until re-marked.
        t.plot(0x7b, 0x37, 5);
        assert_ne!(px(&compose(&t, &EGA_PALETTE, &p, 0), 2, 5, 5), [9, 8, 7]);
    }

    #[test]
    fn each_castle_flag_follows_the_originals_frame() {
        let mut t = Term::new();
        t.remap = Some([0; 16]);
        let cols = [[10, 0, 0, 255], [0, 20, 0, 255], [0, 0, 30, 255]];
        let mut i = 0;
        let p = Pack::parse("[screen.castle]\nframes = [\"a.png\", \"b.png\", \"c.png\"]", |_| {
            i += 1;
            Ok(solid(640, 400, cols[i - 1]))
        })
        .unwrap();
        t.mark(ArtId::Screen("castle"), None, 0, 0, 0, W, H);
        t.art_pos((1, 2));
        let out = compose(&t, &EGA_PALETTE, &p, 0);
        assert_eq!(px(&out, 2, 2 * 100, 2 * 60), [0, 20, 0]);
        assert_eq!(px(&out, 2, 2 * 190, 2 * 60), [0, 0, 30]);
        // Same frames at any clock: the flags move only when the original's do.
        assert_eq!(compose(&t, &EGA_PALETTE, &p, 12345), out);
        t.art_pos((2, 2));
        assert_eq!(px(&compose(&t, &EGA_PALETTE, &p, 0), 2, 2 * 100, 2 * 60), [0, 0, 30]);
    }

    #[test]
    fn a_standing_backdrop_survives_redraws() {
        let mut t = Term::new();
        t.fill(0, 0, W - 1, H - 1, 0);
        t.backdrop = Some((ArtId::Screen("menu"), 8, 8, 0x130, 0xb8));
        let p = Pack::parse("[screen.menu]\nfile = \"m.png\"\nbehind = true", |_| Ok(solid(608, 368, [50, 40, 30, 255]))).unwrap();
        // Drawing inside (which drops any noted region there) keeps it.
        t.fill(10, 10, 0x12c, 0xbe, 0);
        t.plot(100, 100, 15);
        let out = compose(&t, &EGA_PALETTE, &p, 0);
        assert_eq!(px(&out, 2, 2 * 50, 2 * 50), [50, 40, 30]);
        assert_ne!(px(&out, 2, 2 * 100, 2 * 100), [50, 40, 30]);
        t.backdrop = None;
        assert_eq!(px(&compose(&t, &EGA_PALETTE, &p, 0), 2, 2 * 50, 2 * 50), [0, 0, 0]);
    }

    #[test]
    fn start_up_screen_sizes() {
        for n in ["nif", "title", "intro", "castle", "menu", "loading"] {
            assert_eq!(parse_id(&format!("screen.{n}")), Some(ArtId::Screen(n)));
            assert!(screen_size(n).is_some());
        }
        assert_eq!(screen_size("menu"), Some((0x130, 0xb8)));
        assert_eq!(parse_id("screen.logo"), None);
        // A wrong-size castle is fitted to the whole screen.
        let p = Pack::parse("[screen.castle]\nfile = \"c.png\"", |_| Ok(solid(320, 168, [1, 1, 1, 255]))).unwrap();
        let f = &p.entries[&ArtId::Screen("castle")].frames[0];
        assert_eq!((f.w, f.h), (640, 400));
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
    fn a_box_over_part_of_a_tile_leaves_the_rest_of_its_art() {
        let mut t = Term::new();
        t.blit_art(ArtId::World(0), None, 24, 16, 16, 16, &[1; 256], 0);
        let p = Pack::parse(MANIFEST, load).unwrap();
        let mut over = t.clone();
        over.fill(31, 23, 287, 127, 0);
        over.plot(40, 30, 12);
        // Composed on its own the covered tile drops to the original...
        assert_eq!(px(&compose(&over, &EGA_PALETTE, &p, 0), 2, 2 * 25, 2 * 17), EGA_PALETTE[1]);
        // ...but with the box laid over the screen it keeps its art.
        let rgb = compose_boxed(&t, &over, &EGA_PALETTE, &p, 0, (31, 23, 287, 127));
        assert_eq!(px(&rgb, 2, 2 * 25, 2 * 17), [0, 0, 200]);
        assert_eq!(px(&rgb, 2, 2 * 30 + 1, 2 * 22 + 1), [0, 0, 200]);
        assert_eq!(px(&rgb, 2, 2 * 31, 2 * 23), [0, 0, 0], "inside: the box");
        assert_eq!(px(&rgb, 2, 2 * 40, 2 * 30), EGA_PALETTE[12]);
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
    fn dungeon_art_takes_any_size() {
        let text =
            "tile_size = 32\n[dungeon.wall]\nfile = \"a\"\n[dungeon.field]\nframes = [\"a\", \"a\"]\nfps = 6\n[dmon.24]\nfile = \"a\"\n[dungeon.nothing]\nfile = \"a\"\n[dmon.25]\nfile = \"a\"\n";
        let p = Pack::parse(text, |_| Ok(Rgba::new(37, 53))).unwrap();
        assert_eq!(p.entries[&ArtId::Dun(dun::WALL)].frames[0].w, 37);
        assert_eq!(p.entries[&ArtId::Dun(dun::FIELD)].frames.len(), 2);
        assert_eq!(p.entries[&ArtId::DMon(24)].frames[0].h, 53);
        assert_eq!(parse_id("dungeon.hole_down"), Some(ArtId::Dun(dun::HOLE_DOWN)));
        // Unknown parts and monster numbers are reported, not loaded.
        assert_eq!(p.entries.len(), 3);
        assert_eq!(p.warnings.len(), 2, "{:?}", p.warnings);
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
