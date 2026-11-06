//! The character program (GEN.EXE equivalent, SPEC §17): the main menu,
//! character generation, roster replacement and selection, and the
//! "please wait" screen shown while the game loads.
//!
//! All text and the cursor shapes are read from the user's GEN.EXE at run
//! time; the drawing follows the original's order so the screens match.

use std::path::{Path, PathBuf};

use crate::exe::Exe;
use crate::gamedir::{GameDir, VideoMode};
use crate::io::{keys, Io, IoResult};
use crate::term::{ArtId, Term, RAW};

/// GEN.EXE's data segment (paragraph from the start of the load image).
const DS_PARA: usize = 0x426;

/// Data-segment offsets of the strings and tables used.
mod o {
    pub const MENU_TITLE: usize = 0x1b2;
    pub const MENU_LINES: [usize; 3] = [0x1c3, 0x1d0, 0x1dd];
    pub const MENU_A: usize = 0x1ec;
    pub const MENU_B: usize = 0x206;
    pub const CHOICE: usize = 0x220;
    pub const RACE_ASK: usize = 0x22d;
    pub const RACE_OPTS: [usize; 4] = [0x23f, 0x248, 0x24f, 0x258];
    pub const RACE: usize = 0x262;
    pub const RACE_NAMES: usize = 0x13c;
    pub const SEX_ASK: usize = 0x269;
    pub const SEX_OPTS: [usize; 2] = [0x27a, 0x282];
    pub const SEX: usize = 0x28c;
    pub const SEX_NAMES: [usize; 2] = [0x292, 0x297];
    pub const CLASS_ASK: usize = 0x29e;
    pub const CLASS_OPTS: [usize; 4] = [0x2b1, 0x2bc, 0x2c6, 0x2d0];
    pub const CLASS: usize = 0x2d9;
    pub const CLASS_NAMES: usize = 0x160;
    pub const NAME_ASK: usize = 0x2e1;
    pub const GEN_TITLE: usize = 0x2f2;
    pub const SAVE_ASK: usize = 0x309;
    pub const YES: usize = 0x325;
    pub const BLANK: usize = 0x329;
    pub const POINTS: usize = 0x32b;
    pub const STATS: [usize; 6] = [0x346, 0x359, 0x36c, 0x37f, 0x392, 0x3a5];
    pub const HELP: [usize; 5] = [0x3b8, 0x3dd, 0x3fe, 0x421, 0x444];
    pub const WRITE_ERR: usize = 0x463;
    pub const PRESS_RETURN: usize = 0x47f;
    pub const PAREN: usize = 0x48f;
    pub const REPL_TITLE: usize = 0x492;
    pub const REPL_TEXT: [usize; 5] = [0x4aa, 0x4c8, 0x4e8, 0x503, 0x521];
    pub const WAIT: [usize; 2] = [0x579, 0x58c];
    pub const SEL_TITLE: usize = 0x59b;
    pub const SEL_NONE: [usize; 4] = [0x5b1, 0x5d2, 0x5f4, 0x616];
    pub const SEL_TEXT: [usize; 3] = [0x634, 0x656, 0x678];
    pub const CURSOR: usize = 0x9fe;
}

/// Original timer ticks after the "please wait" screen is drawn (~0.5 s at
/// 18.2 Hz). The original stayed up while OUT.EXE loaded from disk; modern
/// loads finish almost at once, so this alone flashes by.
pub const LOADING_DWELL_ORIGINAL: u32 = 9;

/// Readable dwell when Message pauses is on (~2.5 s at 18.2 Hz). No
/// press-any-key; the front end keeps polling so the window stays responsive.
pub const LOADING_DWELL_READABLE: u32 = 46;

/// How long the "Please wait whilst / thy game loads" screen stays after it
/// is drawn. Tied to Message pauses (default on): off keeps the original
/// short pause; on gives a readable dwell. Other messages are unchanged.
pub fn loading_dwell_ticks(message_pauses: bool) -> u32 {
    if message_pauses {
        LOADING_DWELL_READABLE
    } else {
        LOADING_DWELL_ORIGINAL
    }
}

/// GEN.EXE's text and cursor shapes plus the video mode it runs in.
pub struct GenFiles {
    exe: Exe,
    /// 0 CGA, 1 EGA, 2 Tandy.
    pub mode: u8,
}

