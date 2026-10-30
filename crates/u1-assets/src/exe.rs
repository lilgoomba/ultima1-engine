//! Read-only access to strings and tables in the user's OUT.EXE.
//!
//! The engine never embeds the original's messages or tables: everything
//! below is a location in the program's data segment (file offset
//! `DATA_BASE` + offset), "verified from original executable". Messages are
//! read at run time from the user's own copy.

use crate::error::{AssetError, Result};
use crate::locations::OUT_EXE_SIZE;
use crate::rules::DATA_BASE;

#[derive(Debug, Clone)]
pub struct Exe {
    data: Vec<u8>,
}

impl Exe {
    pub fn new(data: Vec<u8>) -> Result<Self> {
        if data.len() != OUT_EXE_SIZE {
            return Err(AssetError::BadSize { what: "OUT.EXE".into(), expected: OUT_EXE_SIZE.to_string(), got: data.len() });
        }
        Ok(Exe { data })
    }

    /// Like `new` but without the size check (for synthetic test images).
    pub fn unchecked(data: Vec<u8>) -> Self {
        Exe { data }
    }

    /// NUL-terminated string at a data-segment offset.
    pub fn s(&self, off: usize) -> String {
        let start = DATA_BASE + off;
        if start >= self.data.len() {
            return String::new();
        }
        let end = self.data[start..].iter().position(|&b| b == 0).map_or(self.data.len(), |p| start + p);
        self.data[start..end].iter().map(|&b| b as char).collect()
    }

    /// Little-endian word at a data-segment offset.
    pub fn w(&self, off: usize) -> u16 {
        let o = DATA_BASE + off;
        match self.data.get(o..o + 2) {
            Some(b) => u16::from_le_bytes([b[0], b[1]]),
            None => 0,
        }
    }

    /// Entry `i` of a table of string pointers.
    pub fn ptr(&self, table: usize, i: usize) -> String {
        self.s(self.w(table + 2 * i) as usize)
    }
}

