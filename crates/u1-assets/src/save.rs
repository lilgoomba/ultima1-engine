//! Player record (`PLAYER1.U1`..`PLAYER4.U1`, `INUSE.U1`). See docs/SPEC.md §7.
//!
//! 820 bytes, little-endian 16-bit fields, per ModdingWiki "Ultima I Save
//! Game Format" (TheAlmightyGuru & Daniel D'Agostino). Only the fields
//! documented there are exposed; everything else is preserved verbatim in
//! `raw` so a record round-trips byte-for-byte.

use crate::error::{AssetError, Result};

pub const SAVE_SIZE: usize = 820;
pub const OBJECTS_OFFSET: usize = 0xB4;
pub const OBJECT_COUNT: usize = 40;
pub const OBJECT_SIZE: usize = 16;

/// Documented field offsets.
pub mod off {
    pub const NAME: usize = 0x00; // ASCIIZ, 14 chars max + NUL
    pub const RACE: usize = 0x10;
    pub const CLASS: usize = 0x12;
    pub const SEX: usize = 0x14;
    pub const HITS: usize = 0x16;
    pub const STATS: usize = 0x18; // STR AGI STA CHA WIS INT
    pub const COIN: usize = 0x24;
    pub const EXPERIENCE: usize = 0x26;
    pub const FOOD: usize = 0x28;
    pub const READY_WEAPON: usize = 0x2A;
    pub const READY_SPELL: usize = 0x2C;
    pub const READY_ARMOUR: usize = 0x2E;
    pub const TRANSPORT: usize = 0x30;
    /// The random seed word the programs start from.
    pub const SEED: usize = 0x32;
    pub const POS_X: usize = 0x34;
    pub const POS_Y: usize = 0x36;
    pub const GEMS: usize = 0x4C; // 4 x i16: red, green, blue, white
    pub const ARMOUR: usize = 0x56; // 5 x i16
    pub const WEAPONS: usize = 0x62; // 15 x i16
    pub const SPELLS: usize = 0x82; // 10 x i16
    pub const VEHICLES: usize = 0x98; // 7 x i16: horse cart raft frigate aircar shuttle timemachine
    pub const LAST_SIGNPOST: usize = 0xA8;
    /// Enemy vessels destroyed (space).
    pub const ENEMIES: usize = 0xA6;
    /// Index of the last used object slot (slot 0 is the player).
    pub const LAST_OBJECT: usize = 0xAA;
    /// Sound on/off flag toggled by the "Noise" command.
    pub const SOUND: usize = 0x38;
    /// Quest words, one per castle: slot k (1..=8) at QUESTS + 2k; slot 0 unused.
    pub const QUESTS: usize = 0x3A;
    /// Outdoor move counter, 32-bit: low word here, high word at STEPS_HIGH.
    pub const STEPS: usize = 0xAC;
    pub const STEPS_HIGH: usize = 0xAE;
    /// Both set to 1000 when a shuttle is bought (probably fuel/shield values
    /// for the space section; meaning not confirmed).
    pub const SHUTTLE_0: usize = 0xB0;
    pub const SHUTTLE_1: usize = 0xB2;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapObject {
    pub kind: u16,
    pub unknown_02: u16,
    pub x: u16,
    pub y: u16,
    pub hits: u16,
}

/// Value of a castle quest word (SPEC §7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestState {
    None,
    /// 0xFFFF: given by the king, not yet done.
    Active,
    /// 1: done, waiting to be reported to the king.
    Done,
    Other(u16),
}

