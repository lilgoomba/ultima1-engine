//! The first-person dungeon view (SPEC §6.6), written from our reading of
//! the original's drawing order ("verified from original executable").
//!
//! Every coordinate table and monster/object outline comes from the
//! user's OUT.EXE at run time; this file only holds the drawing rules.

use crate::dungeon::{Cell, DungeonLevel, Item};
use crate::exe::{m, Exe};
use crate::term::{Term, BLACK, WHITE};

/// Force-field colour (EGA light red).
pub const FIELD: u8 = 12;
const CX: i32 = 16; // left margin of the corridor geometry
const RX: i32 = 303; // mirrored right edge
const BY: i32 = 151; // bottom of the viewport

/// Facing codes as the original stores them.
pub const WEST: u16 = 1;
pub const EAST: u16 = 2;
pub const NORTH: u16 = 3;
pub const SOUTH: u16 = 4;

/// One step forward for a facing: (da, db).
pub fn forward(f: u16) -> (i32, i32) {
    match f {
        WEST => (-1, 0),
        EAST => (1, 0),
        NORTH => (0, -1),
        _ => (0, 1),
    }
}

/// Offsets of the cells to the left and right of the corridor.
fn sides(f: u16) -> ((i32, i32), (i32, i32)) {
    match f {
        WEST => ((0, 1), (0, -1)),
        EAST => ((0, -1), (0, 1)),
        NORTH => ((-1, 0), (1, 0)),
        _ => ((1, 0), (-1, 0)),
    }
}

/// Tables read from OUT.EXE.
#[derive(Debug, Clone)]
pub struct ViewTables {
    pub x: [i32; 6],
    pub y: [i32; 6],
    pub base: [i32; 5],
    pub shapes: Vec<Vec<u8>>,
}

impl ViewTables {
    pub fn from_exe(e: &Exe) -> Self {
        let sw = |o: usize| e.w(o) as i16 as i32;
        let mut x = [0; 6];
        let mut y = [0; 6];
        for i in 0..6 {
            x[i] = sw(m::VIEW_X + 2 * i);
            y[i] = sw(m::VIEW_Y + 2 * i);
        }
        let mut base = [0; 5];
        for (i, b) in base.iter_mut().enumerate() {
            *b = sw(m::SHAPE_Y + 2 * i);
        }
        let mut shapes = Vec::new();
        for i in 0..30 {
            let mut p = e.w(m::SHAPES + 2 * i) as usize;
            let mut v = Vec::new();
            loop {
                let c = e.b(p);
                v.push(c);
                p += 1;
                if c == 0x7f || v.len() > 2048 {
                    break;
                }
            }
            shapes.push(v);
        }
        ViewTables { x, y, base, shapes }
    }
}

/// The view renderer; `t` is the screen, names go to the message window.
pub struct View<'a> {
    pub t: &'a mut Term,
    pub e: &'a Exe,
    pub v: &'a ViewTables,
    pub fg: u8,
}

