//! The start-up program (ULTIMA.EXE equivalent, SPEC §17): the title text
//! screen, the Origin logo dissolve with "Presents...", the two text pages,
//! the castle picture with its waving flags, and the credits page shown
//! when a key is pressed, after which the character menu (GEN) follows.
//!
//! Layout, colours, timings and the order of events are "verified from
//! original executable". The logo bitmaps (rows of `*`), the flag frames,
//! the texts and the castle pictures are read at run time from the user's
//! ULTIMA.EXE and CASTLE.4 / CASTLE.16; nothing of them is embedded.

use crate::exe::Exe;
use crate::gamedir::{GameDir, VideoMode};
use crate::io::{Io, IoResult};
use crate::term::{ArtId, Term, H, RAW, W};

/// Data segment paragraph of ULTIMA.EXE (relative to the load image).
const DS_PARA: usize = 0x3ed;

/// Data-segment offsets in ULTIMA.EXE.
mod o {
    pub const LOGO_LEFT: usize = 0x3262; // 64 row pointers, 199 columns each
    pub const LOGO_RIGHT: usize = 0x4320; // 64 row pointers, 76 columns each
    pub const TAPS: usize = 0x43a0; // feedback masks by bit count - 2
    pub const FLAGS: [usize; 3] = [0x124, 0x164, 0x1a4]; // 7x7 cells, rows of 8
    pub const SEED: usize = 0x4992;
    pub const CASTLE_PAL: usize = 0x49bc;
    pub const ULTIMA_I: usize = 0x26e;
    pub const FIRST_AGE: usize = 0x277;
    pub const COPYRIGHT: usize = 0x291;
    pub const TM1: usize = 0x43c0;
    pub const TM2: usize = 0x43e7;
    pub const TM3: usize = 0x440a;
    pub const CONVERSION: usize = 0x441f;
    pub const BLANK: usize = 0x44a6;
    pub const PAGE1: [usize; 3] = [0x44b2, 0x44cf, 0x44ec];
    pub const PAGE2: [usize; 2] = [0x4509, 0x4524];
    pub const PRESENTS: usize = 0x453e;
}

/// Everything the intro needs from the user's files.
#[derive(Debug, Clone)]
pub struct TitleFiles {
    exe: Exe,
    /// 0 CGA, 1 EGA, 2 Tandy (the original's video variable).
    pub mode: u8,
    /// The castle picture as 320x200 device pixel values.
    castle: Vec<u8>,
}

impl TitleFiles {
    pub fn load(g: &GameDir, video: VideoMode) -> crate::Result<Self> {
        let data = g.read("ULTIMA.EXE")?;
        if data.len() < 0x4000 {
            return Err(crate::AssetError::BadSize { what: "ULTIMA.EXE".into(), expected: ">16K".into(), got: data.len() });
        }
        let base = Exe::mz_data_base(&data, DS_PARA);
        let mode = match video {
            VideoMode::Cga => 0,
            VideoMode::Ega => 1,
            VideoMode::Tandy => 2,
        };
        let castle = if mode == 0 {
            // CASTLE.4: a CGA screen image (even lines, then odd lines at
            // 0x2000; four 2-bit pixels per byte, leftmost in the top bits).
            let b = g.read("CASTLE.4")?;
            if b.len() < 0x4000 {
                return Err(crate::AssetError::BadSize { what: "CASTLE.4".into(), expected: "16384".into(), got: b.len() });
            }
            let mut px = vec![0u8; W * H];
            for y in 0..H {
                for x in 0..W {
                    let v = b[(y & 1) * 0x2000 + (y / 2) * 80 + x / 4];
                    px[y * W + x] = (v >> (6 - 2 * (x % 4))) & 3;
                }
            }
            px
        } else {
            // CASTLE.16: 200 lines of 160 bytes, two 4-bit pixels per byte
            // (left one in the high half).
            let b = g.read("CASTLE.16")?;
            if b.len() < 32000 {
                return Err(crate::AssetError::BadSize { what: "CASTLE.16".into(), expected: "32000".into(), got: b.len() });
            }
            let mut px = vec![0u8; W * H];
            for y in 0..H {
                for x in 0..W {
                    let v = b[y * 160 + x / 2];
                    px[y * W + x] = if x % 2 == 0 { v >> 4 } else { v & 15 };
                }
            }
            px
        };
        Ok(TitleFiles { exe: Exe::with_base(data, base), mode, castle })
    }

    /// Text colour (`4a20`): white, or CGA colour 3.
    fn c_text(&self) -> u8 {
        if self.mode == 0 { 3 | RAW } else { 15 }
    }
    /// Logo colour: light blue, or CGA colour 1 (`4a22`).
    fn c_logo(&self) -> u8 {
        if self.mode == 0 { 1 | RAW } else { 9 }
    }
    /// Line colour (`4a24`): blue, or CGA colour 2.
    fn c_line(&self) -> u8 {
        if self.mode == 0 { 2 | RAW } else { 1 }
    }
    /// Page text colour (`4a26`): light cyan, or CGA colour 3.
    fn c_page(&self) -> u8 {
        if self.mode == 0 { 3 | RAW } else { 11 }
    }