impl GenFiles {
    pub fn load(g: &GameDir, video: VideoMode) -> crate::Result<Self> {
        let data = g.read("GEN.EXE")?;
        if data.len() < 0x4800 {
            return Err(crate::AssetError::BadSize { what: "GEN.EXE".into(), expected: ">18K".into(), got: data.len() });
        }
        let base = Exe::mz_data_base(&data, DS_PARA);
        let mode = match video {
            VideoMode::Cga => 0,
            VideoMode::Ega => 1,
            VideoMode::Tandy => 2,
        };
        Ok(GenFiles { exe: Exe::with_base(data, base), mode })
    }

    fn s(&self, off: usize) -> String {
        self.exe.s(off)
    }

    /// The colour variables set up by `3e9` (CGA ones are device values).
    fn white(&self) -> u8 {
        if self.mode == 0 {
            3 | RAW
        } else {
            15
        }
    }
    fn blue(&self) -> u8 {
        if self.mode == 0 {
            2 | RAW
        } else {
            1
        }
    }
    fn red(&self) -> u8 {
        if self.mode == 0 {
            1 | RAW
        } else {
            12
        }
    }
    fn text(&self) -> u8 {
        if self.mode == 0 {
            3 | RAW
        } else {
            11
        }
    }
}

const BLACK: u8 = RAW;

/// How the character program ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenEnd {
    /// Play the character in this roster file (slot 1..=4).
    Play { slot: u8, path: PathBuf },
}

/// The program's state: the roster "disk", the four names and the
/// character record being built (kept between generations, as the
/// original's buffer is).
pub struct Gen<'a> {
    f: &'a GenFiles,
    dir: PathBuf,
    names: [[u8; 15]; 4],
    used: [bool; 4],
    count: usize,
    rec: [u8; crate::save::SAVE_SIZE],
    frame: usize,
    clock: Box<dyn FnMut() -> [u8; 4] + 'a>,
}

/// The roster file for slot 0..=3 in `dir`, matching an existing file's
/// case if there is one.
pub fn roster_file(dir: &Path, slot: usize) -> PathBuf {
    let want = format!("PLAYER{}.U1", slot + 1);
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            if e.file_name().to_string_lossy().eq_ignore_ascii_case(&want) {
                return e.path();
            }
        }
    }
    dir.join(want)
}

/// `3ec4`: the number the original mixes into a new character's seed,
/// from the DOS clock (hour, minute, second, hundredths).
pub fn clock_value(t: [u8; 4]) -> u16 {
    let [h, m, s, c] = t.map(u16::from);
    let a = (((s << 3) & 0xff) << 8 | c).wrapping_add((((h << 1) & 0xff) << 8) | ((m << 2) & 0xff));
    (a ^ 0x91eb) & 0xfff
}

impl<'a> Gen<'a> {
    /// `13b2`: read the first 15 bytes (the name) of each roster file.
    pub fn new(f: &'a GenFiles, dir: &Path, clock: Box<dyn FnMut() -> [u8; 4] + 'a>) -> Self {
        let mut g = Gen { f, dir: dir.to_path_buf(), names: [[0; 15]; 4], used: [false; 4], count: 0, rec: [0; crate::save::SAVE_SIZE], frame: 0, clock };
        for i in 0..4 {
            if let Ok(b) = std::fs::read(roster_file(dir, i)) {
                let n = b.len().min(15);
                g.names[i][..n].copy_from_slice(&b[..n]);
                g.used[i] = true;
                g.count += 1;
            }
        }
        g
    }

    fn name(&self, i: usize) -> String {
        self.names[i].iter().take_while(|&&b| b != 0).map(|&b| b as char).collect()
    }

    fn line(&self, t: &mut Term, x0: i32, y0: i32, x1: i32, y1: i32, c: u8) {
        t.line_u1(x0, y0, x1, y1, c);
    }

    fn at(&self, t: &mut Term, col: usize, row: usize, off: usize, c: u8) {
        t.print_at_attr(col, row, &self.f.s(off), c);
    }

