//! Overworld rules and the small numeric tables they use. See docs/SPEC.md §8.
//!
//! The tables (monster names, damage and hit-point bases, spawn chances, food
//! rates, item names, messages) are *not* embedded: [`ExeTables::from_out_exe`]
//! reads them at run time from the user's own `OUT.EXE` (69,858-byte DOS
//! build), at the data-segment offsets listed in the spec. The formulas
//! (prices, shop stock, passability, food, spawning, monster movement) are
//! written fresh from the plain-language description in the spec.

use crate::error::{AssetError, Result};
use crate::locations::OUT_EXE_SIZE;
use crate::rng::U1Rng;

/// Offset of the data segment inside `OUT.EXE` (file offset = this + DS offset).
pub const DATA_BASE: usize = 0xD680;
pub const MAP_W: i32 = 168;
pub const MAP_H: i32 = 156;
pub const VIEW_COLS: i32 = 19;
pub const VIEW_ROWS: i32 = 9;
pub const PLAYER_COL: i32 = 9;
pub const PLAYER_ROW: i32 = 4;
pub const MONSTER_TYPES: usize = 15;
pub const FIRST_MONSTER_TILE: u8 = 0x13;

/// DS offsets of the tables (SPEC §8.1).
pub mod ds {
    pub const SPAWN_CHANCE: usize = 0x16CC; // 10 words
    /// 15 words read from here; the last nine overlap the HP-base table,
    /// exactly as the original reads them.
    pub const MONSTER_DAMAGE: usize = 0x368C;
    pub const MONSTER_HP_BASE: usize = 0x3698; // 15 words
    pub const MONSTER_MOVE_CHANCE: usize = 0x36E8; // 8 words, by transport
    pub const FOOD_RATE: usize = 0x36F8; // 8 words, by transport
    pub const MONSTER_NAMES: usize = 0x2AB8; // 15 pointers
    pub const MOVE_MESSAGES: usize = 0x37AE; // 7 pointers
    pub const DIRECTION_NAMES: usize = 0x37D4; // 5 pointers (index 1..=4 used)
    pub const WEAPON_NAMES: usize = 0x277E; // 16 pointers
    pub const ARMOUR_NAMES: usize = 0x2968; // 6 pointers
    pub const SPELL_NAMES: usize = 0x2910; // 11 pointers
    pub const TRANSPORT_NAMES: usize = 0x29AE; // 8 pointers
    pub const LANDMARK_NAMES: usize = 0x30CE; // 9 pointers
}

/// Tables read from the user's `OUT.EXE`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExeTables {
    pub spawn_chance: [u16; 10],
    pub monster_damage: [u16; MONSTER_TYPES],
    pub monster_hp_base: [u16; MONSTER_TYPES],
    pub monster_move_chance: [u16; 8],
    pub food_rate: [u16; 8],
    pub monster_names: Vec<String>,
    /// Indexed by [`MoveBlock`] message number 0..=6.
    pub move_messages: Vec<String>,
    /// Index 1..=4 = West, East, North, South (index 0 unused).
    pub direction_names: Vec<String>,
    pub weapon_names: Vec<String>,
    pub armour_names: Vec<String>,
    pub spell_names: Vec<String>,
    pub transport_names: Vec<String>,
    pub landmark_names: Vec<String>,
}

