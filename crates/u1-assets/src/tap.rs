//! Tap actions (SPEC §24, an engine option): a tap on the game picture
//! becomes the same keys a player would type. The game describes what
//! is around the player at each command prompt ([`Scene`]); [`resolve`]
//! turns a tap at a point of the 320x200 picture into keys. Nothing
//! here changes a rule: the keys go through the normal command loop,
//! and their prompts and messages appear as usual.

use crate::io::keys;

/// What a tap on a map cell finds there.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Thing {
    /// Ground the player may try to walk on (the move itself decides).
    #[default]
    Open,
    /// A wall, a mountain, deep water and the like: a step that way is
    /// still tried when it is the only way (the game says why it fails).
    Blocked,
    /// A monster, or a townsperson once the town is hostile: Attack.
    Foe,
    /// A horse, cart, raft, frigate, shuttle or time machine: walk on, Board.
    Vehicle,
    /// A town, castle, dungeon entrance or landmark: walk on, Enter.
    Entrance,
    /// A townsperson or the king: Transact.
    Person,
}

/// What the player's own cell has, for a tap on it (or a long press).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Here {
    pub thing: Thing,
    /// In or on a vehicle (a long press gets out: X-it).
    pub riding: bool,
}

/// What the dungeon view shows for taps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Dungeon {
    /// A monster straight ahead, next to the player.
    pub foe_ahead: bool,
    /// A ladder underfoot (K-limb).
    pub ladder: bool,
    /// A coffin underfoot (Open) or a chest (Unlock).
    pub coffin: bool,
    pub chest: bool,
}

/// What is on screen at a command prompt.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Scene {
    /// Not at a command prompt (or a view taps do nothing in).
    #[default]
    None,
    /// The overworld or a town/castle: a grid of cells in the picture.
    Grid(Grid),
    /// The first-person dungeon view.
    Dungeon(Dungeon),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Grid {
    /// Top-left pixel of cell (0, 0) and the cell size.
    pub x0: i32,
    pub y0: i32,
    pub cw: i32,
    pub ch: i32,
    pub cols: i32,
    pub rows: i32,
    /// The player's cell.
    pub px: i32,
    pub py: i32,
    /// Row by row, `cols * rows` cells.
    pub cells: Vec<Thing>,
    pub here: Here,
    /// A town or castle (people are talked to with Transact there).
    pub town: bool,
}

impl Grid {
    pub fn at(&self, c: i32, r: i32) -> Thing {
        if (0..self.cols).contains(&c) && (0..self.rows).contains(&r) {
            self.cells[(r * self.cols + c) as usize]
        } else {
            Thing::Blocked
        }
    }

    /// The cell a picture point falls in.
    pub fn cell_of(&self, x: i32, y: i32) -> Option<(i32, i32)> {
        if x < self.x0 || y < self.y0 || self.cw <= 0 || self.ch <= 0 {
            return None;
        }
        let (c, r) = ((x - self.x0) / self.cw, (y - self.y0) / self.ch);
        ((0..self.cols).contains(&c) && (0..self.rows).contains(&r)).then_some((c, r))
    }

    /// The picture rectangle of a cell (for the tap highlight).
    pub fn rect(&self, c: i32, r: i32) -> (i32, i32, i32, i32) {
        (self.x0 + c * self.cw, self.y0 + r * self.ch, self.cw, self.ch)
    }
}

/// The dungeon view's rectangle on the 320x200 picture.
pub const DUNGEON_VIEW: (i32, i32, i32, i32) = (8, 8, 304, 144);

/// The arrow key for a step.
pub fn arrow(dx: i32, dy: i32) -> u8 {
    match (dx.signum(), dy.signum()) {
        (-1, _) => keys::LEFT,
        (1, _) => keys::RIGHT,
        (_, -1) => keys::UP,
        _ => keys::DOWN,
    }
}

/// What a tap does: keys to send now, and a key to send at the next
/// prompt if the player then stands on `then.0` (walk onto a horse, then
/// Board it, only once the step worked).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Action {
    pub keys: Vec<u8>,
    pub then: Option<(Thing, u8)>,
    /// The tapped cell, for the highlight.
    pub cell: Option<(i32, i32)>,
    /// A plain step on a known way to the tapped cell, with no foe next to
    /// the player: a held press may keep walking.
    pub walk: bool,
}