/// Data-segment offsets of messages and tables used by towns and castles.
pub mod m {
    pub const EMPTY: usize = 0x019B;
    pub const QUESTION: usize = 0x0235;
    pub const COLON: usize = 0x0237;
    pub const NOTHING: usize = 0x0220;
    pub const MISSED: usize = 0x01E1;
    pub const ATTACK_WITH: usize = 0x0228;
    pub const BOARD_Q: usize = 0x023A;
    pub const NO_EFFECT: usize = 0x034C;
    pub const CAST_DASH: usize = 0x0385;
    pub const PENCE_PROMPT: usize = 0x03D7;
    pub const NONE: usize = 0x03DF;
    pub const NOT_THAT_MUCH: usize = 0x03E4;
    pub const OK: usize = 0x03CB;
    pub const SHAZAM: usize = 0x03CF;
    pub const ALAKAZOT: usize = 0x03FD;
    pub const DROP_TITLE: usize = 0x0407;
    pub const DROP_Q: usize = 0x0423;
    pub const DROP_MENU: usize = 0x0429;
    pub const PAREN: usize = 0x0417;
    pub const IOLO_STOLE: usize = 0x0443;
    pub const JESTER_STOLE: usize = 0x0459;
    pub const IOLO_SINGS: usize = 0x0476;
    pub const IOLO_SONG: usize = 0x048B;
    pub const JESTER_SINGS: usize = 0x049B;
    pub const JESTER_SONG: usize = 0x04B3;
    pub const ATTACKED_BY_GUARD: usize = 0x04C5;
    pub const HIT_DOTS: usize = 0x04D8;
    pub const HIT_SP: usize = 0x04E0;
    pub const NPC_KING: usize = 0x04E5;
    pub const NPC_GUARD: usize = 0x04EC;
    pub const NPC_MERCHANT: usize = 0x04F4;
    pub const NPC_PRINCESS: usize = 0x04FF;
    pub const NPC_BARD: usize = 0x050A;
    pub const NPC_JESTER: usize = 0x0511;
    pub const NPC_WENCH: usize = 0x051A;
    pub const NPC_LECHER: usize = 0x0522;
    pub const KILLED: usize = 0x052B;
    pub const FOUND_KEY: usize = 0x0533;
    pub const BLOCKED: usize = 0x054A;
    pub const DAMAGE: usize = 0x01D8;
    pub const PRINCESS_SAVED: [usize; 4] = [0x08C8, 0x08E6, 0x08F9, 0x0913];
    pub const PRINCESS_ACE: [usize; 4] = [0x0926, 0x0943, 0x0962, 0x097F];
    pub const ENTER: usize = 0x0BB0;
    pub const ING: usize = 0x0BB6;
    pub const CITY_OF: usize = 0x0BBD;
    pub const FIRE_Q: usize = 0x0BCA;
    pub const NOT_KINGS: usize = 0x0BEF;
    pub const PERMISSION: usize = 0x0C08;
    pub const NOTHING_HERE: usize = 0x0C14;
    pub const GET: usize = 0x0C25;
    pub const WHAT: usize = 0x0C29;
    pub const HYPERJUMP_Q: usize = 0x0C2F;
    pub const INFORM: usize = 0x0C59;
    pub const CITY_OF_LC: usize = 0x0C6B;
    pub const AT_SEA: usize = 0x0C78;
    pub const IN_WOODS: usize = 0x0C87;
    pub const IN_LANDS: usize = 0x0C9C;
    pub const KLIMB: usize = 0x0CDC;
    pub const SP_WHAT: usize = 0x0CE3;
    pub const NOISE: usize = 0x0D05;
    pub const ON: usize = 0x0D0C;
    pub const OFF: usize = 0x0D0F;
    pub const OPEN: usize = 0x0D13;
    pub const SAVING: usize = 0x0D2E;
    pub const SAVED: usize = 0x0D3D;
    pub const QUIT: usize = 0x0D44;
    pub const ONLY_OUTDOORS: usize = 0x0D4C;
    pub const CAUGHT: usize = 0x0D63;
    pub const FIND_A: usize = 0x0D7D;
    pub const FIND: usize = 0x0C3A;
    pub const BAGS_OF: usize = 0x0D8E;
    pub const FOOD_EXCL: usize = 0x0D97;
    pub const A_SP: usize = 0x0D9D;
    pub const STEAL: usize = 0x0DA0;
    pub const SOLD: usize = 0x0DA6;
    pub const DONE: usize = 0x0DAC;
    pub const CANT_AFFORD: usize = 0x0DB2;
    pub const BEST_KNOW: usize = 0x0DCC;
    pub const BROKE: [usize; 3] = [0x10D0, 0x10E0, 0x10F9];
    pub const KEEPER: usize = 0x110E;
    pub const COLD_ONE: usize = 0x1128;
    pub const ALE: usize = 0x113F;
    pub const PLENTY: [usize; 2] = [0x1143, 0x115B];
    pub const NO_WEAPONS_TO_SELL: usize = 0x1164;
    pub const NO_ARMOUR_TO_SELL: usize = 0x1186;
    pub const NO_SPELLS_BOUGHT: usize = 0x11A5;
    pub const USED_FOOD: usize = 0x11C3;
    pub const PACKS_COST: usize = 0x11DA;
    pub const PENCE: usize = 0x11F2;
    pub const EACH_HOW_MANY: usize = 0x11F9;
    pub const WISH_PURCHASE: usize = 0x1213;
    pub const PACKS_FOOD: usize = 0x1225;
    pub const NO_USED_STUFF: [usize; 2] = [0x1231, 0x1249];
    pub const CLOSED: usize = 0x1255;
    pub const TRANSACT: usize = 0x1269;
    pub const NONE_WILL_TALK: usize = 0x1272;
    pub const NOT_BY_COUNTER: usize = 0x128A;
    pub const BUY_SELL: usize = 0x12A5;
    pub const SP_NOTHING: usize = 0x12B2;
    pub const BLANK9: usize = 0x12C1;
    pub const BLANK12: usize = 0x12CB;
    pub const SELL: usize = 0x12D8;
    pub const DASH: usize = 0x1183;
    pub const GO_KILL: usize = 0x12DF;
    pub const KILL_TARGETS: [usize; 4] = [0x12F1, 0x1302, 0x1313, 0x131D];
    pub const GO_FIND: usize = 0x1328;
    pub const DO_NOT_RETURN: [usize; 2] = [0x133A, 0x134E];
    pub const WILL_GIVE: usize = 0x1361;
    pub const POINT: usize = 0x137B;
    pub const OF_STRENGTH: usize = 0x1382;
    pub const SHALL_TELL: usize = 0x1390;
    /// Gem speeches by land: (column, row, message) lines.
    pub const GEM_LINES: [[(usize, usize, usize); 4]; 4] = [
        [(5, 9, 0x13AB), (5, 10, 0x13CA), (5, 12, 0x13E6), (0, 0, 0)],
        [(5, 9, 0x1405), (5, 10, 0x1423), (5, 12, 0x143E), (5, 13, 0x1459)],
        [(5, 9, 0x146F), (5, 10, 0x148B), (5, 12, 0x14A6), (5, 13, 0x14C0)],
        [(5, 9, 0x14D4), (5, 10, 0x14F2), (5, 12, 0x1510), (5, 13, 0x152E)],
    ];
    pub const AH: usize = 0x1543;
    pub const THOU_HAST: usize = 0x1548;
    pub const COMPLETED: usize = 0x1555;
    pub const TRANSACT_KING: usize = 0x156A;
    pub const NOT_HERE: usize = 0x157D;
    pub const REJECTS: usize = 0x158D;
    pub const OFFER: [usize; 2] = [0x15A5, 0x15BB];
    pub const NEITHER: usize = 0x15C8;
    pub const PENCE_WORD: usize = 0x15D0;
    pub const HOW_MUCH: usize = 0x15D6;
    pub const IN_RETURN: usize = 0x15E1;
    pub const THEE: usize = 0x15F7;
    pub const HIT_POINTS: usize = 0x15FD;
    pub const ON_QUEST: [usize; 2] = [0x1609, 0x161D];
    pub const TRANSACT_Q: usize = 0x162D;
    pub const VIEW_Q: usize = 0x1637;
    pub const NO_KEY: usize = 0x165E;
    pub const WRONG_KEY: [usize; 2] = [0x1673, 0x1681];
    pub const DOOR_OPEN: usize = 0x1692;
    pub const UNLOCK: usize = 0x16A4;
    pub const XIT_Q: usize = 0x16AB;
    pub const PASS: usize = 0x16C1;
    pub const HUH: usize = 0x16C6;
    pub const PRESS_SPACE: usize = 0x19D2;
    pub const READY_TITLE: usize = 0x267E;
    pub const READY_WEAPON: usize = 0x2689;
    pub const READY_ARMOUR: usize = 0x2692;
    pub const READY_SPELL: usize = 0x269B;
    pub const READY_MENU: usize = 0x26A3;
    pub const READY_NOTHING: usize = 0x26BE;
    pub const ZTATS: usize = 0x3969;
    pub const INVENTORY: usize = 0x396F;
    pub const PLAYER: usize = 0x397B;
    pub const A_LEVEL: usize = 0x3984;
    pub const MALE: usize = 0x398D;
    pub const FEMALE: usize = 0x3994;
    pub const COPPER: usize = 0x399D;
    pub const SILVER: usize = 0x39AB;
    pub const GOLD: usize = 0x39B9;
    pub const ENEMY_VESSELS: usize = 0x39C7;
    pub const MORE: usize = 0x3962;

