//! Dungeon play (SPEC §6): entering, moving, climbing, the monsters' turn,
//! fighting, spells, chests and coffins. Written from our own reading of
//! the original's behaviour ("verified from original executable"); every
//! message and table comes from the user's OUT.EXE at run time.

use crate::dungeon::{self, Cell, DungeonLevel, Item, DUNGEON_SIZE};
use crate::dview::{self, ViewTables, EAST, NORTH, SOUTH, WEST};
use crate::exe::m;
use crate::game::{slot, Game, AGI, HITS, INT, OUTDOORS, STA, STR};
use crate::io::Io;
use crate::save::off;
use crate::term::TEXT;

pub const DUNGEON: u8 = 3;
/// `953`'s damage value meaning "kill outright" (the Kill spell).
const KILL: i32 = 10000;

/// Live dungeon state (the original's globals at `ad3e`..`ad54`).
#[derive(Debug, Clone)]
pub struct DungeonState {
    pub level: DungeonLevel,
    pub a: i32,
    pub b: i32,
    pub depth: u16,
    pub facing: u16,
    /// Hit points owed when the player climbs out (`ad54`).
    pub bonus: i32,
    /// Redraw the "Level n" title on the next view (`ad44`).
    pub title: bool,
    pub moved: [[bool; DUNGEON_SIZE]; DUNGEON_SIZE],
}

/// `ad24`: the original's time-of-day seed from hour, minute, second and
/// hundredths, kept to 12 bits.
pub fn time_seed(h: u8, mi: u8, s: u8, hund: u8) -> u16 {
    let cx = (u16::from(h.wrapping_mul(2)) << 8) | u16::from(mi.wrapping_mul(4));
    let dx = (u16::from(s.wrapping_mul(8)) << 8) | u16::from(hund);
    (cx.wrapping_add(dx) ^ 0x91EB) & 0x0FFF
}

/// [`time_seed`] of an `[hour, minute, second, hundredths]` reading.
pub fn time_seed_from(t: [u8; 4]) -> u16 {
    time_seed(t[0], t[1], t[2], t[3])
}

fn turn_left(f: u16) -> u16 {
    match f {
        WEST => SOUTH,
        SOUTH => EAST,
        EAST => NORTH,
        _ => WEST,
    }
}

fn turn_right(f: u16) -> u16 {
    match f {
        WEST => NORTH,
        NORTH => EAST,
        EAST => SOUTH,
        _ => WEST,
    }
}

fn turn_around(f: u16) -> u16 {
    match f {
        WEST => EAST,
        EAST => WEST,
        NORTH => SOUTH,
        _ => NORTH,
    }
}

impl Game {
    fn ds(&self) -> &DungeonState {
        self.dungeon.as_ref().expect("in a dungeon")
    }
    fn dsm(&mut self) -> &mut DungeonState {
        self.dungeon.as_mut().expect("in a dungeon")
    }

    /// The 12-bit time seed now (or the fixed one used by tests).
    pub fn time_seed_now(&self) -> u16 {
        // Read through `time_seed_from` so the clock and tests agree.
        match self.clock {
            Some(f) => {
                let [h, mi, s, c] = f();
                time_seed(h, mi, s, c)
            }
            None => self.fixed_time & 0x0FFF,
        }
    }

    /// Candidate feature spots from OUT.EXE (two 16-word arrays).
    fn spots(&self) -> [(u8, u8); 16] {
        let mut s = [(1u8, 1u8); 16];
        for (i, p) in s.iter_mut().enumerate() {
            let a = self.a.exe.w(0x3820 + 2 * i).clamp(1, 9) as u8;
            let b = self.a.exe.w(0x3840 + 2 * i).clamp(1, 9) as u8;
            *p = (a, b);
        }
        s
    }

    /// Level setup `4761`.
    fn setup_level(&mut self) {
        let (x, y) = (self.ow.x as u16, self.ow.y as u16);
        let save = self.rec.u16_at(0x32);
        let spots = self.spots();
        let item_seed = self.time_seed_now().wrapping_add(0x6F1);
        let (depth, a, b) = {
            let d = self.ds();
            (d.depth, d.a, d.b)
        };
        let lv = dungeon::build(&mut self.ow.rng, x, y, depth, save, &spots, item_seed, (a as usize, b as usize));
        let d = self.dsm();
        d.level = lv;
        d.title = true;
        d.moved = [[false; DUNGEON_SIZE]; DUNGEON_SIZE];
    }