impl Action {
    fn keys(k: &[u8]) -> Self {
        Action { keys: k.to_vec(), ..Default::default() }
    }
}

/// The key that follows arriving on a thing: Board or Enter.
pub fn arrive_key(t: Thing) -> Option<u8> {
    match t {
        Thing::Vehicle => Some(b'B'),
        Thing::Entrance => Some(b'E'),
        _ => None,
    }
}

/// A tap at picture point (`x`, `y`); `long` for a long press.
pub fn resolve(s: &Scene, x: i32, y: i32, long: bool) -> Action {
    match s {
        Scene::None => Action::default(),
        Scene::Dungeon(d) => dungeon(d, x, y, long),
        Scene::Grid(g) => grid(g, x, y, long),
    }
}

fn dungeon(d: &Dungeon, x: i32, y: i32, long: bool) -> Action {
    let (vx, vy, vw, vh) = DUNGEON_VIEW;
    if x < vx || y < vy || x >= vx + vw || y >= vy + vh {
        return Action::default();
    }
    let context = if d.ladder {
        Some(b'K')
    } else if d.coffin {
        Some(b'O')
    } else if d.chest {
        Some(b'U')
    } else {
        None
    };
    let (third, band) = ((x - vx) * 3 / vw, (y - vy) * 3 / vh);
    if long {
        return context.map(|k| Action::keys(&[k])).unwrap_or_default();
    }
    match (third, band) {
        (0, _) => Action::keys(&[keys::LEFT]),
        (2, _) => Action::keys(&[keys::RIGHT]),
        (_, 2) => Action::keys(&[keys::DOWN]),
        // The middle of the corridor: what is underfoot, if anything.
        (_, 1) if context.is_some() => Action::keys(&[context.unwrap()]),
        _ if d.foe_ahead => Action::keys(b"A"),
        _ => Action::keys(&[keys::UP]),
    }
}

fn grid(g: &Grid, x: i32, y: i32, long: bool) -> Action {
    let Some((c, r)) = g.cell_of(x, y) else {
        // On the border just past the map, next to the player: a step that
        // way (out of a town, as with the arrow key).
        let (c, r) = ((x - g.x0).div_euclid(g.cw.max(1)), (y - g.y0).div_euclid(g.ch.max(1)));
        let (dx, dy) = (c - g.px, r - g.py);
        return if dx.abs() + dy.abs() == 1 { Action::keys(&[arrow(dx, dy)]) } else { Action::default() };
    };
    let (dx, dy) = (c - g.px, r - g.py);
    let mut a = if (dx, dy) == (0, 0) {
        own(g, long)
    } else if dx.abs() + dy.abs() == 1 {
        next_to(g, g.at(c, r), dx, dy)
    } else if (dx == 0 || dy == 0) && g.at(c, r) == Thing::Foe && clear_line(g, dx, dy) {
        // A foe in a straight line: a ranged weapon may reach (the game
        // decides, as with A and a direction).
        Action::keys(&[b'A', arrow(dx, dy)])
    } else {
        step_toward(g, dx, dy)
    };
    a.cell = Some((c, r));
    a
}

fn own(g: &Grid, long: bool) -> Action {
    if long {
        return if g.here.riding { Action::keys(b"X") } else { Action::default() };
    }
    match g.here.thing {
        Thing::Entrance => Action::keys(b"E"),
        Thing::Vehicle if !g.here.riding => Action::keys(b"B"),
        _ => Action::keys(b" "),
    }
}

fn next_to(g: &Grid, t: Thing, dx: i32, dy: i32) -> Action {
    let k = arrow(dx, dy);
    match t {
        Thing::Foe => Action::keys(&[b'A', k]),
        Thing::Person if g.town => Action::keys(b"T"),
        Thing::Vehicle if g.here.riding => Action::keys(&[k]),
        Thing::Vehicle | Thing::Entrance => Action { keys: vec![k], then: arrive_key(t).map(|b| (t, b)), cell: None, walk: false },
        _ => Action::keys(&[k]),
    }
}

/// Nothing but open ground between the player and (dx, dy) on one line.
fn clear_line(g: &Grid, dx: i32, dy: i32) -> bool {
    let n = dx.abs().max(dy.abs());
    (1..n).all(|k| matches!(g.at(g.px + dx.signum() * k, g.py + dy.signum() * k), Thing::Open | Thing::Entrance))
}

