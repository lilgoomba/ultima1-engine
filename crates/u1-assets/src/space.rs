//! The space program (SPACE.EXE): launch, the galaxy, top-down and
//! first-person flight, fighters, docking and landing (SPEC §12).
//!
//! Everything runs on the player record; the strings and bitmaps come
//! from the user's SPACE.EXE, EGASPACE.BIN and EGAFIGHT.BIN at runtime.

use crate::exe::Exe;
use crate::game::Game;
use crate::gamedir::GameDir;
use crate::io::{Halt, Io, IoResult};
use crate::rng::U1Rng;
use crate::save::off;
use crate::sound::Sfx;
use crate::term::{space_part as sp, BLACK, RAW, TEXT, W};

/// Size of the supported SPACE.EXE.
pub const SPACE_EXE_SIZE: usize = 39824;
/// The data segment's paragraph in the load image.
const DS_PARA: usize = 0x72d;

const C_FLAME: u8 = 12;
const C_WHITE: u8 = 15;
const C_BLUE: u8 = 1;
const C_RED: u8 = 12;
const C_FRAME: u8 = 7;

/// Data-segment offsets in SPACE.EXE.
mod d {
    pub const TD_NAMES: usize = 0x1c4;
    pub const FP_NAMES: usize = 0x1e4;
    pub const HIT_W: usize = 0x1f6;
    pub const HIT_H: usize = 0x204;
    pub const HIT_X0: usize = 0x212;
    pub const HIT_Y0: usize = 0x220;
    pub const STAR: usize = 0x22e;
    pub const STATION: usize = 0x4da;
    pub const EARTH: usize = 0x7fa;
    pub const PORT_DX: usize = 0xaee;
    pub const PORT_DY: usize = 0xb06;
    pub const TURN: usize = 0xbd2;
    pub const RETRACE: usize = 0xc9e;
    pub const PORT_NAMES: usize = 0xe18;
    pub const EMPTY: usize = 0x19a;
    pub const HUH: usize = 0xb31;
    pub const VIEW: usize = 0xb36;
    pub const QMARK: usize = 0xb3b;
    pub const NO_VIEW_1: usize = 0xb3d;
    pub const NO_VIEW_2: usize = 0xb53;
    pub const NOT_ENOUGH_FUEL: usize = 0xb69;
    pub const ELIM_1: usize = 0xb7a;
    pub const ELIM_2: usize = 0xb92;
    pub const INFORM: usize = 0xba5;
    pub const SCAN: usize = 0xbb7;
    pub const FIRE_Q: usize = 0xbc4;
    pub const SPEED: usize = 0xbca;
    pub const NO_FUEL: usize = 0xbe2;
    pub const DRIFT: usize = 0xbec;
    pub const CRUNCH: usize = 0xc07;
    pub const DRAINED: usize = 0xc0f;
    pub const ATTACK_Q: usize = 0xc26;
    pub const BOARD_Q: usize = 0xc2e;
    pub const CAST_Q: usize = 0xc35;
    pub const DROP_Q: usize = 0xc3b;
    pub const ENTER_Q: usize = 0xc41;
    pub const GET_Q: usize = 0xc48;
    pub const KLIMB_Q: usize = 0xc4d;
    pub const OPEN_Q: usize = 0xc54;
    pub const QUIT_Q: usize = 0xc5a;
    pub const READY_Q: usize = 0xc60;
    pub const STEAL_Q: usize = 0xc67;
    pub const TRANSACT_Q: usize = 0xc6e;
    pub const UNLOCK_Q: usize = 0xc78;
    pub const XIT_Q: usize = 0xc80;
    pub const PASS: usize = 0xc86;
    pub const CENTER: usize = 0xc8b;
    pub const HYPER: usize = 0xcb0;
    pub const HYPER_1: usize = 0xcba;
    pub const HYPER_2: usize = 0xcd5;
    pub const DOTS3: usize = 0xce6;
    pub const LIGHTSPEED: usize = 0xcea;
    pub const COMPLETED: usize = 0xcf6;
    pub const ACE_1: usize = 0xd07;
    pub const ACE_2: usize = 0xd1a;
    pub const ACE_3: usize = 0xd26;
    pub const FIRE: usize = 0xd31;
    pub const FIRE_NO_FUEL: usize = 0xd37;
    pub const HIT: usize = 0xd52;
    pub const STILL_ACE: usize = 0xd59;
    pub const MELTS: usize = 0xd75;
    pub const LANDED: usize = 0xd93;
    pub const HEAT_1: usize = 0xdac;
    pub const HEAT_2: usize = 0xdc9;
    pub const ALIEN_FIRES: usize = 0xde4;
    pub const BEEN_HIT: usize = 0xdf1;
    pub const DOCKED: usize = 0xe20;
    pub const VACUUM_1: usize = 0xe3a;
    pub const VACUUM_2: usize = 0xe58;
    pub const FEE_1: usize = 0xe73;
    pub const FEE_2: usize = 0xe8d;
    pub const FEE_3: usize = 0xea5;
    pub const CHOOSE: usize = 0xebb;
    pub const NO_PORT: usize = 0xecd;
    pub const IN_THE: usize = 0xee9;
    pub const SHIP: usize = 0xefa;
    pub const NOISE: usize = 0xf01;
    pub const ON: usize = 0xf08;
    pub const OFF: usize = 0xf0b;
    pub const DOTS2: usize = 0xf5a;
    pub const LIFTED_OFF: usize = 0xf5d;
    pub const SHLD: usize = 0xf73;
    pub const FUEL: usize = 0xf79;
}

/// SPACE.EXE and its two sprite files.
#[derive(Clone)]
pub struct SpaceFiles {
    pub exe: Exe,
    /// The original's video index `112c`: 0 CGA, 1 EGA, 2 Tandy; it picks
    /// the sprite files and their layout (tables `12fe`.. in SPACE.EXE).
    pub mode: usize,
    pub ships: Vec<u8>,
    pub fighter: Vec<u8>,
}

impl SpaceFiles {
    pub fn load(g: &GameDir, video: crate::gamedir::VideoMode) -> crate::Result<Self> {
        use crate::gamedir::VideoMode;
        let (mode, pre) = match video {
            VideoMode::Cga => (0, "CGA"),
            VideoMode::Ega => (1, "EGA"),
            VideoMode::Tandy => (2, "T1K"),
        };
        let data = g.read("SPACE.EXE")?;
        if data.len() != SPACE_EXE_SIZE {
            return Err(crate::AssetError::BadSize { what: "SPACE.EXE".into(), expected: SPACE_EXE_SIZE.to_string(), got: data.len() });
        }
        let base = Exe::mz_data_base(&data, DS_PARA);
        let ships = g.read(&format!("{pre}SPACE.BIN"))?;
        let fighter = g.read(&format!("{pre}FIGHT.BIN"))?;
        if ships.len() < 3 * 4 * 0x260 {
            return Err(crate::AssetError::BadSize { what: "EGASPACE.BIN".into(), expected: "7296".into(), got: ships.len() });
        }
        if fighter.len() < 7 * 0x1c8 {
            return Err(crate::AssetError::BadSize { what: "EGAFIGHT.BIN".into(), expected: "3192".into(), got: fighter.len() });
        }
        Ok(SpaceFiles { exe: Exe::with_base(data, base), mode, ships, fighter })
    }
}

/// How the space program ended: back to OUT either way (a death leaves
/// hits and food at 0 in the record).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpaceEnd {
    Landed,
    Died,
}

enum Stop {
    Halt,
    Exit,
}

impl From<Halt> for Stop {
    fn from(_: Halt) -> Self {
        Stop::Halt
    }
}

type R<T> = Result<T, Stop>;

#[derive(Clone, Copy, Default)]
struct Slot {
    x: i16,
    y: i16,
    ty: i16,
    fuel: i16,
    shield: i16,
    hd: i16,
}

#[derive(Clone, Copy, Default)]
struct Sector {
    enemies: i16,
    sx: i16,
    sy: i16,
    stx: i16,
    sty: i16,
    slots: [Slot; 4],
}

