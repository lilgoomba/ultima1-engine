//! Mondain's lair (MONDAIN.EXE): the time machine trip, the fight in the
//! arena, the gem, Mondain's transformation, his death and the ending
//! (SPEC §13).
//!
//! The strings, the arena layout and the weapon/spell tables come from the
//! user's MONDAIN.EXE; the pictures from EGAMOND.BIN, EGATILES.BIN and
//! NIF.BIN, all read at runtime.

use crate::exe::Exe;
use crate::game::Game;
use crate::gamedir::GameDir;
use crate::io::{keys, Halt, Io, IoResult};
use crate::rng::U1Rng;
use crate::save::off;
use crate::sound::Sfx;
use crate::term::{ArtId, BLACK, BLUE, RAW, TEXT, WHITE};
use crate::tiles::{TileSet, CGA_MOND, EGA_MOND, T1K_MOND};

/// Size of the supported MONDAIN.EXE.
pub const MONDAIN_EXE_SIZE: usize = 31650;
const DS_PARA: usize = 0x597;
/// NIF.BIN: a 320x168 one-bit picture.
const NIF_SIZE: usize = 6720;

const COLS: usize = 19;
const ROWS: usize = 9;

/// Arena cell values (also the tile slot drawn for them).
const EMPTY: u8 = 0;
const MACHINE: u8 = 1;
const WALL: u8 = 5;
const GEM: u8 = 6;
const FIRE: u8 = 0x0e;
const PLAYER: u8 = 0x14;

/// Mondain's tile bases and phases.
const T_WIZARD: i16 = 2;
const T_DEAD: i16 = 4;
const T_DEMON: i16 = 0x0b;
const P_WIZARD: i16 = 0x14;
const P_DEAD: i16 = 0x1e;
const P_DEMON: i16 = 0x28;

const GEM_X: usize = 14;
const GEM_Y: usize = 4;

/// Data-segment offsets in MONDAIN.EXE.
mod d {
    pub const GRID: usize = 0x194;
    pub const WEAPON_RANGE: usize = 0xcd2;
    pub const WEAPON_NAMES: usize = 0xdf8;
    pub const SPELL_NAMES: usize = 0xf04;
    pub const DIR_NAMES: usize = 0x1dc8;
    pub const PASS: usize = 0x24c;
    pub const HUH: usize = 0x251;
    pub const NOISE: usize = 0x256;
    pub const ON: usize = 0x25d;
    pub const OFF: usize = 0x260;
    pub const BLOCKED: usize = 0x264;
    pub const BURNED: usize = 0x26d;
    pub const DAMAGE: usize = 0x276;
    pub const MISSED: usize = 0x27f;
    pub const HIT_MONDAIN: usize = 0x287;
    pub const NOTHING: usize = 0x295;
    pub const ATTACK_WITH: usize = 0x29d;
    pub const EMPTY_A: usize = 0x2aa;
    pub const COLON: usize = 0x2ac;
    pub const EMPTY_B: usize = 0x2af;
    pub const DONE: usize = 0x2c2;
    pub const KILL_1: usize = 0x2c8;
    pub const KILL_2: usize = 0x2db;
    pub const KILL_3: usize = 0x2ef;
    pub const FAILED: usize = 0x305;
    pub const CAST: usize = 0x30d;
    pub const USED_UP: usize = 0x313;
    pub const GET_GEM: usize = 0x32e;
    pub const NOTHING_HERE: usize = 0x338;
    pub const DOTS: usize = 0x34d;
    pub const GEM_DESTROYED: usize = 0x352;
    pub const INFORM: usize = 0x368;
    pub const AURA_1: usize = 0x37a;
    pub const AURA_2: usize = 0x396;
    pub const AURA_3: usize = 0x3b4;
    pub const LOOKS_1: usize = 0x3cf;
    pub const LOOKS_2: usize = 0x3db;
    pub const LOOKS_3: usize = 0x3f9;
    pub const BOARD_Q: usize = 0x3fe;
    pub const DROP_Q: usize = 0x405;
    pub const ENTER_Q: usize = 0x40b;
    pub const FIRE_Q: usize = 0x412;
    pub const HYPER_Q: usize = 0x418;
    pub const KLIMB_Q: usize = 0x423;
    pub const OPEN_Q: usize = 0x42a;
    pub const QUIT: usize = 0x430;
    pub const STEAL_1: usize = 0x447;
    pub const STEAL_2: usize = 0x45e;
    pub const TRANSACT_1: usize = 0x475;
    pub const TRANSACT_2: usize = 0x48d;
    pub const UNLOCK_Q: usize = 0x49c;
    pub const VIEW_Q: usize = 0x4a4;
    pub const XIT_Q: usize = 0x4aa;
    pub const DEAD: usize = 0x4fa;
    pub const DEFEATED_1: usize = 0x509;
    pub const DEFEATED_2: usize = 0x527;
    pub const DOOMED: usize = 0x541;
    pub const HIT: usize = 0x570;
    pub const ATTACKED: usize = 0x576;
    pub const CASTS: usize = 0x58b;
    pub const MISSILE: usize = 0x59a;
    pub const BLASTER: usize = 0x5a9;
    pub const STATS_REDUCED: usize = 0x5b7;
    pub const SHOCK: usize = 0x5d0;
    pub const CHANTING: usize = 0x5df;
    pub const IS_DEAD: usize = 0x5fc;
    pub const OR_IS_HE: usize = 0x60d;
    pub const VICTORIOUS: usize = 0x8c3;
    pub const GEM_LABELS: usize = 0x657;
}