/// One step toward a far cell: along the longer way first, the other
/// way if that cell is blocked or taken.
fn step_toward(g: &Grid, dx: i32, dy: i32) -> Action {
    // The first step of a shortest way over the open cells seen (a
    // breadth-first search back from the target); with none, the free
    // neighbour nearest the target; with no free neighbour, nothing.
    let free = |c: i32, r: i32| matches!(g.at(c, r), Thing::Open | Thing::Entrance | Thing::Vehicle);
    let (tx, ty) = (g.px + dx, g.py + dy);
    let n = (g.cols * g.rows).max(0) as usize;
    let mut dist = vec![u32::MAX; n];
    let idx = |c: i32, r: i32| (r * g.cols + c) as usize;
    let dirs = [(-1, 0), (1, 0), (0, -1), (0, 1)];
    if (0..g.cols).contains(&tx) && (0..g.rows).contains(&ty) {
        let mut q = std::collections::VecDeque::from([(tx, ty)]);
        dist[idx(tx, ty)] = 0;
        while let Some((c, r)) = q.pop_front() {
            for (sx, sy) in dirs {
                let (nc, nr) = (c + sx, r + sy);
                if (nc, nr) == (g.px, g.py) || !free(nc, nr) || dist[idx(nc, nr)] != u32::MAX {
                    continue;
                }
                dist[idx(nc, nr)] = dist[idx(c, r)] + 1;
                q.push_back((nc, nr));
            }
        }
    }
    let score = |(sx, sy): (i32, i32)| {
        let (c, r) = (g.px + sx, g.py + sy);
        if (c, r) != (tx, ty) && !free(c, r) {
            return None;
        }
        let d = if (c, r) == (tx, ty) { 0 } else { dist[idx(c, r)] };
        let far = (tx - c).abs() + (ty - r).abs();
        // Ties: along the longer way first.
        let along = if (sx != 0) == (dx.abs() >= dy.abs()) { 0 } else { 1 };
        Some((d, far, along))
    };
    let foe_near = (-1..=1).any(|sy| (-1..=1).any(|sx| g.at(g.px + sx, g.py + sy) == Thing::Foe));
    match dirs.iter().filter_map(|&d| score(d).map(|s| (s, d))).min_by_key(|x| x.0) {
        Some((s, (sx, sy))) => Action { walk: s.0 != u32::MAX && !foe_near && g.at(g.px + sx, g.py + sy) == Thing::Open, ..Action::keys(&[arrow(sx, sy)]) },
        None => Action::default(),
    }
}

#[cfg(test)]
mod path_tests {
    use super::*;

    #[test]
    fn a_tap_on_the_border_next_to_the_player_steps_off() {
        let g = Grid { x0: 8, y0: 8, cw: 16, ch: 16, cols: 5, rows: 3, px: 0, py: 1, cells: vec![Thing::Open; 15], here: Here::default(), town: true };
        assert_eq!(resolve(&Scene::Grid(g.clone()), 2, 30, false).keys, vec![keys::LEFT]);
        assert!(resolve(&Scene::Grid(g), 2, 2, false).keys.is_empty());
    }

    #[test]
    fn a_far_tap_walks_around_a_wall() {
        // A wall right of the player with a gap below it.
        let (cols, rows) = (5, 3);
        let mut cells = vec![Thing::Open; 15];
        cells[2] = Thing::Blocked;
        cells[7] = Thing::Blocked;
        let g = Grid { x0: 0, y0: 0, cw: 16, ch: 16, cols, rows, px: 1, py: 1, cells, here: Here::default(), town: true };
        assert_eq!(step_toward(&g, 3, 0).keys, vec![arrow(0, 1)]);
        assert!(step_toward(&g, 3, 0).walk);
        // A foe next to the player stops a held walk.
        let mut f = g.clone();
        f.cells[0] = Thing::Foe;
        assert!(!step_toward(&f, 3, 0).walk);
        // Boxed in: no step at all.
        let mut cells = vec![Thing::Blocked; 15];
        cells[6] = Thing::Open;
        let g = Grid { cells, ..g };
        assert!(step_toward(&g, 3, 0).keys.is_empty());
    }
}

impl crate::game::Game {
    /// What a tap would find now (SPEC §24), for the command prompt.
    pub fn tap_scene(&self) -> Scene {
        use crate::game::{CASTLE, OUTDOORS, TOWN};
        match self.mode {
            OUTDOORS => Scene::Grid(self.tap_world()),
            TOWN | CASTLE => Scene::Grid(self.tap_town()),
            crate::dgame::DUNGEON if self.dungeon.is_some() => Scene::Dungeon(self.tap_dungeon()),
            _ => Scene::None,
        }
    }