/// Flame pixels per ship type and heading: (dx, step x, dy, step y,
/// white?, count) — plotted at (x+dx+i*sx, y+dy+i*sy) for i in 0..count.
type Flame = (i16, i16, i16, i16, bool, i16);
const R_: bool = false;
const W_: bool = true;
#[rustfmt::skip]
const FLAMES: [[&[Flame]; 4]; 3] = [
    [
        &[(19,0,4,1,R_,3),(19,0,11,1,R_,3),(20,0,5,0,W_,1),(20,0,6,0,W_,1),(20,0,11,0,W_,1),(20,0,12,0,W_,1),(21,0,6,0,R_,1),(21,0,11,0,R_,1)],
        &[(-1,0,4,1,R_,3),(-1,0,11,1,R_,3),(-2,0,5,0,W_,1),(-2,0,6,0,W_,1),(-2,0,11,0,W_,1),(-2,0,12,0,W_,1),(-3,0,6,0,R_,1),(-3,0,11,0,R_,1)],
        &[(5,1,19,0,R_,3),(12,1,19,0,R_,3),(6,0,20,0,W_,1),(7,0,20,0,W_,1),(12,0,20,0,W_,1),(13,0,20,0,W_,1),(7,0,21,0,R_,1),(12,0,21,0,R_,1)],
        &[(5,1,-1,0,R_,3),(12,1,-1,0,R_,3),(6,0,-2,0,W_,1),(7,0,-2,0,W_,1),(12,0,-2,0,W_,1),(13,0,-2,0,W_,1),(7,0,-3,0,R_,1),(12,0,-3,0,R_,1)],
    ],
    [
        &[(17,0,7,1,R_,2),(17,0,9,1,R_,2),(18,0,8,1,R_,2),(19,0,6,1,R_,6)],
        &[(-1,0,7,1,R_,2),(-1,0,9,1,R_,2),(-2,0,8,1,R_,2),(-3,0,6,1,R_,6)],
        &[(7,1,18,0,R_,2),(9,1,18,0,R_,2),(8,1,19,0,R_,2),(6,1,20,0,R_,6)],
        &[(7,1,0,0,R_,2),(9,1,0,0,R_,2),(8,1,-1,0,R_,2),(6,1,-2,0,R_,6)],
    ],
    [
        &[(17,1,1,0,R_,2),(17,1,2,0,R_,2),(17,1,15,0,R_,2),(17,1,16,0,R_,2),(17,0,7,1,R_,4),(18,0,6,0,R_,1),(18,0,11,0,R_,1)],
        &[(-1,-1,1,0,R_,2),(-1,-1,2,0,R_,2),(-1,-1,15,0,R_,2),(-1,-1,16,0,R_,2),(-1,0,7,1,R_,4),(-2,0,6,0,R_,1),(-2,0,11,0,R_,1)],
        &[(0,1,18,0,R_,2),(0,1,19,0,R_,2),(14,1,18,0,R_,2),(14,1,19,0,R_,2),(6,1,18,0,R_,4),(5,0,19,0,R_,1),(10,0,19,0,R_,1)],
        &[(0,1,0,0,R_,2),(0,1,-1,0,R_,2),(14,1,0,0,R_,2),(14,1,-1,0,R_,2),(6,1,0,0,R_,4),(5,0,-1,0,R_,1),(10,0,-1,0,R_,1)],
    ],
];

fn sign(v: i16) -> i16 {
    v.signum()
}

struct Space<'a> {
    g: &'a mut Game,
    io: &'a mut dyn Io,
    f: SpaceFiles,
    /// A display chosen in the options menu has no space pictures: it
    /// waits for the return to OUT.
    keep_video: bool,
    rng: U1Rng,
    gal: Box<[[Sector; 7]; 7]>,
    gx: usize,
    gy: usize,
    cur: usize,
    fp: bool,
    speed: i16,
    entry_speed: i16,
    vx: i16,
    vy: i16,
    last_hd: i16,
    flame: i16,
    flame_on: bool,
    redraw: bool,
    redrew: bool,
    cx: i16,
    cy: i16,
    dvx: i16,
    dvy: i16,
    star_mode: i16,
    rel: [(i16, i16); 40],
    old: [(i16, i16); 40],
    ex: i16,
    ey: i16,
    eox: i16,
    eoy: i16,
    esize: i16,
    hyper: bool,
    grow: i16,
}

/// Run the space program from a shuttle launch until it hands back to
/// OUT. Assumes the record is in sync with the live game.
pub fn run_space(g: &mut Game, io: &mut dyn Io) -> IoResult<SpaceEnd> {
    let Some(files) = g.space.take() else { return Ok(SpaceEnd::Landed) };
    io.in_space(true);
    let mut s = Space::new(g, io, files);
    let r = s.main();
    let died = s.g.hits() <= 0;
    if !matches!(r, Err(Stop::Halt)) {
        s.g.term.sscene = None;
    }
    let Space { f: files, .. } = s;
    io.in_space(false);
    g.space = Some(files);
    match r {
        Err(Stop::Halt) => Err(Halt),
        _ => Ok(if died { SpaceEnd::Died } else { SpaceEnd::Landed }),
    }
}

impl<'a> Space<'a> {
    fn new(g: &'a mut Game, io: &'a mut dyn Io, files: SpaceFiles) -> Self {
        let seed = g.rec.u16_at(off::SEED);
        Space {
            g,
            io,
            f: files,
            keep_video: false,
            rng: U1Rng::new(seed),
            gal: Box::new([[Sector::default(); 7]; 7]),
            gx: 3,
            gy: 3,
            cur: 0,
            fp: false,
            speed: 5,
            entry_speed: 0,
            vx: 0,
            vy: 0,
            last_hd: 0,
            flame: 0,
            flame_on: false,
            redraw: false,
            redrew: false,
            cx: 159,
            cy: 79,
            dvx: 0,
            dvy: 0,
            star_mode: 2,
            rel: [(0, 0); 40],
            old: [(0, 0); 40],
            ex: 0,
            ey: 0,
            eox: 0,
            eoy: 0,
            esize: 0,
            hyper: false,
            grow: 0,
        }
    }

    // ---------------------------------------------------------- helpers

