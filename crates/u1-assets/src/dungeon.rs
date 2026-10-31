//! Dungeon level generation (SPEC §6), written from a prose description of
//! the original algorithm ("verified from original executable").
//!
//! A level is an 11 x 11 grid indexed `[a][b]` (both 0..=10) whose outer
//! ring is solid wall, leaving the documented 9 x 9 playable interior.

use crate::rng::U1Rng;

pub const DUNGEON_SIZE: usize = 11;

/// Cell structure codes used by the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cell {
    Empty,
    Wall,
    SecretDoor,
    Door,
    LadderDown,
    LadderUp,
    ForceField,
    Other(u16),
}

impl Cell {
    pub fn from_code(c: u16) -> Cell {
        match c {
            0 => Cell::Empty,
            1 => Cell::Wall,
            2 => Cell::SecretDoor,
            3 => Cell::Door,
            6 => Cell::LadderDown,
            7 => Cell::LadderUp,
            8 => Cell::ForceField,
            n => Cell::Other(n),
        }
    }
    pub fn code(self) -> u16 {
        match self {
            Cell::Empty => 0,
            Cell::Wall => 1,
            Cell::SecretDoor => 2,
            Cell::Door => 3,
            Cell::LadderDown => 6,
            Cell::LadderUp => 7,
            Cell::ForceField => 8,
            Cell::Other(n) => n,
        }
    }
}

/// Loose objects lying in a cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Item {
    None,
    Chest,
    Coffin,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DungeonLevel {
    pub level: u16,
    pub cells: [[Cell; DUNGEON_SIZE]; DUNGEON_SIZE],
    pub items: [[Item; DUNGEON_SIZE]; DUNGEON_SIZE],
    /// Monster kind (0..=24) in a cell, -1 for none.
    pub monsters: [[i16; DUNGEON_SIZE]; DUNGEON_SIZE],
    /// Hit points, kept per *cell*: a monster that walks leaves its hit
    /// points behind (the original's behaviour, see SPEC §6.4).
    pub hp: [[i16; DUNGEON_SIZE]; DUNGEON_SIZE],
    /// Which of the level pair's five monster kinds are on the level.
    pub used: [bool; 5],
}

impl DungeonLevel {
    pub fn cell(&self, a: i32, b: i32) -> Cell {
        if (0..DUNGEON_SIZE as i32).contains(&a) && (0..DUNGEON_SIZE as i32).contains(&b) {
            self.cells[a as usize][b as usize]
        } else {
            Cell::Wall
        }
    }

    pub fn monster(&self, a: i32, b: i32) -> i16 {
        if (0..DUNGEON_SIZE as i32).contains(&a) && (0..DUNGEON_SIZE as i32).contains(&b) {
            self.monsters[a as usize][b as usize]
        } else {
            -1
        }
    }

    /// Place one monster (`4602`, verified from original executable): up
    /// to 500 random interior cells are tried for one that has no monster,
    /// is not wall, secret door or force field, and holds no chest; the
    /// player's own cell cancels the placement. The kind is one of the five
    /// of the level pair (levels 1-2 kinds 0-4, 3-4 kinds 5-9, ...), chosen
    /// at random among those not yet on the level; hit points are
    /// `R(1, L*L+1) + 10 + kind`.
    pub fn place_monster(&mut self, rng: &mut U1Rng, player: (usize, usize)) {
        let mut tries = 0;
        let (mut a, mut b);
        loop {
            a = (rng.range(0, 242) % 9 + 1) as usize;
            b = (rng.range(0, 242) % 9 + 1) as usize;
            let ok = self.monsters[a][b] == -1 && !matches!(self.cells[a][b], Cell::Wall | Cell::SecretDoor | Cell::ForceField) && self.items[a][b] != Item::Chest;
            if ok {
                break;
            }
            tries += 1;
            if tries > 500 {
                break;
            }
        }
        if tries >= 500 || (a, b) == player {
            return;
        }
        // The original loops until it finds an unused kind; with all five
        // in use that would never end, so give up after many draws.
        let mut k = 0usize;
        let mut found = false;
        for _ in 0..10_000 {
            k = (rng.range(0, 49) / 10) as usize;
            if !self.used[k] {
                found = true;
                break;
            }
        }
        if !found {
            return;
        }
        self.used[k] = true;
        let l = self.level;
        let id = ((l as i16 - 1) / 2) * 5 + k as i16;
        self.monsters[a][b] = id;
        self.hp[a][b] = rng.range(1, l.wrapping_mul(l).wrapping_add(1)) as i16 + 10 + id;
    }

