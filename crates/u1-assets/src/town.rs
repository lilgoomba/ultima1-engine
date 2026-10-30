//! Towns and castles: walking, the NPCs' turn, fighting, stealing, the
//! jail door and the princess (SPEC §9; verified from original executable).

use crate::sound::Sfx;
use crate::exe::m;
use crate::game::*;
use crate::io::{Io, IoResult};
use crate::overworld::Entity;
use crate::rules::{self, Dir};
use crate::save::off;
use crate::tcd::{TCD_HEIGHT, TCD_WIDTH};
use crate::term::{BLACK, BLUE, TEXT};

/// Result of `23e7`: what stops a move into a cell.
pub const FREE: i16 = -1;
pub const SPECIAL: i16 = 0x3F;

impl Game {
    pub fn tcell(&self, x: i32, y: i32) -> u8 {
        self.maps[self.layout].cell(x as usize, y as usize)
    }

    fn in_town(x: i32, y: i32) -> bool {
        (0..TCD_WIDTH as i32).contains(&x) && (0..TCD_HEIGHT as i32).contains(&y)
    }

    /// `2355`: who stands on a town cell (-1 = nobody).
    pub fn occupant(&self, x: i32, y: i32) -> i16 {
        if !Self::in_town(x, y) {
            return -1;
        }
        if (x, y) == (self.px, self.py) {
            return PLAYER_TILE;
        }
        match self.npcs.iter().find(|n| n.tile != -1 && i32::from(n.x) == x && i32::from(n.y) == y) {
            Some(n) if n.tile != 0 => n.tile,
            _ => -1,
        }
    }

    /// `23e7`: -1 free, 0x3F the special edge/exit cell, 0 blocked, or the
    /// tile of whoever stands there.
    pub fn passable(&self, x: i32, y: i32) -> i16 {
        if !Self::in_town(x, y) {
            return SPECIAL;
        }
        let o = self.occupant(x, y);
        if o != -1 {
            return o;
        }
        let c = self.tcell(x, y);
        if c == 0x3F {
            SPECIAL
        } else if c == 1 || c >= 0x33 {
            FREE
        } else {
            0
        }
    }

    /// Town move `2e22`.
    pub fn town_move(&mut self, io: &mut dyn Io, d: Dir) -> IoResult<()> {
        let (dx, dy) = d.delta();
        let (nx, ny) = (self.px + dx, self.py + dy);
        if !Self::in_town(nx, ny) {
            if self.mode == CASTLE && self.princess_adjacent() {
                self.princess_reward(io)?;
            }
            self.mode = OUTDOORS;
            self.redraw();
            return Ok(());
        }
        let p = self.passable(nx, ny);
        if p == FREE || p == SPECIAL {
            let dn = self.a.t.direction_names[d as usize].clone();
            self.term.println(&dn);
            self.px = nx;
            self.py = ny;
            self.draw_town();
            self.snd(io, 4);
        } else {
            self.err(io, m::BLOCKED);
        }
        Ok(())
    }

    fn princess_adjacent(&self) -> bool {
        (-1..=1).any(|dx| (-1..=1).any(|dy| (dx, dy) != (0, 0) && self.occupant(self.px + dx, self.py + dy) == NPC_PRINCESS))
    }