    // Pointer tables.
    pub const WEAPON_LONG: usize = 0x277E;
    pub const WEAPON_SHORT: usize = 0x2804;
    pub const WEAPON_RANGE: usize = 0x26DE;
    pub const ARMOUR_NAMES: usize = 0x2968;
    pub const SPELL_NAMES: usize = 0x2910;
    pub const TRANSPORT_NAMES: usize = 0x29AE;
    pub const STAT_NAMES: usize = 0x28AA;
    pub const RACE_NAMES: usize = 0x283C;
    pub const CLASS_NAMES: usize = 0x2860;
    pub const LOCATION_NAMES: usize = 0x307C;
    pub const REGION_NAMES: usize = 0x3174;
    pub const PRINCESS_NAMES: usize = 0x31CC;
    pub const NPC_TABLES: usize = 0x31DC;
    pub const GEM_NAMES: usize = 0x29E4;
    /// Quest landmarks for odd slots 1,3,5,7: (pointer word, column).
    pub const QUEST_PLACES: [(usize, usize); 4] = [(0x30DC, 8), (0x30D8, 10), (0x30D4, 9), (0x30D0, 9)];

    /// Pub tips 1..=8 after "Thou had best know": (column, row, message).
    pub const TIPS: [&[(usize, usize, usize)]; 8] = [
        &[(10, 8, 0x0E1A), (10, 9, 0x0E2E), (9, 10, 0x0E43), (11, 11, 0x0E5A)],
        &[(11, 8, 0x0E6C)],
        &[(5, 8, 0x0E89), (5, 9, 0x0EA6), (5, 10, 0x0EC3), (10, 11, 0x0EE2)],
        &[(7, 8, 0x0EF7)],
        &[(10, 8, 0x0F12), (13, 9, 0x0F26)],
        &[(5, 8, 0x0F34), (5, 9, 0x0F51)],
        &[(10, 8, 0x0F6F)],
        &[(5, 8, 0x0F85), (5, 9, 0x0FA2), (5, 10, 0x0FC0), (5, 11, 0x0FDE), (15, 12, 0x0FFB)],
    ];
    pub const TIP_WENCH: usize = 0x0E7A;
    pub const TIP_LECHER: usize = 0x0E81;
    pub const SEDUCED: [(usize, usize, usize); 3] = [(9, 8, 0x0DDF), (11, 10, 0x0DF7), (13, 11, 0x0E0B)];
    pub const QUEST_TEXT: [(usize, usize, usize); 7] = [
        (5, 7, 0x1005),
        (5, 8, 0x1023),
        (8, 9, 0x1041),
        (5, 10, 0x105A),
        (5, 11, 0x1077),
        (5, 12, 0x1095),
        (5, 13, 0x10B2),
    ];