    /// Enter a dungeon `4bb0`.
    pub fn enter_dungeon(&mut self) {
        if self.view_tables.is_none() {
            self.view_tables = Some(ViewTables::from_exe(&self.a.exe));
        }
        self.mode = DUNGEON;
        self.dungeon = Some(DungeonState {
            level: dungeon::generate(0, 0, 1, 0, &[(1, 1); 16], 0),
            a: 1,
            b: 1,
            depth: 1,
            facing: SOUTH,
            bonus: 0,
            title: true,
            moved: [[false; DUNGEON_SIZE]; DUNGEON_SIZE],
        });
        self.setup_level();
    }

    /// `414a`: the whole view, and the title when it is due.
    pub fn draw_dungeon(&mut self) {
        let Some(vt) = self.view_tables.take() else { return };
        let d = self.dungeon.take().expect("in a dungeon");
        if d.title {
            dview::title(&mut self.term, &self.a.exe, d.depth);
        }
        dview::draw(&mut self.term, &self.a.exe, &vt, &d.level, d.a, d.b, d.facing);
        self.dungeon = Some(d);
        self.dsm().title = false;
        self.view_tables = Some(vt);
    }

    /// Leave the dungeon views: put the borders back (`ad9d`, `ad4d`).
    pub fn restore_dungeon_borders(&mut self) {
        dview::restore_title(&mut self.term);
        dview::restore_compass(&mut self.term);
    }

    fn here(&self) -> Cell {
        let d = self.ds();
        d.level.cell(d.a, d.b)
    }

    fn ahead(&self, k: i32) -> (i32, i32) {
        let d = self.ds();
        let (fa, fb) = dview::forward(d.facing);
        (d.a + fa * k, d.b + fb * k)
    }

    fn item_here(&self) -> Item {
        let d = self.ds();
        d.level.items[d.a as usize][d.b as usize]
    }

    fn set_item_here(&mut self, it: Item) {
        let d = self.dsm();
        let (a, b) = (d.a as usize, d.b as usize);
        d.level.items[a][b] = it;
    }

    // ------------------------------------------------------------ moving

    /// Arrow keys `4b73`.
    pub fn dungeon_move(&mut self, io: &mut dyn Io, key: u8) {
        use crate::io::keys;
        let f = self.ds().facing;
        match key {
            keys::LEFT => {
                self.dsm().facing = turn_left(f);
                self.sayln(m::TURN_LEFT);
            }
            keys::RIGHT => {
                self.dsm().facing = turn_right(f);
                self.sayln(m::TURN_RIGHT);
            }
            keys::DOWN => {
                self.dsm().facing = turn_around(f);
                self.sayln(m::TURN_AROUND);
            }
            _ => {
                self.say(m::FORWARD);
                if self.blocked() {
                    self.snd(io, 0);
                    self.sayln(m::PATH_BLOCKED);
                } else {
                    let (a, b) = self.ahead(1);
                    let d = self.dsm();
                    d.a = a;
                    d.b = b;
                    self.nl();
                }
            }
        }
    }

    /// `3f7f`: walls, secret doors, force fields, monsters, and a door
    /// straight after a door stop the player.
    fn blocked(&self) -> bool {
        let (a, b) = self.ahead(1);
        let d = self.ds();
        let t = d.level.cell(a, b);
        matches!(t, Cell::Wall | Cell::SecretDoor | Cell::ForceField)
            || (t == Cell::Door && self.here() == Cell::Door)
            || d.level.monster(a, b) != -1
    }

    // ------------------------------------------------------------ K-limb