    fn tap_world(&self) -> Grid {
        use crate::rules::{self, PLAYER_COL, PLAYER_ROW, VIEW_COLS, VIEW_ROWS};
        let tr = self.transport();
        let kind = |v: u8| -> Thing {
            if rules::is_monster_tile(v) {
                Thing::Foe
            } else if (0x0B..=0x12).contains(&v) {
                Thing::Vehicle
            } else if (4..=7).contains(&v) {
                Thing::Entrance
            } else if rules::check_move(v, tr).is_err() {
                Thing::Blocked
            } else {
                Thing::Open
            }
        };
        let mut cells = Vec::with_capacity((VIEW_COLS * VIEW_ROWS) as usize);
        for row in 0..VIEW_ROWS {
            for col in 0..VIEW_COLS {
                let (mx, my) = rules::view_cell(self.ow.x, self.ow.y, col, row);
                cells.push(kind(self.cell_or_object(mx, my)));
            }
        }
        let here = kind(self.cell_or_object(self.ow.x, self.ow.y));
        Grid {
            x0: 8,
            y0: 8,
            cw: self.a.world_tiles.width.max(1) as i32,
            ch: self.a.world_tiles.height.max(1) as i32,
            cols: VIEW_COLS,
            rows: VIEW_ROWS,
            px: PLAYER_COL,
            py: PLAYER_ROW,
            cells,
            here: Here { thing: if tr != 0 && here == Thing::Vehicle { Thing::Open } else { here }, riding: tr != 0 },
            town: false,
        }
    }