/// What a quest slot asks for: even slots = kill a dungeon monster, odd slots
/// = visit a landmark (landmark index 8 - k).
pub fn quest_landmark(k: usize) -> Option<usize> {
    (k % 2 == 1 && k <= 7).then(|| 8 - k)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerRecord {
    raw: Vec<u8>,
}

fn rd(b: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([b[o], b[o + 1]])
}

impl PlayerRecord {
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() != SAVE_SIZE {
            return Err(AssetError::BadSize {
                what: "PLAYERn.U1".into(),
                expected: SAVE_SIZE.to_string(),
                got: data.len(),
            });
        }
        Ok(PlayerRecord { raw: data.to_vec() })
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.raw
    }

    pub fn u16_at(&self, o: usize) -> u16 {
        rd(&self.raw, o)
    }

    pub fn set_u16(&mut self, o: usize, v: u16) {
        self.raw[o..o + 2].copy_from_slice(&v.to_le_bytes());
    }

    pub fn name(&self) -> String {
        let n = &self.raw[off::NAME..off::NAME + 15];
        let end = n.iter().position(|&c| c == 0).unwrap_or(n.len());
        n[..end].iter().map(|&c| c as char).collect()
    }

    pub fn race(&self) -> u16 { self.u16_at(off::RACE) }
    pub fn class(&self) -> u16 { self.u16_at(off::CLASS) }
    pub fn sex(&self) -> u16 { self.u16_at(off::SEX) }
    pub fn hits(&self) -> u16 { self.u16_at(off::HITS) }
    pub fn coin(&self) -> u16 { self.u16_at(off::COIN) }
    pub fn experience(&self) -> u16 { self.u16_at(off::EXPERIENCE) }
    pub fn food(&self) -> u16 { self.u16_at(off::FOOD) }
    pub fn transport(&self) -> u16 { self.u16_at(off::TRANSPORT) }

    /// Strength, Agility, Stamina, Charisma, Wisdom, Intelligence.
    pub fn stats(&self) -> [u16; 6] {
        std::array::from_fn(|i| self.u16_at(off::STATS + 2 * i))
    }

    pub fn position(&self) -> (u16, u16) {
        (self.u16_at(off::POS_X), self.u16_at(off::POS_Y))
    }

    /// 32-bit outdoor move counter.
    pub fn moves(&self) -> u32 {
        u32::from(self.u16_at(off::STEPS)) | (u32::from(self.u16_at(off::STEPS_HIGH)) << 16)
    }

    pub fn set_moves(&mut self, m: u32) {
        self.set_u16(off::STEPS, m as u16);
        self.set_u16(off::STEPS_HIGH, (m >> 16) as u16);
    }

    /// Quest state for castle slot k (1..=8; castle location id - 32).
    pub fn quest(&self, k: usize) -> QuestState {
        match self.u16_at(off::QUESTS + 2 * k) {
            0 => QuestState::None,
            0xFFFF => QuestState::Active,
            1 => QuestState::Done,
            n => QuestState::Other(n),
        }
    }

    /// Overwrite the first ten bytes of object slot `i`.
    pub fn set_object(&mut self, i: usize, o: &MapObject) {
        let b = OBJECTS_OFFSET + i * OBJECT_SIZE;
        for (k, v) in [o.kind, o.unknown_02, o.x, o.y, o.hits].into_iter().enumerate() {
            self.set_u16(b + 2 * k, v);
        }
    }

    pub fn objects(&self) -> Vec<MapObject> {
        (0..OBJECT_COUNT)
            .map(|i| {
                let b = OBJECTS_OFFSET + i * OBJECT_SIZE;
                MapObject {
                    kind: rd(&self.raw, b),
                    unknown_02: rd(&self.raw, b + 2),
                    x: rd(&self.raw, b + 4),
                    y: rd(&self.raw, b + 6),
                    hits: rd(&self.raw, b + 8),
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synthetic() -> Vec<u8> {
        let mut d = vec![0u8; SAVE_SIZE];
        d[..6].copy_from_slice(b"Avatar");
        let put = |d: &mut Vec<u8>, o: usize, v: u16| d[o..o + 2].copy_from_slice(&v.to_le_bytes());
        put(&mut d, off::RACE, 2);
        put(&mut d, off::CLASS, 1);
        put(&mut d, off::SEX, 1);
        put(&mut d, off::HITS, 9999); // 0F 27 as in the public cheat notes
        for i in 0..6 {
            put(&mut d, off::STATS + 2 * i, 10 + i as u16);
        }
        put(&mut d, off::FOOD, 321);
        put(&mut d, off::POS_X, 40);
        put(&mut d, off::POS_Y, 50);
        let o = OBJECTS_OFFSET + 39 * OBJECT_SIZE;
        put(&mut d, o, 0x0B);
        put(&mut d, o + 4, 7);
        put(&mut d, o + 6, 8);
        d
    }

    #[test]
    fn parses_documented_fields() {
        let d = synthetic();
        assert_eq!(&d[off::HITS..off::HITS + 2], &[0x0F, 0x27]);
        let p = PlayerRecord::parse(&d).unwrap();
        assert_eq!(p.name(), "Avatar");
        assert_eq!((p.race(), p.class(), p.sex()), (2, 1, 1));
        assert_eq!(p.hits(), 9999);
        assert_eq!(p.stats(), [10, 11, 12, 13, 14, 15]);
        assert_eq!(p.food(), 321);
        assert_eq!(p.position(), (40, 50));
        let objs = p.objects();
        assert_eq!(objs.len(), 40);
        assert_eq!((objs[39].kind, objs[39].x, objs[39].y), (0x0B, 7, 8));
        assert_eq!(OBJECTS_OFFSET + OBJECT_COUNT * OBJECT_SIZE, SAVE_SIZE);
    }

    #[test]
    fn round_trips_and_edits() {
        let d = synthetic();
        let mut p = PlayerRecord::parse(&d).unwrap();
        assert_eq!(p.as_bytes(), &d[..]);
        p.set_u16(off::COIN, 500);
        assert_eq!(p.coin(), 500);
        assert!(PlayerRecord::parse(&d[..334]).is_err());
    }

    #[test]
    fn moves_and_quests() {
        let mut p = PlayerRecord::parse(&synthetic()).unwrap();
        p.set_moves(70_000);
        assert_eq!(p.u16_at(off::STEPS_HIGH), 1);
        assert_eq!(p.moves(), 70_000);
        p.set_u16(off::QUESTS + 2 * 3, 0xFFFF);
        p.set_u16(off::QUESTS + 2 * 4, 1);
        assert_eq!(p.quest(3), QuestState::Active);
        assert_eq!(p.quest(4), QuestState::Done);
        assert_eq!(p.quest(5), QuestState::None);
        assert_eq!(quest_landmark(1), Some(7));
        assert_eq!(quest_landmark(7), Some(1));
        assert_eq!(quest_landmark(2), None);
    }
}