    /// K-limb `581a`.
    pub fn cmd_klimb(&mut self, io: &mut dyn Io) {
        self.say(m::KLIMB);
        if self.mode != DUNGEON {
            self.err(io, m::QUESTION);
            return;
        }
        let here = self.here();
        if !matches!(here, Cell::LadderUp | Cell::LadderDown) {
            self.err(io, m::SP_WHAT);
            return;
        }
        let f = self.ds().facing;
        if f == WEST || f == EAST {
            self.nl();
            self.err(io, m::FACE_LADDER);
            return;
        }
        if here == Cell::LadderUp {
            self.dsm().depth -= 1;
            self.say(m::UP_TO_LEVEL);
            let l = self.ds().depth;
            self.term.num(l, 1, ' ', TEXT, true);
            self.nl();
            if l == 0 {
                self.leave_dungeon();
            } else {
                self.setup_level();
                let d = self.dsm();
                let (a, b) = (d.a as usize, d.b as usize);
                d.level.monsters[a][b] = -1;
            }
        } else {
            self.dsm().depth += 1;
            self.say(m::DOWN_TO_LEVEL);
            let l = self.ds().depth;
            self.term.num(l, 1, ' ', TEXT, true);
            self.nl();
            self.setup_level();
        }
    }

    /// Climbing out of level 1: the hit point bonus earned below.
    fn leave_dungeon(&mut self) {
        let mut bonus = self.ds().bonus;
        let hits = i32::from(self.stat(HITS));
        if hits + bonus > 9999 {
            bonus = 9999 - hits;
        }
        if bonus != 0 {
            self.say(m::THOU_DOST_GAIN);
            self.term.num(bonus as u16, 1, ' ', TEXT, true);
            self.say(m::SP_HIT_SP);
            if bonus > 999 {
                self.nl();
            }
            self.sayln(m::POINTS);
            self.set_stat(HITS, (hits + bonus) as i16);
        }
        self.mode = OUTDOORS;
        self.dungeon = None;
        self.restore_dungeon_borders();
        self.redraw();
    }

    // ------------------------------------------------------------ monsters