fn word(exe: &[u8], ds_off: usize) -> Result<u16> {
    let o = DATA_BASE + ds_off;
    exe.get(o..o + 2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .ok_or_else(|| AssetError::Invalid(format!("OUT.EXE too short for DS:{ds_off:04X}")))
}

fn words<const N: usize>(exe: &[u8], ds_off: usize) -> Result<[u16; N]> {
    let mut out = [0u16; N];
    for (i, w) in out.iter_mut().enumerate() {
        *w = word(exe, ds_off + 2 * i)?;
    }
    Ok(out)
}

fn cstr(exe: &[u8], ds_off: usize) -> Result<String> {
    let o = DATA_BASE + ds_off;
    let tail = exe.get(o..).ok_or_else(|| AssetError::Invalid("string pointer out of range".into()))?;
    let end = tail.iter().position(|&b| b == 0).ok_or_else(|| AssetError::Invalid("unterminated string".into()))?;
    Ok(String::from_utf8_lossy(&tail[..end]).into_owned())
}

fn strings(exe: &[u8], table: usize, n: usize) -> Result<Vec<String>> {
    (0..n).map(|i| cstr(exe, word(exe, table + 2 * i)? as usize)).collect()
}

impl ExeTables {
    pub fn from_out_exe(exe: &[u8]) -> Result<Self> {
        if exe.len() != OUT_EXE_SIZE {
            return Err(AssetError::BadSize { what: "OUT.EXE".into(), expected: OUT_EXE_SIZE.to_string(), got: exe.len() });
        }
        let mut direction_names = strings(exe, ds::DIRECTION_NAMES + 2, 4)?;
        direction_names.insert(0, String::new());
        Ok(ExeTables {
            spawn_chance: words(exe, ds::SPAWN_CHANCE)?,
            monster_damage: words(exe, ds::MONSTER_DAMAGE)?,
            monster_hp_base: words(exe, ds::MONSTER_HP_BASE)?,
            monster_move_chance: words(exe, ds::MONSTER_MOVE_CHANCE)?,
            food_rate: words(exe, ds::FOOD_RATE)?,
            monster_names: strings(exe, ds::MONSTER_NAMES, MONSTER_TYPES)?,
            move_messages: strings(exe, ds::MOVE_MESSAGES, 7)?,
            direction_names,
            weapon_names: strings(exe, ds::WEAPON_NAMES, 16)?,
            armour_names: strings(exe, ds::ARMOUR_NAMES, 6)?,
            spell_names: strings(exe, ds::SPELL_NAMES, 11)?,
            transport_names: strings(exe, ds::TRANSPORT_NAMES, 8)?,
            landmark_names: strings(exe, ds::LANDMARK_NAMES, 9)?,
        })
    }

    /// "a" or "an" + monster name, as used in "Blocked by a(n) …".
    pub fn monster_with_article(&self, kind: usize) -> String {
        let name = &self.monster_names[kind];
        let vowel = name.chars().next().is_some_and(|c| "AEIOUaeiou".contains(c));
        format!("{} {name}", if vowel { "an" } else { "a" })
    }
}

// ---------------------------------------------------------------- geometry

/// Wrap a coordinate onto the 168 x 156 torus.
pub fn wrap(x: i32, y: i32) -> (i32, i32) {
    (x.rem_euclid(MAP_W), y.rem_euclid(MAP_H))
}

/// Where the player ends up after one step from (x,y). Verified from the
/// original executable: north/south and the drawn view wrap like a plain
/// torus, but stepping off the east or west edge also moves the player 77
/// rows (down if y < 77, otherwise up), so the edge joins the north half of
/// the world to the south half. Monsters and the view use plain [`wrap`].
pub fn player_step(x: i32, y: i32, dx: i32, dy: i32) -> (i32, i32) {
    let (nx, ny) = wrap(x + dx, y + dy);
    let crossed = (dx < 0 && x == 0) || (dx > 0 && x == MAP_W - 1);
    if crossed {
        let ny = if ny >= EDGE_SHIFT { ny - EDGE_SHIFT } else { ny + EDGE_SHIFT };
        (nx, ny)
    } else {
        (nx, ny)
    }
}

/// Row shift applied when the player crosses the east/west edge.
pub const EDGE_SHIFT: i32 = 77;

/// Map cell shown at viewport cell (col,row) when the player is at (px,py).
pub fn view_cell(px: i32, py: i32, col: i32, row: i32) -> (i32, i32) {
    wrap(px - PLAYER_COL + col, py - PLAYER_ROW + row)
}

/// Which of the four lands (0..=3) a cell belongs to: bit 0 = east half
/// (x > 83), bit 1 = south half (y > 77).
pub fn land_of(x: i32, y: i32) -> u8 {
    u8::from(x > 83) | (u8::from(y > 77) << 1)
}

/// Monsters never appear (or step) within 9 columns / 6 rows of the north-west
/// corner of a land.
pub fn in_monster_band(x: i32, y: i32) -> bool {
    let lx = if x > 83 { x - 84 } else { x };
    let ly = if y > 77 { y - 78 } else { y };
    (9..159).contains(&lx) && (6..150).contains(&ly)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dir {
    West = 1,
    East = 2,
    North = 3,
    South = 4,
}

impl Dir {
    pub fn delta(self) -> (i32, i32) {
        match self {
            Dir::West => (-1, 0),
            Dir::East => (1, 0),
            Dir::North => (0, -1),
            Dir::South => (0, 1),
        }
    }
}

// ------------------------------------------------------------- passability

/// Why a move was refused. The number is the index into
/// [`ExeTables::move_messages`] (5 = "Blocked by a").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveBlock {
    Water,
    Mountain,
    AircarWoods,
    FrigateLand,
    RaftLand,
    /// Blocked by monster type 0..=14.
    Monster(u8),
    CantMoveOnLand,
}

impl MoveBlock {
    pub fn message_index(self) -> usize {
        match self {
            MoveBlock::Monster(_) => 5,
            other => other.code(),
        }
    }
    fn code(self) -> usize {
        match self {
            MoveBlock::Water => 0,
            MoveBlock::Mountain => 1,
            MoveBlock::AircarWoods => 2,
            MoveBlock::FrigateLand => 3,
            MoveBlock::RaftLand => 4,
            MoveBlock::Monster(_) => 5,
            MoveBlock::CantMoveOnLand => 6,
        }
    }
}

pub fn is_monster_tile(tile: u8) -> bool {
    (0x13..=0x2F).contains(&tile)
}

pub fn monster_kind(tile: u8) -> usize {
    ((tile - FIRST_MONSTER_TILE) / 2) as usize
}

/// Can the player, using `transport` (0 = foot … 7 = time machine), enter a
/// cell showing `tile` (an object's tile if one stands there, else terrain)?
pub fn check_move(tile: u8, transport: u16) -> std::result::Result<(), MoveBlock> {
    if is_monster_tile(tile) {
        return Err(MoveBlock::Monster(monster_kind(tile) as u8));
    }
    if tile == 3 {
        return Err(MoveBlock::Mountain);
    }
    if tile == 0 {
        return if (3..=5).contains(&transport) { Ok(()) } else { Err(MoveBlock::Water) };
    }
    match transport {
        5 if tile == 2 => Err(MoveBlock::AircarWoods),
        4 => Err(MoveBlock::FrigateLand),
        3 => Err(MoveBlock::RaftLand),
        t if t >= 6 => Err(MoveBlock::CantMoveOnLand),
        _ => Ok(()),
    }
}

/// Overworld tile drawn for terrain value `t` at animation counter `c`
/// (one count per keyboard poll, ~3 timer ticks). Castles and towns flicker;
/// `prev` is what was drawn last time (castles are only redrawn on some counts).
pub fn animated_terrain_tile(t: u8, c: u32, prev: u8) -> u8 {
    match t {
        4 => {
            if c % 6 == 0 {
                4
            } else if c % 3 == 0 {
                5
            } else {
                prev
            }
        }
        6 => {
            if c & 3 != 0 {
                8
            } else {
                7
            }
        }
        _ => terrain_tile(t),
    }
}

/// Base tile of a terrain nibble (0 water, 1 grass, 2 forest, 3 mountain,
/// 4 castle, 5 signpost, 6 town, 7 dungeon).
pub fn terrain_tile(t: u8) -> u8 {
    match t {
        0..=4 => t,
        5 => 6,
        6 => 7,
        7 => 9,
        n => n,
    }
}

// --------------------------------------------------------------------- food

/// Food consumption: one unit every `rate[transport]` moves outdoors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FoodClock {
    pub counter: u16,
}

