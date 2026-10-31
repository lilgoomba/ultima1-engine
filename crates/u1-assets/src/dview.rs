//! The first-person dungeon view (SPEC §6.6), written from our reading of
//! the original's drawing order ("verified from original executable").
//!
//! Every coordinate table and monster/object outline comes from the
//! user's OUT.EXE at run time; this file only holds the drawing rules.

use crate::dungeon::{Cell, DungeonLevel, Item};
use crate::exe::{m, Exe};
use crate::term::{dun, ArtId, Term, BLACK, WHITE};

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
    /// The vertices of shape `id` at depth `d`, in screen coordinates,
    /// scaled exactly as `View::shape` draws them.
    pub fn shape_points(&self, id: usize, d: i32) -> Vec<(i32, i32)> {
        let mut cx = d - 1;
        if !(0..=4).contains(&cx) {
            cx = 4;
        }
        let base = self.base[cx as usize];
        let shift = if cx == 4 { 5 } else { cx as u32 };
        let sc = |b: u8| -> i32 {
            let w = (b as i8 as i16 as u16) >> shift;
            (w as u8) as i8 as i32
        };
        let Some(bytes) = self.shapes.get(id) else { return Vec::new() };
        let mut pts = Vec::new();
        let mut i = 0;
        while i < bytes.len() {
            let b = bytes[i];
            i += 1;
            if b == 0x7f {
                break;
            }
            let (bx, by) = if b == 0x7e {
                if i + 1 >= bytes.len() {
                    break;
                }
                i += 2;
                (bytes[i - 2], bytes[i - 1])
            } else {
                if i >= bytes.len() {
                    break;
                }
                i += 1;
                (b, bytes[i - 1])
            };
            pts.push((sc(bx) + 160, sc(by) + base));
        }
        pts
    }

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

/// A surface of the view between two vertical edges (screen pixels):
/// the top edge runs from `top.0` at `x0` to `top.1` at `x1`, the bottom
/// edge likewise. `plane` is the depth of each edge (0 = the screen).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DFace {
    pub id: ArtId,
    pub x0: i32,
    pub x1: i32,
    pub top: (i32, i32),
    pub bot: (i32, i32),
    pub plane: (i32, i32),
}

/// An object (monster, chest, ladder ...) and the box its original
/// outline covers, at depth `plane`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DObj {
    pub id: ArtId,
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
    pub plane: i32,
    /// For a hole in the floor or ceiling: the corners of the original's
    /// four-sided outline (top-left, top-right, bottom-right,
    /// bottom-left), whose top and bottom edges are level.
    pub quad: Option<[(i32, i32); 4]>,
}

/// One piece of the view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DPiece {
    Face(DFace),
    Obj(DObj),
}

/// The view as the enhanced renderer redraws it: the pieces with a
/// painter's key (larger is farther, drawn first) and the corridor
/// tables, which place the floor and ceiling cells. The geometry is the
/// original's; only the filling differs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DScene {
    pub pieces: Vec<(i32, DPiece)>,
    pub x: [i32; 6],
    pub y: [i32; 6],
}

impl DScene {
    /// The pieces, farthest first (stable for equal keys).
    pub fn ordered(&self) -> Vec<&DPiece> {
        let mut v: Vec<&(i32, DPiece)> = self.pieces.iter().collect();
        v.sort_by(|a, b| b.0.cmp(&a.0));
        v.into_iter().map(|p| &p.1).collect()
    }
}

/// Screen box of the viewport's corridor geometry: x 16..=303, y 8..=151.
pub const GEOM: (i32, i32, i32, i32) = (CX, 8, RX, BY);