    fn s(&self, o: usize) -> String {
        self.f.exe.s(o)
    }
    fn ptr(&self, table: usize, i: usize) -> String {
        let o = self.f.exe.w(table + 2 * i) as usize;
        self.f.exe.s(o)
    }
    fn wd(&self, o: usize) -> i16 {
        self.f.exe.w(o) as i16
    }
    fn puts(&mut self, o: usize) {
        let t = self.s(o);
        self.g.term.print_attr(&t, TEXT);
    }
    fn println(&mut self, o: usize) {
        let t = self.s(o);
        self.g.term.println(&t);
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
    fn srand(&mut self, s: u16) {
        self.rng.seed(s);
    }
    fn timer(&mut self) -> u16 {
        self.g.time_seed_now()
    }
    fn delay(&mut self, ticks: u32) -> R<()> {
        self.io.pause(&self.g.term, ticks)?;
        Ok(())
    }
    fn retrace(&mut self, n: u32) -> R<()> {
        self.io.retrace(&self.g.term, n)?;
        Ok(())
    }
    fn prompt(&mut self) {
        self.g.term.ornament_left(0, 24);
        self.g.term.cursor(1, 24);
    }
    fn sec(&self) -> &Sector {
        &self.gal[self.gx][self.gy]
    }
    fn sec_mut(&mut self) -> &mut Sector {
        &mut self.gal[self.gx][self.gy]
    }
    fn slot(&self) -> Slot {
        self.sec().slots[self.cur]
    }
    fn slot_mut(&mut self) -> &mut Slot {
        let c = self.cur;
        &mut self.sec_mut().slots[c]
    }

    fn show_fuel(&mut self) {
        let v = self.slot().fuel;
        self.g.term.stat(22, i32::from(v), true);
    }
    fn show_shield(&mut self) {
        let v = self.slot().shield;
        self.g.term.stat(21, i32::from(v), true);
    }
    fn show_exp(&mut self) {
        let v = self.g.exp();
        self.g.term.stat(23, i32::from(v), false);
    }
    fn show_coin(&mut self) {
        let v = self.g.coin();
        self.g.term.stat(24, i32::from(v), false);
    }
    fn use_shield(&mut self, n: i16) {
        let s = self.slot_mut();
        s.shield = (s.shield - n).max(0);
        self.show_shield();
    }
    fn use_fuel(&mut self, n: i16) {
        let s = self.slot_mut();
        s.fuel = (s.fuel - n).max(0);
        self.show_fuel();
    }
    /// `910`: a crash costs a quarter of the shield plus 5.
    fn crash_damage(&mut self) {
        let n = self.slot().shield / 4 + 5;
        self.use_shield(n);
    }

    // --------------------------------------------------------- program

    fn main(&mut self) -> R<()> {
        self.countdown()?;
        self.g.term.print_at_attr(31, 21, &self.s(d::SHLD), TEXT);
        self.g.term.print_at_attr(31, 22, &self.s(d::FUEL), TEXT);
        self.star_mode = 2;
        let seed = self.g.rec.u16_at(off::SEED);
        self.srand(seed.wrapping_mul(2));
        self.make_galaxy();
        let t = self.timer();
        self.srand(t.wrapping_add(seed));
        self.new_stars();
        self.draw_all();
        self.show_fuel();
        self.show_shield();
        loop {
            if self.slot().shield == 0 {
                self.g.rec.set_u16(off::SHUTTLE_0, 0);
                self.g.rec.set_u16(off::SHUTTLE_1, 0);
                self.show_fuel();
                self.show_shield();
                self.death()?;
            }
            // `3bf9`: the idle counter starts at zero; 1500 polls (2 ticks
            // each in the top-down view) before an automatic Pass, which
            // DOSBox shows every ~165 s.
            let mut n: u16 = 0;
            self.prompt();
            let key = loop {
                self.tick()?;
                let k = self.io.poll_now(&self.g.term)?;
                self.switch_video();
                n = n.wrapping_add(1);
                if !self.fp {
                    self.delay(2)?;
                }
                if let Some(k) = k {
                    break k.to_ascii_uppercase();
                }
                if n >= 1500 {
                    break b' ';
                }
            };
            self.command(key)?;
            if self.redraw {
                self.draw_all();
            }
            if self.redrew {
                self.redrew = false;
                if self.flame != 0 && !self.fp {
                    self.flame_on = !self.flame_on;
                }
            }
        }
    }

    /// A display chosen in the options menu, taken at the command poll:
    /// its pictures, the screen's colours converted, and the view redrawn
    /// as it stands (nothing in the flight changes).
    fn switch_video(&mut self) {
        if self.keep_video {
            return;
        }
        let Some(mut v) = self.io.video_switch() else { return };
        let Some(f) = v.space.take() else {
            self.keep_video = true;
            return;
        };
        self.f = f;
        let mode = v.mode;
        self.g.take_video(v);
        self.io.video_shown(mode);
        if self.fp {
            self.g.term.fill(16, 16, 303, 143, BLACK);
            self.frame();
            self.g.term.space_begin(self.fp);
            self.crosshair(C_WHITE);
        } else {
            let r = self.redraw;
            self.draw_all();
            self.redraw = r;
        }
        self.show_fuel();
        self.show_shield();
    }

    fn command(&mut self, key: u8) -> R<()> {
        use crate::io::keys;
        if self.fp && (b'1'..=b'8').contains(&key) {
            return self.cmd_speed((key - b'0') as i16);
        }
        let q = |o| Some(o);
        let ask = match key {
            b'A' => q(d::ATTACK_Q),
            b'B' => q(d::BOARD_Q),
            b'C' => q(d::CAST_Q),
            b'D' => q(d::DROP_Q),
            b'E' => q(d::ENTER_Q),
            b'G' => q(d::GET_Q),
            b'K' => q(d::KLIMB_Q),
            b'O' => q(d::OPEN_Q),
            b'Q' => q(d::QUIT_Q),
            b'R' => q(d::READY_Q),
            b'S' => q(d::STEAL_Q),
            b'T' => q(d::TRANSACT_Q),
            b'U' => q(d::UNLOCK_Q),
            b'X' => q(d::XIT_Q),
            _ => None,
        };
        if let Some(o) = ask {
            self.println(o);
            self.snd(1);
            return Ok(());
        }
        match key {
            keys::LEFT | keys::RIGHT | keys::UP | keys::DOWN => self.cmd_arrow(key as i16)?,
            b' ' => self.cmd_center(),
            b'F' => self.cmd_fire()?,
            b'H' => self.cmd_hyper()?,
            b'I' => {
                self.cmd_inform()?;
                self.redraw = true;
                self.redrew = true;
            }
            b'N' => self.cmd_noise(),
            b'V' => self.cmd_view(),
            b'Z' => {
                self.g.term.sscene = None;
                self.g.cmd_ztats(self.io)?;
                self.redraw = true;
                self.redrew = true;
                self.prompt();
            }
            _ => {
                self.println(d::HUH);
                self.snd(1);
            }
        }
        Ok(())
    }

    /// `3a4e`: the launch countdown.
    fn countdown(&mut self) -> R<()> {
        self.g.term.cursor(1, 24);
        let mut key = None;
        let mut n = 10;
        while n > 0 {
            self.g.term.print_attr(&n.to_string(), TEXT);
            if n != 1 {
                self.puts(d::DOTS2);
            }
            if key.is_none() {
                key = self.io.poll_now(&self.g.term)?;
                if key.is_none() {
                    self.delay(18)?;
                }
            }
            if key.is_some() {
                break;
            }
            n -= 1;
        }
        self.g.term.println("");
        self.println(d::LIFTED_OFF);
        Ok(())
    }

    /// `f25`: hits and food to zero, then leave.
    fn death(&mut self) -> R<()> {
        self.g.set(off::HITS, 0);
        self.g.set(off::FOOD, 0);
        self.g.term.stat(21, 0, true);
        self.g.term.stat(22, 0, true);
        self.exit()
    }

    /// `ebd`: back to OUT (a dead pilot's ship is emptied first).
    fn exit(&mut self) -> R<()> {
        if self.g.hits() == 0 {
            let sh = self.slot().shield;
            self.use_shield(sh);
            let fu = self.slot().fuel;
            self.use_fuel(fu);
        }
        Err(Stop::Exit)
    }

    // ---------------------------------------------------------- galaxy

    fn max_fuel(ty: i16) -> i16 {
        match ty {
            0 => 1000,
            1 => 5000,
            _ => 2500,
        }
    }
    fn max_shield(ty: i16) -> i16 {
        if ty == 2 {
            5000
        } else {
            1000
        }
    }
    fn port_heading(p: i16) -> i16 {
        [1, 0, 3, 2][(p & 3) as usize]
    }
    fn port_pos(&self, sec: &Sector, p: i16, ty: i16) -> (i16, i16) {
        let i = (p * 3 + ty) as usize;
        (sec.stx + self.wd(d::PORT_DX + 2 * i), sec.sty + self.wd(d::PORT_DY + 2 * i))
    }

    /// `9af`: one sector.
    fn make_sector(&mut self, gx: usize, gy: usize) {
        let mut s = Sector::default();
        for sl in &mut s.slots {
            sl.ty = -1;
        }
        s.enemies = self.rnd(0, 240) / 40;
        if s.enemies > 3 {
            s.enemies = 0;
        }
        if self.rnd(1, 200) < 77 {
            s.sx = self.rnd(50, 200);
            s.sy = self.rnd(50, 120);
        }
        if self.rnd(1, 200) < 77 {
            s.stx = self.rnd(50, 200);
            s.sty = self.rnd(50, 100);
            if (s.stx - s.sx).abs() < 40 || (s.sty - s.sy).abs() < 40 {
                s.stx = 0;
                s.sty = 0;
            }
        }
        if s.stx != 0 {
            let t = self.rnd(0, 98) % 3;
            if t != 0 {
                let p = self.rnd(0, 199) % 4;
                let ty = self.rnd(0, 98) % 3;
                self.dock_ship(&mut s, 0, p, ty);
                if t == 2 {
                    let mut ty2 = self.rnd(0, 199) % 3;
                    while ty2 == ty {
                        ty2 = self.rnd(0, 199) % 3;
                    }
                    let mut p2 = self.rnd(0, 98) % 3;
                    while p2 == p {
                        p2 = self.rnd(0, 98) % 3;
                    }
                    self.dock_ship(&mut s, 1, p2, ty2);
                }
            }
        }
        self.gal[gx][gy] = s;
    }

    fn dock_ship(&self, s: &mut Sector, i: usize, p: i16, ty: i16) {
        let (x, y) = self.port_pos(s, p, ty);
        s.slots[i] = Slot { x, y, ty, fuel: Self::max_fuel(ty), shield: Self::max_shield(ty), hd: Self::port_heading(p) };
    }

    /// `d88`: the whole galaxy, and the home sector around Earth.
    fn make_galaxy(&mut self) {
        let seed = self.g.rec.u16_at(off::SEED) as i16;
        self.srand(seed.wrapping_mul(3).wrapping_div(2) as u16);
        for gx in 0..7 {
            for gy in 0..7 {
                self.make_sector(gx, gy);
            }
        }
        self.cur = 2;
        let mut h = Sector { enemies: 0, sx: 140, sy: 70, stx: 200, sty: 100, ..Default::default() };
        for sl in &mut h.slots {
            sl.ty = -1;
        }
        self.dock_ship(&mut h, 0, 3, 2);
        self.dock_ship(&mut h, 1, 0, 1);
        let fuel = self.g.rec.u16_at(off::SHUTTLE_0) as i16;
        let shield = self.g.rec.u16_at(off::SHUTTLE_1) as i16;
        h.slots[2] = Slot { x: 260, y: 90, ty: 0, fuel, shield, hd: 0 };
        self.gal[3][3] = h;
        self.gx = 3;
        self.gy = 3;
    }

    /// `3741`: the port a slot's ship sits docked at, if any.
    fn docked_port(&self, slot: usize) -> Option<i16> {
        let sec = self.sec();
        let sl = sec.slots[slot];
        if sl.ty < 0 || sec.stx == 0 {
            return None;
        }
        (0..4).find(|&p| self.port_pos(sec, p, sl.ty) == (sl.x, sl.y) && Self::port_heading(p) == sl.hd)
    }

    // --------------------------------------------------------- drawing

    /// `23f3`: the dotted frame.
    fn frame(&mut self) {
        // 7, or the white colour variable `5ad4` on CGA.
        let fc = if self.f.mode == 0 { C_WHITE } else { C_FRAME };
        let t = &mut self.g.term;
        t.fill(8, 8, 311, 15, fc);
        t.fill(8, 144, 311, 151, fc);
        t.fill(8, 16, 15, 144, fc);
        t.fill(304, 16, 311, 144, fc);
        for i in 1..=19 {
            let x = 15 * i + 10;
            for y in [10, 13, 146, 149] {
                t.plot(x, y, BLACK);
            }
        }
        for i in 1..=9 {
            let y = 15 * i + 3;
            for x in [10, 13, 306, 309] {
                t.plot(x, y, BLACK);
            }
        }
    }

    /// Plot a picture from SPACE.EXE and note it as space part `part`.
    #[allow(clippy::too_many_arguments)]
    fn bitmap(&mut self, part: u16, base: usize, w: usize, h: usize, x: i16, y: i16, col: impl Fn(i16) -> Option<u8>) {
        let mut lit = Vec::new();
        for r in 0..h {
            for c in 0..w {
                let v = self.wd(base + 2 * (r * w + c));
                if let Some(cl) = col(v) {
                    let (px, py) = (i32::from(x) + c as i32, i32::from(y) + r as i32);
                    self.g.term.plot(px, py, cl);
                    if (0..320).contains(&px) && (0..200).contains(&py) {
                        lit.push((py as usize * W + px as usize) as u32);
                    }
                }
            }
        }
        self.g.term.space_obj(part, 0, lit, false, false);
    }

    /// `259e`: clear and redraw the view.
    fn draw_all(&mut self) {
        self.redraw = false;
        self.g.term.fill(16, 16, 303, 143, BLACK);
        self.frame();
        self.g.term.space_begin(self.fp);
        if self.fp {
            self.new_stars();
            self.crosshair(C_WHITE);
            return;
        }
        if (self.gx, self.gy) == (3, 3) {
            // `528`: 1, 10, 7, or the CGA colour variables 1, 2, 3.
            let [c1, c2, c3] = if self.f.mode == 0 { [1 | RAW, 2 | RAW, 3 | RAW] } else { [1, 10, 7] };
            self.bitmap(sp::EARTH, d::EARTH, 21, 18, 60, 75, |v| match v {
                1 | 2 => Some(c1),
                3 => Some(c2),
                4 => Some(c3),
                _ => None,
            });
        }
        let s = *self.sec();
        if s.sx != 0 {
            // `426`: 14 and 12, or the CGA colour variables 3 and 2.
            let [c1, c2] = if self.f.mode == 0 { [3 | RAW, 2 | RAW] } else { [14, 12] };
            self.bitmap(sp::SUN, d::STAR, 19, 18, s.sx, s.sy, |v| match v {
                1 => Some(c1),
                2 => Some(c2),
                _ => None,
            });
        }
        if s.stx != 0 {
            self.bitmap(sp::STATION, d::STATION, 20, 20, s.stx, s.sty, |v| (v != 0).then_some(C_WHITE));
        }
        for sl in s.slots {
            if sl.ty > -1 {
                self.ship(sl.ty, sl.hd, sl.x, sl.y);
            }
        }
    }

    /// Sprite layout for the current video mode: (bytes per row, shift
    /// mask, bytes per shift, bits per pixel) from SPACE.EXE tables.
    fn layout(&self, row_tab: usize, mask_tab: usize, step_tab: usize) -> (usize, i16, usize, usize) {
        let m = self.f.mode;
        let e = &self.f.exe;
        let bpp = [2, 1, 4][m.min(2)];
        (e.w(row_tab + 2 * m) as usize, e.w(mask_tab + 2 * m) as i16, e.w(step_tab + 2 * m) as usize, bpp)
    }

    /// Pixels of one sprite row: (x offset, device value) of each set pixel.
    fn sprite_row(data: &[u8], at: usize, n: usize, bpp: usize) -> Vec<(i32, u8)> {
        let per = 8 / bpp;
        let mut out = Vec::new();
        for b in 0..n {
            let byte = data.get(at + b).copied().unwrap_or(0);
            for k in 0..per {
                let v = (byte >> (8 - bpp * (k + 1))) & ((1u8 << bpp) - 1);
                if v != 0 {
                    out.push(((b * per + k) as i32, v));
                }
            }
        }
        out
    }

    /// `603e`: XOR a ship sprite; true when it touched something.
    fn ship(&mut self, ty: i16, hd: i16, x: i16, y: i16) -> bool {
        let (row, mask, step, bpp) = self.layout(0x12fe, 0x130a, 0x1310);
        let base = ((ty * 4 + hd) as usize) * 0x260 + ((x & mask) as usize) * step;
        let bx = i32::from(x & !mask);
        let ega = self.f.mode == 1;
        let mut hit = false;
        let mut lit = Vec::new();
        for r in 0..19 {
            let py = i32::from(y) + r;
            if !(18..=141).contains(&py) {
                continue;
            }
            for (dx, v) in Self::sprite_row(&self.f.ships, base + r as usize * row, row, bpp) {
                let px = bx + dx;
                if (0..320).contains(&px) {
                    let i = py as usize * W + px as usize;
                    let old = self.g.term.px[i];
                    // EGA tests plane 0 and XORs all four planes; CGA and
                    // Tandy test and XOR the sprite's own bits.
                    let (m, x) = if ega { (1, 15) } else { (v, v) };
                    if old & m != 0 {
                        hit = true;
                    }
                    self.g.term.px[i] = old ^ x;
                    lit.push(i as u32);
                }
            }
        }
        // Headings 0-3 point west, east, north, south.
        let rot = [3, 1, 0, 2][(hd & 3) as usize];
        self.g.term.space_obj(sp::SHIP0 + ty.clamp(0, 2) as u16, rot, lit, true, false);
        hit
    }

    /// `614e`: the fighter seen ahead, drawn or erased.
    fn fighter(&mut self, x: i16, y: i16, size: i16, on: bool) {
        let (row, mask, step, bpp) = self.layout(0x1318, 0x1324, 0x132a);
        let base = (size.clamp(0, 6) as usize) * 0x1c8 + ((x & mask) as usize) * step;
        let bx = i32::from(x & !mask);
        let ega = self.f.mode == 1;
        let mut lit = Vec::new();
        for r in 0..19 {
            let py = i32::from(y) + r as i32;
            for (dx, v) in Self::sprite_row(&self.f.fighter, base + r * row, row, bpp) {
                let px = bx + dx;
                if (0..320).contains(&px) && (0..200).contains(&py) {
                    let i = py as usize * W + px as usize;
                    let old = self.g.term.px[i];
                    self.g.term.px[i] = match (ega, on) {
                        (true, true) => C_WHITE,
                        (_, false) => {
                            if ega {
                                BLACK
                            } else {
                                old & !v
                            }
                        }
                        (false, true) => old | v,
                    };
                    lit.push(i as u32);
                }
            }
        }
        if on {
            self.g.term.space_obj(sp::FIGHTER, 0, lit, false, true);
        } else {
            self.g.term.space_drop(sp::FIGHTER);
        }
    }

    /// `3109`: show or hide the exhaust flame of the current ship.
    fn flames(&mut self, on: bool) {
        if !on {
            self.flame -= 1;
        }
        if self.flame_on != on {
            self.flame_on = !self.flame_on;
            let sl = self.slot();
            let list = FLAMES[sl.ty.clamp(0, 2) as usize][(sl.hd & 3) as usize];
            for &(dx, sx, dy, sy, white, n) in list {
                for i in 0..n {
                    let x = sl.x + dx + i * sx;
                    let y = sl.y + dy + i * sy;
                    if x > 20 && x < 300 && y > 18 && y < 141 {
                        let c = if white { C_WHITE } else { C_FLAME };
                        let old = self.g.term.at(x as usize, y as usize);
                        self.g.term.plot(i32::from(x), i32::from(y), old ^ c);
                    }
                }
            }
        }
    }

    fn crosshair(&mut self, c: u8) {
        let (x, y) = (i32::from(self.cx), i32::from(self.cy));
        self.g.term.line_u1(x - 2, y, x + 2, y, c);
        self.g.term.line_u1(x, y - 2, x, y + 2, c);
        if c == BLACK {
            self.g.term.space_drop(sp::RETICLE);
        } else {
            let mut lit: Vec<u32> = (-2..=2).map(|d| ((y as usize) * W + (x + d) as usize) as u32).collect();
            lit.extend([-2, -1, 1, 2].map(|d| (((y + d) as usize) * W + x as usize) as u32));
            self.g.term.space_obj(sp::RETICLE, 0, lit, false, true);
        }
    }

    // ------------------------------------------------------- starfield

    fn in_view(x: i16, y: i16) -> bool {
        x > 20 && x < 300 && y > 20 && y < 140
    }

    /// `1b21`: a new star somewhere around the crosshair.
    fn new_star(&mut self, i: usize) {
        let q = self.rnd(0, 255) / 64;
        let mut dx = self.rnd(2, 255) / 2;
        let mut dy = self.rnd(4, 255) / 4;
        if q == 1 || q == 2 {
            dx = -dx;
        }
        if q > 1 {
            dy = -dy;
        }
        if !Self::in_view(self.cx + dx, self.cy + dy) {
            dx = 5000;
            dy = 5000;
        }
        self.rel[i] = (dx, dy);
    }

    /// `1dda`
    fn new_stars(&mut self) {
        for i in 0..40 {
            self.new_star(i);
            self.old[i] = (self.cx + 3, self.cy + 3);
        }
    }

    /// `1c2a`: one frame of the stars streaming outwards.
    fn stars(&mut self) {
        self.snd(9);
        let moving = self.dvx != 0 || self.dvy != 0 || self.star_mode != 2;
        if moving {
            self.crosshair(BLACK);
        }
        for i in 0..40 {
            let (rx, ry) = self.rel[i];
            let mut sx = rx / 8;
            if sx == 0 {
                sx = sign(rx);
            }
            let mut sy = ry / 6;
            if sy == 0 {
                sy = sign(ry);
            }
            let (ox, oy) = self.old[i];
            if self.star_mode != 1 && Self::in_view(ox, oy) {
                self.g.term.plot(i32::from(ox), i32::from(oy), BLACK);
            }
            self.rel[i] = (rx.wrapping_add(sx), ry.wrapping_add(sy));
            let (nx, ny) = (self.rel[i].0.wrapping_add(self.cx), self.rel[i].1.wrapping_add(self.cy));
            if !Self::in_view(nx, ny) {
                self.new_star(i);
            } else {
                self.g.term.plot(i32::from(nx), i32::from(ny), C_WHITE);
            }
            self.old[i] = (self.rel[i].0.wrapping_add(self.cx), self.rel[i].1.wrapping_add(self.cy));
        }
        if self.star_mode == 2 {
            self.cx += self.dvx;
            if self.cx >= 300 || self.cx <= 20 {
                self.cx -= self.dvx;
                self.dvx = 0;
            }
            self.cy += self.dvy;
            if self.cy >= 140 || self.cy <= 20 {
                self.cy -= self.dvy;
                self.dvy = 0;
            }
        }
        if moving {
            self.crosshair(C_WHITE);
        }
        self.snd(9);
    }

    // ------------------------------------------------------------ ticks

    /// `386b`: one tick of flight (top-down), or `3330` in front view.
    fn tick(&mut self) -> R<()> {
        if self.fp {
            return self.fp_tick();
        }
        let sl = self.slot();
        let moving = sl.hd != self.last_hd || self.vx != 0 || self.vy != 0;
        if !moving && self.flame == 0 {
            return Ok(());
        }
        if moving {
            self.ship(sl.ty, self.last_hd, sl.x, sl.y);
        }
        if self.flame != 0 {
            self.flames(false);
        }
        if moving {
            let (vx, vy) = (self.vx, self.vy);
            let s = self.slot_mut();
            s.x += vx;
            if s.x > 275 {
                s.x = 20;
            } else if s.x < 20 {
                s.x = 275;
            }
            s.y += vy;
            if s.y > 130 {
                s.y = 10;
            } else if s.y < 10 {
                s.y = 130;
            }
            let sl = *s;
            let col = self.ship(sl.ty, sl.hd, sl.x, sl.y);
            self.last_hd = sl.hd;
            if col {
                self.collision()?;
            } else if self.docked_port(self.cur).is_some() {
                self.dock()?;
                self.flame = 0;
            }
        }
        if self.flame != 0 {
            self.flames(true);
        }
        Ok(())
    }

    fn spawn(&mut self) {
        if !self.hyper {
            self.ex = self.rnd(40, 280);
            self.ey = self.rnd(40, 120);
            self.esize = 0;
        } else {
            self.ex = 0;
            self.ey = 0;
        }
    }

    /// `3330`: one tick of front view: stars and the fighter.
    fn fp_tick(&mut self) -> R<()> {
        if self.flame != 0 {
            self.flame -= 1;
        }
        self.flame_on = false;
        self.stars();
        let n = self.wd(d::RETRACE + 2 * self.speed.clamp(0, 8) as usize);
        self.retrace(n as u32)?;
        if self.sec().enemies == 0 {
            return Ok(());
        }
        self.eox = self.ex;
        self.eoy = self.ey;
        let dx = self.cx - self.ex;
        let dy = self.cy - self.ey;
        self.ex -= sign(dx);
        self.ey -= sign(dy);
        if self.ex > 280 || self.ex < 30 || self.ey > 125 || self.ey < 24 {
            if self.eox != 0 {
                self.fighter(self.eox, self.eoy, self.esize, false);
            }
            self.spawn();
        } else {
            if self.eox != 0 {
                self.fighter(self.eox, self.eoy, self.esize, false);
            }
            if (self.hyper && self.grow >= 8 - self.speed) || self.grow > 15 {
                let r = self.rnd(1, (self.speed + 3) * 10);
                if r > 20 || self.hyper {
                    self.esize += 1;
                }
                if self.esize > 6 {
                    self.spawn();
                }
                self.grow = 0;
            } else {
                self.grow += 1;
            }
        }
        if self.ex != 0 {
            self.fighter(self.ex, self.ey, self.esize, true);
            self.alien_fire()?;
            if dx.abs() < 4 || dy.abs() < 4 {
                self.crosshair(C_WHITE);
            }
        }
        Ok(())
    }

    /// `31d7`: now and then the fighter shoots back.
    fn alien_fire(&mut self) -> R<()> {
        if self.rnd(1, 200) >= 4 || self.esize == 0 || self.hyper || self.ex == 0 {
            return Ok(());
        }
        self.println(d::ALIEN_FIRES);
        let r = self.rnd(1, 255);
        let (tx, ty) = if r < 64 {
            (159, 135)
        } else if r < 128 {
            (159, 20)
        } else if r > 192 {
            (20, 79)
        } else {
            (290, 79)
        };
        let sz = self.esize as usize;
        let x0 = self.ex + self.wd(d::HIT_W + 2 * sz) / 2;
        let y0 = self.ey + self.wd(d::HIT_H + 2 * sz);
        self.g.term.line_u1(x0.into(), y0.into(), tx, ty, C_RED);
        self.snd_ev(8, Sfx::EnemyLaser);
        self.delay(2)?;
        self.g.term.line_u1(x0.into(), y0.into(), tx, ty, BLACK);
        if r < 64 {
            self.println(d::BEEN_HIT);
            self.snd_ev(2, Sfx::Hurt);
            let n = self.slot().shield.min(321);
            self.use_shield(n);
            if self.slot().shield == 0 {
                self.println(d::DRAINED);
                self.death()?;
            }
        }
        self.prompt();
        Ok(())
    }

    // --------------------------------------------------------- commands

    /// `165c`: a new speed (1-8) in front view.
    fn cmd_speed(&mut self, n: i16) -> R<()> {
        self.puts(d::SPEED);
        self.g.term.num(n as u16, 1, ' ', TEXT, true);
        self.println(d::EMPTY);
        let cost = (n - self.speed).abs() * 4;
        if cost > self.slot().fuel {
            self.println(d::NOT_ENOUGH_FUEL);
        } else {
            self.speed = n;
            self.use_fuel(cost);
        }
        Ok(())
    }

    /// `1919`: arrows steer (front view) or turn and thrust (top-down).
    fn cmd_arrow(&mut self, dir: i16) -> R<()> {
        if !self.fp {
            return self.td_move(dir);
        }
        let name = self.ptr(d::FP_NAMES, dir as usize);
        self.g.term.println(&name);
        if self.slot().fuel < 2 {
            self.println(d::NOT_ENOUGH_FUEL);
            return Ok(());
        }
        self.use_fuel(2);
        match dir {
            1 => (self.dvx, self.dvy) = (-4, 0),
            2 => (self.dvx, self.dvy) = (4, 0),
            3 => (self.dvx, self.dvy) = (0, -4),
            _ => (self.dvx, self.dvy) = (0, 4),
        }
        Ok(())
    }

    /// `16d3`: top-down turning, thrust and retro.
    fn td_move(&mut self, dir: i16) -> R<()> {
        let name = self.ptr(d::TD_NAMES, dir as usize);
        self.g.term.println(&name);
        let mut cost = if dir <= 2 { 2 } else { 5 };
        let mut delta = -1;
        let fuel = self.slot().fuel;
        if fuel == 0 {
            self.println(d::NO_FUEL);
            self.println(d::DRIFT);
        }
        if fuel < cost {
            cost = fuel;
        }
        if self.docked_port(self.cur).is_some() && dir != 4 {
            self.println(d::CRUNCH);
            self.snd_ev(0, Sfx::Crash);
            self.crash_damage();
            if self.slot().shield == 0 {
                self.println(d::DRAINED);
                self.death()?;
            }
            return Ok(());
        }
        self.use_fuel(cost);
        let h = self.slot().hd;
        if dir <= 2 {
            if self.flame != 0 {
                self.flames(false);
            }
            let nh = self.wd(d::TURN + 2 * ((dir - 1) * 4 + h) as usize);
            self.slot_mut().hd = nh;
            self.flame = 0;
            delta = 0;
        } else if dir == 3 {
            delta = 1;
        }
        match h {
            0 => self.vx -= delta,
            1 => self.vx += delta,
            2 => self.vy -= delta,
            _ => self.vy += delta,
        }
        self.vx = self.vx.clamp(-9, 9);
        self.vy = self.vy.clamp(-9, 9);
        if dir == 3 {
            self.flame += 10;
        } else if dir == 4 {
            self.flames(false);
            self.flame = 0;
        }
        Ok(())
    }

    /// `350a`: the ship touched something.
    fn collision(&mut self) -> R<()> {
        let sl = self.slot();
        let s = *self.sec();
        if s.sx != 0 && (s.sx - sl.x).abs() < 25 && (s.sy - sl.y).abs() < 25 {
            self.println(d::MELTS);
            return self.death();
        }
        if (self.gx, self.gy) == (3, 3) && (sl.x - 60).abs() < 25 && (sl.y - 75).abs() < 25 {
            if sl.ty == 0 {
                self.println(d::LANDED);
                self.g.rec.set_u16(off::SHUTTLE_0, sl.fuel as u16);
                self.g.rec.set_u16(off::SHUTTLE_1, sl.shield as u16);
                return self.exit();
            }
            self.println(d::HEAT_1);
            self.println(d::HEAT_2);
            self.g.rec.set_u16(off::SHUTTLE_0, 0);
            self.g.rec.set_u16(off::SHUTTLE_1, 0);
            return self.death();
        }
        self.println(d::CRUNCH);
        self.prompt();
        self.snd_ev(0, Sfx::Crash);
        self.crash_damage();
        if self.slot().shield == 0 {
            self.println(d::DRAINED);
            return self.death();
        }
        self.vx = -self.vx;
        self.vy = -self.vy;
        Ok(())
    }

    /// `3567`: docked at a space station.
    fn dock(&mut self) -> R<()> {
        let count = self.sec().slots.iter().filter(|s| s.ty > -1).count();
        self.println(d::DOCKED);
        self.cue(Sfx::Dock);
        self.vx = 0;
        self.vy = 0;
        if self.g.rec.u16_at(off::READY_ARMOUR) < 4 {
            self.println(d::VACUUM_1);
            self.println(d::VACUUM_2);
            return self.death();
        }
        if self.g.coin() < 500 {
            self.println(d::FEE_1);
            self.println(d::FEE_2);
            self.puts(d::FEE_3);
            self.io.key(&self.g.term)?;
            self.g.term.println("");
        } else {
            self.g.add(off::COIN, -500);
            let s = self.slot_mut();
            s.fuel = Self::max_fuel(s.ty);
            s.shield = Self::max_shield(s.ty);
        }
        if count > 1 {
            loop {
                self.puts(d::CHOOSE);
                let k = self.io.key(&self.g.term)?;
                if (b' '..=b'~').contains(&k) {
                    self.g.term.print(&(k as char).to_string());
                }
                self.g.term.println("");
                if !(b'1'..=b'4').contains(&k) {
                    continue;
                }
                let p = (k - b'1') as i16;
                match (0..4).find(|&i| self.docked_port(i) == Some(p)) {
                    None => self.println(d::NO_PORT),
                    Some(i) => {
                        self.last_hd = self.sec().slots[i].hd;
                        self.cur = i;
                        let pn = self.ptr(d::PORT_NAMES, p as usize);
                        let t = format!("{}{}{}", self.s(d::IN_THE), pn, self.s(d::SHIP));
                        self.g.term.println(&t);
                        break;
                    }
                }
            }
        }
        self.show_fuel();
        self.show_shield();
        self.show_coin();
        self.prompt();
        self.g.term.cursor(1, 24);
        Ok(())
    }

    /// `1a97`: Pass (top-down) or re-centre the sights.
    fn cmd_center(&mut self) {
        if !self.fp {
            self.println(d::PASS);
            return;
        }
        self.crosshair(BLACK);
        self.cx = 159;
        self.cy = 79;
        self.dvx = 0;
        self.dvy = 0;
        self.g.term.fill(16, 16, 303, 143, BLACK);
        self.crosshair(C_WHITE);
        self.println(d::CENTER);
    }

    fn cmd_noise(&mut self) {
        self.puts(d::NOISE);
        let on = self.g.rec.u16_at(off::SOUND) == 0;
        self.g.rec.set_u16(off::SOUND, u16::from(on));
        self.println(if on { d::ON } else { d::OFF });
    }

    /// `3e9`: integer square root rounded up.
    fn isqrt_up(n: i32) -> i16 {
        let mut r = 0;
        while r * r < n {
            r += 1;
        }
        r as i16
    }

    fn rescale(&self, v: i16, k: i16) -> i16 {
        let mut v = v.wrapping_mul(k);
        if v % 100 > 49 {
            v = v.wrapping_add(100);
        }
        if v % 1000 > 499 {
            v = v.wrapping_add(1000);
        }
        (v / 1000).min(6)
    }

    fn velocity_from_heading(&mut self) {
        match self.slot().hd {
            0 => self.vx = -1,
            1 => self.vx = 1,
            2 => self.vy = -1,
            _ => self.vy = 1,
        }
    }

    /// `10b3`: switch between top-down and front view.
    fn cmd_view(&mut self) {
        self.puts(d::VIEW);
        if !self.fp {
            if self.docked_port(self.cur).is_some() {
                self.println(d::QMARK);
                self.println(d::NO_VIEW_1);
                self.println(d::NO_VIEW_2);
                return;
            }
            self.println(d::EMPTY);
            if self.slot().fuel == 0 {
                self.println(d::NOT_ENOUGH_FUEL);
                return;
            }
            self.fp = true;
            self.redraw = true;
            let (vx, vy) = (i32::from(self.vx), i32::from(self.vy));
            self.speed = Self::isqrt_up(vx * vx + vy * vy).min(8);
            if self.speed == 0 {
                self.speed = 1;
                self.velocity_from_heading();
            }
            self.entry_speed = self.speed;
        } else {
            if self.sec().enemies != 0 {
                self.println(d::QMARK);
                self.println(d::ELIM_1);
                self.println(d::ELIM_2);
                return;
            }
            self.fp = false;
            self.println(d::EMPTY);
            self.redraw = true;
            let k = (self.speed.wrapping_mul(1000)).wrapping_div(self.entry_speed.max(1));
            self.vx = self.rescale(self.vx, k);
            self.vy = self.rescale(self.vy, k);
            if self.vx == 0 && self.vy == 0 {
                self.velocity_from_heading();
            }
        }
    }

    /// `2252`: lasers.
    fn cmd_fire(&mut self) -> R<()> {
        if !self.fp {
            self.println(d::FIRE_Q);
            self.snd(1);
            return Ok(());
        }
        self.println(d::FIRE);
        if self.slot().fuel < 10 {
            self.println(d::FIRE_NO_FUEL);
            return Ok(());
        }
        self.use_fuel(10);
        let (dx, dy) = if self.ex != 0 {
            self.fighter(self.ex, self.ey, self.esize, true);
            (self.cx - self.ex, self.cy - self.ey)
        } else {
            (-100, -100)
        };
        let (cx, cy) = (i32::from(self.cx), i32::from(self.cy));
        self.g.term.line_u1(20, 140, cx, cy, C_RED);
        self.g.term.line_u1(300, 140, cx, cy, C_RED);
        self.snd(8);
        let sz = self.esize as usize;
        let (w, h) = (self.wd(d::HIT_W + 2 * sz), self.wd(d::HIT_H + 2 * sz));
        if self.ex != 0 {
            self.fighter(self.ex, self.ey, self.esize, false);
        }
        let (x0, y0) = (self.wd(d::HIT_X0 + 2 * sz), self.wd(d::HIT_Y0 + 2 * sz));
        if x0 <= dx && dx <= w && y0 <= dy && dy <= h {
            self.snd_ev(2, Sfx::Explosion);
            self.println(d::HIT);
            self.g.add(off::EXPERIENCE, 100);
            self.show_exp();
            self.g.add(off::ENEMIES, 1);
            let kills = self.g.w(off::ENEMIES);
            if kills == 20 {
                self.space_ace()?;
            } else if kills > 20 {
                self.println(d::STILL_ACE);
            }
            self.sec_mut().enemies -= 1;
            if self.sec().enemies != 0 {
                self.spawn();
            } else {
                self.ex = 0;
                self.ey = 0;
            }
        }
        self.g.term.line_u1(20, 140, cx, cy, BLACK);
        self.g.term.line_u1(300, 140, cx, cy, BLACK);
        self.crosshair(C_WHITE);
        Ok(())
    }

    /// `21fc`: the twentieth kill.
    fn space_ace(&mut self) -> R<()> {
        self.io.flush();
        self.g.term.boxed(C_BLUE);
        for (col, row, o) in [(11, 6, d::ACE_1), (14, 8, d::ACE_2), (15, 10, d::ACE_3)] {
            let t = self.s(o);
            self.g.term.print_at_attr(col, row, &t, TEXT);
        }
        self.delay(5)?;
        self.io.flush();
        self.io.wait_or_key(&self.g.term)?;
        self.draw_all();
        Ok(())
    }

    /// `2003`: the jump to light speed into the next sector.
    fn cmd_hyper(&mut self) -> R<()> {
        self.puts(d::HYPER);
        if !self.fp {
            self.println(d::QMARK);
            self.println(d::HYPER_1);
            self.println(d::HYPER_2);
            return Ok(());
        }
        if self.slot().fuel < 100 {
            self.println(d::EMPTY);
            self.println(d::NOT_ENOUGH_FUEL);
            return Ok(());
        }
        self.grow = 20;
        self.puts(d::DOTS3);
        self.cue(Sfx::Hyperjump);
        let s = self.timer() | 0xbbbb;
        self.dvx = 0;
        self.dvy = 0;
        let s0 = self.speed;
        let n = if s0 < 4 { 10 } else { 15 };
        self.hyper = true;
        for sp in s0..=8 {
            self.speed = sp;
            for _ in 0..=n {
                self.fp_tick()?;
            }
        }
        self.speed = 0;
        for i in 0..40 {
            let (x, y) = self.old[i];
            if Self::in_view(x, y) {
                self.g.term.plot(x.into(), y.into(), BLACK);
            }
        }
        self.puts(d::LIGHTSPEED);
        self.g.term.cursor(10, 24);
        self.srand(s);
        self.new_stars();
        self.star_mode = 1;
        for _ in 0..=200 {
            self.stars();
        }
        self.srand(s);
        self.new_stars();
        self.star_mode = 3;
        for _ in 0..200 {
            self.stars();
        }
        self.star_mode = 2;
        let mut sp = 8;
        while s0 < sp {
            self.speed = sp;
            for _ in 0..=n {
                self.stars();
                let r = self.wd(d::RETRACE + 2 * sp as usize);
                self.retrace(r as u32)?;
            }
            sp -= 1;
        }
        self.crosshair(C_WHITE);
        self.println(d::COMPLETED);
        self.use_fuel(100);
        let t = self.timer();
        self.srand(t.wrapping_add(1234));
        self.speed = s0;
        self.io.flush();
        self.ex = 0;
        self.ey = 0;
        self.eox = 0;
        self.eoy = 0;
        self.next_sector();
        self.hyper = false;
        self.flame = 0;
        Ok(())
    }

    /// `1e1c`: move the ship into the neighbouring sector.
    fn next_sector(&mut self) {
        let hd = self.slot().hd;
        let mut ddx = sign(self.vx);
        let mut ddy = sign(self.vy);
        if ddx == 0 && ddy == 0 {
            match hd {
                0 => ddx = -1,
                1 => ddx = 1,
                2 => ddy = -1,
                _ => ddy = 1,
            }
        }
        let wrap = |v: i16| if v < 0 { 6 } else { v % 7 };
        let nx = wrap(self.gx as i16 + ddx) as usize;
        let ny = wrap(self.gy as i16 + ddy) as usize;
        let ship = self.slot();
        if let Some(i) = self.gal[nx][ny].slots.iter().position(|s| s.ty == -1) {
            self.slot_mut().ty = -1;
            self.gx = nx;
            self.gy = ny;
            self.cur = i;
            self.gal[nx][ny].slots[i] = Slot { x: 260, y: 90, ..ship };
        }
        if self.sec().enemies != 0 {
            self.spawn();
        }
    }

    /// `14e5`: the sector scan of the galaxy.
    fn cmd_inform(&mut self) -> R<()> {
        self.puts(d::INFORM);
        self.g.term.sscene = None;
        self.g.term.boxed(C_BLUE);
        let t = self.s(d::SCAN);
        self.g.term.print_at_attr(5, 4, &t, TEXT);
        let t = &mut self.g.term;
        t.line_u1(130, 44, 200, 44, C_WHITE);
        t.lineto(200, 115, C_WHITE);
        t.lineto(130, 115, C_WHITE);
        t.lineto(130, 44, C_WHITE);
        for gx in 0..7 {
            for gy in 0..7 {
                self.scan_icon(gx, gy);
            }
        }
        let x = 10 * self.gx as i32 + 132;
        let y = 10 * self.gy as i32 + 46;
        loop {
            if self.io.poll_now(&self.g.term)?.is_some() {
                break;
            }
            self.delay(7)?;
            for i in 0..8 {
                self.g.term.line_u1(x + i, y, x + i, y + 8, BLACK);
            }
            if self.io.poll_now(&self.g.term)?.is_some() {
                break;
            }
            self.delay(7)?;
            self.scan_icon(self.gx, self.gy);
        }
        self.redraw = true;
        self.println(d::EMPTY);
        Ok(())
    }

    /// `142b`: one cell of the scan.
    fn scan_icon(&mut self, gx: usize, gy: usize) {
        let x = 10 * gx as i32 + 132;
        let y = 10 * gy as i32 + 46;
        let s = self.gal[gx][gy];
        let t = &mut self.g.term;
        if s.enemies != 0 {
            for i in 0..2 {
                t.line_u1(x + i, y, x + i, y + 7, C_WHITE);
                t.line_u1(x + i + 5, y, x + i + 5, y + 7, C_WHITE);
                t.line_u1(x, y + 3 + i, x + 6, y + 3 + i, C_WHITE);
            }
        } else if s.stx != 0 {
            for i in 0..2 {
                t.line_u1(x + 3 + i, y, x + 3 + i, y + 7, C_WHITE);
                t.line_u1(x, y + 3 + i, x + 7, y + 3 + i, C_WHITE);
            }
        } else if s.sx != 0 {
            for i in 0..3 {
                t.line_u1(x + 3 - i, y + 1 + i, x + 3 + i, y + 1 + i, C_WHITE);
                t.line_u1(x + 3 - i, y + 6 - i, x + 3 + i, y + 6 - i, C_WHITE);
            }
        } else {
            t.plot(x + 1, y + 1, C_WHITE);
            t.plot(x + 3, y + 3, C_RED);
            t.plot(x + 6, y + 2, C_BLUE);
            t.plot(x + 4, y + 5, C_WHITE);
            t.plot(x + 2, y + 6, C_BLUE);
        }
    }
}

/// Checks against the user's own files (`U1_GAME_DIR`); skipped without.
#[cfg(test)]
mod real_game {
    use super::*;
    use crate::game::Assets;
    use crate::io::ScriptIo;
    use crate::save::PlayerRecord;