impl Default for FoodClock {
    fn default() -> Self {
        FoodClock { counter: 1 }
    }
}

impl FoodClock {
    /// Advance one outdoor turn; returns true when a unit of food is eaten.
    pub fn tick(&mut self, rate: &[u16; 8], transport: u16) -> bool {
        let r = rate[(transport as usize).min(7)];
        let eat = self.counter >= r;
        if eat {
            self.counter = 0;
        }
        self.counter += 1;
        eat
    }
}

// ------------------------------------------------------------------- shops

// All shop arithmetic is 16-bit signed with truncating division, as in the
// original (verified from original executable).

/// Weapon purchase price, item 1..=15.
pub fn weapon_buy_price(item: i16, int: i16) -> i16 {
    (255i16.wrapping_sub(int)).wrapping_mul(item).wrapping_mul(item) / 256 + 5
}
/// What a weapon shop pays for item 1..=15.
pub fn weapon_sell_price(item: i16, cha: i16) -> i16 {
    (cha.wrapping_add(40)).wrapping_mul(item).wrapping_mul(item) / 256 + 1
}
/// Armour purchase price, item 1..=5.
pub fn armour_buy_price(item: i16, int: i16) -> i16 {
    (200i16.wrapping_sub(int) / 4).wrapping_mul(item)
}
pub fn armour_sell_price(item: i16, cha: i16) -> i16 {
    (cha / 4).wrapping_mul(item)
}
/// Spell price, spell 1..=10 (magic shops do not buy spells).
pub fn spell_price(item: i16, wis: i16) -> i16 {
    (200i16.wrapping_sub(wis) / 32).wrapping_mul(item)
}
/// Transport price, item 1..=6 (shops do not buy used vehicles).
pub fn transport_price(item: i16, int: i16) -> i16 {
    (200i16.wrapping_sub(int) / 5).wrapping_mul(item).wrapping_mul(item)
}
/// Price of 10 packs of food.
pub fn food_price_per_10(int: i16) -> i16 {
    5 - int / 20
}

