//! What the player has seen, for the automap and the mini-map (SPEC §23.1,
//! §23.2). Kept in a sidecar file of our own beside the save
//! (`PLAYER1.U1.map`), never in the 820-byte record, which the original
//! must go on reading unchanged. Only squares that were on the screen are
//! recorded, and only as they looked: a secret door not yet found is a
//! wall, a trap is never shown.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::dungeon::{Cell, DungeonLevel, DUNGEON_SIZE};
use crate::map::{WORLD_HEIGHT, WORLD_WIDTH};

/// One remembered dungeon square.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Seen {
    Unseen,
    Floor,
    Wall,
    Door,
    LadderUp,
    LadderDown,
    Field,
}

impl Seen {
    /// As the player saw `c`: secret doors look like walls.
    pub fn of(c: Cell) -> Seen {
        match c {
            Cell::Wall | Cell::SecretDoor => Seen::Wall,
            Cell::Door => Seen::Door,
            Cell::LadderUp => Seen::LadderUp,
            Cell::LadderDown => Seen::LadderDown,
            Cell::ForceField => Seen::Field,
            Cell::Empty | Cell::Other(_) => Seen::Floor,
        }
    }

    fn ch(self) -> char {
        match self {
            Seen::Unseen => '?',
            Seen::Floor => '.',
            Seen::Wall => '#',
            Seen::Door => 'D',
            Seen::LadderUp => 'U',
            Seen::LadderDown => 'V',
            Seen::Field => 'F',
        }
    }

    fn from_ch(c: char) -> Option<Seen> {
        Some(match c {
            '?' => Seen::Unseen,
            '.' => Seen::Floor,
            '#' => Seen::Wall,
            'D' => Seen::Door,
            'U' => Seen::LadderUp,
            'V' => Seen::LadderDown,
            'F' => Seen::Field,
            _ => return None,
        })
    }
}

pub type Grid = [[Seen; DUNGEON_SIZE]; DUNGEON_SIZE];

/// A dungeon level: the overworld square of its entrance and the depth.
pub type LevelKey = (u8, u8, u16);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Explore {
    /// One bit per overworld square seen.
    world: Vec<u8>,
    /// Location ids (1..=84) entered.
    pub visited: BTreeSet<u8>,
    /// Indexed `[a][b]` as the dungeon's cells.
    pub levels: BTreeMap<LevelKey, Grid>,
}

impl Default for Explore {
    fn default() -> Self {
        Explore { world: vec![0; (WORLD_WIDTH * WORLD_HEIGHT).div_ceil(8)], visited: BTreeSet::new(), levels: BTreeMap::new() }
    }
}

/// The sidecar beside a save file.
pub fn sidecar_path(save: &Path) -> PathBuf {
    let mut s = save.as_os_str().to_os_string();
    s.push(".map");
    PathBuf::from(s)
}

const MAGIC: &str = "# ultima1-engine explored map (automap and mini-map); the save itself is untouched";

impl Explore {
    pub fn see_world(&mut self, x: i32, y: i32) {
        let (x, y) = (x.rem_euclid(WORLD_WIDTH as i32) as usize, y.rem_euclid(WORLD_HEIGHT as i32) as usize);
        let i = y * WORLD_WIDTH + x;
        self.world[i / 8] |= 1 << (i % 8);
    }

    pub fn seen_world(&self, x: usize, y: usize) -> bool {
        let i = y * WORLD_WIDTH + x;
        x < WORLD_WIDTH && y < WORLD_HEIGHT && self.world[i / 8] & (1 << (i % 8)) != 0
    }

    pub fn world_count(&self) -> usize {
        self.world.iter().map(|b| b.count_ones() as usize).sum()
    }

    /// The overworld squares on screen with the player at (x, y): the
    /// 19x9 view, which shows everything in it.
    pub fn see_view(&mut self, x: i32, y: i32) {
        use crate::rules::{PLAYER_COL, PLAYER_ROW, VIEW_COLS, VIEW_ROWS};
        for row in 0..VIEW_ROWS {
            for col in 0..VIEW_COLS {
                self.see_world(x - PLAYER_COL + col, y - PLAYER_ROW + row);
            }
        }
    }