    fn setup() -> Option<(Game, SpaceFiles)> {
        let g = crate::gamedir::GameDir::new(std::env::var_os("U1_GAME_DIR")?).ok()?;
        let mode = g.detect_video_mode()?;
        let exe = g.read("OUT.EXE").ok()?;
        let assets = Assets {
            t: crate::rules::ExeTables::from_out_exe(&exe).unwrap(),
            locs: crate::locations::LocationTable::from_out_exe(&exe).unwrap(),
            exe: crate::exe::Exe::new(exe).unwrap(),
            map: g.load_world_map().unwrap(),
            world_tiles: g.load_world_tiles(mode).unwrap(),
            town_tiles: g.load_town_tiles(mode).unwrap(),
            tcd: crate::tcd::TownCastleMaps::parse(&g.read("TCD.BIN").unwrap()).unwrap(),
            video: mode,
        };
        let rec = PlayerRecord::parse(&g.read("PLAYER2.U1").ok()?).unwrap();
        Some((Game::new(assets, rec, 1), SpaceFiles::load(&g, crate::gamedir::VideoMode::Ega).ok()?))
    }

    /// Put a size-1 fighter where the laser box of `2252` catches it.
    fn fighter_in_sights(s: &mut Space) {
        s.fp = true;
        s.slot_mut().fuel = 500;
        s.sec_mut().enemies = 3;
        s.esize = 1;
        s.ex = s.cx - 2;
        s.ey = s.cy - 5;
    }