    /// The monsters' turn `8ac9`.
    pub fn dungeon_monsters(&mut self, io: &mut dyn Io) {
        let mut moved = false;
        'outer: for a in 1..=9i32 {
            for b in 1..=9i32 {
                let (id, flag, pa, pb) = {
                    let d = self.ds();
                    (d.level.monsters[a as usize][b as usize], d.moved[a as usize][b as usize], d.a, d.b)
                };
                if id != -1 && !flag {
                    let (dx, dy) = (pa - a, pb - b);
                    let dist = dx.abs() + dy.abs();
                    if dist == 1 {
                        self.monster_attack(io, a, b);
                    } else if dist < 7 {
                        let (sx, sy) = (dx.signum(), dy.signum());
                        if sx != 0 {
                            moved = self.monster_try(a, b, sx, 0);
                        }
                        if !moved && sy != 0 {
                            moved = self.monster_try(a, b, 0, sy);
                        }
                    }
                }
                if self.hits() <= 0 || self.food() <= 0 {
                    continue 'outer;
                }
            }
        }
        if let Some(d) = self.dungeon.as_mut() {
            d.moved = [[false; DUNGEON_SIZE]; DUNGEON_SIZE];
        }
    }

    /// `8a04`: step a monster; its hit points stay in the cell it left.
    fn monster_try(&mut self, a: i32, b: i32, sx: i32, sy: i32) -> bool {
        let (ta, tb) = (a + sx, b + sy);
        let d = self.dsm();
        if matches!(d.level.cell(ta, tb), Cell::Wall | Cell::SecretDoor | Cell::ForceField) || d.level.monster(ta, tb) != -1 {
            return false;
        }
        let id = d.level.monsters[a as usize][b as usize];
        d.level.monsters[ta as usize][tb as usize] = id;
        d.level.monsters[a as usize][b as usize] = -1;
        d.moved[ta as usize][tb as usize] = true;
        true
    }

    /// A monster's printed name (`3c31`: the mimic shows as "Mimic").
    fn monster_name(&self, id: i16) -> String {
        if id == 11 {
            self.s(m::MIMIC)
        } else {
            self.ptr(m::MONSTER_NAMES, id as usize)
        }
    }

    /// A monster next to the player attacks (`873d`).
    fn monster_attack(&mut self, io: &mut dyn Io, a: i32, b: i32) {
        let here = self.here();
        if here == Cell::ForceField {
            return;
        }
        let (id, mcell) = {
            let d = self.ds();
            (d.level.monsters[a as usize][b as usize], d.level.cell(a, b))
        };
        if mcell == Cell::Door && matches!(here, Cell::Door | Cell::Wall | Cell::SecretDoor) {
            return;
        }
        self.say(m::ATTACKED_BY);
        let name = self.monster_name(id);
        self.term.print(&name);
        self.sayln(m::EXCL);
        self.snd(io, 3);
        let armour = i32::from(self.rec.u16_at(off::READY_ARMOUR));
        let def = i32::from(self.stat(STA)) / 2 + 8 * armour + 56;
        if self.r(1, 255) <= def {
            self.sayln(m::MISSED);
            return;
        }
        let r2 = self.r(1, 255);
        let l = i32::from(self.ds().depth);
        let id32 = i32::from(id);
        let mut dmg = id32 * id32 + l;
        if dmg > 255 {
            dmg = self.r(id32 + 1, 255);
        }
        let mut hurt = true;
        if id == 9 && armour != 0 {
            self.sayln(m::ARMOR_DESTROYED);
            self.add(slot::ARMOUR + 2 * armour as usize, -1);
            self.rec.set_u16(off::READY_ARMOUR, 0);
            hurt = false;
        } else if id == 16 {
            self.sayln(m::GREMLIN);
            let f = self.food() / 2;
            self.set(off::FOOD, f);
            self.show_food();
            hurt = false;
        } else if id == 21 && r2 < 128 {
            self.sayln(m::MENTAL);
            let i = self.stat(INT) / 2 + 5;
            self.set_stat(INT, i);
            hurt = false;
        } else if id == 2 {
            let ready = self.rec.u16_at(off::READY_WEAPON) as usize;
            if let Some(w) = (1..=15).find(|&w| self.qty(slot::WEAPON, w) > 0 && w != ready) {
                self.say(m::THIEF_STOLE);
                let wn = self.ptr(m::WEAPON_LONG, w);
                if wn.starts_with(['A', 'E', 'I', 'O', 'U', 'a', 'e', 'i', 'o', 'u']) {
                    self.say(m::N_SP);
                } else {
                    self.term.print(" ");
                }
                if wn.len() > 11 {
                    self.nl();
                }
                self.term.println(&wn);
                self.add(slot::WEAPON + 2 * w, -1);
            }
        }
        if hurt {
            self.snd(io, 2);
            self.say(m::HIT_EX);
            self.term.num(dmg as u16, 1, ' ', TEXT, true);
            self.sayln(m::DAMAGE);
            let h = self.hits() - dmg as i16;
            self.set_stat(HITS, h);
            self.show_hits();
        }
    }

    // ------------------------------------------------------------ fighting

    /// `baa`: how far ahead (1..range) the first monster is, or 0 when a
    /// wall, door or field comes first.
    fn target_ahead(&self, range: i32) -> i32 {
        let mut i = 1;
        loop {
            let (a, b) = self.ahead(i);
            let d = self.ds();
            let blocked = d.level.monster(a, b) != -1
                || matches!(d.level.cell(a, b), Cell::Wall | Cell::SecretDoor | Cell::ForceField | Cell::Door);
            if i >= range || blocked {
                break;
            }
            i += 1;
        }
        let (a, b) = self.ahead(i);
        let d = self.ds();
        if d.level.monster(a, b) == -1 || matches!(d.level.cell(a, b), Cell::Wall | Cell::SecretDoor | Cell::ForceField) {
            0
        } else {
            i
        }
    }

    /// Attack in the dungeon (`10c2` → `c52`).
    pub fn dungeon_attack(&mut self, io: &mut dyn Io, range: i32, wpn: usize) {
        self.nl();
        self.snd(io, 7);
        let d = self.target_ahead(range);
        if d == 0 {
            self.sayln(m::NOTHING_CAP);
            return;
        }
        let hit = i32::from(self.stat(AGI)) + 50;
        let dmg = self.r(2, i32::from(self.stat(STR)) + 8 * wpn as i32);
        let (a, b) = self.ahead(d);
        self.resolve(io, 2, hit, dmg, a, b);
    }

    /// `953`: a blow or spell landing on the cell (a,b).
    fn resolve(&mut self, io: &mut dyn Io, snd: u8, hit: i32, dmg: i32, a: i32, b: i32) {
        let here = self.here();
        let tcell = self.ds().level.cell(a, b);
        let ok = if here == Cell::Door { matches!(tcell, Cell::Empty | Cell::LadderUp | Cell::LadderDown) } else { true };
        let roll = self.r(1, 100);
        if roll > hit || matches!(here, Cell::Wall | Cell::SecretDoor | Cell::ForceField) || !ok {
            self.sayln(m::MISSED);
            return;
        }
        self.snd(io, snd);
        if dmg != KILL {
            self.say(m::HIT_EX);
        }
        let (ua, ub) = (a as usize, b as usize);
        let hp = i32::from(self.ds().level.hp[ua][ub]);
        if hp > dmg {
            self.term.num(dmg as u16, 1, ' ', TEXT, true);
            self.sayln(m::DAMAGE);
            self.dsm().level.hp[ua][ub] -= dmg as i16;
            return;
        }
        let id = self.ds().level.monsters[ua][ub];
        let name = self.monster_name(id);
        self.term.print(&name);
        self.sayln(if dmg == KILL { m::DESTROYED } else { m::KILLED_EX });
        self.quest_kill(io, id);
        let l = i32::from(self.ds().depth);
        let gold = self.r(2, 3 * l + 10 + i32::from(id));
        self.say(m::THOU_DOST_FIND);
        self.loot(gold as i16, 0);
        {
            let d = self.dsm();
            d.level.used[(id % 5) as usize] = false;
            d.level.monsters[ua][ub] = -1;
            d.level.hp[ua][ub] = 0;
        }
        let e = self.r(2, l * l + 10);
        self.add(off::EXPERIENCE, e as i16);
        self.dsm().bonus += 2 * e;
        self.show_stats();
        let (pa, pb) = (self.ds().a as usize, self.ds().b as usize);
        let d = self.dungeon.as_mut().expect("in a dungeon");
        d.level.place_monster(&mut self.ow.rng, (pa, pb));
    }

    /// `8ee`: the four castle quests ask for these monsters.
    fn quest_kill(&mut self, io: &mut dyn Io, id: i16) {
        let k = match id {
            9 => 2,
            14 => 4,
            19 => 6,
            24 => 8,
            _ => return,
        };
        if self.w(slot::QUEST + 2 * k) == -1 {
            self.set(slot::QUEST + 2 * k, 1);
            self.say(m::QUEST_COMPLETED);
            self.snd(io, 5);
            self.nl();
        }
    }

    // ------------------------------------------------------------ chests

    /// Open (coffins) `59d7`.
    pub fn cmd_open(&mut self, io: &mut dyn Io) {
        self.say(m::OPEN);
        if self.mode != DUNGEON {
            self.err(io, m::QUESTION);
            return;
        }
        self.say(m::SP_COFFIN);
        if self.item_here() != Item::Coffin {
            self.err(io, m::NONE_HERE);
            return;
        }
        self.nl();
        let r = self.r(1, 255);
        let (a, b) = self.ahead(1);
        let (tc, tm) = {
            let d = self.ds();
            (d.level.cell(a, b), d.level.monster(a, b))
        };
        if r < 104 && !matches!(tc, Cell::Wall | Cell::SecretDoor) && tm == -1 {
            if (1..10).contains(&a) && (1..10).contains(&b) {
                let d = self.dungeon.as_mut().expect("in a dungeon");
                d.level.coffin_spawn(&mut self.ow.rng, a as usize, b as usize);
            }
        } else {
            self.set_item_here(Item::None);
            self.find_treasure();
        }
    }

    /// "Thou dost find: " and `R(3, L*L+9)` coins.
    fn find_treasure(&mut self) {
        self.say(m::THOU_DOST_FIND);
        let l = i32::from(self.ds().depth);
        let n = self.r(3, l * l + 9);
        self.loot(n as i16, 0);
    }

    /// Unlock in the dungeon (`7b63`): chests, perhaps trapped.
    pub fn dungeon_unlock(&mut self, io: &mut dyn Io) {
        let r = self.r(1, 75);
        self.say(m::SP_CHEST);
        if self.item_here() != Item::Chest {
            self.err(io, m::NONE_HERE);
            return;
        }
        self.nl();
        if r > i32::from(self.stat(AGI)) && self.rec.class() != 3 {
            self.say(m::TRAP);
            self.snd(io, 2);
            self.nl();
            let h = self.hits() - self.ds().depth as i16;
            self.set_stat(HITS, h);
            self.show_hits();
        } else {
            self.set_item_here(Item::None);
            self.find_treasure();
        }
    }

    /// Inform and search in the dungeon: secret doors.
    pub fn dungeon_inform(&mut self) {
        self.sayln(m::INFORM);
        self.say(m::FIND);
        let (a, b) = self.ahead(1);
        if self.ds().level.cell(a, b) == Cell::SecretDoor && self.here() != Cell::Door {
            self.sayln(m::SECRET_DOOR);
            self.dsm().level.cells[a as usize][b as usize] = Cell::Door;
        } else {
            self.sayln(m::NOTHING);
        }
    }

    // ------------------------------------------------------------ spells

    /// Cast `19ff`.
    pub fn cmd_cast_spell(&mut self, io: &mut dyn Io) {
        if self.mode == 1 || self.mode == 2 || self.mode == 4 {
            self.say(m::CAST_DASH);
            self.sayln(m::NO_EFFECT);
            self.snd(io, 6);
            return;
        }
        self.say(m::CAST);
        let s = self.rec.u16_at(off::READY_SPELL) as usize;
        let name = self.ptr(m::SPELL_NAMES, s);
        self.term.print(&name);
        if s == 0 {
            self.prayer(io);
            return;
        }
        if self.qty(slot::SPELL, s) == 0 {
            self.nl();
            self.sayln(m::USED_UP);
            self.snd(io, 6);
            return;
        }
        self.add(slot::SPELL + 2 * s, -1);
        if self.mode == DUNGEON {
            self.spell(io, s, true);
        } else if s != 3 && s != 10 {
            self.nl();
            self.sayln(m::DUNGEON_ONLY);
            self.snd(io, 6);
        } else {
            // Outdoor magic missile / kill: not part of this engine yet.
            self.nl();
            self.sayln(m::FAILED);
            self.snd(io, 6);
        }
    }

    /// Prayer `1769`.
    fn prayer(&mut self, io: &mut dyn Io) {
        self.nl();
        let inc = self.ptr(m::INCANTATIONS, 0);
        self.term.println(&inc);
        if self.mode == DUNGEON {
            let mut k = self.r(1, 10) as usize;
            if k == 4 {
                k = 5;
            }
            self.spell(io, k, false);
            return;
        }
        if self.hits() < 15 {
            self.set_stat(HITS, 15);
            self.sayln(m::SHAZAM_SP);
            self.show_hits();
            self.snd(io, 5);
        } else if self.food() < 15 {
            self.set(off::FOOD, 15);
            self.sayln(m::SHAZAM_SP);
            self.show_food();
            self.snd(io, 5);
        } else {
            // (The 1-in-4 removal of an adjacent outdoor monster is not
            // reproduced; see SPEC §6.7.)
            let _ = self.r(1, 100);
            self.sayln(m::NO_EFFECT);
            self.snd(io, 6);
        }
    }

    /// `13a8`: the dungeon effect of spell `s`.
    fn spell(&mut self, io: &mut dyn Io, s: usize, announce: bool) {
        if announce {
            let inc = self.ptr(m::INCANTATIONS, s);
            self.term.println(&inc);
        }
        let (pa, pb, depth) = {
            let d = self.ds();
            (d.a, d.b, d.depth)
        };
        let here = self.here();
        let odd = pa & 1 == 1 || pb & 1 == 1;
        let ok = match s {
            1 | 2 => {
                let want = if s == 1 { Item::Coffin } else { Item::Chest };
                if self.item_here() == want {
                    self.say(if s == 1 { m::COFFIN } else { m::CHEST });
                    self.sayln(m::OPENED);
                    self.set_item_here(Item::None);
                    self.find_treasure();
                    true
                } else {
                    false
                }
            }
            3 => {
                let d = self.target_ahead(5);
                if d == 0 {
                    false
                } else {
                    let dmg = self.magic_damage();
                    let (a, b) = self.ahead(d);
                    self.resolve(io, 5, 101, dmg, a, b);
                    true
                }
            }
            4 => true,
            5 => {
                if depth == 10 || here == Cell::ForceField || !odd {
                    false
                } else {
                    self.set_here(Cell::LadderDown);
                    self.sayln(m::LADDER_CREATED);
                    true
                }
            }
            6 => {
                if odd && here != Cell::ForceField {
                    self.set_here(Cell::LadderUp);
                    self.sayln(m::LADDER_CREATED);
                    true
                } else {
                    false
                }
            }
            7 => {
                loop {
                    let a = self.r(1, 9);
                    let b = self.r(1, 9);
                    let d = self.ds();
                    if (a, b) != (pa, pb)
                        && !matches!(d.level.cell(a, b), Cell::ForceField | Cell::Wall | Cell::SecretDoor)
                        && d.level.monster(a, b) == -1
                    {
                        let d = self.dsm();
                        d.a = a;
                        d.b = b;
                        break;
                    }
                }
                self.sayln(m::TELEPORTED);
                true
            }
            8 => {
                let (a, b) = self.ahead(1);
                let d = self.ds();
                if d.level.cell(a, b) == Cell::Empty && d.level.monster(a, b) == -1 && item_at(&d.level, a, b) == Item::None {
                    self.dsm().level.cells[a as usize][b as usize] = Cell::ForceField;
                    self.sayln(m::FIELD_CREATED);
                    true
                } else {
                    false
                }
            }
            9 => {
                let (a, b) = self.ahead(1);
                if self.ds().level.cell(a, b) == Cell::ForceField {
                    self.dsm().level.cells[a as usize][b as usize] = Cell::Empty;
                    self.sayln(m::FIELD_DESTROYED);
                    true
                } else {
                    false
                }
            }
            10 => {
                let (a, b) = self.ahead(1);
                if self.ds().level.monster(a, b) != -1 {
                    self.resolve(io, 5, 101, KILL, a, b);
                    true
                } else {
                    false
                }
            }
            _ => false,
        };
        if !ok {
            self.sayln(m::FAILED);
            self.snd(io, 6);
        } else if s != 3 && s != 10 {
            self.snd(io, 5);
        }
    }

    fn set_here(&mut self, c: Cell) {
        let d = self.dsm();
        let (a, b) = (d.a as usize, d.b as usize);
        d.level.cells[a][b] = c;
    }

    /// `12f2`: R(1, INT), raised by the ready weapon (wand x2, amulet
    /// x3/2, staff and triangle x3).
    fn magic_damage(&mut self) -> i32 {
        let i = i32::from(self.stat(INT));
        let mut d = self.r(1, i);
        match self.rec.u16_at(off::READY_WEAPON) {
            9 => d *= 2,
            8 => d = d * 3 / 2,
            10 | 11 => d *= 3,
            _ => {}
        }
        d
    }
}