    /// `5b2` and `48a`: the menu screen's border.
    fn border(&self, t: &mut Term) {
        let (b, w) = (self.f.blue(), self.f.white());
        t.fill(0, 0, 0x13c, 6, b);
        t.fill(0, 6, 6, 0xc7, b);
        t.fill(0x139, 2, 0x13f, 0xc7, b);
        t.fill(0, 0xc1, 0x13f, 0xc7, b);
        self.line(t, 7, 7, 0x138, 7, w);
        self.line(t, 7, 7, 7, 0xc0, w);
        self.line(t, 0x138, 7, 0x138, 0xc0, w);
        self.line(t, 7, 0xc0, 0x138, 0xc0, w);
        for i in 1..=4 {
            self.line(t, i, 0, 0, i, BLACK);
            self.line(t, 0x13f - i, 0, 0x13f, i, BLACK);
            self.line(t, 0, 0xc7 - i, i, 0xc7, BLACK);
            self.line(t, 0x13f, 0xc7 - i, 0x13f - i, 0xc7, BLACK);
        }
        for (x, y) in [(0, 0), (0, 5), (5, 0), (0x13f, 0), (0x13a, 0), (0x13f, 5), (0, 0xc7), (0, 0xc2), (5, 0xc7), (0x13f, 0xc7), (0x13f, 0xc2), (0x13a, 0xc7)] {
            t.plot(x, y, BLACK);
        }
    }

    /// `670`: clear the inside and mend the top border under a title.
    fn mend(&self, t: &mut Term) {
        t.fill(8, 8, 0x137, 0xbf, BLACK);
        t.fill(0x1e, 0, 0x12c, 6, self.f.blue());
        self.line(t, 0x1e, 7, 0x12c, 7, self.f.white());
    }

    /// `387e`: the left ornament (an arrow outlined in white).
    fn orn_left(&self, t: &mut Term, col: usize, row: usize) {
        let (b, w) = (self.f.blue(), self.f.white());
        let (x, y) = ((col * 8) as i32, (row * 8) as i32);
        t.cursor(col, row);
        t.print_attr("\u{10}", b);
        self.line(t, x, y, x + 7, y + 3, w);
        self.line(t, x + 7, y + 3, x, y + 7, w);
        self.line(t, x, y + 1, x, y + 6, b);
    }

    /// `3920`: the right ornament.
    fn orn_right(&self, t: &mut Term, col: usize, row: usize) {
        let (b, w) = (self.f.blue(), self.f.white());
        let (x, y) = ((col * 8) as i32, (row * 8) as i32);
        t.cursor(col, row);
        t.print_attr("\u{11}", b);
        self.line(t, x + 7, y, x, y + 3, w);
        self.line(t, x, y + 3, x + 7, y + 7, w);
        self.line(t, x + 7, y + 1, x + 7, y + 6, b);
    }

    /// An ornamented title in the top border.
    fn title(&self, t: &mut Term, left: usize, off: usize, right: usize) {
        self.orn_left(t, left, 0);
        t.print_attr(&self.f.s(off), self.f.white());
        self.orn_right(t, right, 0);
    }

    /// `413c`: draw the next cursor frame at the text cursor (frame 4 is
    /// blank); nothing in colour 0.
    fn cursor(&mut self, t: &mut Term, colour: u8) {
        if colour != 0 {
            let mut rows = [0u8; 8];
            if self.frame < 4 {
                for (r, row) in rows.iter_mut().enumerate() {
                    // Stored with the leftmost pixel in the top bit.
                    *row = self.f.exe.b(o::CURSOR + self.frame * 8 + r).reverse_bits();
                }
            }
            t.cursor_frame(rows, colour);
        }
        self.frame = (self.frame + 1) & 3;
    }

    /// `39c5`: wait for a key with the animated cursor in `colour`
    /// (0: no cursor).
    fn key(&mut self, t: &mut Term, io: &mut dyn Io, colour: u8) -> IoResult<u8> {
        loop {
            self.cursor(t, colour);
            match io.poll_now(t)? {
                None => io.pause(t, 3)?,
                Some(k) => {
                    io.pause(t, 1)?;
                    io.flush();
                    self.frame = 4;
                    self.cursor(t, colour);
                    return Ok(k);
                }
            }
        }
    }