    /// The monster a coffin releases (`58f8`) into cell (a,b), if that
    /// cell is open floor or a ladder and empty.
    pub fn coffin_spawn(&mut self, rng: &mut U1Rng, a: usize, b: usize) {
        if !matches!(self.cells[a][b], Cell::Empty | Cell::LadderUp | Cell::LadderDown) || self.monsters[a][b] != -1 {
            return;
        }
        let mut k;
        let mut n = 0;
        loop {
            k = (rng.range(0, 49) / 10) as usize;
            if !self.used[k] {
                break;
            }
            n += 1;
            if n >= 50 {
                break;
            }
        }
        self.used[k] = true;
        let l = self.level;
        let id = ((l as i16 - 1) / 2) * 5 + k as i16;
        self.monsters[a][b] = id;
        self.hp[a][b] = rng.range(1, l.wrapping_mul(l).wrapping_add(1)) as i16 + 10 + id;
    }
}

/// Read the 16 candidate spots from the user's own `OUT.EXE` (69,858-byte
/// DOS build): two arrays of 16 little-endian words at file offsets
/// 0x10EA0 (first index `a`) and 0x10EC0 (second index `b`).
pub fn fixed_spots_from_out_exe(exe: &[u8]) -> crate::Result<[(u8, u8); 16]> {
    if exe.len() != crate::locations::OUT_EXE_SIZE {
        return Err(crate::AssetError::BadSize { what: "OUT.EXE".into(), expected: crate::locations::OUT_EXE_SIZE.to_string(), got: exe.len() });
    }
    let w = |o: usize| u16::from_le_bytes([exe[o], exe[o + 1]]);
    let mut spots = [(0u8, 0u8); 16];
    for (i, s) in spots.iter_mut().enumerate() {
        let (a, b) = (w(0x10EA0 + 2 * i), w(0x10EC0 + 2 * i));
        if a > 9 || b > 9 || a == 0 || b == 0 {
            return Err(crate::AssetError::Invalid("dungeon spot table out of range".into()));
        }
        *s = (a as u8, b as u8);
    }
    Ok(spots)
}

/// Seed used for a level's structure: 5*X + 3*Y + 17*level + save word 0x32
/// (X, Y = the dungeon entrance on the overworld), all wrapping 16-bit.
pub fn structure_seed(x: u16, y: u16, level: u16, save_seed: u16) -> u16 {
    x.wrapping_mul(5).wrapping_add(y.wrapping_mul(3)).wrapping_add(level.wrapping_mul(17)).wrapping_add(save_seed)
}

/// Generate one level (1..=10) with its own generator (see [`build`]).
pub fn generate(x: u16, y: u16, level: u16, save_seed: u16, fixed_spots: &[(u8, u8); 16], item_seed: u16) -> DungeonLevel {
    let mut rng = U1Rng::new(0);
    build(&mut rng, x, y, level, save_seed, fixed_spots, item_seed, (1, 1))
}