    /// `4bd6`: the reward for leading a princess out.
    fn princess_reward(&mut self, io: &mut dyn Io) -> IoResult<()> {
        let ace = self.w(off::ENEMIES) >= 20 && self.exp() as u16 >= 7000;
        let o = if ace { 0 } else { 2 };
        self.term.boxed(BLUE);
        self.at(5, 5 + o, m::PRINCESS_SAVED[0]);
        let name = self.ptr(m::PRINCESS_NAMES, self.princess);
        self.term.print_at(5, 6 + o, &name);
        self.say(m::PRINCESS_SAVED[1]);
        self.at(8, 7 + o, m::PRINCESS_SAVED[2]);
        self.at(12, 8 + o, m::PRINCESS_SAVED[3]);
        self.add(off::COIN, 500);
        self.add(off::HITS, 500);
        self.add(off::EXPERIENCE, 500);
        self.show_stats();
        if ace {
            for (i, &s) in m::PRINCESS_ACE.iter().enumerate() {
                self.at(5, 10 + i, s);
            }
            let tm = self.w(slot::TIME_MACHINES);
            if tm < 9 {
                self.set(slot::TIME_MACHINES, tm + 1);
                let land = rules::land_of(self.ow.x, self.ow.y);
                let bx = if land & 1 == 1 { 83 } else { 0 };
                let by = if land & 2 == 2 { 77 } else { 0 };
                for _ in 0..500 {
                    let x = bx + self.r(12, 21);
                    let y = by + self.r(11, 15);
                    if self.a.map.cell(x as usize, y as usize) == 2 && self.ow.entity_at(x, y).is_none() {
                        if self.ow.entities.len() < 39 {
                            self.ow.entities.push(Entity { tile: 0x12, x, y, hp: 0, aux: 2 });
                        }
                        break;
                    }
                }
            }
        }
        io.message_pause(&self.term, 5)?;
        self.wait(io)?;
        self.term.cursor(1, 24);
        Ok(())
    }

    /// Townsperson `i` has moved: the original's step effect, which the
    /// enhanced sound mode plays only for one near the player.
    fn npc_step(&mut self, io: &mut dyn Io, i: usize) {
        let (dx, dy) = (i32::from(self.npcs[i].x) - self.px, i32::from(self.npcs[i].y) - self.py);
        if Sfx::npc_step_heard(dx, dy) {
            self.snd_ev(io, 4, Sfx::NpcStep);
        } else if self.rec.u16_at(off::SOUND) != 0 {
            io.sound_distant(4);
        }
    }

    /// `2203`: a random step of -1, 0 or +1.
    fn rand_step(&mut self) -> i32 {
        match self.r(1, 215) % 3 {
            0 => 0,
            1 => 1,
            _ => -1,
        }
    }

    /// `26e2`: move NPC i by (dx,dy) if the cell is free.
    fn try_move(&mut self, i: usize, dx: i32, dy: i32) -> bool {
        let (x, y) = (i32::from(self.npcs[i].x) + dx, i32::from(self.npcs[i].y) + dy);
        if !Self::in_town(x, y) {
            return false;
        }
        let p = self.passable(x, y);
        if p == FREE || p == SPECIAL {
            self.npcs[i].x = x as i16;
            self.npcs[i].y = y as i16;
            true
        } else {
            false
        }
    }

    /// NPC turn `2a73`.
    pub fn npc_turn(&mut self, io: &mut dyn Io) -> IoResult<()> {
        if self.mode == OUTDOORS {
            return Ok(());
        }
        if self.redraw_town {
            self.redraw_town = false;
            self.redraw();
        }
        if self.mode != self.prev_mode {
            self.draw_town();
            return Ok(());
        }
        let mut attackers = Vec::new();
        for i in 0..self.npcs.len() {
            match self.npcs[i].tile {
                NPC_BARD => self.bard(io, i),
                NPC_WENCH => self.wench(io, i),
                NPC_PRINCESS => self.princess_ai(io, i),
                NPC_GUARD if self.hostile => {
                    if !self.guard(io, i) {
                        attackers.push(i);
                    }
                }
                _ => {}
            }
            if self.mode == OUTDOORS {
                return Ok(());
            }
        }
        self.draw_town();
        if self.hostile {
            for i in attackers {
                if self.hits() <= 0 {
                    break;
                }
                self.guard_attack(io, i);
            }
        }
        Ok(())
    }