    pub fn visit(&mut self, id: usize) {
        if (1..=84).contains(&id) {
            self.visited.insert(id as u8);
        }
    }

    /// Note the dungeon squares on screen: those [`crate::dview::visible`]
    /// lists. A door once seen stays a door (the secret doors found).
    pub fn see_level(&mut self, key: LevelKey, lv: &DungeonLevel, cells: &[(i32, i32)]) {
        let g = self.levels.entry(key).or_insert([[Seen::Unseen; DUNGEON_SIZE]; DUNGEON_SIZE]);
        for &(a, b) in cells {
            if !(0..DUNGEON_SIZE as i32).contains(&a) || !(0..DUNGEON_SIZE as i32).contains(&b) {
                continue;
            }
            let s = Seen::of(lv.cell(a, b));
            let old = &mut g[a as usize][b as usize];
            if !(*old == Seen::Door && s == Seen::Wall) {
                *old = s;
            }
        }
    }

    pub fn level(&self, key: LevelKey) -> Option<&Grid> {
        self.levels.get(&key)
    }

    pub fn is_empty(&self) -> bool {
        self.world_count() == 0 && self.visited.is_empty() && self.levels.is_empty()
    }

    pub fn to_text(&self) -> String {
        let mut s = format!("{MAGIC}\nversion 1\n");
        let v: Vec<String> = self.visited.iter().map(|n| n.to_string()).collect();
        s += &format!("visited {}\n", v.join(" "));
        let row_bytes = WORLD_WIDTH / 8;
        for y in 0..WORLD_HEIGHT {
            let row = &self.world[y * row_bytes..(y + 1) * row_bytes];
            if row.iter().any(|&b| b != 0) {
                let hex: String = row.iter().map(|b| format!("{b:02x}")).collect();
                s += &format!("world {y} {hex}\n");
            }
        }
        for (&(x, y, d), g) in &self.levels {
            let rows: Vec<String> = (0..DUNGEON_SIZE).map(|b| (0..DUNGEON_SIZE).map(|a| g[a][b].ch()).collect()).collect();
            s += &format!("level {x} {y} {d} {}\n", rows.join("/"));
        }
        s
    }

    /// Read a sidecar; lines that make no sense are skipped.
    pub fn parse(text: &str) -> Explore {
        let mut e = Explore::default();
        let row_bytes = WORLD_WIDTH / 8;
        for l in text.lines() {
            let mut w = l.split_whitespace();
            match w.next() {
                Some("visited") => {
                    for n in w.filter_map(|n| n.parse::<usize>().ok()) {
                        e.visit(n);
                    }
                }
                Some("world") => {
                    let (Some(y), Some(hex)) = (w.next().and_then(|y| y.parse::<usize>().ok()), w.next()) else { continue };
                    if y >= WORLD_HEIGHT || hex.len() != row_bytes * 2 {
                        continue;
                    }
                    for i in 0..row_bytes {
                        if let Ok(b) = u8::from_str_radix(&hex[2 * i..2 * i + 2], 16) {
                            e.world[y * row_bytes + i] = b;
                        }
                    }
                }
                Some("level") => {
                    let n: Vec<Option<u16>> = (0..3).map(|_| w.next().and_then(|v| v.parse().ok())).collect();
                    let (Some(x), Some(y), Some(d), Some(rows)) = (n[0], n[1], n[2], w.next()) else { continue };
                    let rows: Vec<&str> = rows.split('/').collect();
                    if rows.len() != DUNGEON_SIZE || x > 255 || y > 255 {
                        continue;
                    }
                    let mut g = [[Seen::Unseen; DUNGEON_SIZE]; DUNGEON_SIZE];
                    for (b, r) in rows.iter().enumerate() {
                        for (a, c) in r.chars().take(DUNGEON_SIZE).enumerate() {
                            g[a][b] = Seen::from_ch(c).unwrap_or(Seen::Unseen);
                        }
                    }
                    e.levels.insert((x as u8, y as u8, d), g);
                }
                _ => {}
            }
        }
        e
    }

