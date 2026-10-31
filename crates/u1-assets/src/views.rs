//! The engine's own optional screens (SPEC §23): the dungeon automap, the
//! overworld mini-map, the character screen and the command help, plus the
//! prompt hints and the "are you sure" box. None of them is the original's;
//! each is off unless chosen in the options menu, and each leaves the game
//! exactly where it was (the screen is put back as it was, nothing in the
//! record changes, no time passes).

use crate::dungeon::DUNGEON_SIZE;
use crate::exe::m;
use crate::explore::Seen;
use crate::game::{slot, Game, OUTDOORS};
use crate::io::{keys, Io, IoResult};
use crate::locations::{kind_of, LocationKind};
use crate::map::{WORLD_HEIGHT, WORLD_WIDTH};
use crate::save::{off, QuestState};
use crate::term::{BLACK, BLUE, TEXT, WARN, WHITE};

const GREY: u8 = 7;
const DARK: u8 = 8;
const YELLOW: u8 = 14;
const GREEN: u8 = 2;
const LGREEN: u8 = 10;
const BROWN: u8 = 6;
const MAGENTA: u8 = 13;

/// Which screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Map,
    Help,
    Character,
}

impl View {
    /// The engine key that opens (and closes) this screen.
    pub fn key(self) -> u8 {
        match self {
            View::Map => keys::MAP,
            View::Help => keys::HELP,
            View::Character => keys::CHARSHEET,
        }
    }

    pub fn of_key(k: u8) -> Option<View> {
        match k {
            keys::MAP => Some(View::Map),
            keys::HELP => Some(View::Help),
            keys::CHARSHEET => Some(View::Character),
            _ => None,
        }
    }
}

/// One line of a hint shown when a command asks for more (SPEC §23.4).
pub fn hint(cmd: u8, mode: u8) -> Option<&'static str> {
    let town = mode == crate::game::TOWN || mode == crate::game::CASTLE;
    Some(match cmd {
        b'A' if mode != crate::dgame::DUNGEON => "(arrow: which way)",
        b'F' => "(arrow: which way)",
        b'D' if town => "(P coin, W weapon, A armour)",
        b'R' => "(W weapon, A armour, S spell)",
        b'T' if town => "(B buy, S sell)",
        _ => return None,
    })
}

/// Text centred on a 40-column row.
fn centre(t: &mut crate::term::Term, row: usize, s: &str, attr: u8) {
    let s: String = s.chars().take(38).collect();
    t.print_at_attr((40 - s.len()) / 2, row, &s, attr);
}

impl Game {
    /// Colour `ega` on a 16-colour screen, `cga` (0..=3 as drawn) on CGA.
    fn vc(&self, ega: u8, cga: u8) -> u8 {
        if self.term.cga {
            // `Term::dc` keeps the low two bits but sends BLUE to 2 and
            // WARN to 1; pass values that land where asked.
            [BLACK, 13, 2, 3][cga as usize & 3]
        } else {
            ega
        }
    }

    /// A blank full screen in a blue frame with a title.
    fn view_frame(&mut self, title: &str, foot: &str) {
        self.term.backdrop = None;
        self.term.fill(0, 0, 319, 199, BLACK);
        let b = self.vc(BLUE, 2);
        for i in 0..2 {
            self.term.line(2 + i, 2 + i, 317 - i, 2 + i, b);
            self.term.line(2 + i, 197 - i, 317 - i, 197 - i, b);
            self.term.line(2 + i, 2 + i, 2 + i, 197 - i, b);
            self.term.line(317 - i, 2 + i, 317 - i, 197 - i, b);
        }
        let w = self.vc(WHITE, 3);
        centre(&mut self.term, 1, title, w);
        let t = self.vc(TEXT, 1);
        centre(&mut self.term, 23, foot, t);
    }

    /// Note what is on the screen now in the explored map (each turn, while
    /// the automap or the mini-map is on).
    pub fn note_explored(&mut self) {
        if let Some(d) = self.dungeon.as_ref().filter(|_| self.mode == crate::dgame::DUNGEON) {
            let cells = crate::dview::visible(&d.level, d.a, d.b, d.facing);
            let key = (self.ow.x as u8, self.ow.y as u8, d.depth);
            self.explore.see_level(key, &d.level, &cells);
        } else if self.mode == OUTDOORS {
            self.explore.see_view(self.ow.x, self.ow.y);
        }
    }