    /// Bard / jester `252c`.
    fn bard(&mut self, io: &mut dyn Io, i: usize) {
        if self.hostile {
            return;
        }
        let dx = self.rand_step();
        let dy = self.rand_step();
        let (x, y) = (i32::from(self.npcs[i].x) + dx, i32::from(self.npcs[i].y) + dy);
        let t = if Self::in_town(x, y) { self.passable(x, y) } else { SPECIAL };
        if t == FREE {
            self.npcs[i].x = x as i16;
            self.npcs[i].y = y as i16;
            self.npc_step(io, i);
        } else if t == PLAYER_TILE {
            let ready = self.rec.u16_at(off::READY_WEAPON) as usize;
            let mut stolen = false;
            for k in (1..=15).rev() {
                if self.qty(slot::WEAPON, k) != 0 && k != ready {
                    self.add(slot::WEAPON + 2 * k, -1);
                    stolen = true;
                    break;
                }
            }
            let agi = i32::from(self.stat(AGI));
            if self.r(1, 255) < agi + 128 && stolen {
                self.sayln(if self.mode == TOWN { m::IOLO_STOLE } else { m::JESTER_STOLE });
            }
            if !stolen && self.r(1, 255) < 15 {
                if self.mode == TOWN {
                    self.sayln(m::IOLO_SINGS);
                    self.sayln(m::IOLO_SONG);
                } else {
                    self.sayln(m::JESTER_SINGS);
                    self.sayln(m::JESTER_SONG);
                }
            }
        }
    }

    /// Wench `266c`.
    fn wench(&mut self, io: &mut dyn Io, i: usize) {
        if self.hostile {
            return;
        }
        let dx = self.rand_step();
        let dy = self.rand_step();
        let (x, y) = (i32::from(self.npcs[i].x) + dx, i32::from(self.npcs[i].y) + dy);
        if Self::in_town(x, y) && self.passable(x, y) == FREE {
            self.npcs[i].x = x as i16;
            self.npcs[i].y = y as i16;
            self.npc_step(io, i);
        }
    }

    /// Princess `2776`: wanders, and once the castle is roused follows
    /// the player.
    fn princess_ai(&mut self, io: &mut dyn Io, i: usize) {
        if !self.hostile {
            self.wench(io, i);
            return;
        }
        let sx = (self.px - i32::from(self.npcs[i].x)).signum();
        let sy = (self.py - i32::from(self.npcs[i].y)).signum();
        let moved = if self.r(1, 100) < 50 {
            (sy != 0 && self.try_move(i, 0, sy)) || (sx != 0 && self.try_move(i, sx, 0))
        } else {
            (sx != 0 && self.try_move(i, sx, 0)) || (sy != 0 && self.try_move(i, 0, sy))
        };
        if moved {
            self.npc_step(io, i);
        }
    }

    /// Guard `286b`; false means "in reach, attack".
    fn guard(&mut self, io: &mut dyn Io, i: usize) -> bool {
        let dx = i32::from(self.npcs[i].x) - self.px;
        let dy = i32::from(self.npcs[i].y) - self.py;
        if dx.abs() < 2 && dy.abs() < 2 {
            return false;
        }
        if dx.abs() + dy.abs() >= 13 {
            return false;
        }
        let (sx, sy) = (-dx.signum(), -dy.signum());
        let mut moved = sx != 0 && self.try_move(i, sx, 0);
        if !moved && sy != 0 {
            moved = self.try_move(i, 0, sy);
        }
        if moved {
            self.npc_step(io, i);
        }
        moved
    }