/// The game's "era": 0..=3 from the move counter.
pub fn era(moves: u32) -> u32 {
    if moves > 3000 {
        3
    } else {
        ((moves % 32767) / 1500).min(3)
    }
}

/// Town map layout (2..=9) of town `id` (1..=32).
pub fn town_layout(town_id: u32) -> u32 {
    (town_id - 1) % 8 + 2
}

/// Weapons offered by a town with map `layout`: every other item from
/// 1 or 2, era + 2 of them.
pub fn weapon_stock(layout: u32, moves: u32) -> Vec<u32> {
    let first = 1 + ((layout - 1) & 1);
    (0..era(moves) + 2).map(|k| first + 2 * k).collect()
}

/// Armour offered (same in every town).
pub fn armour_stock(moves: u32) -> Vec<u32> {
    if moves > 3000 { vec![1, 2, 3, 4, 5] } else { vec![1, 2, 3] }
}

/// Spells listed by a magic shop with map `layout`; wizards (class 2) see
/// more. Spell 4 is never listed.
pub fn spell_stock(layout: u32, wizard: bool) -> Vec<u32> {
    let p = layout & 1;
    let last = if wizard { 9 + p } else { 5 + p };
    (1 + p..=last).step_by(2).filter(|&s| s != 4).collect()
}

// ---------------------------------------------------------------- monsters

/// Terrain nibble a monster type needs to appear on: 0..=3 sea, 4..=8 forest,
/// 9..=14 grass.
pub fn spawn_terrain(kind: usize) -> u8 {
    match kind {
        0..=3 => 0,
        4..=8 => 2,
        _ => 1,
    }
}