    /// Shop titles by kind and town index (layout - 2): lines of
    /// (column, row, message). Empty = no such shop in that town.
    pub const TITLE_TRANSPORT: [&[(usize, usize, usize)]; 8] = [
        &[(10, 4, 0x553), (13, 5, 0x569)],
        &[],
        &[],
        &[(9, 4, 0x579)],
        &[],
        &[(7, 4, 0x590)],
        &[],
        &[(8, 4, 0x5AC), (15, 5, 0x5C5)],
    ];
    pub const TITLE_ARMOUR: [&[(usize, usize, usize)]; 8] = [
        &[(11, 4, 0x5D1), (8, 5, 0x5E4)],
        &[(15, 4, 0x5FE)],
        &[(13, 4, 0x60C)],
        &[(10, 4, 0x61A)],
        &[],
        &[(5, 4, 0x62F)],
        &[],
        &[(11, 4, 0x64D)],
    ];
    pub const TITLE_WEAPON: [&[(usize, usize, usize)]; 8] = [
        &[(11, 4, 0x660), (8, 5, 0x5E4)],
        &[(11, 4, 0x673)],
        &[(12, 4, 0x686)],
        &[(10, 4, 0x697)],
        &[(12, 4, 0x6AC)],
        &[(13, 4, 0x6BD)],
        &[(12, 4, 0x6CC)],
        &[(10, 4, 0x697)],
    ];
    pub const TITLE_MAGIC: [&[(usize, usize, usize)]; 8] =
        [&[(9, 4, 0x6DC)], &[(12, 4, 0x6F4)], &[(13, 4, 0x705)], &[(5, 4, 0x714)], &[], &[], &[], &[(11, 4, 0x733)]];
    pub const TITLE_FOOD: [&[(usize, usize, usize)]; 8] = [
        &[(5, 4, 0x747)],
        &[(8, 4, 0x766)],
        &[],
        &[(13, 4, 0x77F)],
        &[(9, 4, 0x78D)],
        &[(11, 4, 0x7A4)],
        &[(6, 4, 0x7B6)],
        &[(8, 4, 0x7D3)],
    ];
    pub const TITLE_PUB: [&[(usize, usize, usize)]; 8] = [
        &[(14, 4, 0x7EC)],
        &[(11, 4, 0x7F8)],
        &[(13, 4, 0x80A)],
        &[(13, 4, 0x819)],
        &[],
        &[(12, 4, 0x828)],
        &[(10, 4, 0x836)],
        &[(11, 4, 0x84A)],
    ];
}