    /// The whole program: the menu until a character is chosen.
    pub fn run(&mut self, t: &mut Term, io: &mut dyn Io) -> IoResult<GenEnd> {
        t.fill(0, 0, 0x13f, 0xc7, BLACK);
        self.border(t);
        // The inside of the border, for the enhanced pack's backdrop: it
        // stands through every screen of the menu, so none shows the
        // original black first.
        t.backdrop = Some((ArtId::Screen("menu"), 8, 8, 0x130, 0xb8));
        loop {
            let k = self.menu(t, io)?;
            match k {
                b'A' => {
                    let mut slot = self.free_slot();
                    if slot.is_none() {
                        slot = self.replace(t, io)?;
                        t.fill(0x100, 0, 0x108, 6, self.f.blue());
                        self.line(t, 0x100, 7, 0x108, 7, self.f.white());
                    }
                    if let Some(s) = slot {
                        self.generate(t, io, s)?;
                    }
                    self.mend(t);
                }
                b'B' => {
                    if let Some(end) = self.select(t, io)? {
                        return Ok(end);
                    }
                    self.mend(t);
                }
                _ => {}
            }
            if k == b'A' || k == b'B' {
                t.fill(10, 10, 0x12c, 0xbe, BLACK);
            }
        }
    }

    /// `6bb`: the main menu; returns A, B or Q.
    fn menu(&mut self, t: &mut Term, io: &mut dyn Io) -> IoResult<u8> {
        let (w, c) = (self.f.white(), self.f.text());
        self.at(t, 12, 6, o::MENU_TITLE, w);
        for (i, (col, off)) in [(14, o::MENU_LINES[0]), (14, o::MENU_LINES[1]), (13, o::MENU_LINES[2])].into_iter().enumerate() {
            self.at(t, col, 9 + i, off, w);
        }
        self.at(t, 8, 14, o::MENU_A, c);
        self.at(t, 8, 15, o::MENU_B, c);
        self.at(t, 13, 18, o::CHOICE, c);
        loop {
            let k = self.key(t, io, c)?.to_ascii_uppercase();
            if matches!(k, b'A' | b'B' | b'Q') {
                return Ok(k);
            }
        }
    }

    /// `1423`: the first empty roster slot.
    fn free_slot(&self) -> Option<usize> {
        (0..4).find(|&i| !self.used[i])
    }

    /// `1455`: the numbered roster.
    fn roster(&self, t: &mut Term) {
        let w = self.f.white();
        for i in 0..4 {
            t.cursor(15, 6 + i);
            t.num(i as u16 + 1, 1, ' ', w, true);
            t.print_attr(&self.f.s(o::PAREN), w);
            t.print_attr(&self.name(i), w);
        }
    }

    /// `14bb`: pick a character to overwrite; None on Escape.
    fn replace(&mut self, t: &mut Term, io: &mut dyn Io) -> IoResult<Option<usize>> {
        t.fill(10, 10, 0x12c, 0xb4, BLACK);
        self.title(t, 8, o::REPL_TITLE, 0x20);
        self.roster(t);
        let c = self.f.text();
        for (i, &off) in o::REPL_TEXT.iter().enumerate() {
            self.at(t, 4, 14 + i, off, c);
        }
        loop {
            match self.key(t, io, c)? {
                k @ b'1'..=b'4' => return Ok(Some((k - b'1') as usize)),
                keys::ESC => return Ok(None),
                _ => {}
            }
        }
    }

    /// `16dc`: choose the character to play.
    fn select(&mut self, t: &mut Term, io: &mut dyn Io) -> IoResult<Option<GenEnd>> {
        t.fill(10, 10, 0x135, 0xbd, BLACK);
        self.title(t, 9, o::SEL_TITLE, 0x1f);
        if self.count == 0 {
            let w = self.f.white();
            for (i, &off) in o::SEL_NONE.iter().enumerate() {
                self.at(t, 4, 10 + i, off, w);
            }
            loop {
                match self.key(t, io, w)? {
                    keys::ENTER => {
                        self.generate(t, io, 0)?;
                        return Ok(None);
                    }
                    keys::ESC => return Ok(None),
                    _ => {}
                }
            }
        }
        self.roster(t);
        let c = self.f.text();
        for (i, &off) in o::SEL_TEXT.iter().enumerate() {
            self.at(t, 4, 14 + i, off, c);
        }
        loop {
            match self.key(t, io, c)? {
                k @ b'1'..=b'4' if self.used[(k - b'1') as usize] => {
                    let slot = k - b'1';
                    self.loading(t);
                    return Ok(Some(GenEnd::Play { slot: slot + 1, path: roster_file(&self.dir, slot as usize) }));
                }
                keys::ESC => return Ok(None),
                _ => {}
            }
        }
    }