/// Level setup `4761` (verified from original executable). The game's one
/// generator is reseeded with the structure seed, then with `item_seed`
/// (the original uses the time of day plus 0x6F1) before the chests and
/// coffins; three monsters are then placed and play continues on the same
/// generator. `fixed_spots` is the game's list of 16 candidate (a, b)
/// positions for extra walls / force fields, read from the user's OUT.EXE.
#[allow(clippy::too_many_arguments)]
pub fn build(rng: &mut U1Rng, x: u16, y: u16, level: u16, save_seed: u16, fixed_spots: &[(u8, u8); 16], item_seed: u16, player: (usize, usize)) -> DungeonLevel {
    let n = DUNGEON_SIZE;
    rng.seed(structure_seed(x, y, level, save_seed));
    let mut cells = [[Cell::Empty; DUNGEON_SIZE]; DUNGEON_SIZE];
    let mut items = [[Item::None; DUNGEON_SIZE]; DUNGEON_SIZE];
    for i in 0..n {
        cells[i][0] = Cell::Wall;
        cells[0][i] = Cell::Wall;
        cells[n - 1][i] = Cell::Wall;
        cells[i][n - 1] = Cell::Wall;
    }
    // Lines a = 2, 4, 6, 8 receive random structure 0..=3 in b = 1..=9.
    for a in (2..=8).step_by(2) {
        for b in 1..=9 {
            cells[a][b] = Cell::from_code(rng.range(1, 4) - 1);
        }
    }
    // 2*level + 1 extra features at the fixed candidate spots.
    for _ in 0..=(2 * level) {
        let kind = if rng.range(1, 255) > 160 { Cell::ForceField } else { Cell::Wall };
        let (p, q) = fixed_spots[(rng.range(1, 16) - 1) as usize];
        let (a, b) = if level & 1 == 1 { (p, q) } else { (q, p) };
        cells[a as usize][b as usize] = kind;
    }
    // Items: level + 1 attempts on the reseeded generator.
    rng.seed(item_seed);
    for _ in 0..=level {
        let item = if rng.range(1, 100) & 1 == 1 { Item::Coffin } else { Item::Chest };
        let a = (rng.range(10, 99) / 10) as usize;
        let b = (rng.range(10, 99) / 10) as usize;
        if !matches!(cells[a][b], Cell::Wall | Cell::SecretDoor | Cell::ForceField) {
            items[a][b] = item;
        }
    }
    // The cell east of the entrance is always open.
    cells[1][2] = Cell::Empty;
    // Ladders alternate between two fixed spots by level parity.
    let (down, up) = if level & 1 == 1 { ((3, 7), (7, 3)) } else { ((7, 3), (3, 7)) };
    cells[down.0][down.1] = Cell::LadderDown;
    cells[up.0][up.1] = Cell::LadderUp;
    if level == 10 {
        cells[7][3] = Cell::Empty; // no way further down
    }
    if level == 1 {
        cells[1][1] = Cell::LadderUp; // the exit next to the entrance
        cells[7][3] = Cell::Empty;
    }
    let mut d = DungeonLevel { level, cells, items, monsters: [[-1; DUNGEON_SIZE]; DUNGEON_SIZE], hp: [[0; DUNGEON_SIZE]; DUNGEON_SIZE], used: [false; 5] };
    for _ in 0..3 {
        d.place_monster(rng, player);
    }
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    // Synthetic candidate spots (not the game's table).
    const SPOTS: [(u8, u8); 16] = [(1, 1), (1, 3), (1, 5), (1, 7), (1, 9), (3, 1), (3, 3), (3, 5), (5, 1), (5, 3), (5, 5), (5, 9), (9, 1), (9, 3), (9, 5), (9, 9)];

    #[test]
    fn seed_formula() {
        assert_eq!(structure_seed(10, 20, 3, 100), 50 + 60 + 51 + 100);
        assert_eq!(structure_seed(0xFFFF, 0, 0, 1), 0xFFFBu16.wrapping_add(1));
    }

    #[test]
    fn border_ladders_and_determinism() {
        for level in 1..=10 {
            let d = generate(48, 11, level, 961, &SPOTS, 7);
            for i in 0..DUNGEON_SIZE {
                assert_eq!(d.cells[0][i], Cell::Wall);
                assert_eq!(d.cells[10][i], Cell::Wall);
                assert_eq!(d.cells[i][0], Cell::Wall);
                assert_eq!(d.cells[i][10], Cell::Wall);
            }
            let downs = d.cells.iter().flatten().filter(|c| **c == Cell::LadderDown).count();
            let ups = d.cells.iter().flatten().filter(|c| **c == Cell::LadderUp).count();
            match level {
                1 => assert_eq!((downs, ups), (1, 1)),
                10 => assert_eq!((downs, ups), (0, 1)),
                _ => assert_eq!((downs, ups), (1, 1)),
            }
            // Same inputs -> same structure; item seed only moves items.
            let e = generate(48, 11, level, 961, &SPOTS, 99);
            assert_eq!(d.cells, e.cells);
        }
        let l1 = generate(48, 11, 1, 961, &SPOTS, 7);
        assert_eq!(l1.cells[1][1], Cell::LadderUp);
        assert_eq!(l1.cells[3][7], Cell::LadderDown);
        let l2 = generate(48, 11, 2, 961, &SPOTS, 7);
        assert_eq!(l2.cells[3][7], Cell::LadderUp, "down from L1 lands on L2's up ladder");
    }

    #[test]
    fn odd_lines_only_change_via_spots() {
        let d = generate(1, 2, 3, 4, &SPOTS, 5);
        for a in (1..=9).step_by(2) {
            for b in 1..=9 {
                let c = d.cells[a][b];
                let spot = SPOTS.iter().any(|&(p, q)| (p as usize, q as usize) == (a, b));
                let ladder = matches!(c, Cell::LadderUp | Cell::LadderDown);
                assert!(c == Cell::Empty || spot || ladder, "unexpected {c:?} at {a},{b}");
            }
        }
    }
}