fn item_at(lv: &DungeonLevel, a: i32, b: i32) -> Item {
    if (0..DUNGEON_SIZE as i32).contains(&a) && (0..DUNGEON_SIZE as i32).contains(&b) {
        lv.items[a as usize][b as usize]
    } else {
        Item::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_seed_is_12_bits() {
        assert_eq!(time_seed(0, 0, 0, 0), 0x91EB & 0xFFF);
        for h in [0u8, 7, 23] {
            assert!(time_seed(h, 59, 59, 99) <= 0xFFF);
        }
        // hour*2 lands in the high byte, minute*4 in the low byte.
        assert_eq!(time_seed(1, 0, 0, 0), (0x0200 ^ 0x91EB) & 0xFFF);
        assert_eq!(time_seed(0, 1, 0, 0), (0x0004 ^ 0x91EB) & 0xFFF);
        assert_eq!(time_seed(0, 0, 0, 5), (0x0005 ^ 0x91EB) & 0xFFF);
    }

    #[test]
    fn turns() {
        for f in 1..=4 {
            assert_eq!(turn_right(turn_left(f)), f);
            assert_eq!(turn_around(turn_around(f)), f);
        }
        assert_eq!(turn_left(SOUTH), EAST);
        assert_eq!(turn_right(NORTH), EAST);
    }
}