/// Attack reach (Chebyshev distance) of a monster tile.
pub fn attack_range(tile: u8) -> i32 {
    match tile {
        0x17 | 0x19 | 0x1F | 0x2F => 3,
        _ => 1,
    }
}

/// Can monster `tile` step onto terrain nibble `t`?
pub fn monster_can_enter(tile: u8, t: u8) -> bool {
    match tile {
        0x13..=0x1A => t == 0,
        0x1F | 0x20 | 0x23 | 0x24 => t == 2,
        // Everyone else: grass or forest, and may also walk over castle,
        // signpost, town and dungeon cells.
        _ => matches!(t, 1 | 2 | 4..=7),
    }
}

/// Starting hit points of a freshly spawned monster.
pub fn monster_hp(tables: &ExeTables, kind: usize, exp: u16) -> u16 {
    tables.monster_hp_base[kind] * 10 + exp / 1000 + 2
}

/// Chance test: does a monster appear this turn?
pub fn spawn_roll(tables: &ExeTables, exp: u16, rng: &mut U1Rng) -> bool {
    let idx = ((exp / 1000) as usize).min(9);
    rng.range(1, 255) < tables.spawn_chance[idx]
}

/// Damage roll for monster `kind`, or None for a miss.
pub fn monster_attack(tables: &ExeTables, kind: usize, sta: u16, armour: u16, rng: &mut U1Rng) -> Option<u16> {
    let to_hit = 200i32 - sta as i32 / 2 - 8 * armour as i32;
    if (rng.range(1, 255) as i32) < to_hit {
        Some(rng.range(1, 2 * tables.monster_damage[kind] + 1))
    } else {
        None
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Build a synthetic 69,858-byte "OUT.EXE" holding made-up tables at the
    /// documented offsets.
    pub fn synthetic_exe() -> Vec<u8> {
        let mut exe = vec![0u8; OUT_EXE_SIZE];
        let put = |exe: &mut Vec<u8>, ds: usize, v: u16| {
            exe[DATA_BASE + ds..DATA_BASE + ds + 2].copy_from_slice(&v.to_le_bytes())
        };
        for i in 0..10 {
            put(&mut exe, ds::SPAWN_CHANCE + 2 * i, 10 * i as u16 + 1);
        }
        for i in 0..6 {
            put(&mut exe, ds::MONSTER_DAMAGE + 2 * i, i as u16 + 1);
        }
        for i in 0..15 {
            put(&mut exe, ds::MONSTER_HP_BASE + 2 * i, 15 - i as u16);
        }
        for i in 0..8 {
            put(&mut exe, ds::MONSTER_MOVE_CHANCE + 2 * i, 100 - i as u16);
            put(&mut exe, ds::FOOD_RATE + 2 * i, i as u16 + 2);
        }
        let mut s = 0x3900usize;
        let mut table = |exe: &mut Vec<u8>, at: usize, names: &[&str]| {
            for (i, n) in names.iter().enumerate() {
                exe[DATA_BASE + s..DATA_BASE + s + n.len()].copy_from_slice(n.as_bytes());
                put(exe, at + 2 * i, s as u16);
                s += n.len() + 1;
            }
        };
        let mons: Vec<String> = (0..15).map(|i| if i == 2 { "Eel".into() } else { format!("Mon{i}") }).collect();
        let mons: Vec<&str> = mons.iter().map(|s| s.as_str()).collect();
        table(&mut exe, ds::MONSTER_NAMES, &mons);
        table(&mut exe, ds::MOVE_MESSAGES, &["m0", "m1", "m2", "m3", "m4", "m5", "m6"]);
        table(&mut exe, ds::DIRECTION_NAMES, &["", "W", "E", "N", "S"]);
        table(&mut exe, ds::WEAPON_NAMES, &["w"; 16]);
        table(&mut exe, ds::ARMOUR_NAMES, &["a"; 6]);
        table(&mut exe, ds::SPELL_NAMES, &["s"; 11]);
        table(&mut exe, ds::TRANSPORT_NAMES, &["Foot", "t1", "t2", "t3", "t4", "t5", "t6", "t7"]);
        table(&mut exe, ds::LANDMARK_NAMES, &["l"; 9]);
        exe
    }

    #[test]
    fn loads_tables_from_synthetic_exe() {
        let t = ExeTables::from_out_exe(&synthetic_exe()).unwrap();
        assert_eq!(t.spawn_chance[3], 31);
        assert_eq!(t.monster_damage[5], 6);
        assert_eq!(t.monster_damage[14], 7); // overlaps hp_base[8]
        assert_eq!(t.food_rate, [2, 3, 4, 5, 6, 7, 8, 9]);
        assert_eq!(t.monster_names[1], "Mon1");
        assert_eq!(t.monster_with_article(2), "an Eel");
        assert_eq!(t.monster_with_article(1), "a Mon1");
        assert_eq!(t.direction_names[4], "S");
        assert_eq!(t.transport_names[0], "Foot");
        assert!(ExeTables::from_out_exe(&[0u8; 100]).is_err());
    }

    #[test]
    fn wraps_like_a_torus() {
        assert_eq!(wrap(-1, -1), (167, 155));
        assert_eq!(wrap(168, 156), (0, 0));
    }

    #[test]
    fn east_west_edge_switches_hemisphere() {
        assert_eq!(player_step(0, 89, -1, 0), (167, 12));
        assert_eq!(player_step(0, 10, -1, 0), (167, 87));
        assert_eq!(player_step(167, 77, 1, 0), (0, 0));
        assert_eq!(player_step(167, 76, 1, 0), (0, 153));
        assert_eq!(player_step(5, 0, 0, -1), (5, 155));
        assert_eq!(player_step(5, 155, 0, 1), (5, 0));
        assert_eq!(player_step(84, 40, -1, 0), (83, 40));
        assert_eq!(view_cell(3, 89, 0, 0), (162, 85));
        assert_eq!(view_cell(3, 89, PLAYER_COL, PLAYER_ROW), (3, 89));
        assert_eq!(land_of(83, 77), 0);
        assert_eq!(land_of(84, 78), 3);
        assert!(!in_monster_band(84 + 8, 30));
        assert!(in_monster_band(84 + 9, 78 + 6));
    }

    #[test]
    fn passability_by_transport() {
        assert_eq!(check_move(1, 0), Ok(()));
        assert_eq!(check_move(0, 0), Err(MoveBlock::Water));
        assert_eq!(check_move(0, 4), Ok(()));
        assert_eq!(check_move(3, 5), Err(MoveBlock::Mountain));
        assert_eq!(check_move(2, 5), Err(MoveBlock::AircarWoods));
        assert_eq!(check_move(1, 4), Err(MoveBlock::FrigateLand));
        assert_eq!(check_move(1, 3), Err(MoveBlock::RaftLand));
        assert_eq!(check_move(1, 6), Err(MoveBlock::CantMoveOnLand));
        assert_eq!(check_move(7, 1), Ok(()));
        assert_eq!(check_move(0x16, 0), Err(MoveBlock::Monster(1)));
        assert_eq!(MoveBlock::Monster(1).message_index(), 5);
        assert_eq!(MoveBlock::RaftLand.message_index(), 4);
    }

    #[test]
    fn food_clock_eats_every_rate_moves() {
        let rate = [2, 3, 3, 4, 5, 6, 11, 0];
        let mut c = FoodClock::default();
        let eaten: Vec<bool> = (0..10).map(|_| c.tick(&rate, 4)).collect();
        // counter starts at 1: first unit after 5 moves, then every 5.
        assert_eq!(eaten.iter().filter(|&&e| e).count(), 2);
        assert!(eaten[4] && eaten[9]);
    }

    #[test]
    fn prices_and_stock() {
        assert_eq!(weapon_buy_price(1, 15), 240 / 256 + 5);
        // 16-bit arithmetic: 255*225 wraps negative, as in the original.
        assert_eq!(weapon_buy_price(15, 0), (255i32 * 225) as i16 / 256 + 5);
        assert_eq!(weapon_buy_price(10, 0), 255 * 100 / 256 + 5);
        assert_eq!(weapon_buy_price(15, 99), -113); // never offered for sale
        assert_eq!(weapon_sell_price(2, 20), 60 * 4 / 256 + 1);
        assert_eq!(armour_buy_price(3, 20), 45 * 3);
        assert_eq!(spell_price(10, 40), 50);
        assert_eq!(transport_price(2, 50), 120);
        assert_eq!(food_price_per_10(45), 3);
        assert_eq!(town_layout(1), 2);
        assert_eq!(weapon_stock(town_layout(1), 0), vec![2, 4]);
        assert_eq!(weapon_stock(town_layout(2), 1600), vec![1, 3, 5]);
        assert_eq!(weapon_stock(town_layout(9), 5000), vec![2, 4, 6, 8, 10]);
        assert_eq!(armour_stock(10), vec![1, 2, 3]);
        assert_eq!(spell_stock(town_layout(1), false), vec![1, 3, 5]);
        assert_eq!(spell_stock(town_layout(2), false), vec![2, 6]);
        assert_eq!(spell_stock(town_layout(2), true), vec![2, 6, 8, 10]);
    }

    #[test]
    fn monster_rules() {
        let t = ExeTables::from_out_exe(&synthetic_exe()).unwrap();
        assert_eq!(spawn_terrain(3), 0);
        assert_eq!(spawn_terrain(8), 2);
        assert_eq!(spawn_terrain(14), 1);
        assert_eq!(attack_range(0x2F), 3);
        assert_eq!(attack_range(0x15), 1);
        assert!(monster_can_enter(0x13, 0) && !monster_can_enter(0x13, 1));
        assert!(monster_can_enter(0x23, 2) && !monster_can_enter(0x23, 1));
        assert!(monster_can_enter(0x25, 6) && !monster_can_enter(0x25, 3));
        assert_eq!(monster_hp(&t, 0, 2500), 15 * 10 + 2 + 2);
        let mut rng = U1Rng::new(7);
        for _ in 0..50 {
            if let Some(d) = monster_attack(&t, 4, 0, 0, &mut rng) {
                assert!((1..=11).contains(&d)); // damage[4] = 5
            }
        }
        // Armour 30 makes the to-hit threshold negative: always a miss.
        assert_eq!(monster_attack(&t, 4, 0, 30, &mut rng), None);
    }

    #[test]
    fn terrain_animation() {
        assert_eq!(animated_terrain_tile(6, 0, 7), 7);
        assert_eq!(animated_terrain_tile(6, 1, 7), 8);
        assert_eq!(animated_terrain_tile(4, 6, 5), 4);
        assert_eq!(animated_terrain_tile(4, 3, 4), 5);
        assert_eq!(animated_terrain_tile(4, 4, 5), 5);
        assert_eq!(animated_terrain_tile(5, 1, 0), 6);
        assert_eq!(animated_terrain_tile(7, 1, 0), 9);
    }
}

/// Optional check against a real install: set `U1_GAME_DIR` to the folder
/// holding the user's own OUT.EXE. Skipped (passes) when unset.
#[cfg(test)]
mod real_game {
    use super::*;

    #[test]
    fn loads_from_user_out_exe_if_present() {
        let Ok(dir) = std::env::var("U1_GAME_DIR") else { return };
        let gd = crate::gamedir::GameDir::new(dir).unwrap();
        let t = ExeTables::from_out_exe(&gd.read("OUT.EXE").unwrap()).unwrap();
        assert_eq!(t.monster_names.len(), 15);
        assert_eq!(t.transport_names.len(), 8);
        eprintln!("{t:#?}");
    }
}