/// The view renderer; `t` is the screen, names go to the message window.
pub struct View<'a> {
    pub t: &'a mut Term,
    pub e: &'a Exe,
    pub v: &'a ViewTables,
    pub fg: u8,
    /// What was drawn, for the enhanced renderer.
    pub scene: DScene,
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
    pub fn shape(&mut self, id: usize, d: i32, c: u8) -> Option<(i32, i32, i32, i32)> {
        let mut cx = d - 1;
        if !(0..=4).contains(&cx) {
            cx = 4;
        }
        let base = self.v.base[cx as usize];
        let shift = if cx == 4 { 5 } else { cx as u32 };
        let Some(bytes) = self.v.shapes.get(id).cloned() else { return None };
        let mut ext: Option<(i32, i32, i32, i32)> = None;
        let mut grow = |x: i32, y: i32| {
            ext = Some(match ext {
                None => (x, y, x, y),
                Some((a, b, c, d)) => (a.min(x), b.min(y), c.max(x), d.max(y)),
            });
        };
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
                grow(x, y);
            } else {
                if i >= bytes.len() {
                    break;
                }
                let (x, y) = (sc(b) + 160, sc(bytes[i]) + base);
                i += 1;
                self.t.lineto(x, y, c);
                grow(x, y);
            }
        }
        ext
    }

    // --- scene notes for the enhanced renderer ---

    fn face(&mut self, key: i32, f: DFace) {
        self.scene.pieces.push((key, DPiece::Face(f)));
    }

    fn obj(&mut self, key: i32, id: ArtId, ext: Option<(i32, i32, i32, i32)>, plane: i32) {
        if let Some((x0, y0, x1, y1)) = ext {
            self.scene.pieces.push((key, DPiece::Obj(DObj { id, x0, y0, x1, y1, plane, quad: None })));
        }
    }

    /// A hole in the floor or ceiling (shape 0x1a up, 0x1b down) at
    /// depth `d`, noted with its four-sided outline.
    fn hole(&mut self, key: i32, up: bool, d: i32) {
        let h = self.shape(if up { 0x1a } else { 0x1b }, d, self.fg);
        let Some((x0, y0, x1, y1)) = h else { return };
        let q = self.v.shape_points(if up { 0x1a } else { 0x1b }, d);
        let edge = |y: i32| {
            let xs = q.iter().filter(|p| p.1 == y).map(|p| p.0);
            (xs.clone().min().unwrap_or(x0), xs.max().unwrap_or(x1))
        };
        let ((a, b), (c, e)) = (edge(y0), edge(y1));
        let id = ArtId::Dun(if up { dun::HOLE_UP } else { dun::HOLE_DOWN });
        let quad = Some([(a, y0), (b, y0), (e, y1), (c, y1)]);
        self.scene.pieces.push((key, DPiece::Obj(DObj { id, x0, y0, x1, y1, plane: d, quad })));
    }

    /// The surface of a wall (or door, field) facing the player at depth `d`.
    fn note_front(&mut self, id: u16, d: i32, key: i32) {
        let d = d.min(5);
        let o = if d == 0 { 8 } else { 0 };
        let (x, y) = (self.x(d), self.y(d));
        self.face(key, DFace { id: ArtId::Dun(id), x0: x + CX + o, x1: RX - x - o, top: (y + 8 + o, y + 8 + o), bot: (BY - y - o, BY - y - o), plane: (d, d) });
    }

    /// A wall cell straight ahead at depth `d` (plain or secret).
    fn note_wall(&mut self, secret: bool, d: i32) {
        self.note_front(if secret { dun::SECRET } else { dun::WALL }, d, 10 * d);
    }

    /// A side wall (cell `d-1` to the left or right).
    fn note_side(&mut self, right: bool, id: u16, d: i32) {
        if d > 5 {
            return;
        }
        let (xa, ya, xb, yb) = (self.x(d - 1), self.y(d - 1), self.x(d), self.y(d));
        let f = if right {
            DFace { id: ArtId::Dun(id), x0: RX - xb, x1: RX - xa, top: (yb + 8, ya + 8), bot: (BY - yb, BY - ya), plane: (d, d - 1) }
        } else {
            DFace { id: ArtId::Dun(id), x0: xa + CX, x1: xb + CX, top: (ya + 8, yb + 8), bot: (BY - ya, BY - yb), plane: (d - 1, d) }
        };
        self.face(10 * (d - 1) + 5, f);
    }

    /// The far wall of a side passage seen through an opening.
    fn note_opening(&mut self, right: bool, d: i32) {
        if d > 5 {
            return;
        }
        let (xa, xb, y) = (self.x(d - 1), self.x(d), self.y(d));
        let (x0, x1) = if right { (RX - xb, RX - xa) } else { (xa + CX, xb + CX) };
        self.face(10 * d + 1, DFace { id: ArtId::Dun(dun::WALL), x0, x1, top: (y + 8, y + 8), bot: (BY - y, BY - y), plane: (d, d) });
    }

    /// The door in a side wall at depth `d` (same outline as the lines).
    fn note_side_door(&mut self, right: bool, d: i32) {
        if d > 5 {
            return;
        }
        let (p, q) = self.door_steps(d);
        let near = if d == 1 { 2 } else { 0 };
        let xa = self.x(d - 1) + 2 * p + CX;
        let xb = 6 * p + CX + self.x(d - 1);
        let (ta, tb) = (self.y(d) + 8 - q, self.y(d) + 8 + q);
        let (ba, bb) = (BY - q - self.y(d - 1) - 1, BY - self.y(d) + 2 * q - near);
        let pa = d - 1;
        let f = if right {
            DFace { id: ArtId::Dun(dun::DOOR), x0: RX - (xb - CX), x1: RX - (xa - CX), top: (tb, ta), bot: (bb, ba), plane: (d, pa) }
        } else {
            DFace { id: ArtId::Dun(dun::DOOR), x0: xa, x1: xb, top: (ta, tb), bot: (ba, bb), plane: (pa, d) }
        };
        self.face(10 * (d - 1) + 4, f);
    }

    /// A door straight ahead: the wall, its doorway, or (`d == 0`) the
    /// frame of the doorway the player stands in.
    fn note_front_door(&mut self, d: i32) {
        if d >= 5 {
            return;
        }
        let (xn, yd, yn) = (self.x(d + 1), self.y(d), self.y(d + 1));
        if d == 0 {
            let (x0, y0) = (self.x(0), self.y(0));
            let (l, r, t, b) = (x0 + CX + 8, RX - x0 - 8, y0 + 16, BY - y0 - 8);
            let w = ArtId::Dun(dun::WALL);
            for (a, c, tt) in [(l, xn + CX, t), (RX - xn, r, t)] {
                self.face(-100, DFace { id: w, x0: a, x1: c, top: (tt, tt), bot: (b, b), plane: (0, 0) });
            }
            self.face(-100, DFace { id: w, x0: xn + CX, x1: RX - xn, top: (t, t), bot: (yn + 8, yn + 8), plane: (0, 0) });
            return;
        }
        self.note_front(dun::WALL, d, 10 * d);
        self.face(10 * d - 1, DFace { id: ArtId::Dun(dun::DOOR), x0: xn + CX, x1: RX - xn, top: (yn + 8, yn + 8), bot: (BY - yd, BY - yd), plane: (d, d) });
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
        self.hole(10 * (d - 1) - 1, up, d);
        let l = self.shape(if ns { 0x1c } else { 0x1d }, d, self.fg);
        self.obj(10 * (d - 1) - 2, ArtId::Dun(if up { dun::LADDER_UP } else { dun::LADDER_DOWN }), l, d);
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
            let (x, y) = (self.x(d), self.y(d));
            self.obj(10 * d - 3, ArtId::DMon(9), Some((x + CX, y + 8, RX - x, BY - y)), d);
        } else {
            let e = self.shape(id as usize, d, self.fg);
            self.obj(10 * d - 3, ArtId::DMon(id as u16), e, d);
        }
    }

    fn coffin(&mut self, d: i32) {
        let s = self.e.s(m::COFFIN);
        self.t.println(&s);
        let e = self.shape(0x19, d, self.fg);
        self.obj(10 * (d - 1) - 3, ArtId::Dun(dun::COFFIN), e, d);
    }

    fn chest(&mut self, d: i32) {
        let s = self.e.s(m::CHEST);
        self.t.println(&s);
        let e = self.shape(0x0b, d, self.fg);
        self.obj(10 * (d - 1) - 3, ArtId::Dun(dun::CHEST), e, d);
    }

    /// What stands in cell (a,b), seen at depth `d`.
    fn contents(&mut self, lv: &DungeonLevel, facing: u16, d: i32, a: i32, b: i32) {
        let id = lv.monster(a, b);
        let s = lv.cell(a, b);
        if id != -1 {
            if matches!(s, Cell::Wall | Cell::SecretDoor | Cell::Door) {
                self.front_wall(d);
                self.note_wall(s == Cell::SecretDoor, d);
            }
            // With a monster on it a ladder is drawn without its rungs.
            if s == Cell::LadderDown {
                self.hole(10 * d - 1, false, d + 1);
            }
            if s == Cell::LadderUp {
                self.hole(10 * d - 1, true, d + 1);
            }
            self.monster(d, id);
            return;
        }
        let ns = facing == NORTH || facing == SOUTH;
        match s {
            Cell::Wall | Cell::SecretDoor => {
                self.front_wall(d);
                self.note_wall(s == Cell::SecretDoor, d);
            }
            Cell::Door => {
                self.front_door(d);
                self.note_front_door(d);
            }
            Cell::LadderDown => self.ladder(false, ns, d + 1),
            Cell::LadderUp => self.ladder(true, ns, d + 1),
            Cell::ForceField => {
                self.field(d);
                if d <= 5 {
                    self.note_front(dun::FIELD, d, 10 * d - 2);
                }
            }
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

/// The squares the view at (pa, pb) facing `facing` shows: the corridor
/// ahead up to where the sight stops, the squares beside it, and the
/// square that stops it (for the automap, SPEC §23.1).
pub fn visible(lv: &DungeonLevel, pa: i32, pb: i32, facing: u16) -> Vec<(i32, i32)> {
    let (fa, fb) = forward(facing);
    let ((la, lb), (ra, rb)) = sides(facing);
    let mut out = vec![(pa, pb)];
    let here = lv.cell(pa, pb);
    if matches!(here, Cell::Wall | Cell::SecretDoor) {
        return out;
    }
    if here == Cell::Door && matches!(lv.cell(pa + fa, pb + fb), Cell::Wall | Cell::Door | Cell::SecretDoor) {
        out.push((pa + fa, pb + fb));
        return out;
    }
    let dmax = sight(lv, pa, pb, fa, fb);
    for k in 0..dmax {
        let (a, b) = (pa + fa * k, pb + fb * k);
        out.push((a, b));
        out.push((a + la, b + lb));
        out.push((a + ra, b + rb));
    }
    out.push((pa + fa * dmax, pb + fb * dmax));
    out.dedup();
    out
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
    let mut w = View { t, e, v, fg: WHITE, scene: DScene { pieces: Vec::new(), x: v.x, y: v.y } };
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
        w.note_wall(here == Cell::SecretDoor, 0);
        finish(w);
        return;
    }
    let (mut last_l, mut last_r) = (100u8, 100u8);
    let (mut prev_l, mut prev_r) = (100u8, 100u8);
    let mut skip_corridor = false;
    if door {
        let ahead = lv.cell(pa + fa, pb + fb);
        if matches!(ahead, Cell::Wall | Cell::Door | Cell::SecretDoor) {
            w.front_wall(0);
            w.note_wall(ahead == Cell::SecretDoor, 0);
            skip_corridor = true;
        } else {
            w.front_door(0);
            w.note_front_door(0);
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
    finish(w);
}

/// Keep the scene and note the view region for the enhanced renderer.
fn finish(w: View) {
    let View { t, scene, .. } = w;
    t.dscene = Some(Box::new(scene));
    let (x0, y0, x1, y1) = crate::term::VIEW;
    t.mark(ArtId::Screen("dungeon"), None, 0, x0, y0, x1 - x0 + 1, y1 - y0 + 1);
}

fn side_left(w: &mut View, d: i32, s: Cell) {
    side(w, false, d, s);
}

fn side_right(w: &mut View, d: i32, s: Cell) {
    side(w, true, d, s);
}

fn side(w: &mut View, right: bool, d: i32, s: Cell) {
    match (s, right) {
        (Cell::Door, false) => w.left_door(d),
        (Cell::Door, true) => w.right_door(d),
        (Cell::Wall | Cell::SecretDoor, false) => w.left_wall(d),
        (Cell::Wall | Cell::SecretDoor, true) => w.right_wall(d),
        (_, false) => w.left_open(d),
        (_, true) => w.right_open(d),
    }
    match s {
        Cell::Door => {
            w.note_side(right, dun::WALL, d);
            w.note_side_door(right, d);
        }
        Cell::Wall => w.note_side(right, dun::WALL, d),
        Cell::SecretDoor => w.note_side(right, dun::SECRET, d),
        _ => w.note_opening(right, d),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visible_squares() {
        let mut lv = DungeonLevel { level: 1, cells: [[Cell::Wall; 11]; 11], items: [[Item::None; 11]; 11], monsters: [[-1; 11]; 11], hp: [[0; 11]; 11], used: [false; 5] };
        // A corridor running south from (1,1) to (1,4), a wall at (1,5).
        for b in 1..=4 {
            lv.cells[1][b] = Cell::Empty;
        }
        let v = visible(&lv, 1, 1, SOUTH);
        assert!(v.contains(&(1, 4)) && v.contains(&(1, 5)), "{v:?}");
        assert!(v.contains(&(0, 3)) && v.contains(&(2, 3)), "the sides");
        assert!(!v.contains(&(1, 6)), "nothing past the wall");
        let v = visible(&lv, 1, 1, NORTH);
        assert_eq!(v, [(1, 1), (0, 1), (2, 1), (1, 0)], "facing the wall: here, beside and it");
    }
}
