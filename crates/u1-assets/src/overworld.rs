//! Outdoor game state and turn logic (SPEC §8). No SDL here: the `u1`
//! binary drives this and draws it, and the tests drive it directly.

use std::collections::VecDeque;

use crate::locations::{kind_of, LocationKind, LocationTable};
use crate::map::WorldMap;
use crate::rng::U1Rng;
use crate::rules::{self, Dir, ExeTables, FoodClock, MoveBlock, MAP_H, MAP_W};
use crate::save::{off, PlayerRecord};

/// Tile drawn for the player on foot.
pub const PLAYER_ON_FOOT: u8 = 0x0A;
/// Most monsters that may be on the map before spawning stops (spawn happens
/// while the count is <= this).
pub const MAX_MONSTERS: usize = 10;
/// Lines kept in the message area.
pub const MESSAGE_LINES: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entity {
    pub tile: u8,
    pub x: i32,
    pub y: i32,
    pub hp: u16,
}

impl Entity {
    pub fn is_monster(&self) -> bool {
        rules::is_monster_tile(self.tile)
    }
}

/// Transport number for a vehicle tile (0x0B..=0x12), as boarding sets it.
pub fn transport_for_tile(tile: u8) -> Option<u16> {
    match tile {
        0x0B..=0x12 => {
            let t = u16::from(tile - 0x0B) + 1;
            Some(if t > 4 { t - 1 } else { t })
        }
        _ => None,
    }
}

/// Vehicle tile for a transport number 1..=7.
pub fn tile_for_transport(t: u16) -> u8 {
    match t {
        0 => PLAYER_ON_FOOT,
        1..=4 => 0x0A + t as u8,
        n => 0x0B + n as u8,
    }
}

#[derive(Debug, Clone)]
pub struct Overworld {
    pub x: i32,
    pub y: i32,
    pub transport: u16,
    pub hits: u16,
    pub food: u16,
    pub exp: u16,
    pub coin: u16,
    pub stats: [u16; 6],
    pub armour: u16,
    pub moves: u32,
    pub food_clock: FoodClock,
    /// Vehicles and monsters (object slots other than the player).
    pub entities: Vec<Entity>,
    pub messages: VecDeque<String>,
    pub rng: U1Rng,
    pub dead: bool,
}

impl Overworld {
    /// A fresh character standing at (x,y) with the stock starting values.
    pub fn new_at(x: i32, y: i32, seed: u16) -> Self {
        Overworld {
            x,
            y,
            transport: 0,
            hits: 150,
            food: 200,
            exp: 0,
            coin: 100,
            stats: [10; 6],
            armour: 0,
            moves: 0,
            food_clock: FoodClock::default(),
            entities: Vec::new(),
            messages: VecDeque::new(),
            rng: U1Rng::new(seed),
            dead: false,
        }
    }

    /// State from a saved PLAYERn.U1 record. Object 0 is the player; the
    /// rest become vehicles and monsters.
    pub fn from_save(p: &PlayerRecord, seed: u16) -> Self {
        let (x, y) = p.position();
        let mut o = Overworld::new_at(i32::from(x), i32::from(y), seed);
        o.transport = p.transport();
        o.hits = p.hits();
        o.food = p.food();
        o.exp = p.experience();
        o.coin = p.coin();
        o.stats = p.stats();
        o.armour = p.u16_at(off::READY_ARMOUR);
        o.moves = p.moves();
        o.entities = p
            .objects()
            .iter()
            .skip(1)
            .filter(|ob| ob.kind != 0)
            .map(|ob| Entity { tile: ob.kind as u8, x: i32::from(ob.x), y: i32::from(ob.y), hp: ob.hits })
            .collect();
        o
    }

    pub fn player_tile(&self) -> u8 {
        tile_for_transport(self.transport)
    }

    pub fn say(&mut self, s: impl Into<String>) {
        self.messages.push_back(s.into());
        while self.messages.len() > MESSAGE_LINES {
            self.messages.pop_front();
        }
    }

    pub fn entity_at(&self, x: i32, y: i32) -> Option<usize> {
        self.entities.iter().position(|e| e.x == x && e.y == y)
    }