impl View<'_> {
    fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, c: u8) {
        self.t.line_u1(x0, y0, x1, y1, c);
    }

    fn x(&self, d: i32) -> i32 {
        self.v.x[d.clamp(0, 5) as usize]
    }
    fn y(&self, d: i32) -> i32 {
        self.v.y[d.clamp(0, 5) as usize]
    }

    /// An outline from the shape table, scaled for depth `d`.
    pub fn shape(&mut self, id: usize, d: i32, c: u8) {
        let mut cx = d - 1;
        if !(0..=4).contains(&cx) {
            cx = 4;
        }
        let base = self.v.base[cx as usize];
        let shift = if cx == 4 { 5 } else { cx as u32 };
        let Some(bytes) = self.v.shapes.get(id).cloned() else { return };
        // Sign-extend, shift right logically as a word, sign-extend the
        // low byte again (exactly what the original does).
        let sc = |b: u8| -> i32 {
            let w = (b as i8 as i16 as u16) >> shift;
            (w as u8) as i8 as i32
        };
        let mut i = 0;
        while i < bytes.len() {
            let b = bytes[i];
            i += 1;
            if b == 0x7f {
                break;
            }
            if b == 0x7e {
                if i + 1 >= bytes.len() {
                    break;
                }
                let (x, y) = (sc(bytes[i]) + 160, sc(bytes[i + 1]) + base);
                i += 2;
                self.t.moveto_plot(x, y, c);
            } else {
                if i >= bytes.len() {
                    break;
                }
                let (x, y) = (sc(b) + 160, sc(bytes[i]) + base);
                i += 1;
                self.t.lineto(x, y, c);
            }
        }
    }

    // --- corridor pieces (d = depth 0..=5) ---

    fn left_wall(&mut self, d: i32) {
        if d > 5 {
            return;
        }
        let (x0, y0, x1, y1) = (self.x(d - 1) + CX, self.y(d - 1), self.x(d) + CX, self.y(d));
        self.line(x0, y0 + 8, x1, y1 + 8, self.fg);
        self.line(x0, BY - y0, x1, BY - y1, self.fg);
    }

    fn right_wall(&mut self, d: i32) {
        if d > 5 {
            return;
        }
        let (x0, y0, x1, y1) = (RX - self.x(d - 1), self.y(d - 1), RX - self.x(d), self.y(d));
        self.line(x0, y0 + 8, x1, y1 + 8, self.fg);
        self.line(x0, BY - y0, x1, BY - y1, self.fg);
    }

    fn left_open(&mut self, d: i32) {
        if d > 5 {
            return;
        }
        let (x0, x1, y) = (self.x(d - 1) + CX, self.x(d) + CX, self.y(d));
        self.line(x0, y + 8, x1, y + 8, self.fg);
        self.line(x0, BY - y, x1, BY - y, self.fg);
    }

    fn right_open(&mut self, d: i32) {
        if d > 5 {
            return;
        }
        let (x0, x1, y) = (RX - self.x(d - 1), RX - self.x(d), self.y(d));
        self.line(x0, y + 8, x1, y + 8, self.fg);
        self.line(x0, BY - y, x1, BY - y, self.fg);
    }

    /// Top and bottom edge of a wall facing the player.
    pub fn front_wall(&mut self, d: i32) {
        if d > 5 {
            return;
        }
        let o = if d == 0 { 8 } else { 0 };
        let (x, y) = (self.x(d), self.y(d));
        self.line(x + CX + o, y + 8 + o, RX - x - o, y + 8 + o, self.fg);
        self.line(x + CX + o, BY - y - o, RX - x - o, BY - y - o, self.fg);
    }

    fn left_vert(&mut self, d: i32) {
        if d > 5 {
            return;
        }
        let (x, y) = (self.x(d) + CX, self.y(d));
        self.line(x, y + 8, x, BY - y, self.fg);
    }

    fn right_vert(&mut self, d: i32) {
        if d > 5 {
            return;
        }
        let (x, y) = (RX - self.x(d), self.y(d));
        self.line(x, y + 8, x, BY - y, self.fg);
    }

    /// Door in the left wall: the wall edges plus a door outline.
    fn left_door(&mut self, d: i32) {
        if d > 5 {
            return;
        }
        self.left_wall(d);
        let (p, q) = self.door_steps(d);
        let x = self.x(d - 1) + 2 * p + CX;
        self.line(x, BY - q - self.y(d - 1) - 1, x, self.y(d) + 8 - q, self.fg);
        let x2 = 6 * p + CX + self.x(d - 1);
        self.t.lineto(x2, self.y(d) + 8 + q, self.fg);
        let near = if d == 1 { 2 } else { 0 };
        self.t.lineto(x2, BY - self.y(d) + 2 * q - near, self.fg);
    }

    fn right_door(&mut self, d: i32) {
        if d > 5 {
            return;
        }
        self.right_wall(d);
        let (p, q) = self.door_steps(d);
        let x = RX - (self.x(d - 1) + 2 * p);
        self.line(x, BY - q - self.y(d - 1) - 1, x, self.y(d) + 8 - q, self.fg);
        let x2 = RX - (6 * p + self.x(d - 1));
        self.t.lineto(x2, self.y(d) + 8 + q, self.fg);
        let near = if d == 1 { 2 } else { 0 };
        self.t.lineto(x2, BY - self.y(d) + 2 * q - near, self.fg);
    }

    /// |(X[d-1]-X[d])/9| and |(Y[d-1]-Y[d])/5| (truncating division).
    fn door_steps(&self, d: i32) -> (i32, i32) {
        let dx = self.x(d - 1) - self.x(d);
        let dy = self.y(d - 1) - self.y(d);
        ((dx / 9).abs(), (dy / 5).abs())
    }

    /// A door straight ahead at depth `d` (`d == 0`: the doorway the
    /// player stands in, whose sill is erased in black).
    fn front_door(&mut self, d: i32) {
        if d >= 5 {
            return;
        }
        let (c, o) = if d == 0 { (BLACK, 8) } else { (self.fg, 0) };
        self.front_wall(d);
        let (xn, yd, yn) = (self.x(d + 1), self.y(d), self.y(d + 1));
        self.line(RX - xn, BY - yd - o, xn + CX, BY - yd - o, c);
        self.t.lineto(xn + CX, yn + 8, self.fg);
        self.t.lineto(RX - xn, yn + 8, self.fg);
        self.t.lineto(RX - xn, BY - yd - o, self.fg);
    }

    fn field(&mut self, d: i32) {
        if d > 5 {
            return;
        }
        let (x, y) = (self.x(d), self.y(d));
        let step = ((y + 8 - (BY - y)) / 4).abs();
        for i in 1..=3 {
            let yy = y + 8 + i * step + d / 2;
            self.line(x + CX, yy, RX - x, yy, FIELD);
        }
    }

    fn ladder(&mut self, up: bool, ns: bool, d: i32) {
        if d > 5 {
            return;
        }
        self.shape(if up { 0x1a } else { 0x1b }, d, self.fg);
        self.shape(if ns { 0x1c } else { 0x1d }, d, self.fg);
    }

    fn monster(&mut self, d: i32, id: i16) {
        if d >= 5 || id < 0 {
            return;
        }
        let name = self.e.ptr(m::MONSTER_NAMES, id as usize);
        self.t.println(&name);
        if id == 9 {
            // The gelatinous cube is a block filling the corridor.
            self.front_wall(d);
            self.left_vert(d);
            self.right_vert(d);
        } else {
            self.shape(id as usize, d, self.fg);
        }
    }

    fn coffin(&mut self, d: i32) {
        let s = self.e.s(m::COFFIN);
        self.t.println(&s);
        self.shape(0x19, d, self.fg);
    }

    fn chest(&mut self, d: i32) {
        let s = self.e.s(m::CHEST);
        self.t.println(&s);
        self.shape(0x0b, d, self.fg);
    }

    /// What stands in cell (a,b), seen at depth `d`.
    fn contents(&mut self, lv: &DungeonLevel, facing: u16, d: i32, a: i32, b: i32) {
        let id = lv.monster(a, b);
        let s = lv.cell(a, b);
        if id != -1 {
            if matches!(s, Cell::Wall | Cell::SecretDoor | Cell::Door) {
                self.front_wall(d);
            }
            // With a monster on it a ladder is drawn without its rungs.
            if s == Cell::LadderDown {
                self.shape(0x1b, d + 1, self.fg);
            }
            if s == Cell::LadderUp {
                self.shape(0x1a, d + 1, self.fg);
            }
            self.monster(d, id);
            return;
        }
        let ns = facing == NORTH || facing == SOUTH;
        match s {
            Cell::Wall | Cell::SecretDoor => self.front_wall(d),
            Cell::Door => self.front_door(d),
            Cell::LadderDown => self.ladder(false, ns, d + 1),
            Cell::LadderUp => self.ladder(true, ns, d + 1),
            Cell::ForceField => self.field(d),
            _ => {}
        }
        match item_at(lv, a, b) {
            Item::Coffin => self.coffin(d + 1),
            Item::Chest => self.chest(d + 1),
            Item::None => {}
        }
    }
}