    /// Logo cell: `*` in the left table for columns up to `split`, else in
    /// the right table at `col - split - 1`.
    fn logo(&self, row: usize, col: usize, split: usize) -> bool {
        let (table, c) = if col <= split { (o::LOGO_LEFT, col) } else { (o::LOGO_RIGHT, col - split - 1) };
        let p = self.exe.w(table + 2 * row) as usize;
        self.exe.b(p + c) == b'*'
    }

    /// The castle palette: on EGA the 6-bit values are read by a 200-line
    /// monitor as IRGB (bit 4 is intensity); Tandy uses their low 4 bits.
    fn castle_remap(&self) -> [u8; 16] {
        let mut r = [0u8; 16];
        for (i, v) in r.iter_mut().enumerate() {
            let p = self.exe.b(o::CASTLE_PAL + i);
            *v = if self.mode == 1 { (p & 7) | ((p >> 1) & 8) } else { p & 15 };
        }
        r
    }
}

/// How the intro ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntroEnd {
    /// A key was pressed; the credits page was shown and GEN follows.
    ToMenu,
}

/// LFSR steps of the logo dissolve per 70 Hz frame (the whole dissolve
/// takes about 1.15 s on the reference machine; the original is paced only
/// by its per-step keyboard check).
const DISSOLVE_STEPS_PER_FRAME: usize = 51;

struct Intro<'a> {
    f: &'a TitleFiles,
    seed: u16,
}