    /// `1643` / `3ad7`: the screen left up while the game program loads.
    pub fn loading(&self, t: &mut Term) {
        let (b, w) = (self.f.blue(), self.f.white());
        t.fill(0, 0, 0x13f, 0xc7, BLACK);
        t.fill(0, 0, 0x13c, 6, b);
        t.fill(0, 7, 7, 0x9e, b);
        t.fill(0x138, 2, 0x13f, 0x9e, b);
        t.fill(0, 0x98, 0x13f, 0x9e, b);
        self.line(t, 8, 7, 0x137, 7, w);
        self.line(t, 7, 8, 7, 0x97, w);
        self.line(t, 0x138, 8, 0x138, 0x97, w);
        self.line(t, 8, 0x98, 0x137, 0x98, w);
        self.line(t, 0, 0x9f, 0x13f, 0x9f, w);
        self.line(t, 0xf1, 0x9f, 0xf1, 0xc7, w);
        self.line(t, 0xf6, 0x9f, 0xf6, 0xc7, w);
        for i in 1..=4 {
            self.line(t, i, 0, 0, i, BLACK);
            self.line(t, 0x13f - i, 0, 0x13f, i, BLACK);
            self.line(t, 0xf1 + i, 0x9f, 0xf1 + i, 0xc7, b);
        }
        for (x, y) in [(0, 0), (0, 5), (5, 0), (0x13f, 0), (0x13a, 0), (0x13f, 5)] {
            t.plot(x, y, BLACK);
        }
        self.at(t, 11, 8, o::WAIT[0], w);
        self.at(t, 13, 10, o::WAIT[1], w);
        t.backdrop = Some((ArtId::Screen("loading"), 8, 8, 0x130, 0x90));
    }

    fn stat(&self, i: usize) -> u16 {
        u16::from_le_bytes([self.rec[0x16 + 2 * i], self.rec[0x17 + 2 * i]])
    }
    fn set(&mut self, off: usize, v: u16) {
        self.rec[off..off + 2].copy_from_slice(&v.to_le_bytes());
    }
    fn set_stat(&mut self, i: usize, v: u16) {
        self.set(0x16 + 2 * i, v);
    }

    /// `7a8`: show stat `i` (1..=6).
    fn show_stat(&self, t: &mut Term, i: usize) {
        t.cursor(0x1c, i + 5);
        t.num(self.stat(i), 2, ' ', self.f.text(), false);
    }

    /// `e07` / `dc2`: the markers either side of stat line `i`.
    fn markers(&self, t: &mut Term, i: usize, on: bool) {
        let c = self.f.text();
        if on {
            self.orn_left(t, 0xb, i + 5);
            self.orn_right(t, 0x1e, i + 5);
        } else {
            let s = self.f.s(o::BLANK);
            t.print_at_attr(0xb, i + 5, &s, c);
            t.print_at_attr(0x1e, i + 5, &s, c);
        }
    }