    pub fn monster_count(&self) -> usize {
        self.entities.iter().filter(|e| e.is_monster()).count()
    }

    fn terrain(map: &WorldMap, x: i32, y: i32) -> u8 {
        let (x, y) = rules::wrap(x, y);
        map.cell(x as usize, y as usize)
    }

    /// Name of the location under the player, if any.
    pub fn location_here<'a>(&self, locs: &'a LocationTable) -> Option<(usize, &'a str)> {
        let id = locs.id_at(self.x as u8, self.y as u8);
        (id != 0).then(|| (id, locs.locations[id - 1].name.as_str()))
    }

    /// One movement command. Prints the direction, applies passability,
    /// then runs the rest of the outdoor turn.
    pub fn step(&mut self, dir: Dir, map: &WorldMap, t: &ExeTables, locs: &LocationTable) {
        if self.dead {
            return;
        }
        self.say(format!(">{}", t.direction_names[dir as usize]));
        let (dx, dy) = dir.delta();
        let (nx, ny) = rules::player_step(self.x, self.y, dx, dy);
        let tile = match self.entity_at(nx, ny) {
            Some(i) => self.entities[i].tile,
            None => Self::terrain(map, nx, ny),
        };
        match rules::check_move(tile, self.transport) {
            Ok(()) => {
                let old_land = rules::land_of(self.x, self.y);
                self.x = nx;
                self.y = ny;
                let land = rules::land_of(nx, ny);
                if land != old_land {
                    self.entities.retain(|e| !e.is_monster() || rules::land_of(e.x, e.y) == land);
                }
                if let Some((id, name)) = self.location_here(locs) {
                    let line = match kind_of(id) {
                        Some(LocationKind::Town) => format!("The town of {name}"),
                        Some(LocationKind::Castle) | Some(LocationKind::Landmark) => name.to_string(),
                        Some(LocationKind::Dungeon) => format!("The dungeon of {name}"),
                        None => name.to_string(),
                    };
                    self.say(line);
                }
            }
            Err(MoveBlock::Monster(k)) => {
                let m = &t.move_messages[5];
                let name = &t.monster_names[k as usize];
                let n = if name.starts_with(['A', 'E', 'I', 'O', 'U']) { "n" } else { "" };
                self.say(format!("{m}{n} {name}"));
            }
            Err(b) => self.say(t.move_messages[b.message_index()].clone()),
        }
        self.end_turn(map, t);
    }

    /// "Pass": a turn without moving (also what the game does by itself
    /// after about 200 s without a key).
    pub fn pass(&mut self, map: &WorldMap, t: &ExeTables) {
        if self.dead {
            return;
        }
        self.say(">Pass");
        self.end_turn(map, t);
    }

    /// Board a vehicle the player is standing on.
    pub fn board(&mut self) {
        self.say(">Board");
        if self.transport != 0 {
            self.say("You are already aboard!");
            return;
        }
        match self.entity_at(self.x, self.y) {
            Some(i) if transport_for_tile(self.entities[i].tile).is_some() => {
                self.transport = transport_for_tile(self.entities[i].tile).unwrap();
                self.entities.remove(i);
            }
            _ => self.say("Board what?"),
        }
    }

    /// Leave the current vehicle, which stays on the map.
    pub fn exit(&mut self) {
        self.say(">X-it");
        if self.transport == 0 {
            self.say("X-it what?");
            return;
        }
        if self.transport < 6 {
            self.entities.push(Entity { tile: tile_for_transport(self.transport), x: self.x, y: self.y, hp: 0 });
        }
        self.transport = 0;
    }

    /// Everything after a command: spawn, move counter, monsters, food.
    pub fn end_turn(&mut self, map: &WorldMap, t: &ExeTables) {
        if self.monster_count() <= MAX_MONSTERS {
            self.try_spawn(map, t);
        }
        self.moves = self.moves.wrapping_add(1);
        self.monsters_act(map, t);
        if self.food_clock.tick(&t.food_rate, self.transport) {
            self.food = self.food.saturating_sub(1);
            if self.food == 0 && !self.dead {
                self.say("You have starved!");
                self.dead = true;
            }
        }
    }

    fn try_spawn(&mut self, map: &WorldMap, t: &ExeTables) {
        if !rules::spawn_roll(t, self.exp, &mut self.rng) {
            return;
        }
        let kind = (self.rng.range(0, 149) / 10) as usize;
        let want = rules::spawn_terrain(kind);
        // The original retries until it finds a cell; we give up after a while.
        for _ in 0..32 {
            let mut dx = i32::from(self.rng.range(10, 15));
            let mut dy = i32::from(self.rng.range(5, 15));
            if self.rng.range(1, 16) & 1 == 1 {
                dx = -dx;
            }
            if self.rng.range(1, 16) & 1 == 1 {
                dy = -dy;
            }
            let (x, y) = rules::wrap(self.x + dx, self.y + dy);
            if Self::terrain(map, x, y) == want && rules::in_monster_band(x, y) && self.entity_at(x, y).is_none() {
                let tile = rules::FIRST_MONSTER_TILE + 2 * kind as u8;
                let hp = rules::monster_hp(t, kind, self.exp);
                self.entities.push(Entity { tile, x, y, hp });
                return;
            }
        }
    }

    fn monsters_act(&mut self, map: &WorldMap, t: &ExeTables) {
        for i in 0..self.entities.len() {
            let e = self.entities[i];
            if !e.is_monster() {
                continue;
            }
            let (dx, dy) = (self.x - e.x, self.y - e.y);
            if dx.abs().max(dy.abs()) <= rules::attack_range(e.tile) {
                self.monster_attacks(rules::monster_kind(e.tile), t);
                continue;
            }
            let chance = t.monster_move_chance[(self.transport as usize).min(7)];
            if self.rng.range(1, 100) > chance {
                continue;
            }
            let steps = [(dx.signum(), 0), (0, dy.signum())];
            let order = if self.rng.range(0, 1) == 0 { [0, 1] } else { [1, 0] };
            for k in order {
                let (sx, sy) = steps[k];
                if sx == 0 && sy == 0 {
                    continue;
                }
                let (nx, ny) = (e.x + sx, e.y + sy);
                if !(0..MAP_W).contains(&nx) || !(0..MAP_H).contains(&ny) {
                    continue;
                }
                let terr = Self::terrain(map, nx, ny);
                let town_like = matches!(terr, 4..=7) && rules::monster_can_enter(e.tile, terr);
                let ok = rules::monster_can_enter(e.tile, terr)
                    && (town_like || rules::in_monster_band(nx, ny))
                    && self.entity_at(nx, ny).is_none()
                    && (nx, ny) != (self.x, self.y);
                if ok {
                    self.entities[i].x = nx;
                    self.entities[i].y = ny;
                    break;
                }
            }
        }
    }

    fn monster_attacks(&mut self, kind: usize, t: &ExeTables) {
        let name = t.monster_names[kind].clone();
        self.say(format!("{name} attacks!"));
        match rules::monster_attack(t, kind, self.stats[2], self.armour, &mut self.rng) {
            Some(d) => {
                self.hits = self.hits.saturating_sub(d);
                self.say(format!("Hit! {d} damage"));
                if self.hits == 0 && !self.dead {
                    self.say("You have died!");
                    self.dead = true;
                }
            }
            None => self.say("Missed!"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::tests::synthetic_exe;

    fn world(fill: u8) -> WorldMap {
        WorldMap { width: 168, height: 156, cells: vec![fill; 168 * 156] }
    }

    fn setup() -> (ExeTables, LocationTable) {
        let t = ExeTables::from_out_exe(&synthetic_exe()).unwrap();
        let locs = LocationTable { locations: vec![] };
        (t, locs)
    }

    #[test]
    fn vehicle_tiles() {
        assert_eq!(transport_for_tile(0x0B), Some(1));
        assert_eq!(transport_for_tile(0x0E), Some(4));
        assert_eq!(transport_for_tile(0x0F), Some(4));
        assert_eq!(transport_for_tile(0x10), Some(5));
        assert_eq!(transport_for_tile(0x12), Some(7));
        assert_eq!(transport_for_tile(0x13), None);
        for t in 1..=7 {
            assert_eq!(transport_for_tile(tile_for_transport(t)), Some(t));
        }
    }

    #[test]
    fn walking_wraps_and_eats() {
        let (mut t, locs) = setup();
        t.spawn_chance = [0; 10]; // no monsters
        let map = world(1);
        let mut o = Overworld::new_at(0, 0, 1);
        o.step(Dir::West, &map, &t, &locs);
        assert_eq!((o.x, o.y), (167, 77)); // edge crossing shifts 77 rows
        o.step(Dir::North, &map, &t, &locs);
        assert_eq!((o.x, o.y), (167, 76));
        assert_eq!(o.moves, 2);
        // synthetic food rate on foot is 2: counter starts at 1, eats on move 2.
        assert_eq!(o.food, 199);
        assert_eq!(o.messages.back().unwrap(), ">N");
    }

    #[test]
    fn blocked_moves_still_take_a_turn() {
        let (mut t, locs) = setup();
        t.spawn_chance = [0; 10];
        let mut map = world(1);
        map.cells[10 * 168 + 11] = 3; // mountain east of (10,10)
        let mut o = Overworld::new_at(10, 10, 1);
        o.step(Dir::East, &map, &t, &locs);
        assert_eq!((o.x, o.y), (10, 10));
        assert_eq!(o.messages.back().unwrap(), "m1");
        assert_eq!(o.moves, 1);
        o.entities.push(Entity { tile: 0x15, x: 10, y: 11, hp: 5 });
        o.step(Dir::South, &map, &t, &locs);
        assert!(o.messages.iter().any(|m| m == "m5 Mon1"));
    }

    #[test]
    fn board_and_exit() {
        let (mut t, locs) = setup();
        t.spawn_chance = [0; 10];
        let mut map = world(0);
        map.cells[5 * 168 + 5] = 1;
        let mut o = Overworld::new_at(5, 5, 1);
        o.entities.push(Entity { tile: 0x0E, x: 6, y: 5, hp: 0 });
        o.step(Dir::East, &map, &t, &locs); // walk onto the frigate
        assert_eq!(o.x, 6);
        o.board();
        assert_eq!(o.transport, 4);
        assert!(o.entities.is_empty());
        o.step(Dir::East, &map, &t, &locs);
        assert_eq!(o.x, 7);
        o.exit();
        assert_eq!(o.transport, 0);
        assert_eq!(o.entities[0].tile, tile_for_transport(4));
    }

    #[test]
    fn spawns_and_monsters_close_in() {
        let (mut t, locs) = setup();
        t.spawn_chance = [255; 10];
        t.monster_move_chance = [100; 8];
        let map = world(1);
        let mut o = Overworld::new_at(40, 40, 3);
        for _ in 0..40 {
            o.pass(&map, &t);
        }
        let _ = locs;
        assert!(o.monster_count() > 0);
        assert!(o.monster_count() <= MAX_MONSTERS + 1);
        // Grass only: every monster is a land type (9..=14).
        assert!(o.entities.iter().all(|e| rules::monster_kind(e.tile) >= 9));
        assert!(o.messages.iter().any(|m| m.ends_with("attacks!") || m.starts_with("Hit") || m == "Missed!") || o.hits == 150);
    }

    #[test]
    fn land_change_drops_old_monsters() {
        let (mut t, locs) = setup();
        t.spawn_chance = [0; 10];
        t.monster_move_chance = [0; 8];
        let map = world(1);
        let mut o = Overworld::new_at(83, 20, 1);
        o.entities.push(Entity { tile: 0x2B, x: 60, y: 20, hp: 5 });
        o.entities.push(Entity { tile: 0x0B, x: 60, y: 21, hp: 0 });
        o.step(Dir::East, &map, &t, &locs);
        assert_eq!(o.x, 84);
        assert_eq!(o.entities.len(), 1);
        assert_eq!(o.entities[0].tile, 0x0B);
    }
}