    /// Show one of the engine's screens over the game and wait for a key;
    /// the screen goes back exactly as it was. A script that ends while a
    /// screen is up leaves it showing (for screenshots).
    pub fn show_view(&mut self, io: &mut dyn Io, v: View) -> IoResult<()> {
        let saved = self.term.clone();
        match v {
            View::Map if self.mode == crate::dgame::DUNGEON => self.draw_automap(),
            View::Map => return self.show_minimap(io, saved),
            View::Help => self.draw_help(),
            View::Character => self.draw_character(),
        }
        io.view_open(Some(v.key()));
        let r = io.key(&self.term);
        io.view_open(None);
        r?;
        self.term = saved;
        Ok(())
    }

    // ------------------------------------------------------------ automap

    pub fn draw_automap(&mut self) {
        let Some(d) = self.dungeon.as_ref() else { return };
        let (pa, pb, facing, depth) = (d.a, d.b, d.facing, d.depth);
        let key = (self.ow.x as u8, self.ow.y as u8, depth);
        let grid = self.explore.level(key).copied();
        self.view_frame(&format!("Automap - dungeon level {depth}"), "Tab or any key: back");
        const S: i32 = 14;
        let (x0, y0) = (16, 24);
        let (wall, floor, door, up, down, field, you, frame) =
            (self.vc(GREY, 3), self.vc(DARK, 2), self.vc(BROWN, 1), self.vc(YELLOW, 3), self.vc(LGREEN, 1), self.vc(MAGENTA, 3), self.vc(WARN, 3), self.vc(BLUE, 2));
        let n = DUNGEON_SIZE as i32;
        self.term.line(x0 - 2, y0 - 2, x0 + n * S + 1, y0 - 2, frame);
        self.term.line(x0 - 2, y0 + n * S + 1, x0 + n * S + 1, y0 + n * S + 1, frame);
        self.term.line(x0 - 2, y0 - 2, x0 - 2, y0 + n * S + 1, frame);
        self.term.line(x0 + n * S + 1, y0 - 2, x0 + n * S + 1, y0 + n * S + 1, frame);
        if let Some(g) = grid {
            for a in 0..n {
                for b in 0..n {
                    let (x, y) = (x0 + a * S, y0 + b * S);
                    let (xu, yu) = (x as usize, y as usize);
                    let su = S as usize;
                    let s = g[a as usize][b as usize];
                    match s {
                        Seen::Unseen => {}
                        Seen::Wall => self.term.fill(xu, yu, xu + su - 1, yu + su - 1, wall),
                        _ => self.term.fill(xu + 1, yu + 1, xu + su - 2, yu + su - 2, floor),
                    }
                    let cx = x + S / 2;
                    match s {
                        Seen::Door => {
                            // A door: a bar across the square both ways a
                            // corridor could pass it.
                            self.term.fill(xu + 4, yu + 2, xu + su - 5, yu + su - 3, door);
                            self.term.fill(xu + 6, yu + 6, xu + 7, yu + 7, BLACK);
                        }
                        Seen::LadderUp | Seen::LadderDown => {
                            let c = if s == Seen::LadderUp { up } else { down };
                            self.term.line(cx - 3, y + 2, cx - 3, y + S - 3, c);
                            self.term.line(cx + 2, y + 2, cx + 2, y + S - 3, c);
                            for r in [y + 4, y + 7, y + 10] {
                                self.term.line(cx - 3, r, cx + 2, r, c);
                            }
                            let tip = if s == Seen::LadderUp { y + 1 } else { y + S - 2 };
                            self.term.plot(cx - 1, tip, c);
                            self.term.plot(cx, tip, c);
                        }
                        Seen::Field => self.field_glyph(x, y, S, field),
                        _ => {}
                    }
                }
            }
        }
        // The player: an arrow pointing the way they face.
        let (cx, cy) = (x0 + pa * S + S / 2 - 1, y0 + pb * S + S / 2 - 1);
        let (fa, fb) = crate::dview::forward(facing);
        for k in -4..=4 {
            self.term.plot(cx + fa * k, cy + fb * k, you);
            self.term.plot(cx + fa * k + fb.abs(), cy + fb * k + fa.abs(), you);
        }
        for k in 1..=4 {
            // Arrow head: two strokes back from the tip.
            let (tx, ty) = (cx + fa * 4, cy + fb * 4);
            self.term.plot(tx - fa * k + fb * k, ty - fb * k + fa * k, you);
            self.term.plot(tx - fa * k - fb * k, ty - fb * k - fa * k, you);
        }
        // Legend on the right.
        let t = self.vc(TEXT, 1);
        let col = 22usize;
        let mut row = 4usize;
        // The direction in the original's own words, as its compass under
        // the dungeon view shows it.
        let face = self.s(m::COMPASS[crate::dview::compass_index(facing)]);
        let w = self.vc(WHITE, 3);
        self.term.print_at_attr(col, row, &format!("Facing {}", face.trim()), w);
        row += 2;
        for (label, c, kind) in [("Wall", wall, 0), ("Corridor", floor, 1), ("Door", door, 2), ("Ladder up", up, 3), ("Ladder down", down, 4), ("Force field", field, 5), ("You", you, 6)] {
            let (x, y) = (col * 8, row * 8);
            match kind {
                5 => self.field_glyph(x as i32, y as i32 - 3, 12, c),
                0..=2 => self.term.fill(x, y, x + 9, y + 6, c),
                3 | 4 => {
                    self.term.line(x as i32 + 2, y as i32, x as i32 + 2, y as i32 + 6, c);
                    self.term.line(x as i32 + 7, y as i32, x as i32 + 7, y as i32 + 6, c);
                    self.term.line(x as i32 + 2, y as i32 + 3, x as i32 + 7, y as i32 + 3, c);
                }
                _ => self.term.fill(x + 3, y + 1, x + 6, y + 5, c),
            }
            self.term.print_at_attr(col + 2, row, label, t);
            row += 2;
        }
        let seen = grid.map_or(0, |g| g.iter().flatten().filter(|s| **s != Seen::Unseen).count());
        self.term.print_at_attr(col, row + 1, &format!("Seen {seen} of {}", n * n), t);
    }