    /// `106a`: generate a character into roster slot `slot`.
    fn generate(&mut self, t: &mut Term, io: &mut dyn Io, slot: usize) -> IoResult<()> {
        let c = self.f.text();
        loop {
            self.title(t, 8, o::GEN_TITLE, 0x1f);
            // `ef6`: the points screen.
            t.fill(10, 10, 0x136, 0xb9, BLACK);
            self.at(t, 6, 4, o::POINTS, c);
            for i in 1..=6 {
                self.at(t, 12, i + 5, o::STATS[i - 1], c);
                self.set_stat(i, 10);
            }
            for (i, &off) in o::HELP.iter().enumerate() {
                self.at(t, 2, 16 + i, off, c);
            }
            let mut cur = 1;
            self.markers(t, cur, true);
            let mut points: u16 = 30;
            loop {
                t.cursor(0x20, 4);
                t.num(points, 3, ' ', c, false);
                match self.key(t, io, 0)? {
                    b' ' => {
                        if points == 0 {
                            break;
                        }
                        io.sound(1);
                    }
                    keys::ESC => return Ok(()),
                    keys::DOWN => {
                        self.markers(t, cur, false);
                        cur = if cur == 6 { 1 } else { cur + 1 };
                        self.markers(t, cur, true);
                    }
                    keys::UP => {
                        self.markers(t, cur, false);
                        cur = if cur == 1 { 6 } else { cur - 1 };
                        self.markers(t, cur, true);
                    }
                    keys::RIGHT => {
                        if self.stat(cur) < 25 && points != 0 {
                            self.set_stat(cur, self.stat(cur) + 1);
                            self.show_stat(t, cur);
                            points -= 1;
                        }
                    }
                    keys::LEFT => {
                        if self.stat(cur) > 10 {
                            self.set_stat(cur, self.stat(cur) - 1);
                            self.show_stat(t, cur);
                            points += 1;
                        }
                    }
                    _ => {}
                }
            }
            self.markers(t, cur, false);

            // `7e2`: race.
            t.fill(0x30, 0x20, 0x12c, 0x28, BLACK);
            t.fill(0xe, 0x80, 0x12e, 0xb0, BLACK);
            self.at(t, 3, 17, o::RACE_ASK, c);
            for (i, &off) in o::RACE_OPTS.iter().enumerate() {
                self.at(t, 12, 19 + i, off, c);
            }
            t.cursor(20, 17);
            let race = self.choose(t, io, 4)?;
            self.set(0x10, race as u16);
            let bump = |g: &mut Self, t: &mut Term, i: usize, d: i16| {
                g.set_stat(i, g.stat(i).wrapping_add(d as u16));
                g.show_stat(t, i);
            };
            match race {
                0 => bump(self, t, 6, 5),
                1 => bump(self, t, 2, 5),
                2 => bump(self, t, 1, 5),
                _ => {
                    bump(self, t, 5, 10);
                    bump(self, t, 1, -5);
                }
            }
            self.at(t, 14, 13, o::RACE, c);
            t.print_attr(&self.f.exe.ptr(o::RACE_NAMES, race), c);

            // `995`: sex.
            t.fill(0x18, 0x91, 0x12c, 0xb8, BLACK);
            self.at(t, 3, 17, o::SEX_ASK, c);
            for (i, &off) in o::SEX_OPTS.iter().enumerate() {
                self.at(t, 12, 19 + i, off, c);
            }
            t.cursor(19, 17);
            let sex = self.choose(t, io, 2)?;
            self.set(0x14, sex as u16);
            self.at(t, 15, 14, o::SEX, c);
            t.print_attr(&self.f.s(o::SEX_NAMES[sex]), c);

            // `a91`: class.
            t.fill(0x18, 0x91, 0x12c, 0xb8, BLACK);
            self.at(t, 3, 17, o::CLASS_ASK, c);
            for (i, &off) in o::CLASS_OPTS.iter().enumerate() {
                self.at(t, 12, 19 + i, off, c);
            }
            t.cursor(21, 17);
            let class = self.choose(t, io, 4)?;
            self.set(0x12, class as u16);
            match class {
                0 => {
                    bump(self, t, 1, 10);
                    bump(self, t, 2, 10);
                }
                1 => bump(self, t, 5, 10),
                2 => bump(self, t, 6, 10),
                _ => bump(self, t, 2, 10),
            }
            self.at(t, 13, 15, o::CLASS, c);
            t.print_attr(&self.f.exe.ptr(o::CLASS_NAMES, class), c);

            // `c2c`: name.
            t.fill(0x18, 0x80, 0x12c, 0xb8, BLACK);
            self.at(t, 3, 17, o::NAME_ASK, c);
            let mut len = 0usize;
            loop {
                let k = self.key(t, io, c)?;
                if (k > 0x20 || (k == 0x20 && len != 0)) && len < 14 {
                    self.rec[len] = k;
                    len += 1;
                    t.print_attr(&(k as char).to_string(), c);
                } else if (k == 1 || k == 8) && len != 0 {
                    len -= 1;
                    t.cursor(0x13 + len, 0x11);
                } else if k == keys::ENTER && len != 0 {
                    break;
                }
            }
            self.rec[len] = 0;
            let name: String = self.rec[..len].iter().map(|&b| b as char).collect();
            t.print_at_attr(12, 4, &name, c);

            // `d50`: keep it?
            t.fill(0x18, 0x80, 0x12c, 0xb8, BLACK);
            self.at(t, 3, 22, o::SAVE_ASK, c);
            let keep = loop {
                match self.key(t, io, c)?.to_ascii_uppercase() {
                    b'Y' => {
                        t.print_attr(&self.f.s(o::YES), c);
                        break true;
                    }
                    b'N' => break false,
                    _ => {}
                }
            };
            if keep {
                self.save(t, io, slot)?;
                return Ok(());
            }
        }
    }