fn item_at(lv: &DungeonLevel, a: i32, b: i32) -> Item {
    if (0..11).contains(&a) && (0..11).contains(&b) {
        lv.items[a as usize][b as usize]
    } else {
        Item::None
    }
}

fn solid(c: Cell) -> bool {
    !matches!(c, Cell::Empty | Cell::LadderUp | Cell::LadderDown | Cell::ForceField)
}

/// A monster that hides what is behind it (not the invisible seeker,
/// the mimic or the gelatinous cube, which the eye passes through).
pub fn opaque_monster(lv: &DungeonLevel, a: i32, b: i32) -> bool {
    let id = lv.monster(a, b);
    !(id == -1 || id == 20 || id == 11 || id == 9)
}

/// How far the player can see ahead (1..=5).
pub fn sight(lv: &DungeonLevel, pa: i32, pb: i32, fa: i32, fb: i32) -> i32 {
    let (mut dx, mut dy, mut n) = (fa, fb, 1);
    loop {
        let (a, b) = (pa + dx, pb + dy);
        let c = lv.cell(a, b);
        if matches!(c, Cell::Wall | Cell::SecretDoor | Cell::Door) || opaque_monster(lv, a, b) {
            break;
        }
        n += 1;
        dx += fa.signum();
        dy += fb.signum();
        if n > 64 {
            break;
        }
    }
    n.min(5)
}

/// Compass text between two ornaments under the view.
pub fn compass(t: &mut Term, e: &Exe, facing: u16) {
    t.ornament_left(16, 19);
    let i = match facing {
        NORTH => 0,
        SOUTH => 1,
        EAST => 2,
        _ => 3,
    };
    t.print(&e.s(m::COMPASS[i]));
    t.ornament_right(24, 19);
    t.cursor(1, 24);
}