    /// A force field on the automap, a `size` square at (x, y): stripes
    /// over the corridor colour on 16 colours; on CGA, where the
    /// corridor and the field share a colour, a lattice of crossing
    /// diagonals on black instead, so the two differ in shape as well.
    fn field_glyph(&mut self, x: i32, y: i32, size: i32, c: u8) {
        if self.term.cga {
            let (xu, yu, su) = (x as usize, y as usize, size as usize);
            self.term.fill(xu + 1, yu + 1, xu + su - 2, yu + su - 2, BLACK);
            for i in 1..size - 1 {
                for j in 1..size - 1 {
                    if (i + j) % 4 == 0 || (i - j).rem_euclid(4) == 0 {
                        self.term.plot(x + i, y + j, c);
                    }
                }
            }
        } else {
            for k in (2..size - 2).step_by(3) {
                self.term.line(x + 2, y + k, x + size - 3, y + k, c);
            }
        }
    }

    // ------------------------------------------------------------ mini-map

    /// The mini-map opens zoomed in round the player; Z switches between
    /// that and the whole world, any other key goes back.
    fn show_minimap(&mut self, io: &mut dyn Io, saved: crate::term::Term) -> IoResult<()> {
        let mut whole = false;
        io.view_open(Some(keys::MAP));
        loop {
            self.draw_minimap(whole);
            let k = io.key(&self.term);
            if k.is_err() {
                io.view_open(None);
            }
            match k? {
                b'z' | b'Z' => whole = !whole,
                _ => break,
            }
            self.term = saved.clone();
        }
        io.view_open(None);
        self.term = saved;
        Ok(())
    }

    /// Cells drawn per side when zoomed in, and the size of each.
    pub const MINIMAP_ZOOM: usize = 4;