    /// A letter a.. for one of `n` choices, read with the cursor.
    fn choose(&mut self, t: &mut Term, io: &mut dyn Io, n: u8) -> IoResult<usize> {
        let c = self.f.text();
        loop {
            let k = self.key(t, io, c)?.to_ascii_uppercase();
            if (b'A'..b'A' + n).contains(&k) {
                return Ok((k - b'A') as usize);
            }
        }
    }

    /// Fill in the new character's fixed values and write the roster file.
    fn save(&mut self, t: &mut Term, io: &mut dyn Io, slot: usize) -> IoResult<()> {
        let seed = clock_value((self.clock)()).wrapping_add(self.stat(5)).wrapping_add(u16::from_le_bytes([self.rec[0x12], self.rec[0x13]]));
        for (off, v) in [(0x16, 150), (0x24, 100), (0x26, 0), (0x28, 200), (0x2a, 1), (0x2c, 0), (0x2e, 1), (0x30, 0), (0x34, 49), (0x36, 40), (0x38, 1)] {
            self.set(off, v);
        }
        self.set(0x32, seed);
        for off in (0x3a..=0x52).step_by(2) {
            self.set(off, 0);
        }
        self.set(0x54, 0xffff);
        self.set(0x56, 1);
        for off in (0x58..=0x5e).step_by(2) {
            self.set(off, 0);
        }
        self.set(0x60, 0xffff);
        self.set(0x62, 2);
        for off in (0x64..=0x7e).step_by(2) {
            self.set(off, 0);
        }
        self.set(0x80, 0xffff);
        for off in (0x82..=0x94).step_by(2) {
            self.set(off, 0);
        }
        self.set(0x96, 0xffff);
        for off in (0x98..=0xa4).step_by(2) {
            self.set(off, 0);
        }
        for off in [0xa6, 0xaa, 0xac, 0xae] {
            self.set(off, 0);
        }
        self.set(0xa8, 0xffff);
        self.set(0xb4, 0xa);
        self.set(0xb6, 1);
        self.set(0xb8, 49);
        self.set(0xba, 40);
        let path = roster_file(&self.dir, slot);
        if std::fs::write(&path, self.rec).is_err() {
            let r = self.f.red();
            self.at(t, 7, 18, o::WRITE_ERR, r);
            self.at(t, 14, 19, o::PRESS_RETURN, r);
            while self.key(t, io, 0)? != keys::ENTER {}
            return Ok(());
        }
        if !self.used[slot] {
            self.used[slot] = true;
            self.count += 1;
        }
        self.names[slot].copy_from_slice(&self.rec[..15]);
        Ok(())
    }

    /// The record most recently generated (for tests).
    pub fn record(&self) -> &[u8] {
        &self.rec
    }
}

#[cfg(test)]
mod dwell_tests {
    use super::*;

    #[test]
    fn loading_dwell_original_when_message_pauses_off() {
        assert_eq!(loading_dwell_ticks(false), LOADING_DWELL_ORIGINAL);
        assert_eq!(LOADING_DWELL_ORIGINAL, 9, "must match the historical pause(&t, 9)");
    }

    #[test]
    fn loading_dwell_readable_when_message_pauses_on() {
        assert_eq!(loading_dwell_ticks(true), LOADING_DWELL_READABLE);
        // ~2.5 s at the engine's 18.2 Hz tick (54_925 µs); keep a readable band.
        let ms = u64::from(LOADING_DWELL_READABLE) * 54_925 / 1_000;
        assert!((2_400..=2_700).contains(&ms), "readable dwell ms = {ms}");
        assert!(LOADING_DWELL_READABLE > LOADING_DWELL_ORIGINAL);
    }
}