/// Intro panels `1b9b`, `1caf`, `1d53`, `1d94`: (col, row, string).
#[rustfmt::skip]
const INTRO: [&[(usize, usize, usize)]; 4] = [
    &[(5, 3, 0x61a), (5, 4, 0x638), (12, 6, d::GEM_LABELS), (4, 9, 0x668), (5, 11, 0x68a), (11, 12, 0x6a9),
      (4, 14, 0x6bd), (6, 15, 0x6de), (6, 16, 0x6fb)],
    &[(4, 4, 0x717), (4, 5, 0x738), (4, 6, 0x759), (16, 7, 0x777), (4, 9, 0x780), (12, 10, 0x7a1),
      (4, 12, 0x7b3), (8, 13, 0x7d5), (9, 14, 0x7ef)],
    &[(4, 8, 0x806), (4, 9, 0x828), (12, 10, 0x849)],
    &[(11, 6, 0x85b), (9, 7, 0x86e), (4, 8, 0x884), (9, 13, 0x8a4)],
];

/// MONDAIN.EXE, its tiles (slot 0..=20 as the original lays them out) and
/// the ending picture.
pub struct MondainFiles {
    pub exe: Exe,
    pub tiles: Vec<Vec<u8>>,
    pub nif: Vec<u8>,
}

impl MondainFiles {
    pub fn load(g: &GameDir, video: crate::gamedir::VideoMode) -> crate::Result<Self> {
        let data = g.read("MONDAIN.EXE")?;
        if data.len() != MONDAIN_EXE_SIZE {
            return Err(crate::AssetError::BadSize { what: "MONDAIN.EXE".into(), expected: MONDAIN_EXE_SIZE.to_string(), got: data.len() });
        }
        let base = Exe::mz_data_base(&data, DS_PARA);
        let wspec = video.world_tiles();
        let mspec = match video {
            crate::gamedir::VideoMode::Ega => EGA_MOND,
            crate::gamedir::VideoMode::Tandy => T1K_MOND,
            crate::gamedir::VideoMode::Cga => CGA_MOND,
        };
        let world = TileSet::decode_spec(&g.read(wspec.file_name)?, &wspec)?;
        let mond = TileSet::decode_spec(&g.read(mspec.file_name)?, &mspec)?;
        // Slot 0 is blank; the first eleven world tiles go to slots 10..=20,
        // then the nineteen lair tiles overwrite slots 1..=19.
        let mut tiles = vec![vec![0u8; 256]; 21];
        tiles[10..21].clone_from_slice(&world.tiles[..11]);
        tiles[1..20].clone_from_slice(&mond.tiles[..19]);
        let nif = g.read("NIF.BIN")?;
        if nif.len() < NIF_SIZE {
            return Err(crate::AssetError::BadSize { what: "NIF.BIN".into(), expected: NIF_SIZE.to_string(), got: nif.len() });
        }
        Ok(MondainFiles { exe: Exe::with_base(data, base), tiles, nif })
    }
}

/// How the lair ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MondainEnd {
    /// The player died: hits and food are 0 and OUT takes over again.
    Died,
    /// The ending screen was shown; the original stops there for good.
    Won,
}

#[derive(Debug)]
enum Stop {
    Halt,
    Died,
    Won,
}

impl From<Halt> for Stop {
    fn from(_: Halt) -> Self {
        Stop::Halt
    }
}

type R<T> = Result<T, Stop>;

struct Lair<'a> {
    g: &'a mut Game,
    io: &'a mut dyn Io,
    f: MondainFiles,
    /// The display chosen in the options menu has no lair pictures.
    keep_video: bool,
    rng: U1Rng,
    grid: [[u8; ROWS]; COLS],
    px: i16,
    py: i16,
    mx: i16,
    my: i16,
    hp: i16,
    /// Fire burst frame, -1 when none (`0x130`).
    burst: i16,
    bx: i16,
    by: i16,
    /// Gem glow frame and step (`0x190`, `0x192`).
    glow: i16,
    glow_step: i16,
    tile: i16,
    anim: i16,
    phase: i16,
    fighting: bool,
    gem_taken: bool,
    ending: bool,
    redraw: bool,
}

/// Run the lair from boarding the time machine with all four gems.
pub fn run_mondain(g: &mut Game, io: &mut dyn Io) -> IoResult<MondainEnd> {
    io.music(crate::music::Track::Lair);
    let Some(files) = g.mondain.take() else { return Ok(MondainEnd::Died) };
    let mut l = Lair::new(g, io, files);
    let r = l.main();
    let Lair { f: files, .. } = l;
    g.mondain = Some(files);
    match r {
        Err(Stop::Halt) => Err(Halt),
        Err(Stop::Won) => Ok(MondainEnd::Won),
        _ => Ok(MondainEnd::Died),
    }
}

impl<'a> Lair<'a> {
    fn new(g: &'a mut Game, io: &'a mut dyn Io, files: MondainFiles) -> Self {
        let mut grid = [[0u8; ROWS]; COLS];
        for (x, col) in grid.iter_mut().enumerate() {
            for (y, c) in col.iter_mut().enumerate() {
                *c = files.exe.b(d::GRID + x * ROWS + y);
            }
        }
        Lair {
            g,
            io,
            f: files,
            keep_video: false,
            rng: U1Rng::new(0),
            grid,
            px: 3,
            py: 4,
            mx: 15,
            my: 4,
            hp: 1000,
            burst: -1,
            bx: 0,
            by: 0,
            glow: 0,
            glow_step: 0,
            tile: T_WIZARD,
            anim: 0,
            phase: P_WIZARD,
            fighting: false,
            gem_taken: false,
            ending: false,
            redraw: false,
        }
    }
}