    pub fn draw_minimap(&mut self, whole: bool) {
        let foot = if whole { "Z zoom in - other keys: back" } else { "Z whole map - other keys: back" };
        self.view_frame("Map of the lands you have seen", foot);
        if !whole {
            return self.draw_minimap_zoomed();
        }
        let (x0, y0) = (12usize, 22usize);
        let colours = [self.vc(BLUE, 2), self.vc(LGREEN, 1), self.vc(GREEN, 1), self.vc(GREY, 3)];
        let plain = self.vc(LGREEN, 1);
        self.minimap_frame(x0, y0);
        for y in 0..WORLD_HEIGHT {
            for x in 0..WORLD_WIDTH {
                if !self.explore.seen_world(x, y) {
                    continue;
                }
                let t = self.a.map.cell(x, y) as usize;
                // Places are shown as the land round them until entered.
                let c = colours.get(t).copied().unwrap_or(plain);
                self.term.plot((x0 + x) as i32, (y0 + y) as i32, c);
            }
        }
        let mark = |k: LocationKind, me: &Game| match k {
            LocationKind::Town => me.vc(WHITE, 3),
            LocationKind::Castle => me.vc(YELLOW, 3),
            LocationKind::Landmark => me.vc(TEXT, 1),
            LocationKind::Dungeon => me.vc(WARN, 2),
        };
        let mut counts = [0usize; 4];
        let visited: Vec<(u8, u8, usize)> = self.a.locs.locations.iter().filter(|l| self.explore.visited.contains(&(l.id as u8))).map(|l| (l.x, l.y, l.id)).collect();
        for &(x, y, id) in &visited {
            let Some(k) = kind_of(id) else { continue };
            counts[k as usize] += 1;
            let c = mark(k, self);
            let (px, py) = (x0 + x as usize, y0 + y as usize);
            self.term.fill(px.saturating_sub(1), py.saturating_sub(1), px + 1, py + 1, c);
        }
        let you = self.vc(MAGENTA, 2);
        let (px, py) = ((x0 as i32) + self.ow.x.rem_euclid(WORLD_WIDTH as i32), (y0 as i32) + self.ow.y.rem_euclid(WORLD_HEIGHT as i32));
        let blk = self.vc(BLACK, 0);
        for d in 1..=4 {
            for (dx, dy) in [(d, 0), (-d, 0), (0, d), (0, -d)] {
                self.term.plot(px + dx, py + dy, if d == 1 { blk } else { you });
            }
        }
        self.term.plot(px, py, you);
        self.minimap_legend(&visited, counts);
    }

    /// The frame round the map area.
    fn minimap_frame(&mut self, x0: usize, y0: usize) {
        let b = self.vc(DARK, 2);
        let (l, t, r, bt) = (x0 as i32 - 2, y0 as i32 - 2, (x0 + WORLD_WIDTH) as i32 + 1, (y0 + WORLD_HEIGHT) as i32 + 1);
        self.term.line(l, t, r, t, b);
        self.term.line(l, bt, r, bt, b);
        self.term.line(l, t, l, bt, b);
        self.term.line(r, t, r, bt, b);
    }

    /// The same area, each cell a square `MINIMAP_ZOOM` pixels a side,
    /// centred on the player (the world wraps round, as it does to walk).
    fn draw_minimap_zoomed(&mut self) {
        let z = Self::MINIMAP_ZOOM;
        let (x0, y0) = (12usize, 22usize);
        self.minimap_frame(x0, y0);
        let (cols, rows) = (WORLD_WIDTH / z, WORLD_HEIGHT / z);
        let (w, h) = (WORLD_WIDTH as i32, WORLD_HEIGHT as i32);
        let (cx, cy) = (self.ow.x.rem_euclid(w), self.ow.y.rem_euclid(h));
        let (ox, oy) = (cx - cols as i32 / 2, cy - rows as i32 / 2);
        let colours = [self.vc(BLUE, 2), self.vc(LGREEN, 1), self.vc(GREEN, 1), self.vc(GREY, 3)];
        let plain = self.vc(LGREEN, 1);
        let at = |i: usize, j: usize| (x0 + i * z, y0 + j * z);
        for j in 0..rows {
            for i in 0..cols {
                let (x, y) = ((ox + i as i32).rem_euclid(w) as usize, (oy + j as i32).rem_euclid(h) as usize);
                if !self.explore.seen_world(x, y) {
                    continue;
                }
                let c = colours.get(self.a.map.cell(x, y) as usize).copied().unwrap_or(plain);
                let (px, py) = at(i, j);
                self.term.fill(px, py, px + z - 1, py + z - 1, c);
            }
        }
        let mut counts = [0usize; 4];
        let visited: Vec<(u8, u8, usize)> = self.a.locs.locations.iter().filter(|l| self.explore.visited.contains(&(l.id as u8))).map(|l| (l.x, l.y, l.id)).collect();
        let blk = self.vc(BLACK, 0);
        for &(x, y, id) in &visited {
            let Some(k) = kind_of(id) else { continue };
            counts[k as usize] += 1;
            let (i, j) = ((x as i32 - ox).rem_euclid(w), (y as i32 - oy).rem_euclid(h));
            if i >= cols as i32 || j >= rows as i32 {
                continue;
            }
            let c = match k {
                LocationKind::Town => self.vc(WHITE, 3),
                LocationKind::Castle => self.vc(YELLOW, 3),
                LocationKind::Landmark => self.vc(TEXT, 1),
                LocationKind::Dungeon => self.vc(WARN, 2),
            };
            // A ring of the place's colour with a dark middle.
            let (px, py) = at(i as usize, j as usize);
            self.term.fill(px, py, px + z - 1, py + z - 1, c);
            self.term.fill(px + 1, py + 1, px + z - 2, py + z - 2, blk);
        }
        let you = self.vc(MAGENTA, 2);
        let (px, py) = at(cols / 2, rows / 2);
        let (px, py) = ((px + z / 2) as i32, (py + z / 2) as i32);
        for d in 1..=6 {
            for (dx, dy) in [(d, 0), (-d, 0), (0, d), (0, -d)] {
                self.term.plot(px + dx, py + dy, if d <= 2 { blk } else { you });
            }
        }
        self.term.plot(px, py, you);
        self.minimap_legend(&visited, counts);
    }

