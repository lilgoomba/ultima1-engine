//! Shops, the pub and the kings (SPEC §9; verified from original
//! executable). Prices come from [`crate::rules`]; every message and
//! title is read from the user's OUT.EXE.

use crate::exe::m;
use crate::game::*;
use crate::io::{Io, IoResult};
use crate::overworld::Entity;
use crate::rules;
use crate::save::off;
use crate::term::{BLUE, TEXT};

type Title = [&'static [(usize, usize, usize)]; 8];

impl Game {
    /// Transact `7b17`.
    pub fn cmd_transact(&mut self, io: &mut dyn Io) -> IoResult<()> {
        match self.mode {
            TOWN => self.shop(io),
            CASTLE => self.king(io),
            _ => {
                self.err(io, m::TRANSACT_Q);
                Ok(())
            }
        }
    }

    /// `5e6c` / `5e88` / `5ea4`.
    fn sold(&mut self, io: &mut dyn Io) -> IoResult<()> {
        self.at(18, 9, m::SOLD);
        self.wait(io)
    }
    fn done(&mut self, io: &mut dyn Io) -> IoResult<()> {
        self.at(18, 9, m::DONE);
        self.wait(io)
    }
    fn cant_afford(&mut self, io: &mut dyn Io) -> IoResult<()> {
        self.at(8, 9, m::CANT_AFFORD);
        self.snd(io, 1);
        self.wait(io)
    }

    /// Box and shop name; returns the first list row.
    fn title(&mut self, t: &Title, idx0: usize, idx7: bool) -> usize {
        let i = self.layout - 2;
        self.term.boxed(BLUE);
        for &(c, r, o) in t[i] {
            self.at(c, r, o);
        }
        if i == 0 || (idx7 && i == 7) { idx0 } else { 6 }
    }

    /// `7377`: the counter the player stands at.
    fn shop(&mut self, io: &mut dyn Io) -> IoResult<()> {
        self.say(m::TRANSACT);
        if self.hostile {
            self.nl();
            self.sayln(m::NONE_WILL_TALK);
            return Ok(());
        }
        let c = self.tcell(self.px, self.py);
        if !(0x36..=0x3E).contains(&c) {
            self.nl();
            self.err(io, m::NOT_BY_COUNTER);
            return Ok(());
        }
        self.say(m::BUY_SELL);
        let k = self.key_upper(io)?;
        let buy = match k {
            b'B' => {
                self.term.cursor(13, 24);
                self.say(m::BLANK9);
                self.term.cursor(13, 24);
                self.say(m::COLON);
                true
            }
            b'S' => {
                self.term.cursor(10, 24);
                self.say(m::BLANK12);
                self.term.cursor(10, 24);
                self.say(m::SELL);
                false
            }
            _ => {
                self.term.cursor(9, 24);
                self.sayln(m::SP_NOTHING);
                return Ok(());
            }
        };
        self.redraw_town = true;
        match c {
            0x36 | 0x37 => self.armour_shop(io, buy)?,
            0x38 | 0x39 => self.food_shop(io, buy)?,
            0x3A | 0x3B => self.weapon_shop(io, buy)?,
            0x3C => self.magic_shop(io, buy)?,
            0x3D => self.pub_(io, buy)?,
            _ => self.transport_shop(io, buy)?,
        }
        Ok(())
    }

    /// A priced list: rows from `row`, `letter) name` at col 9 and
    /// `- price` at col 26.
    fn price_list(&mut self, row: usize, items: &[(usize, i16)], names: usize) {
        for (n, &(k, price)) in items.iter().enumerate() {
            let name = self.ptr(names, k);
            let p = self.s(m::PAREN);
            self.term.print_at(9, row + n, &format!("{}{p}{name}", (b'a' + k as u8) as char));
            let d = self.s(m::DASH);
            self.term.print_at(26, row + n, &d);
            self.term.num(price as u16, 3, ' ', TEXT, true);
        }
    }

    /// Buying from a list: key at (15,24).
    fn buy_item(&mut self, io: &mut dyn Io, k: Option<usize>, price: i16, base: usize, names: usize) -> IoResult<()> {
        let Some(k) = k else {
            self.say(m::NOTHING);
            return Ok(());
        };
        if price > self.coin() {
            self.say(m::NOTHING);
            return self.cant_afford(io);
        }
        self.add(off::COIN, -price);
        if self.qty(base, k) < 255 {
            self.add(base + 2 * k, 1);
        }
        let name = self.ptr(names, k);
        self.term.print(&name);
        self.show_coin();
        self.term.cursor(15 + name.len(), 24);
        self.sold(io)
    }

    fn weapon_shop(&mut self, io: &mut dyn Io, buy: bool) -> IoResult<()> {
        let row = self.title(&m::TITLE_WEAPON, 7, false);
        if !buy {
            return self.sell(io, row, slot::WEAPON, m::WEAPON_LONG, 15, off::READY_WEAPON, m::NO_WEAPONS_TO_SELL, 0);
        }
        let int = self.stat(INT);
        let stock = rules::weapon_stock(self.layout as u32, self.moves());
        let items: Vec<(usize, i16)> = stock.iter().map(|&k| (k as usize, rules::weapon_buy_price(k as i16, int))).collect();
        self.price_list(row, &items, m::WEAPON_LONG);
        self.term.cursor(15, 24);
        let key = self.key(io)?.to_ascii_lowercase();
        let k = key.wrapping_sub(b'a') as usize;
        let first = items.first().map_or(1, |i| i.0);
        let last = items.last().map_or(0, |i| i.0);
        let ok = (first..=last).contains(&k) && (k & 1) == (self.layout & 1);
        let price = rules::weapon_buy_price(k as i16, int);
        self.buy_item(io, ok.then_some(k), price, slot::WEAPON, m::WEAPON_LONG)?;
        self.nl();
        Ok(())
    }

    fn armour_shop(&mut self, io: &mut dyn Io, buy: bool) -> IoResult<()> {
        let row = self.title(&m::TITLE_ARMOUR, 7, false);
        if !buy {
            return self.sell(io, row, slot::ARMOUR, m::ARMOUR_NAMES, 5, off::READY_ARMOUR, m::NO_ARMOUR_TO_SELL, 1);
        }
        let int = self.stat(INT);
        let stock = rules::armour_stock(self.moves());
        let items: Vec<(usize, i16)> = stock.iter().map(|&k| (k as usize, rules::armour_buy_price(k as i16, int))).collect();
        self.price_list(row, &items, m::ARMOUR_NAMES);
        self.term.cursor(15, 24);
        let key = self.key(io)?.to_ascii_lowercase();
        let k = key.wrapping_sub(b'a') as usize;
        let ok = (1..=items.len()).contains(&k);
        let price = rules::armour_buy_price(k as i16, int);
        self.buy_item(io, ok.then_some(k), price, slot::ARMOUR, m::ARMOUR_NAMES)?;
        self.nl();
        Ok(())
    }

    /// Selling weapons (`63f2`) or armour (`6823`).
    #[allow(clippy::too_many_arguments)]
    fn sell(&mut self, io: &mut dyn Io, row: usize, base: usize, names: usize, last: usize, ready: usize, none: usize, row_off: usize) -> IoResult<()> {
        let cha = self.stat(CHA);
        let price = |k: usize| if base == slot::WEAPON { rules::weapon_sell_price(k as i16, cha) } else { rules::armour_sell_price(k as i16, cha) };
        let have: Vec<usize> = (1..=last).filter(|&k| self.qty(base, k) != 0).take(8).collect();
        if have.is_empty() {
            self.at(5, row + 1, none);
            self.wait(io)?;
            self.term.cursor(16, 24);
            self.sayln(m::NOTHING);
            return Ok(());
        }
        let items: Vec<(usize, i16)> = have.iter().map(|&k| (k, price(k))).collect();
        self.price_list(row + row_off, &items, names);
        self.term.cursor(16, 24);
        let key = self.key_upper(io)?;
        let k = key.wrapping_sub(b'A') as usize;
        if (1..=last).contains(&k) && self.qty(base, k) != 0 {
            let p = price(k);
            self.add(off::COIN, p);
            self.add(base + 2 * k, -1);
            if self.rec.u16_at(ready) as usize == k {
                self.rec.set_u16(ready, 0);
            }
            let name = self.ptr(names, k);
            self.term.print(&name);
            self.show_coin();
            self.term.cursor(16 + name.len(), 24);
            self.done(io)?;
        } else {
            self.say(m::NOTHING);
        }
        self.nl();
        Ok(())
    }

    fn magic_shop(&mut self, io: &mut dyn Io, buy: bool) -> IoResult<()> {
        let row = self.title(&m::TITLE_MAGIC, 6, false);
        if !buy {
            self.at(5, row + 2, m::NO_SPELLS_BOUGHT);
            self.wait(io)?;
            self.term.cursor(16, 24);
            self.sayln(m::NOTHING);
            return Ok(());
        }
        let wis = self.stat(WIS);
        let wizard = self.rec.class() == 2;
        let stock = rules::spell_stock(self.layout as u32, wizard);
        let items: Vec<(usize, i16)> = stock.iter().map(|&k| (k as usize, rules::spell_price(k as i16, wis))).collect();
        self.price_list(row, &items, m::SPELL_NAMES);
        self.term.cursor(15, 24);
        let key = self.key(io)?.to_ascii_lowercase();
        let k = key.wrapping_sub(b'a') as usize;
        let p = self.layout & 1;
        let last = if wizard { 9 + p } else { 5 + p };
        let ok = (1 + p..=last).contains(&k) && (k & 1) != p;
        let price = rules::spell_price(k as i16, wis);
        self.buy_item(io, ok.then_some(k), price, slot::SPELL, m::SPELL_NAMES)?;
        self.nl();
        Ok(())
    }

    fn food_shop(&mut self, io: &mut dyn Io, buy: bool) -> IoResult<()> {
        let row = self.title(&m::TITLE_FOOD, 6, false);
        if !buy {
            self.at(9, row + 2, m::USED_FOOD);
            self.wait(io)?;
            self.term.cursor(15, 24);
            self.sayln(m::NOTHING);
            return Ok(());
        }
        let price = rules::food_price_per_10(self.stat(INT));
        self.term.cursor(5, row + 2);
        self.say(m::PACKS_COST);
        self.term.num(price as u16, 1, ' ', TEXT, true);
        self.say(m::PENCE);
        self.at(8, row + 3, m::EACH_HOW_MANY);
        self.at(11, row + 4, m::WISH_PURCHASE);
        let n = self.number(io, 15, 24)?;
        let mut cost = price.wrapping_mul(n);
        if cost < 0 {
            cost = self.coin().wrapping_add(1);
        }
        if n == 0 || cost > self.coin() {
            self.term.cursor(15, 24);
            self.say(m::NOTHING);
            if cost > self.coin() {
                self.cant_afford(io)?;
            }
        } else {
            self.term.cursor(15 + digits(n), 24);
            self.say(m::PACKS_FOOD);
            self.add(off::COIN, -cost);
            self.add(off::FOOD, n.wrapping_mul(10));
            self.show_coin();
            self.show_food();
            self.term.cursor(15 + digits(n) + self.s(m::PACKS_FOOD).len(), 24);
            self.sold(io)?;
        }
        self.nl();
        Ok(())
    }

    fn transport_shop(&mut self, io: &mut dyn Io, buy: bool) -> IoResult<()> {
        let row = self.title(&m::TITLE_TRANSPORT, 7, true);
        if !buy {
            self.at(9, row + 2, m::NO_USED_STUFF[0]);
            self.at(15, row + 3, m::NO_USED_STUFF[1]);
            self.wait(io)?;
            self.term.cursor(16, 24);
            self.sayln(m::NOTHING);
            return Ok(());
        }
        let shuttle = self.w(off::SHUTTLE_0) != 0;
        let objs = self.vehicle_objs;
        let (w, g, f) = (self.water, self.grass, self.forest);
        if (w == 0 && g == 0 && f == 0) || (shuttle && objs == 15) || (g == 0 && objs == 15) {
            self.at(11, row + 2, m::CLOSED);
            self.wait(io)?;
            self.nl();
            return Ok(());
        }
        let full = !shuttle && objs == 15;
        let int = self.stat(INT);
        let mut ok = [false; 7];
        let mut r = row;
        let line = |g: &mut Game, k: usize, r: &mut usize, advance: bool| {
            let name = g.ptr(m::TRANSPORT_NAMES, k);
            let p = g.s(m::PAREN);
            g.term.print_at(12, *r, &format!("{}{p}{name}", (b'A' + k as u8 - 1) as char));
            let d = g.s(m::DASH);
            g.term.print_at(23, *r, &d);
            g.term.num(rules::transport_price(k as i16, int) as u16, 4, ' ', TEXT, false);
            if advance {
                *r += 1;
            }
        };
        if (f > 0 || g > 0) && !full {
            for k in [1, 2] {
                line(self, k, &mut r, true);
                ok[k] = true;
            }
        }
        if w > 0 && !full {
            for k in [3, 4] {
                line(self, k, &mut r, true);
                ok[k] = true;
            }
        }
        if self.moves() > 3000 && g > 0 {
            if !full {
                line(self, 5, &mut r, true);
                ok[5] = true;
            }
            if !shuttle {
                line(self, 6, &mut r, false);
            }
            ok[6] = true;
        }
        self.term.cursor(16, 24);
        let key = self.key_upper(io)?;
        let k = key.wrapping_sub(b'A' - 1) as usize;
        if !(1..=6).contains(&k) || !ok[k] {
            self.say(m::NOTHING);
            self.nl();
            return Ok(());
        }
        let price = rules::transport_price(k as i16, int);
        if price > self.coin() {
            self.say(m::NOTHING);
            self.cant_afford(io)?;
            self.nl();
            return Ok(());
        }
        let name = self.ptr(m::TRANSPORT_NAMES, k);
        self.term.print(&name);
        self.add(off::COIN, -price);
        self.show_coin();
        self.vehicle_objs += 1;
        self.term.cursor(16 + name.len(), 24);
        self.sold(io)?;
        if k == 6 {
            self.set(off::SHUTTLE_0, 1000);
            self.set(off::SHUTTLE_1, 1000);
        }
        self.place_vehicle(k);
        self.nl();
        Ok(())
    }

    /// `6f58`: the new vehicle appears next to the town.
    fn place_vehicle(&mut self, k: usize) {
        self.add(slot::VEHICLE + 2 * k, 1);
        let terrain = if k <= 2 && self.forest > 0 {
            self.forest -= 1;
            2
        } else if k == 3 || k == 4 {
            self.water -= 1;
            0
        } else {
            self.grass -= 1;
            1
        };
        let (mut dx, mut dy) = (2, 2);
        'scan: for y in -1..=1 {
            for x in -1..=1 {
                if self.cell_or_object(self.ow.x + x, self.ow.y + y) == terrain {
                    (dx, dy) = (x, y);
                    break 'scan;
                }
            }
        }
        let tile = (k + 0x0A + usize::from(k > 4)) as u8;
        let (x, y) = rules::wrap(self.ow.x + dx, self.ow.y + dy);
        if self.ow.entities.len() < 39 {
            self.ow.entities.push(Entity { tile, x, y, hp: 0, aux: u16::from(terrain) });
        }
    }

    // ------------------------------------------------------------ pub

    fn wench_adjacent(&self) -> bool {
        (-1..=1).any(|dx| (-1..=1).any(|dy| (dx, dy) != (0, 0) && self.occupant(self.px + dx, self.py + dy) == NPC_WENCH))
    }

    /// `5ec9`: a pub tip in the box.
    fn tip(&mut self, io: &mut dyn Io, n: usize) -> IoResult<()> {
        self.term.fill(39, 48, 280, 120, 0);
        if n == 0 {
            for (c, r, o) in m::SEDUCED {
                self.at(c, r, o);
            }
        } else {
            self.at(11, 7, m::BEST_KNOW);
            for &(c, r, o) in m::TIPS[n - 1] {
                self.at(c, r, o);
            }
            if n == 2 {
                self.say(if self.rec.sex() == 0 { m::TIP_WENCH } else { m::TIP_LECHER });
            }
            if n == 8 {
                self.key(io)?;
                self.term.fill(39, 48, 280, 120, 0);
                for (c, r, o) in m::QUEST_TEXT {
                    self.at(c, r, o);
                }
                self.key(io)?;
                return Ok(());
            }
        }
        self.wait(io)
    }

    /// Pub `622c`.
    fn pub_(&mut self, io: &mut dyn Io, buy: bool) -> IoResult<()> {
        let b = self.title(&m::TITLE_PUB, 6, false);
        if !buy {
            self.at(8, b + 2, m::PLENTY[0]);
            self.at(17, b + 3, m::PLENTY[1]);
            self.wait(io)?;
            self.term.cursor(16, 24);
            self.say(m::NOTHING);
            self.nl();
            return Ok(());
        }
        if self.coin() == 0 {
            self.at(12, b + 1, m::BROKE[0]);
            self.at(8, b + 4, m::BROKE[1]);
            self.at(10, b + 5, m::BROKE[2]);
            self.wait(io)?;
            self.term.cursor(15, 24);
            self.say(m::NOTHING);
            self.nl();
            return Ok(());
        }
        self.add(off::COIN, -1);
        self.show_coin();
        self.at(8, b + 1, m::KEEPER);
        self.at(9, b + 4, m::COLD_ONE);
        self.term.cursor(15, 24);
        self.say(m::ALE);
        self.ale += 1;
        self.wait(io)?;
        if self.ale > self.stat(STA) / 4 && self.wench_adjacent() {
            self.tip(io, 0)?;
            let c = self.coin() / 2;
            self.set(off::COIN, c);
            self.show_coin();
            let mut wis = self.stat(WIS) - 1;
            if wis == 4 {
                wis = 5;
            }
            self.set_stat(WIS, wis);
        }
        if self.r(1, 255) < 75 {
            let n = (self.r(11, 89) / 10) as usize;
            self.tip(io, n)?;
        }
        self.nl();
        Ok(())
    }

    // ------------------------------------------------------------ kings

    /// King `794e`.
    fn king(&mut self, io: &mut dyn Io) -> IoResult<()> {
        self.sayln(m::TRANSACT_KING);
        if self.tcell(self.px, self.py) != 0x3E {
            self.sayln(m::NOT_HERE);
            return Ok(());
        }
        if self.hostile {
            self.sayln(m::REJECTS);
            return Ok(());
        }
        if self.quest_done(io)? {
            return Ok(());
        }
        self.sayln(m::OFFER[0]);
        self.say(m::OFFER[1]);
        let k = self.key_upper(io)?;
        match k {
            b'P' => {
                self.sayln(m::PENCE_WORD);
                self.say(m::HOW_MUCH);
                let n = self.number(io, 11, 24)?;
                if n == 0 {
                    self.term.cursor(11, 24);
                    self.sayln(m::NONE);
                } else if n > self.coin() {
                    self.nl();
                    self.sayln(m::NOT_THAT_MUCH);
                    self.snd(io, 1);
                } else {
                    self.add(off::COIN, -n);
                    let hp = n.wrapping_mul(3) / 2;
                    self.nl();
                    self.sayln(m::IN_RETURN);
                    self.say(m::THEE);
                    self.term.num(hp as u16, 1, ' ', TEXT, true);
                    self.sayln(m::HIT_POINTS);
                    self.add(off::HITS, hp);
                    self.show_hits();
                    self.show_coin();
                }
            }
            b'S' => {
                let q = self.quest_slot;
                if self.w(slot::QUEST + 2 * q) == -1 {
                    self.nl();
                    self.sayln(m::ON_QUEST[0]);
                    self.sayln(m::ON_QUEST[1]);
                } else {
                    self.give_quest(io)?;
                }
            }
            _ => self.sayln(m::NEITHER),
        }
        Ok(())
    }

    /// `7526`.
    fn give_quest(&mut self, io: &mut dyn Io) -> IoResult<()> {
        let q = self.quest_slot;
        self.term.boxed(BLUE);
        self.set(slot::QUEST + 2 * q, -1);
        if q % 2 == 0 {
            self.at(12, 6, m::GO_KILL);
            self.term.cursor(12, 8);
            self.say(m::KILL_TARGETS[q / 2 - 1]);
        } else {
            self.at(11, 6, m::GO_FIND);
            let (ptr, col) = m::QUEST_PLACES[q / 2];
            let o = self.a.exe.w(ptr) as usize;
            self.at(col, 8, o);
        }
        self.at(11, 10, m::DO_NOT_RETURN[0]);
        self.at(11, 11, m::DO_NOT_RETURN[1]);
        self.press_space(io)?;
        self.redraw_town = true;
        Ok(())
    }

    /// `78af`: a finished quest is rewarded.
    fn quest_done(&mut self, io: &mut dyn Io) -> IoResult<bool> {
        let q = self.quest_slot;
        if self.w(slot::QUEST + 2 * q) != 1 {
            return Ok(false);
        }
        self.set(slot::QUEST + 2 * q, 0);
        let name = self.rec.name();
        let col = 7 - name.len() / 2;
        self.term.boxed(BLUE);
        self.at(col + 5, 5, m::AH);
        self.term.print(&name);
        self.say(m::THOU_HAST);
        self.at(10, 6, m::COMPLETED);
        if q % 2 == 1 {
            let s = self.stat(STR);
            let mut n = (99 - s) / 8;
            if n == 0 && s != 99 {
                n = 1;
            }
            self.at(8, 8, m::WILL_GIVE);
            self.term.cursor(10, 9);
            self.term.num(n as u16, 1, ' ', TEXT, true);
            self.say(m::POINT);
            if n != 1 {
                self.term.print("s");
            }
            self.say(m::OF_STRENGTH);
            self.set_stat(STR, s + n);
        } else {
            self.at(7, 8, m::SHALL_TELL);
            let land = rules::land_of(self.ow.x, self.ow.y) as usize;
            for (c, r, o) in m::GEM_LINES[land] {
                if o != 0 {
                    self.at(c, r, o);
                }
            }
            self.add(slot::GEM + 2 * land, 1);
            if land == 3 {
                self.storeroom = 9;
            }
            io.pause(&self.term, 10)?;
        }
        self.press_space(io)?;
        self.redraw_town = true;
        Ok(true)
    }
}

fn digits(n: i16) -> usize {
    n.to_string().len()
}