impl Lair<'_> {
    // ---------------------------------------------------------- helpers

    fn s(&self, o: usize) -> String {
        self.f.exe.s(o)
    }
    fn ptr(&self, table: usize, i: usize) -> String {
        self.f.exe.ptr(table, i)
    }
    fn puts(&mut self, o: usize) {
        let t = self.s(o);
        self.g.term.print_attr(&t, TEXT);
    }
    fn println(&mut self, o: usize) {
        let t = self.s(o);
        self.g.term.println(&t);
    }
    fn num(&mut self, n: i16) {
        self.g.term.print_attr(&n.to_string(), TEXT);
    }
    fn snd(&mut self, n: u8) {
        self.g.snd(self.io, n);
    }
    fn snd_ev(&mut self, n: u8, ev: Sfx) {
        self.g.snd_ev(self.io, n, ev);
    }
    fn cue(&mut self, ev: Sfx) {
        self.g.cue(self.io, ev);
    }
    fn rnd(&mut self, lo: i16, hi: i16) -> i16 {
        self.rng.range(lo as u16, hi as u16) as i16
    }
    fn delay(&mut self, ticks: u32) -> R<()> {
        self.io.pause(&self.g.term, ticks)?;
        Ok(())
    }
    fn stat(&self, o: usize) -> i16 {
        self.g.w(o)
    }
    fn show_hits(&mut self) {
        let h = self.g.hits().max(0);
        self.g.set(off::HITS, h.min(9999));
        self.g.term.stat(21, i32::from(h), true);
    }
    fn show_food(&mut self) {
        let f = self.g.food().max(0);
        self.g.set(off::FOOD, f.min(9999));
        self.g.term.stat(22, i32::from(f), true);
    }
    fn hurt(&mut self, n: i16) {
        let h = self.g.hits();
        self.g.set(off::HITS, h.wrapping_sub(n));
    }
    /// `4031`/`3ff9`: the EGA palette swap used as a flash (on CGA a white
    /// background and the other palette).
    fn flash(&mut self, on: bool) {
        self.g.term.inverted = on;
    }
    /// `117e`: a command that does nothing here.
    fn stub(&mut self, o: usize) {
        self.println(o);
        self.snd(1);
    }
    fn cell(&self, x: i16, y: i16) -> u8 {
        self.grid[x as usize][y as usize]
    }
    fn in_bounds(x: i16, y: i16) -> bool {
        (0..COLS as i16).contains(&x) && (0..ROWS as i16).contains(&y)
    }

    /// `355d`: one 16x16 tile at arena cell (x,y).
    fn draw_tile(&mut self, t: i16, x: i16, y: i16) {
        let i = t.clamp(0, 20) as usize;
        let px = &self.f.tiles[i];
        // Sprites stand on the floor tile (a pack's floor shows around them).
        let under = (i != 0 && i != 5).then_some(ArtId::Mond(0));
        self.g.term.blit_art(ArtId::Mond(i as u16), under, 8 + 16 * x as usize, 8 + 16 * y as usize, 16, 16, px, 0);
    }
    fn draw_cell(&mut self, x: i16, y: i16) {
        let c = self.cell(x, y);
        self.draw_tile(i16::from(c), x, y);
    }
    fn draw_mondain(&mut self) {
        let t = self.tile + self.anim;
        self.draw_tile(t, self.mx, self.my);
    }

    /// `13a3`: the whole arena.
    fn draw_all(&mut self) {
        self.redraw = false;
        for x in 0..COLS as i16 {
            for y in 0..ROWS as i16 {
                self.draw_cell(x, y);
            }
        }
        self.draw_mondain();
        if self.glow != 0 {
            self.glow = 1;
            self.glow_step = 1;
        }
    }

    /// A display chosen in the options menu: its pictures, the screen's
    /// colours converted, and the arena redrawn as it stands.
    fn switch_video(&mut self) {
        if self.keep_video || self.g.term.inverted {
            return;
        }
        let Some(mut v) = self.io.video_switch() else { return };
        let Some(f) = v.mondain.take() else {
            self.keep_video = true;
            return;
        };
        self.f = f;
        let mode = v.mode;
        self.g.take_video(v);
        self.io.video_shown(mode);
        for x in 0..COLS as i16 {
            for y in 0..ROWS as i16 {
                self.draw_cell(x, y);
            }
        }
        self.draw_mondain();
        if self.burst > 0 {
            self.draw_tile(i16::from(FIRE) + self.burst - 1, self.bx, self.by);
        }
        if self.glow != 0 {
            self.draw_tile(self.glow + 5, GEM_X as i16, GEM_Y as i16);
        }
    }

    /// `1477`: one animation step (fire burst, gem glow, Mondain).
    fn tick(&mut self) {
        if self.burst >= 0 {
            if self.burst == 6 {
                self.burst = -1;
                self.grid[self.bx as usize][self.by as usize] = EMPTY;
                self.draw_tile(0, self.bx, self.by);
            } else {
                self.draw_tile(i16::from(FIRE) + self.burst, self.bx, self.by);
                self.burst += 1;
            }
        }
        if self.glow != 0 {
            self.glow += self.glow_step;
            if self.glow == 1 {
                self.glow_step = 1;
            } else if self.glow > 4 {
                self.glow_step = -1;
            }
            self.draw_tile(self.glow + 5, GEM_X as i16, GEM_Y as i16);
        }
        if self.tile == T_WIZARD {
            self.anim = if self.rnd(1, 200) < 100 { 1 } else { 0 };
        } else if self.tile == T_DEMON {
            self.anim = (self.anim + 1) % 3;
        }
        self.draw_mondain();
    }

    /// `1b46`: one keyboard poll with its delays.
    fn poll(&mut self) -> R<Option<u8>> {
        let k = self.io.poll_now(&self.g.term)?;
        self.switch_video();
        self.delay(1)?;
        if k.is_none() {
            self.delay(1)?;
            if self.glow == 0 {
                self.delay(1)?;
            } else {
                self.io.retrace(&self.g.term, 4)?;
            }
        }
        Ok(k.map(|k| k.to_ascii_uppercase()))
    }

    /// `5db`: a direction key 1..=4 (W, E, N, S); anything else means none.
    fn direction(&mut self) -> R<i16> {
        loop {
            if let Some(k) = self.poll()? {
                return Ok(match k {
                    keys::LEFT => 1,
                    keys::RIGHT => 2,
                    keys::UP => 3,
                    keys::DOWN => 4,
                    _ => 5 + i16::from(k),
                });
            }
            self.tick();
        }
    }

    fn dxdy(dir: i16) -> (i16, i16) {
        match dir {
            1 => (-1, 0),
            2 => (1, 0),
            3 => (0, -1),
            _ => (0, 1),
        }
    }

    fn prompt(&mut self) {
        self.g.term.ornament_left(0, 24);
        self.g.term.cursor(1, 24);
    }

    // --------------------------------------------------------- program

    fn main(&mut self) -> R<()> {
        self.g.term.restore_labels();
        self.show_hits();
        self.show_food();
        let exp = self.g.exp();
        self.g.term.stat(23, i32::from(exp), false);
        let coin = self.g.coin();
        self.g.term.stat(24, i32::from(coin), false);
        let t = self.g.time_seed_now();
        self.rng.seed(t.wrapping_add(self.g.rec.u16_at(off::SEED)));
        self.intro()?;
        self.draw_all();
        loop {
            self.prompt();
            let mut key = None;
            // At the command prompt, as in OUT and space (the engine's
            // Escape-to-quit is taken only there).
            self.io.command_prompt(true);
            for _ in 0..1500 {
                key = self.poll()?;
                self.tick();
                if key.is_some() {
                    break;
                }
            }
            self.io.command_prompt(false);
            self.command(key.unwrap_or(b' '))?;
            if self.redraw {
                self.draw_all();
            }
            self.state();
            if self.ending {
                self.victory()?;
            }
            self.mondain_turn();
            if self.g.hits() <= 0 || self.g.food() <= 0 {
                self.death()?;
            }
        }
    }

    /// `1de6`: four panels, each closed with "Press Space to continue".
    fn intro(&mut self) -> R<()> {
        for (i, panel) in INTRO.iter().enumerate() {
            self.panel();
            for &(col, row, o) in panel.iter() {
                let t = self.s(o);
                self.g.term.print_at(col, row, &t);
            }
            if i == 0 {
                let gems = if self.g.term.cga { [2 | RAW, 1 | RAW, 1 | RAW, 3 | RAW] } else { [12u8, 10, 9, 15] };
                for (k, c) in gems.into_iter().enumerate() {
                    self.g.term.put_char(12 + 5 * k, 7, '\u{4}', c);
                }
            }
            self.g.press_space(self.io)?;
        }
        Ok(())
    }

    /// `47d`: the intro's black panel with a blue outline.
    fn panel(&mut self) {
        let t = &mut self.g.term;
        t.fill(22, 15, 300, 143, BLACK);
        t.line(25, 18, 297, 18, BLUE);
        t.line(297, 18, 297, 140, BLUE);
        t.line(297, 140, 25, 140, BLUE);
        t.line(25, 140, 25, 18, BLUE);
        t.cursor(4, 3);
    }

    fn command(&mut self, key: u8) -> R<()> {
        match key {
            keys::LEFT | keys::RIGHT | keys::UP | keys::DOWN => self.cmd_move(i16::from(key)),
            b'A' => self.cmd_attack(-1)?,
            b'4' => self.cmd_attack(1)?,
            b'6' => self.cmd_attack(2)?,
            b'8' => self.cmd_attack(3)?,
            b'2' => self.cmd_attack(4)?,
            b'B' => self.stub(d::BOARD_Q),
            b'C' => self.cmd_cast()?,
            b'D' => self.stub(d::DROP_Q),
            b'E' => self.stub(d::ENTER_Q),
            b'F' => self.stub(d::FIRE_Q),
            b'G' => self.cmd_get(),
            b'H' => {
                self.stub(d::HYPER_Q);
                self.glow = 1;
            }
            b'I' => self.cmd_inform(),
            b'K' => self.stub(d::KLIMB_Q),
            b'N' => {
                self.puts(d::NOISE);
                let on = self.g.rec.u16_at(off::SOUND) == 0;
                self.g.rec.set_u16(off::SOUND, u16::from(on));
                self.println(if on { d::ON } else { d::OFF });
            }
            b'O' => self.stub(d::OPEN_Q),
            b'Q' => self.println(d::QUIT),
            b'R' => {
                self.g.cmd_ready(self.io)?;
                self.redraw = true;
            }
            b'S' => {
                self.println(d::STEAL_1);
                self.stub(d::STEAL_2);
            }
            b'T' => {
                self.println(d::TRANSACT_1);
                self.stub(d::TRANSACT_2);
            }
            b'U' => self.stub(d::UNLOCK_Q),
            b'V' => self.stub(d::VIEW_Q),
            b'X' => self.stub(d::XIT_Q),
            b'Z' => {
                self.g.cmd_ztats(self.io)?;
                self.redraw = true;
                self.prompt();
            }
            b' ' => self.println(d::PASS),
            _ => {
                self.println(d::HUH);
                self.snd(1);
            }
        }
        Ok(())
    }

    /// `696`: a step; anything but empty floor burns.
    fn cmd_move(&mut self, dir: i16) {
        let (dx, dy) = Self::dxdy(dir);
        let (x, y) = (self.px + dx, self.py + dy);
        let c = if !Self::in_bounds(x, y) || (x == self.mx && y == self.my) { 2 } else { self.cell(x, y) };
        if c == EMPTY {
            let name = self.ptr(d::DIR_NAMES, dir as usize);
            self.g.term.println(&name);
            self.grid[self.px as usize][self.py as usize] = EMPTY;
            self.draw_cell(self.px, self.py);
            self.px = x;
            self.py = y;
            self.grid[x as usize][y as usize] = PLAYER;
            self.draw_cell(x, y);
            self.snd(4);
        } else if c == MACHINE {
            self.println(d::BLOCKED);
            self.snd(0);
        } else {
            self.snd_ev(0, Sfx::Burn);
            self.flash(true);
            let n = (self.g.hits() / 10).max(1);
            self.puts(d::BURNED);
            self.num(n);
            self.println(d::DAMAGE);
            self.snd_ev(2, Sfx::Hurt);
            self.hurt(n);
            self.show_hits();
            self.flash(false);
        }
        if self.gem_near() && self.glow == 0 {
            self.glow = 1;
            self.fighting = true;
        }
    }

    /// `fc7`: the gem is in one of the eight cells around the player.
    fn gem_near(&self) -> bool {
        for dx in -1..=1 {
            for dy in -1..=1 {
                let (x, y) = (self.px + dx, self.py + dy);
                if (dx != 0 || dy != 0) && Self::in_bounds(x, y) && self.cell(x, y) == GEM {
                    return true;
                }
            }
        }
        false
    }

    /// `a0d`: attack with the ready weapon; `dir` -1 asks for a direction.
    fn cmd_attack(&mut self, dir: i16) -> R<()> {
        self.fighting = true;
        self.puts(d::ATTACK_WITH);
        let w = self.g.rec.u16_at(off::READY_WEAPON).min(15) as usize;
        let name = self.ptr(d::WEAPON_NAMES, w);
        self.g.term.print_attr(&name, TEXT);
        let range = self.f.exe.w(d::WEAPON_RANGE + 2 * w) as i16;
        if range == 0 {
            self.println(d::EMPTY_A);
            self.snd(1);
            return Ok(());
        }
        self.puts(d::COLON);
        let dir = if dir == -1 { self.direction()? } else { dir };
        if dir > 4 {
            self.println(d::NOTHING);
            return Ok(());
        }
        let n = self.ptr(d::DIR_NAMES, dir as usize);
        self.g.term.println(&n);
        let chance = self.stat(off::STATS + 2) + 50;
        let top = self.stat(off::STATS) + 3 * w as i16;
        let dmg = self.rnd(2, top);
        self.shoot(dir, 7, range, dmg, chance);
        Ok(())
    }

    /// `831`: a shot along `dir` for up to `range` cells (walls do not stop
    /// it); it may hit Mondain.
    fn shoot(&mut self, dir: i16, sound: u8, range: i16, dmg: i16, chance: i16) {
        self.snd(sound);
        let (dx, dy) = Self::dxdy(dir);
        let (mut x, mut y) = (self.px, self.py);
        let mut k = 1;
        let mut hit = false;
        let mut gem_hit = false;
        loop {
            x += dx;
            y += dy;
            if Self::in_bounds(x, y) && self.io.features().attack_gem && !self.gem_taken && self.cell(x, y) == GEM {
                gem_hit = true;
                hit = true;
            }
            if Self::in_bounds(x, y) && self.cell(x, y) == EMPTY && x == self.mx && y == self.my {
                hit = true;
            }
            k += 1;
            if k > range || hit || x <= 0 || y <= 0 || x >= COLS as i16 || y >= ROWS as i16 {
                break;
            }
        }
        if hit && self.rnd(1, 100) <= chance {
            if gem_hit {
                self.destroy_gem();
                return;
            }
            let dmg = if self.g.god_mode { self.hp } else { dmg };
            self.hp = if self.g.god_mode { 0 } else { self.hp.wrapping_sub(dmg) };
            self.puts(d::HIT_MONDAIN);
            self.num(dmg);
            self.println(d::DAMAGE);
            self.snd(2);
        } else {
            self.cue(Sfx::Miss);
            self.println(d::MISSED);
        }
    }

    /// `618`: does a spell work?
    fn spell_works(&mut self) -> bool {
        let r = self.rnd(1, 0xf9);
        r < self.stat(off::STATS + 10) + 0xa0 || self.g.rec.u16_at(off::CLASS) == 1
    }
    /// `64a`: Mondain is within reach of a spell.
    fn in_reach(&self) -> bool {
        (self.px - self.mx).abs() + (self.py - self.my).abs() < 7
    }

    /// `ed3`: cast the ready spell.
    fn cmd_cast(&mut self) -> R<()> {
        self.puts(d::CAST);
        let sp = self.g.rec.u16_at(off::READY_SPELL).min(10) as usize;
        let name = self.ptr(d::SPELL_NAMES, sp);
        self.g.term.print_attr(&name, TEXT);
        let o = off::SPELLS - 2 + 2 * sp;
        if self.g.w(o) == 0 {
            self.println(d::EMPTY_B);
            self.println(d::USED_UP);
            self.snd(6);
            return Ok(());
        }
        if sp != 0 {
            self.g.add(o, -1);
        }
        let ok = match sp {
            3 => self.sp_missile()?,
            7 => self.sp_blink(),
            8 => self.sp_wall(true)?,
            9 => self.sp_wall(false)?,
            10 => self.sp_kill(),
            _ => {
                self.println(d::EMPTY_B);
                self.println(d::FAILED);
                self.snd(6);
                return Ok(());
            }
        };
        if !ok {
            self.snd(6);
            self.println(d::FAILED);
        }
        Ok(())
    }

    /// `d08`: Magic Missile.
    fn sp_missile(&mut self) -> R<bool> {
        self.puts(d::COLON);
        let dir = self.direction()?;
        if dir > 4 {
            self.println(d::NOTHING);
            return Ok(true);
        }
        let n = self.ptr(d::DIR_NAMES, dir as usize);
        self.g.term.println(&n);
        if !(self.spell_works() && self.in_reach()) {
            return Ok(false);
        }
        let mut dmg = self.rnd(2, self.stat(off::STATS + 10));
        match self.g.rec.u16_at(off::READY_WEAPON) {
            9 => dmg *= 2,
            8 => dmg = dmg * 3 / 2,
            10 | 11 => dmg *= 3,
            _ => {}
        }
        self.shoot(dir, 5, 5, dmg, 500);
        Ok(true)
    }

    /// `d88`: Blink to a random empty cell.
    fn sp_blink(&mut self) -> bool {
        self.g.term.println("");
        if !self.spell_works() {
            return false;
        }
        let (x, y) = loop {
            let x = self.rnd(1, 255) % COLS as i16;
            let y = self.rnd(1, 255) % ROWS as i16;
            if self.cell(x, y) == EMPTY {
                break (x, y);
            }
        };
        self.draw_tile(0, self.px, self.py);
        self.draw_tile(i16::from(PLAYER), x, y);
        self.grid[self.px as usize][self.py as usize] = EMPTY;
        self.grid[x as usize][y as usize] = PLAYER;
        self.px = x;
        self.py = y;
        self.snd(5);
        self.println(d::DONE);
        true
    }

    /// `b07` Create (a wall) / `be4` Destroy (a wall).
    fn sp_wall(&mut self, create: bool) -> R<bool> {
        self.puts(d::COLON);
        let dir = self.direction()?;
        if dir > 4 {
            self.println(d::NOTHING);
            self.snd(6);
            return Ok(true);
        }
        let n = self.ptr(d::DIR_NAMES, dir as usize);
        self.g.term.println(&n);
        if !self.spell_works() {
            return Ok(false);
        }
        let (dx, dy) = Self::dxdy(dir);
        let (x, y) = (self.px + dx, self.py + dy);
        if !Self::in_bounds(x, y) {
            return Ok(false);
        }
        let c = self.cell(x, y);
        // The original refuses a wall anywhere in Mondain's row or column.
        let ok = if create { c == EMPTY && x != self.mx && y != self.my } else { c == WALL };
        if !ok {
            return Ok(false);
        }
        self.grid[x as usize][y as usize] = if create { WALL } else { EMPTY };
        self.draw_cell(x, y);
        self.snd(5);
        self.println(d::DONE);
        Ok(true)
    }

    /// `cac`: Kill only makes him stronger.
    fn sp_kill(&mut self) -> bool {
        self.println(d::EMPTY_B);
        if !(self.spell_works() && self.in_reach()) {
            return false;
        }
        self.println(d::KILL_1);
        self.println(d::KILL_2);
        self.println(d::KILL_3);
        if self.hp < 15000 {
            self.hp *= 2;
        }
        self.snd(5);
        true
    }

    /// `1057`: take the gem, which costs three quarters of the hits.
    fn cmd_get(&mut self) {
        self.puts(d::GET_GEM);
        if self.gem_taken || !self.gem_near() {
            self.println(d::NOTHING_HERE);
            self.snd(1);
            return;
        }
        self.destroy_gem();
    }

    /// Canonical destruction, shared by G and the optional attack path.
    /// Keeps the explosion cost and Mondain/dead-state prerequisite intact.
    fn destroy_gem(&mut self) {
        if self.gem_taken {
            return;
        }
        self.fighting = true;
        self.redraw = true;
        self.gem_taken = true;
        self.grid[GEM_X][GEM_Y] = EMPTY;
        self.glow = 0;
        let dmg = self.g.hits().wrapping_mul(3) / 4;
        self.puts(d::DOTS);
        self.num(dmg);
        self.println(d::DAMAGE);
        for _ in 0..5 {
            self.snd_ev(2, Sfx::Explosion);
        }
        self.println(d::GEM_DESTROYED);
        self.hurt(dmg);
        self.show_hits();
        if self.phase == P_DEAD {
            self.ending = true;
        }
    }

    /// `1124`: Inform and Search.
    fn cmd_inform(&mut self) {
        self.puts(d::INFORM);
        if self.gem_taken {
            self.g.term.println("");
            self.println(d::AURA_1);
            self.println(d::AURA_2);
            self.println(d::AURA_3);
        } else {
            self.println(d::LOOKS_1);
            self.println(d::LOOKS_2);
            self.println(d::LOOKS_3);
        }
    }

    /// `19c8`: fire bursts and Mondain's form by his hit points.
    fn state(&mut self) {
        if self.fighting && self.burst == -1 && self.rnd(1, 200) < 36 {
            let mut n = 0;
            let (mut x, mut y) = (0, 0);
            while n < 100 {
                let a = self.rnd(1, 255) + self.rnd(1, 255);
                x = a % COLS as i16;
                let b = self.rnd(1, 255) + self.rnd(1, 255);
                y = b % ROWS as i16;
                if self.cell(x, y) == EMPTY {
                    break;
                }
                n += 1;
            }
            if n < 100 && x != self.mx && x != self.px && y != self.my && y != self.py {
                self.bx = x;
                self.by = y;
                self.burst = 0;
                self.grid[x as usize][y as usize] = FIRE;
            }
        }
        if self.hp > 500 {
            self.tile = T_WIZARD;
            self.phase = P_WIZARD;
        } else if self.hp > 0 {
            if self.tile != T_DEMON {
                self.tile = T_DEMON;
                self.phase = P_DEMON;
                self.anim = 0;
            }
        } else if self.phase != P_DEAD {
            // Printed without a newline: the next line ends it, so
            // "...or is he?" follows on the same line (the gem case was
            // compared in DOSBox).
            self.puts(d::IS_DEAD);
            if self.gem_taken {
                self.println(d::EMPTY_B);
                self.ending = true;
            } else {
                self.println(d::OR_IS_HE);
            }
            self.tile = T_DEAD;
            self.phase = P_DEAD;
            self.anim = 0;
        }
    }

    /// `186f`: Mondain's move.
    fn mondain_turn(&mut self) {
        if !self.fighting {
            if self.rnd(1, 200) < 17 {
                self.println(d::CHANTING);
            }
            return;
        }
        if self.tile == T_DEAD {
            self.hp += 25;
            return;
        }
        // The step direction is Mondain minus player: whenever he neither
        // strikes nor casts, he backs away from the player.
        let (mut dx, mut dy) = (self.mx - self.px, self.my - self.py);
        let dist = dx.abs() + dy.abs();
        if dist == 1 && self.phase == P_WIZARD {
            self.melee();
        } else if dist < 7 && self.rnd(1, 255) < 128 && self.phase == P_WIZARD {
            self.spell();
        } else {
            if self.phase == P_DEMON {
                if dx == 0 {
                    dx = self.random_sign();
                }
                if dy == 0 {
                    dy = self.random_sign();
                }
            }
            let (sx, sy) = (dx.signum(), dy.signum());
            let moved = if self.rnd(1, 255) & 1 != 0 { self.step(0, sy) || self.step(sx, 0) } else { self.step(sx, 0) || self.step(0, sy) };
            if !moved && self.phase == P_DEMON {
                self.hp += 5;
            }
        }
    }

    /// `1565`.
    fn random_sign(&mut self) -> i16 {
        if self.rnd(1, 200) < 100 {
            -1
        } else {
            1
        }
    }

    /// `1589`: Mondain steps onto an empty cell.
    fn step(&mut self, dx: i16, dy: i16) -> bool {
        let (x, y) = (self.mx + dx, self.my + dy);
        if !Self::in_bounds(x, y) || self.cell(x, y) != EMPTY {
            return false;
        }
        self.draw_tile(0, self.mx, self.my);
        self.mx = x;
        self.my = y;
        self.draw_mondain();
        true
    }

    /// `1664`: a blow from Mondain.
    fn melee(&mut self) {
        self.println(d::ATTACKED);
        self.snd(3);
        let r = self.rnd(1, 255);
        let mut t = 0xb9 - self.stat(off::STATS + 4) / 2;
        t += self.g.rec.u16_at(off::READY_ARMOUR) as i16 * 8;
        t *= 2;
        if r <= t {
            let n = self.g.hits() / 32 + self.rnd(1, 20);
            self.hit(n);
        } else {
            self.cue(Sfx::Dodge);
            self.println(d::MISSED);
        }
    }

    /// `161e`.
    fn hit(&mut self, n: i16) {
        self.puts(d::HIT);
        self.num(n);
        self.println(d::DAMAGE);
        self.hurt(n);
        self.snd_ev(2, Sfx::Hurt);
        self.show_hits();
    }

    /// `16e0`: one of Mondain's three spells.
    fn spell(&mut self) {
        let k = self.rnd(1, 0xee) % 3;
        self.puts(d::CASTS);
        let hit = match k {
            0 => {
                self.println(d::MISSILE);
                self.snd_ev(5, Sfx::EnemySpell);
                let t = (0xc3 - (self.stat(off::STATS + 8) + self.stat(off::STATS + 10) / 2)) * 2;
                let hit = self.rnd(1, 255) < t;
                if hit {
                    let n = self.rnd(1, 100);
                    self.hit(n);
                }
                hit
            }
            1 => {
                self.println(d::BLASTER);
                self.snd_ev(5, Sfx::EnemySpell);
                let hit = self.rnd(1, 200) > 140;
                if hit {
                    self.println(d::STATS_REDUCED);
                    for i in 0..6 {
                        let o = off::STATS + 2 * i;
                        let v = self.g.w(o);
                        if v > 16 {
                            self.g.set(o, v - v / 8);
                        } else if v >= 2 {
                            self.g.set(o, v - 2);
                        }
                    }
                }
                hit
            }
            _ => {
                self.println(d::SHOCK);
                self.flash(true);
                self.snd_ev(5, Sfx::EnemySpell);
                self.flash(false);
                let hit = self.rnd(1, 200) > 140;
                if hit {
                    let mut n = self.g.hits() / 32;
                    if n > 255 {
                        n -= 254;
                    }
                    self.hit(n);
                }
                hit
            }
        };
        if !hit {
            self.cue(Sfx::Miss);
            self.println(d::MISSED);
        }
    }

    /// `138d`/`12ec`: the player has fallen; OUT takes over.
    fn death(&mut self) -> R<()> {
        if self.g.god_mode {
            if self.g.hits() <= 0 {
                self.g.force_set(off::HITS, 150);
            }
            if self.g.food() <= 0 {
                self.g.force_set(off::FOOD, 99);
            }
            self.show_hits();
            self.show_food();
            return Ok(());
        }
        self.g.force_set(off::HITS, 0);
        self.g.force_set(off::FOOD, 0);
        self.show_hits();
        self.show_food();
        self.println(d::DEAD);
        self.g.term.boxed(BLUE);
        for (row, o) in [(6, d::DEFEATED_1), (7, d::DEFEATED_2), (12, d::DOOMED)] {
            let t = self.s(o);
            self.g.term.print_at(5, row, &t);
        }
        self.delay(50)?;
        Err(Stop::Died)
    }

    /// `1ea8` + `1f31`: the victory panel, then the NIF.BIN picture.
    fn victory(&mut self) -> R<()> {
        self.io.music(crate::music::Track::Victory);
        self.draw_tile(T_DEAD, self.mx, self.my);
        self.g.term.plot(50, 7, WHITE);
        self.g.rec.set_u16(off::SOUND, 1);
        self.g.term.boxed(BLUE);
        let t = self.s(d::VICTORIOUS);
        self.g.term.print_at(11, 8, &t);
        for _ in 0..3 {
            self.snd_ev(5, Sfx::Victory);
        }
        self.delay(10)?;
        self.io.flush();
        // Up to 10000 polls for a key. The original polls with no delay,
        // so its wait depends on the CPU; this takes about 5.5 s (SPEC §18).
        for i in 0..10000u32 {
            if self.io.poll_now(&self.g.term)?.is_some() {
                break;
            }
            if i % 100 == 0 {
                self.delay(1)?;
            }
        }
        self.io.music(crate::music::Track::Ending);
        self.g.term.fill(0, 0, 319, 199, BLACK);
        // The original plots the picture a pixel at a time, so it appears
        // from the top down; in DOSBox at cycles=20000 (the speed the poll
        // timing is matched to) that takes about a second (SPEC §18). Eight
        // rows per timer tick gives the same.
        for row in 0..NIF_SIZE / 40 {
            for i in row * 40..row * 40 + 40 {
                let b = self.f.nif[i];
                for bit in 0..8 {
                    let p = i * 8 + bit;
                    let c = if b & (0x80 >> bit) != 0 { WHITE } else { BLACK };
                    self.g.term.plot((p % 320) as i32, (p / 320) as i32, c);
                }
            }
            if row % 8 == 7 {
                self.delay(1)?;
            }
        }
        self.g.term.mark(ArtId::Screen("nif"), None, 0, 0, 0, 320, NIF_SIZE / 40);
        self.delay(75)?;
        self.g.term.print_at(0, 24, "Press any key to return to title screen");
        // Fresh-key wait discards queued/held repeats in SDL; close is Halt,
        // not victory/restart. Never synthesize OS keys or end the process.
        self.io.any_key(&self.g.term)?;
        Err(Stop::Won)
    }
}