    fn minimap_legend(&mut self, visited: &[(u8, u8, usize)], counts: [usize; 4]) {
        let you = self.vc(MAGENTA, 2);
        let mark = |k: LocationKind, me: &Game| match k {
            LocationKind::Town => me.vc(WHITE, 3),
            LocationKind::Castle => me.vc(YELLOW, 3),
            LocationKind::Landmark => me.vc(TEXT, 1),
            LocationKind::Dungeon => me.vc(WARN, 2),
        };
        // Legend.
        let t = self.vc(TEXT, 1);
        let col = 24usize;
        let mut row = 3usize;
        let names = ["Towns", "Castles", "Signposts", "Dungeons"];
        let kinds = [LocationKind::Town, LocationKind::Castle, LocationKind::Landmark, LocationKind::Dungeon];
        for (i, k) in kinds.iter().enumerate() {
            let c = mark(*k, self);
            let (x, y) = (col * 8 + 1, row * 8 + 2);
            self.term.fill(x, y, x + 3, y + 3, c);
            self.term.print_at_attr(col + 1, row, &format!("{} {}", names[i], counts[*k as usize]), t);
            row += 1;
        }
        let (x, y) = (col * 8 + 2, row * 8 + 3);
        self.term.line(x as i32 - 2, y as i32, x as i32 + 2, y as i32, you);
        self.term.line(x as i32, y as i32 - 2, x as i32, y as i32 + 2, you);
        self.term.print_at_attr(col + 1, row, "You", t);
        row += 2;
        let w = self.vc(WHITE, 3);
        self.term.print_at_attr(col, row, "Places entered", w);
        row += 1;
        let mut shown = 0;
        let mut ids: Vec<usize> = visited.iter().map(|v| v.2).filter(|&id| id <= 48).collect();
        ids.sort_unstable();
        for id in &ids {
            if row > 21 {
                break;
            }
            let n = self.ptr(m::LOCATION_NAMES, *id);
            let n = n.trim().trim_start_matches("the ").to_string();
            let n: String = n.chars().take(15).collect();
            self.term.print_at_attr(col, row, &n, t);
            row += 1;
            shown += 1;
        }
        if shown < ids.len() {
            self.term.print_at_attr(col, 21, &format!("and {} more", ids.len() - shown + 1), t);
        }
        if ids.is_empty() {
            self.term.print_at_attr(col, row, "(none yet)", t);
        }
    }

    // ------------------------------------------------------------ character