    /// `294e`: a guard next to the player strikes.
    fn guard_attack(&mut self, io: &mut dyn Io, i: usize) {
        let dx = i32::from(self.npcs[i].x) - self.px;
        let dy = i32::from(self.npcs[i].y) - self.py;
        if dx.abs() >= 2 || dy.abs() >= 2 {
            return;
        }
        self.sayln(m::ATTACKED_BY_GUARD);
        self.snd(io, 7);
        let armour = i32::from(self.rec.u16_at(off::READY_ARMOUR));
        let sta = i32::from(self.stat(STA));
        if self.r(1, 255) < sta / 2 + 8 * armour + 56 {
            self.cue(io, Sfx::Dodge);
            self.sayln(m::MISSED);
            return;
        }
        let hits = i32::from(self.hits() as u16);
        let d = self.r(2, hits / 128 + 15);
        self.say(m::HIT_DOTS);
        self.term.num(d as u16, 1, ' ', TEXT, true);
        self.sayln(m::DAMAGE);
        self.snd_ev(io, 2, Sfx::Hurt);
        self.add(off::HITS, -(d as i16));
        self.show_hits();
    }

    /// Player attack in a town or castle `2b6d`.
    pub fn town_attack(&mut self, io: &mut dyn Io, d: Dir, range: i32, dmg: i32, hit: i32) {
        self.snd(io, 7);
        let (dx, dy) = d.delta();
        let (mut x, mut y) = (self.px, self.py);
        for _ in 0..range {
            x += dx;
            y += dy;
            let occ = self.occupant(x, y);
            let mut free = self.passable(x, y);
            if free == SPECIAL {
                free = FREE;
            }
            if matches!(occ, 0x11 | 0x13 | 0x14 | 0x15 | 0x16 | 0x32) || free != FREE {
                break;
            }
        }
        let idx = self.npcs.iter().position(|n| n.tile != -1 && i32::from(n.x) == x && i32::from(n.y) == y);
        let Some(i) = idx else {
            self.cue(io, Sfx::Miss);
            self.sayln(m::MISSED);
            return;
        };
        if self.r(1, 100) > hit {
            self.cue(io, Sfx::Miss);
            self.sayln(m::MISSED);
            return;
        }
        let tile = self.npcs[i].tile;
        self.say(m::HIT_SP);
        let name = match tile {
            NPC_KING => m::NPC_KING,
            NPC_GUARD => m::NPC_GUARD,
            NPC_MERCHANT => m::NPC_MERCHANT,
            NPC_PRINCESS => m::NPC_PRINCESS,
            NPC_BARD if self.mode == TOWN => m::NPC_BARD,
            NPC_BARD => m::NPC_JESTER,
            _ if self.rec.sex() == 0 => m::NPC_WENCH,
            _ => m::NPC_LECHER,
        };
        self.say(name);
        self.hostile = true;
        self.snd_ev(io, 2, Sfx::Alarm);
        if dmg >= i32::from(self.npcs[i].hp) {
            self.sayln(m::KILLED);
            if tile == NPC_PRINCESS {
                self.door_open = false;
            } else {
                self.add(off::EXPERIENCE, 1);
                self.show_exp();
            }
            self.npcs[i].tile = -1;
            self.npcs[i].x = 0x7F;
            self.draw_town();
            if tile == NPC_GUARD {
                // The original adds the guard bonus without redrawing the
                // stats, so the shown experience lags by 14 until the next
                // redraw (seen in DOSBox).
                self.add(off::EXPERIENCE, 14);
            }
            if tile == NPC_BARD && self.mode == CASTLE {
                self.sayln(m::FOUND_KEY);
                self.has_key = true;
            }
        } else {
            if !(tile == NPC_KING && self.lord_british) {
                self.npcs[i].hp -= dmg as i16;
            }
            self.term.num(dmg as u16, 2, ' ', TEXT, true);
            self.sayln(m::DAMAGE);
        }
    }

    // ------------------------------------------------------------ steal / get

    /// Steal `5db1`.
    pub fn cmd_steal(&mut self, io: &mut dyn Io) {
        self.say(m::STEAL);
        if self.mode != TOWN && self.mode != CASTLE {
            self.err(io, m::QUESTION);
            return;
        }
        let c = self.tcell(self.px, self.py);
        if !matches!(c, 0x37 | 0x39 | 0x3B) {
            self.err(io, m::NOTHING_HERE);
            return;
        }
        let r = self.r(1, 255);
        let caught = if self.hostile || r < 38 {
            true
        } else if self.rec.class() == 3 {
            false
        } else {
            r <= 77
        };
        if caught {
            self.nl();
            self.sayln(m::CAUGHT);
            self.hostile = true;
            return;
        }
        self.find_item(c);
    }