/// "Level n" title in the top border.
pub fn title(t: &mut Term, e: &Exe, level: u16) {
    t.ornament_left(15, 0);
    t.print(&e.s(m::LEVEL_TITLE));
    t.ornament_right(26, 0);
    t.cursor(23, 0);
    t.num(level, 2, ' ', crate::term::TEXT, false);
    t.cursor(1, 24);
}

/// Put the top border back after leaving the dungeon (or for Ztats).
pub fn restore_title(t: &mut Term) {
    t.fill(80, 0, 216, 8, crate::term::BLUE);
    t.line_u1(80, 7, 216, 7, WHITE);
}

/// Put the border under the view back (removes the compass).
pub fn restore_compass(t: &mut Term) {
    t.fill(80, 153, 216, 158, crate::term::BLUE);
    t.line_u1(80, 159, 216, 159, WHITE);
    t.line_u1(80, 152, 216, 152, WHITE);
}

/// Draw the whole view for the player at (pa,pb) facing `facing`.
pub fn draw(t: &mut Term, e: &Exe, v: &ViewTables, lv: &DungeonLevel, pa: i32, pb: i32, facing: u16) {
    let mut w = View { t, e, v, fg: WHITE };
    let (fa, fb) = forward(facing);
    let ((la, lb), (ra, rb)) = sides(facing);
    w.t.clear_view();
    compass(w.t, e, facing);
    let here = lv.cell(pa, pb);
    let door = here == Cell::Door;
    let in_wall = matches!(here, Cell::Wall | Cell::SecretDoor);
    let dmax = sight(lv, pa, pb, fa, fb);
    if in_wall {
        w.front_wall(0);
        return;
    }
    let (mut last_l, mut last_r) = (100u8, 100u8);
    let (mut prev_l, mut prev_r) = (100u8, 100u8);
    let mut skip_corridor = false;
    if door {
        let ahead = lv.cell(pa + fa, pb + fb);
        if matches!(ahead, Cell::Wall | Cell::Door | Cell::SecretDoor) {
            w.front_wall(0);
            skip_corridor = true;
        } else {
            w.front_door(0);
        }
    }
    if !skip_corridor {
        let mut d = dmax;
        let (mut oa, mut ob) = (fa * dmax, fb * dmax);
        let (mut cur_l, mut cur_r) = (100u8, 100u8);
        while d != 0 {
            oa -= fa;
            ob -= fb;
            if !(door && d <= 1) {
                let ls = lv.cell(pa + oa + la, pb + ob + lb);
                let rs = lv.cell(pa + oa + ra, pb + ob + rb);
                side_left(&mut w, d, ls);
                side_right(&mut w, d, rs);
                cur_l = solid(ls) as u8;
                cur_r = solid(rs) as u8;
                if d == dmax {
                    last_l = cur_l;
                    last_r = cur_r;
                } else {
                    if prev_l != cur_l {
                        w.left_vert(d);
                    }
                    if prev_r != cur_r {
                        w.right_vert(d);
                    }
                }
            }
            w.contents(lv, facing, d, pa + oa + fa, pb + ob + fb);
            prev_l = cur_l;
            prev_r = cur_r;
            d -= 1;
        }
        if !door {
            w.contents(lv, facing, 0, pa, pb);
        }
        let (ea, eb) = (pa + fa * dmax, pb + fb * dmax);
        if opaque_monster(lv, ea, eb) && lv.cell(ea, eb) != Cell::Door && dmax < 5 {
            // Look past the monster at the end of the corridor.
            let ls = lv.cell(pa + la + fa * dmax, pb + lb + fb * dmax);
            let rs = lv.cell(pa + ra + fa * dmax, pb + rb + fb * dmax);
            side_left(&mut w, dmax + 1, ls);
            side_right(&mut w, dmax + 1, rs);
            if !solid(ls) {
                w.left_vert(dmax);
            }
            if !solid(rs) {
                w.right_vert(dmax);
            }
        } else {
            if last_l == 1 {
                w.left_vert(dmax);
            }
            if last_r == 1 {
                w.right_vert(dmax);
            }
        }
    }
    if door {
        match item_at(lv, pa, pb) {
            Item::Coffin => w.coffin(1),
            Item::Chest => w.chest(1),
            Item::None => {}
        }
    }
}

fn side_left(w: &mut View, d: i32, s: Cell) {
    match s {
        Cell::Door => w.left_door(d),
        Cell::Wall | Cell::SecretDoor => w.left_wall(d),
        _ => w.left_open(d),
    }
}

fn side_right(w: &mut View, d: i32, s: Cell) {
    match s {
        Cell::Door => w.right_door(d),
        Cell::Wall | Cell::SecretDoor => w.right_wall(d),
        _ => w.right_open(d),
    }
}