impl Intro<'_> {
    /// `3b96`: the start-up program's own generator, a value in lo..=hi.
    fn rand(&mut self, lo: u16, hi: u16) -> u16 {
        self.seed = self.seed.wrapping_add(0x9248).rotate_right(3);
        let mut a = self.seed & 0x3ff;
        let span = hi.min(0xff) - lo;
        while a > span {
            a -= span;
        }
        (a + lo) & 0xff
    }

    /// `d4c`: clear the screen and dissolve the logo in; `true` if a key
    /// stopped it.
    fn dissolve(&mut self, t: &mut Term, io: &mut dyn Io) -> IoResult<bool> {
        clear(t);
        let c = self.f.c_logo();
        let (x0, y0, x1, y1) = (0x15i32, 0x14i32, 0x129i32, 0x53i32);
        let w = ((x1 - x0) / 4 + (x1 - x0) % 2) as u32;
        let h = ((y1 - y0 - 1) / 2 + (y1 - y0 - 1) % 2) as u32;
        let n = w * h;
        let bits = 32 - (n - 1).leading_zeros();
        let taps = self.f.exe.w(o::TAPS + 2 * (bits as usize - 2)) as i16 as i32 as u32;
        let mut state: u32 = 1;
        let mut steps = 0;
        loop {
            let (q, r) = (state / w, state % w);
            if q < h {
                for k in 0..4 {
                    let col = (r + k * w) as usize;
                    for row in [q, q + h] {
                        if self.f.logo(row as usize, col, 0xc6) {
                            t.plot(x0 - 1 + col as i32, y0 + row as i32, c);
                        }
                    }
                }
            }
            state = if state & 1 != 0 { (state >> 1) ^ taps } else { state >> 1 };
            steps += 1;
            if steps % DISSOLVE_STEPS_PER_FRAME == 0 || state == 1 {
                keep(t, "intro");
                if io.poll_now(t)?.is_some() {
                    return Ok(true);
                }
                io.retrace(t, 1)?;
            }
            if state == 1 {
                break;
            }
        }
        t.plot(0x59, 0x33, c);
        keep(t, "intro");
        Ok(false)
    }

    /// `db3`: the logo, then "Presents...".
    fn presents(&mut self, t: &mut Term, io: &mut dyn Io) -> IoResult<bool> {
        if self.dissolve(t, io)? {
            return Ok(true);
        }
        t.cursor(14, 13);
        io.pause(t, 5)?;
        t.print_attr(&self.f.exe.s(o::PRESENTS), self.f.c_page());
        keep(t, "intro");
        io.pause(t, 0x2d)?;
        Ok(false)
    }

    /// 75 ticks or a key.
    fn page_wait(&mut self, t: &mut Term, io: &mut dyn Io) -> IoResult<bool> {
        for _ in 0..0x4b {
            io.pause(t, 1)?;
            if io.poll_now(t)?.is_some() {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// `c08`: the two text pages and the castle.
    fn pages(&mut self, t: &mut Term, io: &mut dyn Io) -> IoResult<bool> {
        let c = self.f.c_page();
        t.print_at_attr(14, 13, &self.f.exe.s(o::BLANK), c);
        for (i, &s) in o::PAGE1.iter().enumerate() {
            t.print_at_attr(5, 12 + i, &self.f.exe.s(s), c);
        }
        keep(t, "intro");
        if self.page_wait(t, io)? {
            return Ok(true);
        }
        t.fill(0, 0x5a, 0x13f, 0xb3, RAW);
        for (i, &s) in o::PAGE2.iter().enumerate() {
            t.print_at_attr(6, 12 + i, &self.f.exe.s(s), c);
        }
        keep(t, "intro");
        if self.page_wait(t, io)? {
            return Ok(true);
        }
        self.castle(t, io)
    }

    /// `3e9`: one flag frame at x.
    fn flag(&mut self, t: &mut Term, x: i32) {
        let frame = o::FLAGS[(self.rand(1, 0xef) / 0x50).min(2) as usize];
        let (on, off) = if self.f.mode == 0 { (2 | RAW, 1 | RAW) } else { (0xa, 0xb) };
        for row in 0..7 {
            for col in 0..7 {
                let c = if self.f.exe.b(frame + row * 8 + col) != 0 { on } else { off };
                t.plot(x + col as i32, 0x37 + row as i32, c);
            }
        }
    }

    /// `b65`: the castle picture with waving flags, about 17 s.
    fn castle(&mut self, t: &mut Term, io: &mut dyn Io) -> IoResult<bool> {
        clear(t);
        if self.f.mode != 0 {
            t.remap = Some(self.f.castle_remap());
        }
        t.px.copy_from_slice(&self.f.castle);
        keep(t, "castle");
        let mut key = false;
        for _ in 0..100 {
            io.pause(t, 3)?;
            self.flag(t, 0x7b);
            keep(t, "castle");
            if io.poll_now(t)?.is_some() {
                key = true;
                break;
            }
            self.flag(t, 0xc4);
            keep(t, "castle");
            if io.poll_now(t)?.is_some() {
                key = true;
                break;
            }
        }
        clear(t);
        t.remap = None;
        Ok(key)
    }

    /// `926` + `a27`: the whole logo at once, the trademark lines and the
    /// conversion credit, shown for 60 ticks.
    fn credits(&mut self, t: &mut Term, io: &mut dyn Io) -> IoResult<()> {
        t.fill(0, 0x14, 0x13f, 0x53, RAW);
        let c = self.f.c_logo();
        for row in 0..64 {
            for col in 0..0x113 {
                // This copy splits the two tables one column earlier than
                // the dissolve does.
                if self.f.logo(row, col, 0xc5) {
                    t.plot(0x14 + col as i32, 0x14 + row as i32, c);
                }
            }
        }
        let tc = self.f.c_text();
        t.print_at_attr(1, 17, &self.f.exe.s(o::TM1), tc);
        t.print_at_attr(2, 18, &self.f.exe.s(o::TM2), tc);
        t.print_at_attr(11, 19, &self.f.exe.s(o::TM3), tc);
        t.print_at_attr(6, 23, &self.f.exe.s(o::CONVERSION), tc);
        keep(t, "intro");
        io.pause(t, 0x3c)
    }
}

fn clear(t: &mut Term) {
    t.fill(0, 0, W - 1, H - 1, RAW);
}

/// Note the whole screen as the enhanced pack's `screen.NAME` picture as
/// it now stands (redone after each change the original makes, so the
/// region stays intact).
fn keep(t: &mut Term, name: &'static str) {
    t.mark(ArtId::Screen(name), None, 0, 0, 0, W, H);
}

/// A black screen for the start-up programs.
pub fn blank_term(f: &TitleFiles) -> Term {
    let mut t = Term::with_cga(f.mode == 0);
    clear(&mut t);
    t.cursor(0, 0);
    t
}

/// `586`: the text title shown while the castle picture loads.
pub fn title_screen(f: &TitleFiles, t: &mut Term) {
    clear(t);
    for i in 0..3 {
        t.line(0x70, 0x3a + i, 0xc8, 0x3a + i, f.c_line());
        t.line(0x70, 0x4a + i, 0xc8, 0x4a + i, f.c_line());
    }
    t.print_at_attr(16, 8, &f.exe.s(o::ULTIMA_I), f.c_text());
    t.print_at_attr(8, 11, &f.exe.s(o::FIRST_AGE), f.c_text());
    t.print_at_attr(0, 21, &f.exe.s(o::COPYRIGHT), f.c_text());
    keep(t, "title");
}

/// The whole start-up sequence, repeating until a key is pressed; returns
/// after the credits page.
pub fn run_intro(f: &TitleFiles, t: &mut Term, io: &mut dyn Io) -> IoResult<IntroEnd> {
    title_screen(f, t);
    // The original shows this while it reads the castle file (about 0.7 s
    // on the reference machine).
    io.pause(t, 12)?;
    let mut it = Intro { f, seed: f.exe.w(o::SEED) };
    loop {
        if it.presents(t, io)? {
            break;
        }
        if it.pages(t, io)? {
            clear(t);
            break;
        }
    }
    it.credits(t, io)?;
    Ok(IntroEnd::ToMenu)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flag_generator_spans_three_frames() {
        let f = TitleFiles { exe: Exe::with_base(vec![0; 0x5000], 0), mode: 1, castle: vec![0; W * H] };
        let mut it = Intro { f: &f, seed: 0 };
        let mut seen = [false; 3];
        for _ in 0..200 {
            let v = it.rand(1, 0xef);
            assert!((1..=0xef).contains(&v));
            seen[(v / 0x50) as usize] = true;
        }
        assert_eq!(seen, [true; 3]);
    }
}