    /// The sidecar of `save`, or nothing explored.
    pub fn load(save: &Path) -> Explore {
        std::fs::read_to_string(sidecar_path(save)).map(|t| Explore::parse(&t)).unwrap_or_default()
    }

    /// Write the sidecar of `save` (only ever that file).
    /// Written to a temporary file first and renamed, so a write cut
    /// short never leaves half a map.
    pub fn save(&self, save: &Path) -> std::io::Result<()> {
        let p = sidecar_path(save);
        let mut tmp = p.clone().into_os_string();
        tmp.push(".tmp");
        std::fs::write(&tmp, self.to_text())?;
        std::fs::rename(&tmp, &p)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn world_bits_and_wrap() {
        let mut e = Explore::default();
        assert!(e.is_empty());
        e.see_world(-1, 156);
        assert!(e.seen_world(167, 0));
        e.see_view(45, 38);
        assert_eq!(e.world_count(), 19 * 9 + 1);
        assert!(e.seen_world(45, 38) && e.seen_world(36, 34) && e.seen_world(54, 42));
        assert!(!e.seen_world(35, 34) && !e.seen_world(45, 43));
    }

    #[test]
    fn text_round_trip() {
        let mut e = Explore::default();
        e.see_view(3, 3);
        e.visit(5);
        e.visit(49);
        e.visit(0);
        e.visit(99);
        let mut g = [[Seen::Unseen; DUNGEON_SIZE]; DUNGEON_SIZE];
        g[1][1] = Seen::Floor;
        g[2][1] = Seen::Door;
        g[1][2] = Seen::LadderDown;
        g[0][1] = Seen::Wall;
        e.levels.insert((45, 38, 2), g);
        let t = e.to_text();
        assert!(t.contains("visited 5 49\n"));
        assert_eq!(Explore::parse(&t), e);
        assert_eq!(Explore::parse("rubbish\nworld 900 00\nlevel 1 2\n"), Explore::default());
    }

    #[test]
    fn levels_show_what_was_seen() {
        let n = DUNGEON_SIZE;
        let mut lv = DungeonLevel { level: 1, cells: [[Cell::Empty; 11]; 11], items: [[crate::dungeon::Item::None; 11]; 11], monsters: [[-1; 11]; 11], hp: [[0; 11]; 11], used: [false; 5] };
        assert_eq!(n, 11);
        lv.cells[2][1] = Cell::SecretDoor;
        lv.cells[3][1] = Cell::LadderUp;
        let mut e = Explore::default();
        let k = (1, 2, 1);
        e.see_level(k, &lv, &[(1, 1), (2, 1), (3, 1), (-1, 4)]);
        let g = e.level(k).unwrap();
        assert_eq!(g[2][1], Seen::Wall, "an unfound secret door is a wall");
        assert_eq!(g[3][1], Seen::LadderUp);
        assert_eq!(g[5][5], Seen::Unseen, "only what was seen");
        // Found: a door, and it stays one when the level is built again.
        lv.cells[2][1] = Cell::Door;
        e.see_level(k, &lv, &[(2, 1)]);
        lv.cells[2][1] = Cell::SecretDoor;
        e.see_level(k, &lv, &[(2, 1)]);
        assert_eq!(e.level(k).unwrap()[2][1], Seen::Door);
    }

    #[test]
    fn sidecar_beside_the_save() {
        assert_eq!(sidecar_path(Path::new("/s/PLAYER1.U1")), PathBuf::from("/s/PLAYER1.U1.map"));
        let dir = std::env::temp_dir().join(format!("u1-explore-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let save = dir.join("PLAYER2.U1");
        let rec = vec![7u8; crate::save::SAVE_SIZE];
        std::fs::write(&save, &rec).unwrap();
        let mut e = Explore::default();
        e.visit(33);
        e.save(&save).unwrap();
        assert_eq!(std::fs::read(&save).unwrap(), rec, "the record is not touched");
        assert_eq!(Explore::load(&save), e);
        assert_eq!(Explore::load(&dir.join("NONE.U1")), Explore::default());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