    #[test]
    fn twentieth_kill_makes_a_space_ace() {
        let Some((mut g, files)) = setup() else { return };
        g.rec.set_u16(off::ENEMIES, 19);
        let exp = g.rec.u16_at(off::EXPERIENCE);
        let mut io = ScriptIo::from_text("{w1}");
        let mut s = Space::new(&mut g, &mut io, files.clone());
        fighter_in_sights(&mut s);
        let _ = s.cmd_fire();
        assert_eq!(s.g.w(off::ENEMIES), 20);
        assert_eq!(s.sec().enemies, 2);
        assert_eq!(s.slot().fuel, 490);
        drop(s);
        assert_eq!(g.rec.u16_at(off::EXPERIENCE), exp + 100);
        let mut io = ScriptIo::from_text("{w1}");
        let mut s = Space::new(&mut g, &mut io, files.clone());
        fighter_in_sights(&mut s);
        // The boxed panel was replaced by the redraw; a further kill says so.
        let _ = s.cmd_fire();
        assert_eq!(s.g.w(off::ENEMIES), 21);
        assert!(s.g.term.messages().iter().any(|l| l.contains("still a Space")));
    }

    #[test]
    fn the_smallest_fighters_cannot_be_hit() {
        // The original's hit box for size 0 asks for 5 <= dy <= 4.
        let Some((mut g, files)) = setup() else { return };
        let mut io = ScriptIo::from_text("");
        let mut s = Space::new(&mut g, &mut io, files.clone());
        fighter_in_sights(&mut s);
        s.esize = 0;
        for dy in -10..10 {
            s.ey = s.cy - dy;
            let _ = s.cmd_fire();
        }
        assert_eq!(s.g.w(off::ENEMIES), 0);
    }