    /// The loot for counter cell `c` (armour 0x37, food 0x39, weapon 0x3B).
    fn find_item(&mut self, c: u8) {
        self.nl();
        match c {
            0x3B => {
                let k = self.r(1, 15) as usize;
                self.say(m::FIND_A);
                let name = self.ptr(m::WEAPON_LONG, k);
                if name.starts_with(['A', 'E', 'I', 'O', 'U', 'a', 'e', 'i', 'o', 'u']) {
                    self.term.print("n");
                }
                if name.len() > 8 {
                    self.nl();
                } else {
                    self.term.print(" ");
                }
                self.term.println(&name);
                if self.qty(slot::WEAPON, k) < 255 {
                    self.add(slot::WEAPON + 2 * k, 1);
                }
            }
            0x39 => {
                let n = self.r(2, 31);
                self.say(m::FIND);
                self.term.num(n as u16, 1, ' ', TEXT, true);
                self.sayln(m::BAGS_OF);
                self.sayln(m::FOOD_EXCL);
                self.add(off::FOOD, n as i16);
                self.show_food();
            }
            _ => {
                let k = self.r(1, 5) as usize;
                self.say(m::FIND);
                if k == 5 {
                    self.say(m::A_SP);
                }
                let name = self.ptr(m::ARMOUR_NAMES, k);
                self.term.println(&name);
                if self.qty(slot::ARMOUR, k) < 255 {
                    self.add(slot::ARMOUR + 2 * k, 1);
                }
            }
        }
    }

    /// Get in a castle `54fb`: the storeroom, with the king's leave.
    pub fn castle_get(&mut self, io: &mut dyn Io) {
        let c = self.tcell(self.px, self.py);
        if !matches!(c, 0x37 | 0x39 | 0x3B) {
            self.err(io, m::NOTHING_HERE);
            return;
        }
        if self.storeroom <= 0 {
            self.nl();
            self.sayln(m::NOT_KINGS);
            self.err(io, m::PERMISSION);
            return;
        }
        self.storeroom -= 1;
        self.find_item(c);
    }

    // ------------------------------------------------------------ unlock

    /// Unlock `7cf2` / `7c31`.
    pub fn cmd_unlock(&mut self, io: &mut dyn Io) {
        self.say(m::UNLOCK);
        match self.mode {
            CASTLE => {}
            crate::dgame::DUNGEON => {
                self.dungeon_unlock(io);
                return;
            }
            TOWN => {
                self.err(io, m::SP_WHAT);
                return;
            }
            _ => {
                self.err(io, m::QUESTION);
                return;
            }
        }
        let c = self.tcell(self.px, self.py);
        if c != 0x3C && c != 0x3D {
            self.err(io, m::SP_WHAT);
            return;
        }
        self.nl();
        if !self.has_key {
            self.sayln(m::NO_KEY);
        } else if c != self.jail_door {
            self.sayln(m::WRONG_KEY[0]);
            self.sayln(m::WRONG_KEY[1]);
        } else {
            self.sayln(m::DOOR_OPEN);
            self.door_open = true;
            let (x, y) = (self.px as usize, self.py as usize - 1);
            self.maps[self.layout].cells[y * TCD_WIDTH + x] = 1;
            self.draw_town();
        }
    }

    // ------------------------------------------------------------ drop

