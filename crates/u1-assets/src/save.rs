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
    pub const POS_X: usize = 0x34;
    pub const POS_Y: usize = 0x36;
    pub const GEMS: usize = 0x4C; // 4 x i16: red, green, blue, white
    pub const ARMOUR: usize = 0x56; // 5 x i16
    pub const WEAPONS: usize = 0x62; // 15 x i16
    pub const SPELLS: usize = 0x82; // 10 x i16
    pub const VEHICLES: usize = 0x98; // 7 x i16: horse cart raft frigate aircar shuttle timemachine
    pub const LAST_SIGNPOST: usize = 0xA8;
    pub const STEPS: usize = 0xAC;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapObject {
    pub kind: u16,
    pub unknown_02: u16,
    pub x: u16,
    pub y: u16,
    pub hits: u16,
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
}