    pub fn draw_character(&mut self) {
        self.view_frame("Character", "F6 or any key: back");
        let (w, t, hi) = (self.vc(WHITE, 3), self.vc(TEXT, 1), self.vc(WARN, 2));
        let name = self.rec.name();
        let level = (self.exp() as u16) / 1000 + 1;
        let sx = self.s(if self.rec.sex() == 0 { m::MALE } else { m::FEMALE });
        let race = self.ptr(m::RACE_NAMES, self.rec.race() as usize);
        let class = self.ptr(m::CLASS_NAMES, self.rec.class() as usize);
        centre(&mut self.term, 3, name.trim(), w);
        centre(&mut self.term, 4, &format!("Level {level} {}{race} {class}", sx.trim_start()), t);
        let coin = self.coin().max(0);
        let pairs = [
            ("Hit points", self.hits().to_string()),
            ("Food", self.food().to_string()),
            ("Experience", self.exp().to_string()),
            ("Coin", format!("{}g {}s {}c", coin / 100, (coin % 100) / 10, coin % 10)),
        ];
        for (i, (k, v)) in pairs.iter().enumerate() {
            let (col, row) = (2 + 19 * (i % 2), 6 + i / 2);
            let text = if i % 2 == 0 { format!("{k:<11}{v}") } else { format!("{k:<6}{v}") };
            self.term.print_at_attr(col, row, &text, w);
        }
        for i in 1..=6 {
            let n = self.ptr(m::STAT_NAMES, i);
            let (col, row) = (2 + 19 * ((i - 1) % 2), 9 + (i - 1) / 2);
            self.term.print_at_attr(col, row, &format!("{:<13}{}", n.trim(), self.stat(i)), t);
        }
        let lists = [
            ("Weapons", slot::WEAPON, m::WEAPON_LONG, 1, 15, self.rec.u16_at(off::READY_WEAPON) as usize),
            ("Armour", slot::ARMOUR, m::ARMOUR_NAMES, 1, 5, self.rec.u16_at(off::READY_ARMOUR) as usize),
            ("Spells", slot::SPELL, m::SPELL_NAMES, 1, 10, self.rec.u16_at(off::READY_SPELL) as usize),
            ("Transport", slot::VEHICLE, m::TRANSPORT_NAMES, 1, 7, self.transport() as usize),
            ("Gems", slot::GEM, m::GEM_NAMES, 0, 3, usize::MAX),
        ];
        let mut row = 13usize;
        for (title, base, names, first, last, cur) in lists {
            if row > 21 {
                break;
            }
            self.term.print_at_attr(2, row, &format!("{title}:"), w);
            let mut col = 13usize;
            let mut any = false;
            for i in first..=last {
                let q = self.qty(base, i);
                if q == 0 {
                    continue;
                }
                any = true;
                let n = self.ptr(names, i);
                let mut s = n.trim().to_string();
                if q > 1 {
                    s += &format!(" x{q}");
                }
                if i == cur {
                    s += "*";
                }
                if col > 13 && col + s.len() + 2 > 38 {
                    row += 1;
                    col = 13;
                    if row > 21 {
                        break;
                    }
                }
                if col > 13 {
                    self.term.print_at_attr(col - 2, row, ", ", t);
                }
                self.term.print_at_attr(col, row, &s, if i == cur { hi } else { t });
                col += s.len() + 2;
            }
            if !any {
                self.term.print_at_attr(col, row, "none", t);
            }
            row += 1;
        }
        if row <= 21 {
            let mut q: Vec<String> = Vec::new();
            for k in 1..=8 {
                match self.rec.quest(k) {
                    QuestState::Active => q.push(format!("{} (given)", self.castle_short(k))),
                    QuestState::Done => q.push(format!("{} (done)", self.castle_short(k))),
                    _ => {}
                }
            }
            self.term.print_at_attr(2, row, "Quests:", w);
            if q.is_empty() {
                self.term.print_at_attr(13, row, "none", t);
            }
            for s in q {
                if row > 21 {
                    break;
                }
                self.term.print_at_attr(13, row, &s.chars().take(26).collect::<String>(), t);
                row += 1;
            }
        }
        let en = self.w(off::ENEMIES);
        if en > 0 {
            self.term.print_at_attr(2, 22, &format!("Enemy vessels destroyed {en}"), t);
        }
        if self.rec.u16_at(off::READY_WEAPON) != 0 || self.rec.u16_at(off::READY_ARMOUR) != 0 {
            self.term.print_at_attr(29, 22, "* ready", hi);
        }
    }

    fn castle_short(&self, k: usize) -> String {
        let n = self.ptr(m::LOCATION_NAMES, 32 + k);
        n.trim().trim_start_matches("the ").to_string()
    }

    // ------------------------------------------------------------ help