#[cfg(test)]
mod interaction_tests {
    use super::*;
    use crate::game::Assets;
    use crate::io::ScriptIo;

    fn setup() -> Option<(Game, MondainFiles)> {
        let dir = crate::gamedir::GameDir::new(std::env::var_os("U1_GAME_DIR")?).ok()?;
        let mode = dir.detect_video_mode()?;
        let exe = dir.read("OUT.EXE").ok()?;
        let assets = Assets {
            t: crate::rules::ExeTables::from_out_exe(&exe).unwrap(),
            locs: crate::locations::LocationTable::from_out_exe(&exe).unwrap(),
            exe: crate::exe::Exe::new(exe).unwrap(),
            map: dir.load_world_map().unwrap(),
            world_tiles: dir.load_world_tiles(mode).unwrap(),
            town_tiles: dir.load_town_tiles(mode).unwrap(),
            tcd: crate::tcd::TownCastleMaps::parse(&dir.read("TCD.BIN").unwrap()).unwrap(),
            video: mode,
        };
        let rec = Game::default_record("Test", 40, 40);
        Some((Game::new(assets, rec, 1), MondainFiles::load(&dir, mode).ok()?))
    }

    #[test]
    fn gem_get_and_successful_attack_share_destruction_god_on_or_off() {
        for god in [false, true] {
            for get in [false, true] {
                let Some((mut g, files)) = setup() else { return };
                g.force_set(off::HITS, 900);
                g.set(off::STATS + 2, 50); // AGI+50 = guaranteed hit
                g.rec.set_u16(off::READY_WEAPON, 1);
                g.god_mode = god;
                let mut io = ScriptIo::from_text("");
                io.qol.attack_gem = !get; // G works even when option is OFF
                let mut l = Lair::new(&mut g, &mut io, files);
                l.px = GEM_X as i16 - 1;
                l.py = GEM_Y as i16;
                if get {
                    l.command(b'G').unwrap();
                } else {
                    l.cmd_attack(2).unwrap();
                }
                assert!(l.gem_taken);
                assert_eq!(l.grid[GEM_X][GEM_Y], EMPTY);
                assert!(!l.ending, "gem alone must not bypass Mondain");
                assert_eq!(l.g.hits(), if god { 900 } else { 225 });
                assert!(l.g.term.messages().iter().any(|s| s.contains("Gem is DESTROYED")));
                l.cmd_get();
                assert_eq!(l.g.hits(), if god { 900 } else { 225 }, "no duplicate destruction cost");
            }
        }
    }