    /// Drop `204d`.
    pub fn cmd_drop(&mut self, io: &mut dyn Io) -> IoResult<()> {
        if self.mode != TOWN && self.mode != CASTLE {
            self.err(io, m::DROP_Q);
            return Ok(());
        }
        self.say(m::DROP_MENU);
        let k = self.key_upper(io)?;
        self.term.fill(40, 192, 230, 199, BLACK);
        self.term.cursor(6, 24);
        match k {
            b'P' => self.drop_pence(io)?,
            b'W' => {
                self.say(m::DROP_WEAPON);
                self.drop_list(io, slot::WEAPON, m::WEAPON_LONG, 15, off::READY_WEAPON, 15)?;
            }
            b'A' => {
                self.say(m::DROP_ARMOUR);
                self.drop_list(io, slot::ARMOUR, m::ARMOUR_NAMES, 5, off::READY_ARMOUR, 13)?;
            }
            _ => self.say(m::NOTHING),
        }
        self.nl();
        Ok(())
    }

    fn drop_pence(&mut self, io: &mut dyn Io) -> IoResult<()> {
        self.say(m::PENCE_PROMPT);
        let n = self.number(io, 12, 24)?;
        if n == 0 {
            self.term.cursor(13, 24);
            self.say(m::NONE);
            return Ok(());
        }
        self.nl();
        if n > self.coin() {
            self.say(m::NOT_THAT_MUCH);
            self.snd(io, 1);
            return Ok(());
        }
        self.add(off::COIN, -n);
        self.show_coin();
        let c = self.tcell(self.px, self.py);
        if self.mode == CASTLE {
            if matches!(c, 0x33..=0x35) {
                self.say(m::SHAZAM);
                self.snd(io, 5);
                match c {
                    0x33 => {
                        let k = self.r(1, 6) as usize;
                        let v = (self.stat(k) + n / 10).min(99);
                        self.set_stat(k, v);
                    }
                    0x34 => {
                        if n >= 10 {
                            let k = self.r(1, 15) as usize;
                            self.add(slot::WEAPON + 2 * k, 1);
                        }
                    }
                    _ => {
                        self.add(off::FOOD, n.wrapping_mul(3));
                        self.show_food();
                    }
                }
            } else {
                self.say(m::OK);
            }
        } else if c == 0x33 {
            self.add(off::HITS, (n as i32 * 3 / 2) as i16);
            self.show_hits();
            if self.r(1, 255) > 16 {
                self.say(m::SHAZAM);
            } else {
                let mut k = self.r(1, 7) as usize;
                if k == 3 {
                    k = 4;
                }
                self.add(slot::SPELL + 2 * (k + 1), 1);
                self.say(m::ALAKAZOT);
            }
            self.snd(io, 5);
        } else {
            self.say(m::OK);
        }
        Ok(())
    }

    fn drop_list(&mut self, io: &mut dyn Io, base: usize, names: usize, last: usize, ready: usize, col: usize) -> IoResult<()> {
        self.term.clear_view();
        self.term.ornament_left(16, 0);
        self.say(m::DROP_TITLE);
        self.term.ornament_right(23, 0);
        let have: Vec<usize> = (1..=last).filter(|&k| self.qty(base, k) != 0).collect();
        let row0 = 10 - have.len() / 2;
        for (n, &k) in have.iter().enumerate() {
            let name = self.ptr(names, k);
            let p = self.s(m::PAREN);
            self.term.print_at(col, row0 + n, &format!("{}{p}{name}", (b'a' + k as u8) as char));
        }
        self.term.cursor(14, 24);
        let key = self.key(io)?.to_ascii_lowercase();
        let k = key.wrapping_sub(b'a') as usize;
        if (1..=last).contains(&k) && self.qty(base, k) != 0 {
            let name = self.ptr(names, k);
            self.term.print(&name);
            self.add(base + 2 * k, -1);
            if self.rec.u16_at(ready) as usize == k && self.qty(base, k) == 0 {
                self.rec.set_u16(ready, 0);
            }
        } else {
            self.say(m::NOTHING);
        }
        self.term.restore_border(127, 199);
        self.redraw();
        Ok(())
    }
}