    /// Docking: 500 coin refills the ship, then any ship docked at the
    /// station can be taken by its port number.
    #[test]
    fn docking_refuels_and_changes_ships() {
        let Some((mut g, files)) = setup() else { return };
        g.rec.set_u16(off::READY_ARMOUR, 4);
        g.rec.set_u16(off::COIN, 1000);
        g.rec.set_u16(off::SHUTTLE_0, 100);
        let mut io = ScriptIo::from_text("1");
        let mut s = Space::new(&mut g, &mut io, files.clone());
        s.make_galaxy();
        let mut h = *s.sec();
        s.dock_ship(&mut h, 2, 1, 0);
        h.slots[2].fuel = 100;
        s.gal[3][3] = h;
        assert_eq!(s.docked_port(2), Some(1));
        let _ = s.dock();
        assert_eq!(s.g.coin(), 500);
        assert_eq!(s.sec().slots[2].fuel, 1000);
        // Port 1 holds the type-1 ship of the home station.
        assert_eq!(s.cur, 1);
        assert_eq!(s.slot().ty, 1);
        // Without the coin there is no refill.
        let mut io = ScriptIo::from_text("x1");
        let mut s = Space::new(&mut g, &mut io, files.clone());
        s.make_galaxy();
        s.g.rec.set_u16(off::COIN, 100);
        s.slot_mut().fuel = 7;
        let mut h = *s.sec();
        s.cur = 0;
        h.slots[0].fuel = 7;
        s.gal[3][3] = h;
        let _ = s.dock();
        assert_eq!(s.g.coin(), 100);
        assert_eq!(s.sec().slots[0].fuel, 7);
    }
}