    #[test]
    fn gem_attacks_respect_option_hit_roll_direction_range_and_dead_mondain() {
        for god in [false, true] {
            for (enabled, dir, range, chance, expected) in [(false, 2, 1, 100, false), (true, 2, 1, 0, false), (true, 1, 1, 100, false), (true, 2, 1, 100, true), (true, 2, 3, 100, true)] {
                let Some((mut g, files)) = setup() else { return };
                g.god_mode = god;
                let mut io = ScriptIo::from_text("");
                io.qol.attack_gem = enabled;
                let mut l = Lair::new(&mut g, &mut io, files);
                l.px = GEM_X as i16 - 1;
                l.py = GEM_Y as i16;
                l.phase = P_DEAD;
                l.shoot(dir, 7, range, 1, chance);
                assert_eq!(l.gem_taken, expected);
                assert_eq!(l.ending, expected, "dead Mondain + destroyed gem triggers ending");
            }
            let Some((mut g, files)) = setup() else { return };
            let mut io = ScriptIo::from_text("");
            io.qol.attack_gem = true;
            let mut l = Lair::new(&mut g, &mut io, files);
            l.px = GEM_X as i16 - 3;
            l.py = GEM_Y as i16;
            l.shoot(2, 7, 1, 9999, 100);
            assert!(!l.gem_taken, "out of range cannot destroy gem");
        }
    }