    fn tap_town(&self) -> Grid {
        use crate::tcd::{TCD_HEIGHT, TCD_WIDTH};
        let (w, h) = (TCD_WIDTH as i32, TCD_HEIGHT as i32);
        let mut cells = Vec::with_capacity((w * h) as usize);
        for y in 0..h {
            for x in 0..w {
                cells.push(if (x, y) == (self.px, self.py) {
                    Thing::Open
                } else {
                    match self.passable(x, y) {
                        crate::town::FREE | crate::town::SPECIAL => Thing::Open,
                        0 => Thing::Blocked,
                        _ if self.hostile => Thing::Foe,
                        _ => Thing::Person,
                    }
                });
            }
        }
        Grid {
            x0: 8,
            y0: 8,
            cw: self.a.town_tiles.width.max(1) as i32,
            ch: self.a.town_tiles.height.max(1) as i32,
            cols: w,
            rows: h,
            px: self.px,
            py: self.py,
            cells,
            here: Here::default(),
            town: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world() -> Grid {
        Grid { x0: 8, y0: 8, cw: 16, ch: 16, cols: 19, rows: 9, px: 9, py: 4, cells: vec![Thing::Open; 19 * 9], ..Default::default() }
    }
    fn set(g: &mut Grid, c: i32, r: i32, t: Thing) {
        g.cells[(r * g.cols + c) as usize] = t;
    }
    fn centre(g: &Grid, c: i32, r: i32) -> (i32, i32) {
        (g.x0 + c * g.cw + g.cw / 2, g.y0 + r * g.ch + g.ch / 2)
    }
    fn tap(g: &Grid, c: i32, r: i32) -> Action {
        let (x, y) = centre(g, c, r);
        resolve(&Scene::Grid(g.clone()), x, y, false)
    }

    #[test]
    fn picture_points_map_to_cells() {
        let g = world();
        assert_eq!(g.cell_of(8, 8), Some((0, 0)));
        assert_eq!(g.cell_of(23, 23), Some((0, 0)));
        assert_eq!(g.cell_of(24, 8), Some((1, 0)));
        assert_eq!(g.cell_of(8 + 9 * 16 + 3, 8 + 4 * 16 + 15), Some((9, 4)));
        assert_eq!(g.cell_of(7, 30), None, "the border");
        assert_eq!(g.cell_of(8 + 19 * 16, 30), None, "right of the view");
        assert_eq!(g.cell_of(30, 8 + 9 * 16), None, "the message area");
        let t = Grid { cw: 8, ch: 8, cols: 38, rows: 18, ..world() };
        assert_eq!(t.cell_of(8 + 37 * 8, 8 + 17 * 8 + 7), Some((37, 17)));
    }

    #[test]
    fn next_cells_move_attack_board_enter_talk() {
        let mut g = world();
        assert_eq!(tap(&g, 10, 4).keys, [keys::RIGHT]);
        assert_eq!(tap(&g, 9, 3).keys, [keys::UP]);
        set(&mut g, 8, 4, Thing::Foe);
        assert_eq!(tap(&g, 8, 4).keys, [b'A', keys::LEFT]);
        set(&mut g, 9, 5, Thing::Vehicle);
        let a = tap(&g, 9, 5);
        assert_eq!((a.keys, a.then), (vec![keys::DOWN], Some((Thing::Vehicle, b'B'))));
        set(&mut g, 10, 4, Thing::Entrance);
        assert_eq!(tap(&g, 10, 4).then, Some((Thing::Entrance, b'E')));
        set(&mut g, 9, 3, Thing::Person);
        assert_eq!(tap(&g, 9, 3).keys, [keys::UP], "outdoors nobody to talk to");
        g.town = true;
        assert_eq!(tap(&g, 9, 3).keys, [b'T']);
    }

    #[test]
    fn own_cell_and_long_press() {
        let mut g = world();
        assert_eq!(tap(&g, 9, 4).keys, [b' '], "Pass");
        g.here.thing = Thing::Entrance;
        assert_eq!(tap(&g, 9, 4).keys, [b'E']);
        g.here = Here { thing: Thing::Vehicle, riding: false };
        assert_eq!(tap(&g, 9, 4).keys, [b'B']);
        g.here.riding = true;
        assert_eq!(tap(&g, 9, 4).keys, [b' ']);
        let (x, y) = centre(&g, 9, 4);
        assert_eq!(resolve(&Scene::Grid(g.clone()), x, y, true).keys, [b'X']);
        g.here.riding = false;
        assert!(resolve(&Scene::Grid(g), x, y, true).keys.is_empty());
    }

    #[test]
    fn far_taps_step_or_shoot() {
        let mut g = world();
        assert_eq!(tap(&g, 15, 5).keys, [keys::RIGHT], "the longer way first");
        assert_eq!(tap(&g, 10, 0).keys, [keys::UP]);
        assert_eq!(tap(&g, 10, 5).keys, [keys::RIGHT], "a diagonal neighbour: one step");
        set(&mut g, 10, 4, Thing::Blocked);
        assert_eq!(tap(&g, 15, 5).keys, [keys::DOWN], "around a mountain");
        set(&mut g, 9, 1, Thing::Foe);
        assert_eq!(tap(&g, 9, 1).keys, [b'A', keys::UP]);
        set(&mut g, 9, 2, Thing::Blocked);
        assert_eq!(tap(&g, 9, 1).keys, [keys::UP], "no clear line: walk");
    }

    #[test]
    fn dungeon_regions() {
        let (vx, vy, vw, vh) = DUNGEON_VIEW;
        let d = Dungeon::default();
        let t = |d: &Dungeon, fx: i32, fy: i32, long| resolve(&Scene::Dungeon(*d), vx + vw * fx / 6, vy + vh * fy / 6, long).keys;
        assert_eq!(t(&d, 3, 1, false), [keys::UP]);
        assert_eq!(t(&d, 1, 3, false), [keys::LEFT]);
        assert_eq!(t(&d, 5, 3, false), [keys::RIGHT]);
        assert_eq!(t(&d, 3, 5, false), [keys::DOWN]);
        assert_eq!(t(&d, 3, 3, false), [keys::UP]);
        let foe = Dungeon { foe_ahead: true, ..d };
        assert_eq!(t(&foe, 3, 1, false), [b'A']);
        let lad = Dungeon { ladder: true, ..d };
        assert_eq!(t(&lad, 3, 3, false), [b'K']);
        assert_eq!(t(&lad, 3, 1, false), [keys::UP]);
        assert_eq!(t(&lad, 1, 1, true), [b'K']);
        assert_eq!(t(&Dungeon { coffin: true, ..d }, 3, 3, false), [b'O']);
        assert_eq!(t(&Dungeon { chest: true, ..d }, 3, 3, false), [b'U']);
        assert!(resolve(&Scene::Dungeon(d), 2, 2, false).keys.is_empty(), "outside the view");
        assert!(resolve(&Scene::None, 100, 50, false).keys.is_empty());
    }
}