    pub fn draw_help(&mut self) {
        self.view_frame("Commands", "F3 or any key: back");
        let (w, t) = (self.vc(WHITE, 3), self.vc(TEXT, 1));
        // Each entry fits 18 columns so the two columns never run together.
        let cmds: [(&str, &str); 24] = [
            ("Arrows", "move/turn"),
            ("Space", "pass"),
            ("A", "attack"),
            ("8246", "attack NSWE"),
            ("B", "board"),
            ("C", "cast spell"),
            ("D", "drop (town)"),
            ("E", "enter"),
            ("F", "fire guns"),
            ("G", "get chest"),
            ("H", "hyperjump"),
            ("I", "inform/search"),
            ("K", "klimb"),
            ("N", "noise on/off"),
            ("O", "open chest"),
            ("Q", "quit & save"),
            ("R", "ready"),
            ("S", "steal"),
            ("T", "transact"),
            ("U", "unlock cell"),
            ("V", "view (space)"),
            ("X", "x-it vehicle"),
            ("Z", "ztats"),
            ("", ""),
        ];
        for (i, (k, d)) in cmds.iter().enumerate() {
            let (col, row) = (1 + 20 * (i / 12), 3 + i % 12);
            self.term.print_at_attr(col, row, k, w);
            self.term.print_at_attr(col + k.len() + 1, row, d, t);
        }
        self.term.print_at_attr(1, 16, "Engine keys", w);
        let eng =
            [("F1", "options"), ("F2", "save slots"), ("F5/F9", "qsave/qload"), ("PgUp", "message log"), ("Tab", "map (if on)"), ("F6", "character"), ("F3", "this help"), ("Alt+Ent", "fullscreen")];
        for (i, (k, d)) in eng.iter().enumerate() {
            let (col, row) = (1 + 20 * (i % 2), 17 + i / 2);
            self.term.print_at_attr(col, row, k, w);
            self.term.print_at_attr(col + k.len() + 1, row, d, t);
        }
        self.term.print_at_attr(1, 21, "Keys can be changed: F1, Keys.", t);
    }

    // ------------------------------------------------------------ hints

    /// SPEC §23.4: a short line on what the command `k` will ask for.
    pub fn show_hint(&mut self, k: u8) {
        if let Some(h) = hint(k, self.mode) {
            self.term.println(h);
        }
    }

    // ------------------------------------------------------------ confirm

    /// Ask first when the confirmation setting covers `value` (SPEC
    /// §23.5); true when there is nothing to ask.
    pub fn sure(&mut self, io: &mut dyn Io, value: i32, what: &str) -> IoResult<bool> {
        if !io.features().must_confirm(value) {
            return Ok(true);
        }
        self.confirm(io, what)
    }

    /// SPEC §23.5: "are you sure" in a box over the view; true for Y. The
    /// screen goes back as it was.
    pub fn confirm(&mut self, io: &mut dyn Io, what: &str) -> IoResult<bool> {
        let saved = self.term.clone();
        let line1: String = what.chars().take(30).collect();
        let wcols = line1.len().max(14) + 2;
        let c0 = (40 - wcols) / 2;
        let (x0, y0, x1, y1) = (c0 * 8 - 4, 8 * 8 - 4, (c0 + wcols) * 8 + 3, 11 * 8 + 3);
        self.term.fill(x0, y0, x1, y1, BLACK);
        let b = self.vc(WARN, 2);
        self.term.line(x0 as i32, y0 as i32, x1 as i32, y0 as i32, b);
        self.term.line(x0 as i32, y1 as i32, x1 as i32, y1 as i32, b);
        self.term.line(x0 as i32, y0 as i32, x0 as i32, y1 as i32, b);
        self.term.line(x1 as i32, y0 as i32, x1 as i32, y1 as i32, b);
        let w = self.vc(WHITE, 3);
        centre(&mut self.term, 8, &line1, w);
        let t = self.vc(TEXT, 1);
        centre(&mut self.term, 10, "Sure? (Y/N)", t);
        let k = io.key(&self.term)?;
        self.term = saved;
        Ok(k.eq_ignore_ascii_case(&b'Y'))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hints_only_for_prompting_commands() {
        assert!(hint(b'A', OUTDOORS).is_some());
        assert!(hint(b'A', crate::dgame::DUNGEON).is_none());
        assert!(hint(b'T', OUTDOORS).is_none());
        assert!(hint(b'T', crate::game::TOWN).is_some());
        assert!(hint(b'Z', OUTDOORS).is_none());
        for k in *b"AFDRT" {
            if let Some(h) = hint(k, crate::game::TOWN) {
                assert!(h.len() <= 29, "{h}");
            }
        }
        assert_eq!(View::of_key(keys::MAP), Some(View::Map));
        assert_eq!(View::of_key(b'M'), None);
    }
}