    #[test]
    fn victory_requires_fresh_acknowledgement_and_halt_is_not_win() {
        for acknowledge in [false, true] {
            let Some((mut g, files)) = setup() else { return };
            let mut io = ScriptIo::from_text(if acknowledge { "x{w100}y" } else { "x{p}" });
            io.skip_pauses.set(1000); // keep {p} for the final any_key wait
            let mut l = Lair::new(&mut g, &mut io, files);
            let result = l.victory();
            assert_eq!(matches!(result, Err(Stop::Won)), acknowledge);
            assert_eq!(matches!(result, Err(Stop::Halt)), !acknowledge);
            assert!(l.g.term.row_text(24).contains("Press any key to return to title screen"));
            assert!(!l.g.term.row_text(24).contains("ALT"));
            drop(l);
            assert_eq!(io.music, [crate::music::Track::Victory, crate::music::Track::Ending], "one context change, not every animation/poll");
            if acknowledge {
                let dir = crate::gamedir::GameDir::new(std::env::var_os("U1_GAME_DIR").unwrap()).unwrap();
                let mut mode = g.a.video;
                io.keys = ScriptIo::from_text("{p}").keys;
                assert!(crate::front::intro(&dir, &mut mode, &mut io).unwrap().is_some());
                assert_eq!(io.music, [crate::music::Track::Victory, crate::music::Track::Ending, crate::music::Track::Title]);
            }
        }
    }
}
