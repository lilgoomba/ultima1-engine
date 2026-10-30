//! The whole game as the original runs it: one main loop that reads a key,
//! runs a command, and then lets the world take its turn (SPEC §9).
//!
//! Everything is drawn into a [`Term`] (the original's 320x200 screen) and
//! every wait goes through an [`Io`], so the SDL front end, the screenshot
//! mode and the tests all drive the same code. Messages and tables are read
//! at run time from the user's own OUT.EXE ([`Exe`]); the player record is
//! kept in the original 820-byte layout ([`PlayerRecord`]) and is the one
//! store for inventory and statistics, as in the original program.
//!
//! Behaviour marked "verified from original executable" in SPEC §9 was
//! checked by reference study of the original; the code is our own.

use std::path::PathBuf;

use crate::exe::{m, Exe};
use crate::io::{keys, Io, IoResult};
use crate::locations::{kind_of, LocationKind, LocationTable};
use crate::map::{water_scroll, Terrain, WorldMap};
use crate::overworld::{tile_for_transport, Entity, Overworld};
use crate::rules::{self, Dir, ExeTables, VIEW_COLS, VIEW_ROWS};
use crate::save::{off, MapObject, PlayerRecord, SAVE_SIZE};
use crate::tcd::{LocalMap, TownCastleMaps, TCD_HEIGHT, TCD_WIDTH};
use crate::term::{ArtId, Term, BLACK, BLUE, TEXT, WARN, WHITE};
use crate::tiles::TileSet;

/// Save-record offsets of the inventory arrays (item k at base + 2k).
pub mod slot {
    pub const STAT: usize = 0x16; // 0 hits, 1 STR, 2 AGI, 3 STA, 4 CHA, 5 WIS, 6 INT
    pub const QUEST: usize = 0x3A;
    pub const GEM: usize = 0x4C; // k = 0..=3
    pub const ARMOUR: usize = 0x54;
    pub const WEAPON: usize = 0x60;
    pub const SPELL: usize = 0x80;
    pub const VEHICLE: usize = 0x96;
    pub const TIME_MACHINES: usize = 0xA4;
}

/// OUT.EXE data: five 8x8 frames of the blinking cursor (`c655`).
const CURSOR_FRAMES: usize = 0x3a24;

/// World tile of an arrow or cannon ball in flight.
pub const SHOT_TILE: u8 = 0x32;
/// World tile of a magic bolt in flight and of the prayer flash.
pub const MAGIC_TILE: u8 = 0x31;
pub const HITS: usize = 0;
pub const STR: usize = 1;
pub const AGI: usize = 2;
pub const STA: usize = 3;
pub const CHA: usize = 4;
pub const WIS: usize = 5;
pub const INT: usize = 6;

/// Modes of the main loop.
pub const OUTDOORS: u8 = 0;
pub const TOWN: u8 = 1;
pub const CASTLE: u8 = 2;

/// Town / castle NPC tiles.
pub const NPC_GUARD: i16 = 0x11;
pub const PLAYER_TILE: i16 = 0x12;
pub const NPC_BARD: i16 = 0x13;
pub const NPC_KING: i16 = 0x14;
pub const NPC_MERCHANT: i16 = 0x15;
pub const NPC_PRINCESS: i16 = 0x16;
pub const NPC_WENCH: i16 = 0x32;

/// Everything loaded from the user's game directory.
#[derive(Debug, Clone)]
pub struct Assets {
    pub exe: Exe,
    pub t: ExeTables,
    pub locs: LocationTable,
    pub map: WorldMap,
    pub world_tiles: TileSet,
    pub town_tiles: TileSet,
    pub tcd: TownCastleMaps,
    /// The display the original would have picked (SPEC §15).
    pub video: crate::gamedir::VideoMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Npc {
    pub tile: i16,
    pub x: i16,
    pub y: i16,
    pub hp: i16,
}

pub struct Game {
    pub a: Assets,
    /// Town and castle maps as changed during this session (jail doors).
    pub maps: Vec<LocalMap>,
    pub ow: Overworld,
    pub rec: PlayerRecord,
    pub term: Term,
    pub mode: u8,
    pub prev_mode: u8,
    /// Location id (1..=84) of the town or castle we are in.
    pub loc: usize,
    pub layout: usize,
    pub npcs: [Npc; 15],
    pub px: i32,
    pub py: i32,
    pub hostile: bool,
    pub has_key: bool,
    pub door_open: bool,
    pub storeroom: i16,
    pub lord_british: bool,
    pub princess: usize,
    pub quest_slot: usize,
    pub jail_door: u8,
    pub ale: i16,
    pub water: i16,
    pub grass: i16,
    pub forest: i16,
    /// Objects with a vehicle tile (and the player) counted at load; the
    /// transport shop closes at 15.
    pub vehicle_objs: i16,
    pub player_aux: u16,
    pub redraw_town: bool,
    pub counter: u32,
    /// Where Quit writes the player record (never the game directory).
    pub save_to: Option<PathBuf>,
    /// Set when a feature outside this engine's scope was reached (space,
    /// Mondain's time machine, dungeons).
    pub stopped: Option<&'static str>,
    /// The dungeon being explored (mode 3).
    pub dungeon: Option<crate::dgame::DungeonState>,
    pub view_tables: Option<crate::dview::ViewTables>,
    /// Local time (hour, minute, second, hundredths) for the original's
    /// time-of-day seeds; `None` uses `fixed_time` (tests, `--seed`).
    pub clock: Option<fn() -> [u8; 4]>,
    pub fixed_time: u16,
    /// Clock values to use, in order, before falling back to `fixed_time`
    /// (lets tests replay a run recorded at known times).
    pub fixed_times: std::collections::VecDeque<u16>,
    /// SPACE.EXE and its sprites, when present.
    pub space: Option<crate::space::SpaceFiles>,
    /// MONDAIN.EXE and its pictures, when present.
    pub mondain: Option<crate::mondain::MondainFiles>,
    /// OUT was restarted (after space); the turn ends without a world turn.
    pub restarted: bool,
    /// Next blinking-cursor frame (`3a50`).
    pub cursor_phase: u8,
}

impl Game {
    pub fn new(a: Assets, rec: PlayerRecord, seed: u16) -> Self {
        let ow = Overworld::from_save(&rec, seed);
        let cga = a.video == crate::gamedir::VideoMode::Cga;
        let maps = a.tcd.maps.clone();
        let objs = rec.objects();
        let last = usize::from(rec.u16_at(off::LAST_OBJECT)).min(39);
        let vehicle_objs = objs.iter().take(last + 1).filter(|o| o.kind <= 0x11).count() as i16;
        let player_aux = objs[0].unknown_02;
        let mut g = Game {
            a,
            maps,
            ow,
            rec,
            term: Term::with_cga(cga),
            mode: OUTDOORS,
            prev_mode: OUTDOORS,
            loc: 0,
            layout: 0,
            npcs: [Npc::default(); 15],
            px: 0,
            py: 0,
            hostile: false,
            has_key: false,
            door_open: false,
            storeroom: 0,
            lord_british: false,
            princess: 0,
            quest_slot: 0,
            jail_door: 0x3C,
            ale: 0,
            water: 0,
            grass: 0,
            forest: 0,
            vehicle_objs,
            player_aux,
            redraw_town: false,
            counter: 0,
            cursor_phase: 0,
            save_to: None,
            stopped: None,
            dungeon: None,
            view_tables: None,
            clock: None,
            fixed_time: seed,
            fixed_times: Default::default(),
            space: None,
            mondain: None,
            restarted: false,
        };
        g.redraw();
        g
    }

    /// A fresh record for a character with stock values (used when no save
    /// is given; the original creates characters in a separate program).
    pub fn default_record(name: &str, x: u16, y: u16) -> PlayerRecord {
        let mut r = PlayerRecord::parse(&vec![0u8; SAVE_SIZE]).expect("size");
        let mut raw = r.as_bytes().to_vec();
        for (i, b) in name.bytes().take(14).enumerate() {
            raw[i] = b;
        }
        r = PlayerRecord::parse(&raw).expect("size");
        r.set_u16(off::HITS, 150);
        for i in 0..6 {
            r.set_u16(off::STATS + 2 * i, 20);
        }
        r.set_u16(off::COIN, 100);
        r.set_u16(off::FOOD, 200);
        r.set_u16(off::POS_X, x);
        r.set_u16(off::POS_Y, y);
        r.set_u16(slot::WEAPON + 2, 1);
        r.set_u16(slot::ARMOUR + 2, 1);
        r.set_u16(off::SOUND, 1);
        r.set_object(0, &MapObject { kind: 0x0A, unknown_02: 1, x, y, hits: 0 });
        r
    }

    // ------------------------------------------------------------ record

    pub fn w(&self, o: usize) -> i16 {
        self.rec.u16_at(o) as i16
    }

    pub fn set(&mut self, o: usize, v: i16) {
        self.rec.set_u16(o, v as u16);
    }

    pub fn add(&mut self, o: usize, v: i16) {
        let n = self.w(o).wrapping_add(v);
        self.set(o, n);
    }

    pub fn stat(&self, i: usize) -> i16 {
        self.w(slot::STAT + 2 * i)
    }

    pub fn set_stat(&mut self, i: usize, v: i16) {
        self.set(slot::STAT + 2 * i, v);
    }

    pub fn hits(&self) -> i16 {
        self.w(off::HITS)
    }
    pub fn coin(&self) -> i16 {
        self.w(off::COIN)
    }
    pub fn food(&self) -> i16 {
        self.w(off::FOOD)
    }
    pub fn exp(&self) -> i16 {
        self.w(off::EXPERIENCE)
    }
    pub fn qty(&self, base: usize, k: usize) -> i16 {
        self.w(base + 2 * k)
    }
    pub fn transport(&self) -> u16 {
        self.rec.u16_at(off::TRANSPORT)
    }

    /// `R(lo,hi)`: the original's ranged random number.
    pub fn r(&mut self, lo: i32, hi: i32) -> i32 {
        i32::from(self.ow.rng.range(lo as u16, hi as u16))
    }

    pub fn moves(&self) -> u32 {
        self.ow.moves
    }

    /// Copy the record's values into the outdoor state before it runs.
    pub fn to_ow(&mut self) {
        let r = &self.rec;
        self.ow.hits = r.hits();
        self.ow.food = r.food();
        self.ow.exp = r.experience();
        self.ow.coin = r.coin();
        self.ow.stats = r.stats();
        self.ow.armour = r.u16_at(off::READY_ARMOUR);
        self.ow.transport = r.transport();
    }

    /// And back afterwards.
    pub fn from_ow(&mut self) {
        let o = &self.ow;
        let (h, f, e, c, t) = (o.hits, o.food, o.exp, o.coin, o.transport);
        self.rec.set_u16(off::HITS, h);
        self.rec.set_u16(off::FOOD, f);
        self.rec.set_u16(off::EXPERIENCE, e);
        self.rec.set_u16(off::COIN, c);
        self.rec.set_u16(off::TRANSPORT, t);
        self.rec.set_u16(off::POS_X, self.ow.x as u16);
        self.rec.set_u16(off::POS_Y, self.ow.y as u16);
    }

    /// Write the live state (position, objects, move counter) into the
    /// record, as the original's data segment holds it when it saves.
    pub fn sync_record(&mut self) {
        self.from_ow();
        let objs = self.rec.objects();
        let p = objs[0];
        let tile = tile_for_transport(self.transport());
        self.rec.set_object(
            0,
            &MapObject { kind: u16::from(tile), unknown_02: self.player_aux, x: self.ow.x as u16, y: self.ow.y as u16, hits: p.hits },
        );
        let ents: Vec<Entity> = self.ow.entities.iter().take(39).copied().collect();
        for (i, e) in ents.iter().enumerate() {
            self.rec.set_object(i + 1, &MapObject { kind: u16::from(e.tile), unknown_02: e.aux, x: e.x as u16, y: e.y as u16, hits: e.hp });
        }
        self.rec.set_u16(off::LAST_OBJECT, ents.len() as u16);
        let mv = self.ow.moves;
        self.rec.set_moves(mv);
    }

    // ------------------------------------------------------------ output

    pub fn snd(&mut self, io: &mut dyn Io, n: u8) {
        if self.rec.u16_at(off::SOUND) != 0 {
            io.sound(n);
        }
    }

    pub fn s(&self, o: usize) -> String {
        self.a.exe.s(o)
    }

    pub fn ptr(&self, table: usize, i: usize) -> String {
        self.a.exe.ptr(table, i)
    }

    /// `a043`.
    pub fn say(&mut self, o: usize) {
        let s = self.s(o);
        self.term.print(&s);
    }

    /// `ac0a`.
    pub fn sayln(&mut self, o: usize) {
        let s = self.s(o);
        self.term.println(&s);
    }

    /// `21e7`.
    pub fn at(&mut self, col: usize, row: usize, o: usize) {
        let s = self.s(o);
        self.term.print_at(col, row, &s);
    }

    pub fn nl(&mut self) {
        self.term.println("");
    }

    /// A message ending with a sound-1 "error" beep.
    pub fn err(&mut self, io: &mut dyn Io, o: usize) {
        self.sayln(o);
        self.snd(io, 1);
    }

    pub fn show_hits(&mut self) {
        let v = self.hits();
        self.term.stat(21, i32::from(v), true);
    }
    pub fn show_food(&mut self) {
        let v = self.food();
        self.term.stat(22, i32::from(v), true);
    }
    pub fn show_exp(&mut self) {
        let v = self.exp();
        self.term.stat(23, i32::from(v), false);
    }
    pub fn show_coin(&mut self) {
        let v = self.coin();
        self.term.stat(24, i32::from(v), false);
    }
    pub fn show_stats(&mut self) {
        let (c, r) = (self.term.col, self.term.row);
        self.show_hits();
        self.show_food();
        self.show_exp();
        self.show_coin();
        self.term.cursor(c, r);
    }

    /// Redraw the viewport for the current mode and the stats.
    pub fn redraw(&mut self) {
        if self.mode == OUTDOORS {
            self.draw_world();
        } else if self.mode == crate::dgame::DUNGEON {
            self.draw_dungeon();
        } else {
            self.draw_town();
        }
        self.show_stats();
    }

    fn tile(&mut self, set: bool, idx: usize, x: usize, y: usize, shift: usize) {
        self.tile_over(set, idx, None, None, x, y, shift);
    }

    #[allow(clippy::too_many_arguments)]
    fn tile_over(&mut self, set: bool, idx: usize, under: Option<ArtId>, pos: Option<(u16, u16)>, x: usize, y: usize, shift: usize) {
        let tiles = if set { &self.a.town_tiles } else { &self.a.world_tiles };
        let Some(px) = tiles.tiles.get(idx) else { return };
        let (w, h) = (tiles.width, tiles.height);
        let px = px.clone();
        let id = if set { ArtId::Town(idx as u16) } else { ArtId::World(idx as u16) };
        self.term.blit_art(id, under, x, y, w, h, &px, shift);
        if let Some(p) = pos {
            self.term.art_pos(p);
        }
    }

    /// Outdoor viewport: 19x9 tiles of the world around the player.
    pub fn draw_world(&mut self) {
        for row in 0..VIEW_ROWS {
            for col in 0..VIEW_COLS {
                self.draw_world_cell(col, row);
            }
        }
    }

    /// What the view buffer holds for a viewport cell: the player's
    /// transport, an object, else `None` (terrain).
    fn world_object(&self, col: i32, row: i32) -> Option<u8> {
        let (mx, my) = rules::view_cell(self.ow.x, self.ow.y, col, row);
        if col == rules::PLAYER_COL && row == rules::PLAYER_ROW {
            Some(tile_for_transport(self.transport()))
        } else {
            self.ow.entity_at(mx, my).map(|i| self.ow.entities[i].tile)
        }
    }

    /// `c380` for one cell: the object there, else the terrain, in its base
    /// frame (water at the current scroll phase).
    pub(crate) fn draw_world_cell(&mut self, col: i32, row: i32) {
        let tw = self.a.world_tiles.width.max(1);
        let (mx, my) = rules::view_cell(self.ow.x, self.ow.y, col, row);
        match self.world_object(col, row) {
            Some(t) => self.put_world(col, row, t as usize, 0),
            None => {
                let terr = self.a.map.cell(mx as usize, my as usize);
                let f = Terrain::from_cell(terr).tile().0;
                let s = if terr == 0 { water_scroll(self.counter as usize, tw) } else { 0 };
                self.put_world(col, row, f, s);
            }
        }
    }

    fn put_world(&mut self, col: i32, row: i32, t: usize, shift: usize) {
        let tw = self.a.world_tiles.width.max(1);
        let th = self.a.world_tiles.height.max(1);
        let (sx, sy) = (8 + col as usize * tw, 8 + row as usize * th);
        // Objects stand on terrain (tiles 0-9): note which, and the map
        // cell (picks a picture variant), for the enhanced renderer.
        let (mx, my) = rules::view_cell(self.ow.x, self.ow.y, col, row);
        let under = (t > 9).then(|| ArtId::World(Terrain::from_cell(self.a.map.cell(mx as usize, my as usize)).tile().0 as u16));
        let pos = (mx >= 0 && my >= 0).then_some((mx as u16, my as u16));
        self.tile_over(false, t, under, pos, sx, sy, shift);
    }

    /// `af57(p)`: the per-poll outdoor animation, `p` counting the polls
    /// of the current prompt from 1. The water scrolls one step; castles
    /// show tile 4 when `p % 6 == 0`, tile 5 when `p % 3 == 0` and are
    /// left alone otherwise; towns show 7 when `p % 4 == 0`, else 8; the
    /// frigate and tile 0x19 alternate with `p`; every other monster
    /// (0x13..=0x2F) draws its base tile plus `R(1,100) % 2`, so each
    /// visible monster costs one roll of the game's generator per poll,
    /// column by column, top to bottom.
    pub(crate) fn world_tick(&mut self, p: u32) {
        self.counter = self.counter.wrapping_add(1);
        let tw = self.a.world_tiles.width.max(1);
        for col in 0..VIEW_COLS {
            for row in 0..VIEW_ROWS {
                let (mx, my) = rules::view_cell(self.ow.x, self.ow.y, col, row);
                match self.world_object(col, row) {
                    Some(t @ (0x0E | 0x19)) => self.put_world(col, row, (t as u32 + (p & 1)) as usize, 0),
                    Some(t) if rules::is_monster_tile(t) => {
                        let f = t as i32 + self.r(1, 100) % 2;
                        self.put_world(col, row, f as usize, 0);
                    }
                    Some(_) => {}
                    None => match self.a.map.cell(mx as usize, my as usize) {
                        0 => self.put_world(col, row, 0, water_scroll(self.counter as usize, tw)),
                        4 if p % 6 == 0 => self.put_world(col, row, 4, 0),
                        4 if p % 3 == 0 => self.put_world(col, row, 5, 0),
                        6 => self.put_world(col, row, if p % 4 == 0 { 7 } else { 8 }, 0),
                        _ => {}
                    },
                }
            }
        }
    }

    /// `a5b4` with a fixed tile: a shot or the prayer flash on a viewport
    /// cell.
    pub(crate) fn draw_world_tile(&mut self, col: i32, row: i32, t: u8) {
        if !(0..VIEW_COLS).contains(&col) || !(0..VIEW_ROWS).contains(&row) {
            return;
        }
        let tw = self.a.world_tiles.width.max(1);
        let th = self.a.world_tiles.height.max(1);
        self.tile(false, t as usize, 8 + col as usize * tw, 8 + row as usize * th, 0);
    }

    /// `2231` + `22b8`: town or castle map, the player and the NPCs.
    pub fn draw_town(&mut self) {
        let tw = self.a.town_tiles.width.max(1);
        let th = self.a.town_tiles.height.max(1);
        for y in 0..TCD_HEIGHT {
            for x in 0..TCD_WIDTH {
                let t = self.town_ground(x as i32, y as i32).unwrap_or(1);
                self.tile_over(true, t, None, Some((x as u16, y as u16)), 8 + x * tw, 8 + y * th, 0);
            }
        }
        self.town_sprite(PLAYER_TILE as usize, self.px, self.py);
        self.draw_npcs();
    }

    /// The town map tile at a cell (what a person stands on).
    fn town_ground(&self, x: i32, y: i32) -> Option<usize> {
        if !(0..TCD_WIDTH as i32).contains(&x) || !(0..TCD_HEIGHT as i32).contains(&y) {
            return None;
        }
        let c = self.maps[self.layout].cell(x as usize, y as usize);
        Some(if c < 0x33 { c as usize } else { 1 })
    }

    /// A person in town: the tile, noting the ground under it and the
    /// cell for the enhanced renderer.
    fn town_sprite(&mut self, t: usize, x: i32, y: i32) {
        let tw = self.a.town_tiles.width.max(1);
        let th = self.a.town_tiles.height.max(1);
        let Some(g) = self.town_ground(x, y) else { return };
        let under = Some(ArtId::Town(g as u16));
        self.tile_over(true, t, under, Some((x as u16, y as u16)), 8 + x as usize * tw, 8 + y as usize * th, 0);
    }

    pub fn draw_npcs(&mut self) {
        for n in self.npcs {
            if n.tile != -1 {
                self.town_sprite(n.tile as usize, n.x as i32, n.y as i32);
            }
        }
    }

    // ------------------------------------------------------------ input

    pub fn key(&mut self, io: &mut dyn Io) -> IoResult<u8> {
        loop {
            if let Some(k) = self.poll_key(io)? {
                return Ok(k);
            }
        }
    }

    /// `b97f`/`c5bc`: one keyboard poll, drawing the next frame of the
    /// blinking cursor first (four frames from OUT.EXE `3a24`, in the text
    /// colour); a key blanks it (frame 4) and the next poll starts at frame 1.
    pub fn poll_key(&mut self, io: &mut dyn Io) -> IoResult<Option<u8>> {
        self.draw_cursor(self.cursor_phase);
        self.cursor_phase = (self.cursor_phase + 1) & 3;
        let k = io.poll(&self.term)?;
        if k.is_some() {
            self.draw_cursor(4);
            self.cursor_phase = 1;
        }
        Ok(k)
    }

    fn draw_cursor(&mut self, frame: u8) {
        let mut rows = [0u8; 8];
        for (r, v) in rows.iter_mut().enumerate() {
            *v = self.a.exe.b(CURSOR_FRAMES + frame as usize * 8 + r);
        }
        self.term.cursor_frame(rows, TEXT);
    }

    pub fn key_upper(&mut self, io: &mut dyn Io) -> IoResult<u8> {
        Ok(self.key(io)?.to_ascii_uppercase())
    }

    /// `b9b9`: a key, or 0 after 1500 polls.
    pub fn key_timeout(&mut self, io: &mut dyn Io) -> IoResult<u8> {
        for _ in 0..io.timeout_polls() {
            if let Some(k) = self.poll_key(io)? {
                return Ok(k.to_ascii_uppercase());
            }
        }
        Ok(0)
    }

    /// `2141`: a key or a short while.
    pub fn wait(&mut self, io: &mut dyn Io) -> IoResult<()> {
        io.wait_or_key(&self.term)
    }

    /// `ab39`: "Press Space to continue: " and wait for space or enter.
    pub fn press_space(&mut self, io: &mut dyn Io) -> IoResult<()> {
        let s = self.s(m::PRESS_SPACE);
        self.term.print_at(1, 24, &s);
        loop {
            let k = self.key(io)?;
            if k == b' ' || k == keys::ENTER {
                break;
            }
        }
        self.term.fill(0, 192, 230, 199, BLACK);
        self.term.cursor(1, 24);
        Ok(())
    }

    /// `b0b5`: up to four digits typed at (col,row).
    pub fn number(&mut self, io: &mut dyn Io, col: usize, row: usize) -> IoResult<i16> {
        let mut digits: Vec<u8> = Vec::new();
        let mut n = 0usize;
        while n < 4 {
            let k = self.key(io)?;
            if k == keys::BACKSPACE || k == keys::LEFT {
                n = n.saturating_sub(1);
                continue;
            }
            if !k.is_ascii_digit() {
                break;
            }
            if n == 0 && k == b'0' {
                continue;
            }
            self.term.put_char(col + n, row, k as char, TEXT);
            digits.truncate(n);
            digits.push(k - b'0');
            n += 1;
        }
        Ok(digits[..n.min(digits.len())].iter().fold(0i16, |a, &d| a.wrapping_mul(10).wrapping_add(i16::from(d))))
    }

    // ------------------------------------------------------------ main loop

    /// Run until the key source halts (scripted runs) or the window closes.
    pub fn run(&mut self, io: &mut dyn Io) -> IoResult<()> {
        loop {
            self.turn(io)?;
            if self.stopped.is_some() {
                return Ok(());
            }
        }
    }

    /// One pass of the main loop: prompt, key, command, world turn.
    pub fn turn(&mut self, io: &mut dyn Io) -> IoResult<()> {
        if self.hits() <= 0 || self.food() <= 0 {
            self.death(io)?;
        }
        self.term.ornament_left(0, 24);
        self.term.cursor(1, 24);
        if self.mode == OUTDOORS {
            self.draw_world();
        }
        // `8eb9`: poll, count, animate the outdoors after every poll (the
        // one that brings the key too); 1500 polls without a key pass.
        let mut polls = 0u32;
        let key = loop {
            let k = self.poll_key(io)?;
            polls += 1;
            if self.mode == OUTDOORS {
                self.world_tick(polls);
            }
            if let Some(k) = k {
                break k;
            }
            if polls >= 1500 {
                break b' ';
            }
        };
        self.command(io, key.to_ascii_uppercase())?;
        if self.stopped.is_some() {
            return Ok(());
        }
        if self.restarted {
            self.restarted = false;
            return Ok(());
        }
        self.end_of_turn(io)
    }

    pub fn command(&mut self, io: &mut dyn Io, key: u8) -> IoResult<()> {
        match key {
            b' ' => {
                self.sayln(m::PASS);
            }
            keys::LEFT | keys::RIGHT | keys::UP | keys::DOWN if self.mode == crate::dgame::DUNGEON => {
                self.dungeon_move(io, key)
            }
            keys::LEFT => self.cmd_move(io, Dir::West)?,
            keys::RIGHT => self.cmd_move(io, Dir::East)?,
            keys::UP => self.cmd_move(io, Dir::North)?,
            keys::DOWN => self.cmd_move(io, Dir::South)?,
            b'A' => self.cmd_attack(io, None)?,
            b'8' => self.cmd_attack(io, Some(Dir::North))?,
            b'2' => self.cmd_attack(io, Some(Dir::South))?,
            b'6' => self.cmd_attack(io, Some(Dir::East))?,
            b'4' => self.cmd_attack(io, Some(Dir::West))?,
            b'B' => self.cmd_board(io)?,
            b'C' => self.cmd_cast_spell(io),
            b'D' => self.cmd_drop(io)?,
            b'E' => self.cmd_enter(io)?,
            b'F' => self.cmd_fire(io)?,
            b'G' => self.cmd_get(io),
            b'H' => self.err(io, m::HYPERJUMP_Q),
            b'I' if self.mode == crate::dgame::DUNGEON => self.dungeon_inform(),
            b'I' => self.cmd_inform(),
            b'K' => self.cmd_klimb(io),
            b'N' => self.cmd_noise(),
            b'O' => self.cmd_open(io),
            b'Q' => self.cmd_quit(io),
            b'R' => self.cmd_ready(io)?,
            b'S' => self.cmd_steal(io),
            b'T' => self.cmd_transact(io)?,
            b'U' => self.cmd_unlock(io),
            b'V' => self.err(io, m::VIEW_Q),
            b'X' => self.cmd_xit(io),
            b'Z' => {
                if self.mode == crate::dgame::DUNGEON {
                    self.restore_dungeon_borders();
                }
                self.cmd_ztats(io)?;
                if let Some(d) = self.dungeon.as_mut() {
                    d.title = true;
                }
                self.redraw();
                self.term.cursor(0, 24);
            }
            _ => self.err(io, m::HUH),
        }
        Ok(())
    }

    /// After every command: outdoors the monsters and the food clock, in
    /// towns the move counter and the NPCs.
    pub fn end_of_turn(&mut self, io: &mut dyn Io) -> IoResult<()> {
        if self.mode == OUTDOORS {
            self.to_ow();
            self.ow.messages.clear();
            self.ow.attacks.clear();
            self.ow.dead = false;
            self.ow.end_turn(&self.a.map, &self.a.t);
            let msgs: Vec<String> = self.ow.messages.drain(..).collect();
            for s in msgs {
                self.term.println(&s);
            }
            let attacks: Vec<_> = self.ow.attacks.drain(..).collect();
            for at in attacks {
                self.monster_attack_msg(io, at)?;
            }
            self.from_ow();
            self.show_stats();
        } else if self.mode == crate::dgame::DUNGEON {
            // No food is eaten below ground.
            self.ow.moves = self.ow.moves.wrapping_add(1);
            self.dungeon_monsters(io);
            if self.mode == crate::dgame::DUNGEON && self.hits() > 0 {
                self.draw_dungeon();
            }
        } else {
            self.ow.moves = self.ow.moves.wrapping_add(1);
            self.npc_turn(io)?;
        }
        self.prev_mode = self.mode;
        Ok(())
    }

    // ------------------------------------------------------------ outdoors

    /// The lines of an outdoor monster attack (`81fc`).
    fn monster_attack_msg(&mut self, io: &mut dyn Io, at: crate::overworld::MonsterAttack) -> IoResult<()> {
        let name = self.a.t.monster_names[at.kind].clone();
        self.term.print(&name);
        self.say(m::ATTACKS);
        self.snd(io, 3);
        match at.damage {
            None => {
                if name.len() > 8 {
                    self.nl();
                } else {
                    self.term.print(" ");
                }
                self.sayln(m::MISSED);
            }
            Some(d) => {
                self.snd(io, 2);
                if name.len() > 10 {
                    self.nl();
                    self.say(m::HIT_EX);
                } else {
                    self.sayln(m::SP_HIT);
                }
                self.term.num(d, 2, ' ', TEXT, true);
                self.sayln(m::SP_DAMAGE);
                self.show_hits();
            }
        }
        Ok(())
    }

    fn cmd_move(&mut self, io: &mut dyn Io, d: Dir) -> IoResult<()> {
        if self.mode != OUTDOORS {
            return self.town_move(io, d);
        }
        self.to_ow();
        self.ow.messages.clear();
        let (x0, y0) = (self.ow.x, self.ow.y);
        self.ow.step_only(d, &self.a.map, &self.a.t, &self.a.locs);
        let msgs: Vec<String> = self.ow.messages.drain(..).collect();
        for s in msgs {
            self.term.println(s.strip_prefix('>').unwrap_or(&s));
        }
        if (self.ow.x, self.ow.y) == (x0, y0) {
            self.snd(io, 1);
        }
        self.from_ow();
        Ok(())
    }

    /// Board `118d` (verified from original executable).
    fn cmd_board(&mut self, io: &mut dyn Io) -> IoResult<()> {
        if self.mode != OUTDOORS {
            self.err(io, m::BOARD_Q);
            return Ok(());
        }
        if self.transport() != 0 {
            self.err(io, m::XIT_FIRST);
            return Ok(());
        }
        let Some(i) = self.ow.entity_at(self.ow.x, self.ow.y).filter(|&i| (0x0B..=0x12).contains(&self.ow.entities[i].tile))
        else {
            self.err(io, m::NOTHING_TO_BOARD);
            return Ok(());
        };
        let t = self.ow.entities[i].tile;
        self.ow.entities.remove(i);
        let mut tr = u16::from(t - 0x0B) + 1;
        if tr > 4 {
            tr -= 1;
        }
        self.rec.set_u16(off::TRANSPORT, tr);
        self.say(if t <= 0x0C { m::MOUNT } else { m::BOARD });
        let name = self.ptr(m::TRANSPORT_NAMES, tr as usize);
        self.term.println(&name);
        self.draw_world();
        if t == 0x11 {
            if self.space.is_some() {
                self.launch(io)?;
            } else {
                self.stopped = Some("space travel (SPACE.EXE) is not part of this engine yet");
            }
        } else if t == 0x12 {
            if (0..4).all(|k| self.qty(slot::GEM, k) > 0) {
                if self.mondain.is_some() {
                    self.time_travel(io)?;
                } else {
                    self.stopped = Some("MONDAIN.EXE (the time machine ending) was not found");
                }
            } else {
                self.term.boxed(self.term_blue());
                for (col, row, o) in m::TIME_MACHINE_LINES {
                    self.at(col, row, o);
                }
                self.wait(io)?;
                self.redraw();
            }
        }
        Ok(())
    }

    /// Shuttle launch: OUT hands over to SPACE.EXE, which hands back by
    /// restarting OUT from the record (`ebd`, SPEC §12).
    fn launch(&mut self, io: &mut dyn Io) -> IoResult<()> {
        self.ow.transport = self.transport();
        self.sync_record();
        crate::space::run_space(self, io)?;
        self.restart_out();
        Ok(())
    }

    /// The time machine with all four gems: OUT hands over to MONDAIN.EXE.
    /// A fallen player comes back through OUT's death and resurrection; a
    /// victory ends the game (the original waits for a reboot).
    fn time_travel(&mut self, io: &mut dyn Io) -> IoResult<()> {
        self.ow.transport = self.transport();
        self.sync_record();
        match crate::mondain::run_mondain(self, io)? {
            crate::mondain::MondainEnd::Won => self.stopped = Some("Mondain is defeated; the game ends here as in the original"),
            crate::mondain::MondainEnd::Died => self.restart_out(),
        }
        Ok(())
    }

    /// OUT starting again from the record: a new generator seed, fresh
    /// town maps, the outdoor state rebuilt; the message lines stay.
    pub fn restart_out(&mut self) {
        let ts = self.time_seed_now();
        let seed = self.rec.u16_at(off::SEED).wrapping_add(ts % 999);
        self.ow = crate::overworld::Overworld::from_save(&self.rec, seed);
        self.maps = self.a.tcd.maps.clone();
        let objs = self.rec.objects();
        let last = usize::from(self.rec.u16_at(off::LAST_OBJECT)).min(39);
        self.vehicle_objs = objs.iter().take(last + 1).filter(|o| o.kind <= 0x11).count() as i16;
        self.player_aux = objs[0].unknown_02;
        self.mode = OUTDOORS;
        self.prev_mode = OUTDOORS;
        self.term.restore_labels();
        self.redraw();
        self.restarted = true;
    }

    fn term_blue(&self) -> u8 {
        BLUE
    }

    /// X-it `7d6b` (verified from original executable).
    fn cmd_xit(&mut self, io: &mut dyn Io) {
        if self.mode != OUTDOORS {
            self.err(io, m::XIT_Q);
            return;
        }
        if self.transport() == 0 {
            self.err(io, m::XIT_WHAT);
            return;
        }
        let (x, y) = (self.ow.x, self.ow.y);
        let ter = self.cell_or_object(x, y);
        let mut land = false;
        for dx in -2..=2 {
            for dy in -2..=2 {
                let (cx, cy) = rules::wrap(x + dx, y + dy);
                if self.a.map.cell(cx as usize, cy as usize) != 0 {
                    land = true;
                }
            }
        }
        if ter > 2 || !land {
            let o = self.a.exe.w(m::CANT_LEAVE_PTR) as usize;
            self.err(io, o);
            return;
        }
        let old = tile_for_transport(self.transport());
        if self.ow.entities.len() < 39 {
            self.ow.entities.push(Entity { tile: old, x, y, hp: 0, aux: u16::from(old) });
        }
        self.rec.set_u16(off::TRANSPORT, 0);
        self.player_aux = u16::from(old);
        self.sayln(m::XIT);
    }

    /// `c2af(x,y,1)`: object tile at a world cell, else its terrain.
    pub fn cell_or_object(&self, x: i32, y: i32) -> u8 {
        let (x, y) = rules::wrap(x, y);
        match self.ow.entity_at(x, y) {
            Some(i) => self.ow.entities[i].tile,
            None => self.a.map.cell(x as usize, y as usize),
        }
    }

    /// Death and resurrection `c47c` (verified from original executable).
    pub fn death(&mut self, io: &mut dyn Io) -> IoResult<()> {
        if self.mode == crate::dgame::DUNGEON {
            self.restore_dungeon_borders();
            self.dungeon = None;
        }
        self.mode = OUTDOORS;
        self.set(off::FOOD, 0);
        self.set(off::HITS, 0);
        self.set(off::COIN, 0);
        self.show_stats();
        self.term.clear_view();
        self.draw_death_picture();
        for _ in 0..3 {
            self.snd(io, 2);
        }
        let name = self.rec.name();
        let dead = self.s(m::THOU_ART_DEAD);
        self.term.println(&format!("{name}{dead}"));
        io.pause(&self.term, 80)?;
        self.sayln(m::RESURRECTION);
        for h in 1..=99 {
            self.set(off::HITS, h);
            self.show_hits();
            io.pause(&self.term, 1)?;
        }
        // `86e1` once for each of the four lands: every monster outdoors goes.
        self.ow.entities.retain(|e| e.tile < 0x13);
        let land = rules::land_of(self.ow.x, self.ow.y);
        for f in 1..=99 {
            self.set(off::FOOD, f);
            self.show_food();
        }
        let tr = self.transport() as usize;
        if tr != 0 {
            self.add(slot::VEHICLE + 2 * tr, -1);
            self.set(off::TRANSPORT, 0);
        }
        for k in 1..=15 {
            self.set(slot::WEAPON + 2 * k, 0);
        }
        self.set(off::READY_WEAPON, 0);
        self.set(off::READY_SPELL, 0);
        self.set(off::READY_ARMOUR, 0);
        self.snd(io, 5);
        // 8c20: a random grass or forest cell nearby in the same land.
        let (x0, y0) = (self.ow.x, self.ow.y);
        for _ in 0..10_000 {
            let mut x = x0 + self.r(3, 23) - 13;
            let mut y = y0 + self.r(3, 23) - 13;
            if !(2..=165).contains(&x) {
                x = x0;
            }
            if !(2..=153).contains(&y) {
                y = y0;
            }
            if rules::land_of(x, y) != land {
                continue;
            }
            let t = self.a.map.cell(x as usize, y as usize);
            if (t == 1 || t == 2) && self.ow.entity_at(x, y).is_none() {
                self.ow.x = x;
                self.ow.y = y;
                self.player_aux = u16::from(t);
                break;
            }
        }
        // The record holds the restored values; the outdoor state takes
        // them before handing back its new position.
        self.to_ow();
        self.from_ow();
        self.ow.dead = false;
        self.redraw();
        Ok(())
    }

    /// `ab9d`: the death picture, a 64x95 one-colour bitmap from OUT's
    /// data drawn at (128,31) in the foreground colour (white on EGA).
    fn draw_death_picture(&mut self) {
        for r in 0..95 {
            for k in 0..4 {
                let w = self.a.exe.w(m::DEATH_PICTURE + r * 8 + k * 2);
                for i in 0..16 {
                    if w & (0x8000 >> i) != 0 {
                        self.term.plot(128 + (k * 16 + i) as i32, 31 + r as i32, 15);
                    }
                }
            }
        }
    }

    /// Enter `52ab` (verified from original executable).
    fn cmd_enter(&mut self, io: &mut dyn Io) -> IoResult<()> {
        self.say(m::ENTER);
        let p = self.a.locs.id_at(self.ow.x as u8, self.ow.y as u8);
        if p == 0 || self.mode != OUTDOORS {
            self.err(io, m::QUESTION);
            return Ok(());
        }
        self.sayln(m::ING);
        self.water = 0;
        self.grass = 0;
        self.forest = 0;
        for dx in -1..=1 {
            for dy in -1..=1 {
                match self.cell_or_object(self.ow.x + dx, self.ow.y + dy) {
                    0 => self.water += 1,
                    1 => self.grass += 1,
                    2 => self.forest += 1,
                    _ => {}
                }
            }
        }
        self.redraw_town = true;
        self.lord_british = p == 0x21;
        if p < 0x21 {
            self.say(m::CITY_OF);
        }
        let name = self.ptr(m::LOCATION_NAMES, p);
        self.term.println(&name);
        match kind_of(p) {
            Some(LocationKind::Town) => self.enter_town(p),
            Some(LocationKind::Castle) => self.enter_castle(p),
            Some(LocationKind::Landmark) => self.landmark(io, p)?,
            Some(LocationKind::Dungeon) => self.enter_dungeon(),
            None => {}
        }
        Ok(())
    }

    fn load_npcs(&mut self) {
        let base = m::NPC_TABLES + self.layout * 0x78;
        for i in 0..15 {
            let w = |k: usize| self.a.exe.w(base + 8 * i + 2 * k) as i16;
            self.npcs[i] = Npc { tile: w(0), x: w(1), y: w(2), hp: w(3) };
        }
    }

    /// Town entry `4e63`.
    pub fn enter_town(&mut self, p: usize) {
        self.ale = 0;
        self.loc = p;
        self.layout = (p - 1) % 8 + 2;
        self.mode = TOWN;
        self.hostile = false;
        self.load_npcs();
        self.px = 19;
        self.py = 17;
        self.redraw();
    }

    /// Castle entry `4dbf`.
    pub fn enter_castle(&mut self, p: usize) {
        self.loc = p;
        self.princess = p - 0x21;
        self.layout = (p - 1) % 2;
        self.quest_slot = p - 0x20;
        self.mode = CASTLE;
        self.load_npcs();
        self.px = 0;
        self.py = 9;
        self.hostile = false;
        self.has_key = false;
        self.door_open = false;
        self.storeroom = 0;
        let o = if self.layout == 0 { 14 } else { 4 };
        let map = &mut self.maps[self.layout];
        map.cells[o * TCD_WIDTH + 35] = 0x0B;
        map.cells[o * TCD_WIDTH + 31] = 0x0B;
        self.jail_door = if self.r(1, 255) & 1 == 1 { 0x3D } else { 0x3C };
        self.redraw();
    }

    /// Landmark `50c6` (verified from original executable).
    fn landmark(&mut self, io: &mut dyn Io, p: usize) -> IoResult<()> {
        let l = p - 0x29;
        let mut effect = true;
        self.term.boxed(BLUE);
        for &(col, row, o) in m::LANDMARK_TEXT[l] {
            self.at(col, row, o);
        }
        self.term.cursor(1, 24);
        if l & 1 == 1 {
            let k = 8 - l;
            if self.w(slot::QUEST + 2 * k) == -1 {
                self.set(slot::QUEST + 2 * k, 1);
                self.say(m::QUEST_COMPLETED);
                self.snd(io, 5);
                self.nl();
            }
        }
        if l == self.rec.u16_at(off::LAST_SIGNPOST) as usize {
            effect = false;
        } else {
            self.rec.set_u16(off::LAST_SIGNPOST, l as u16);
            let idx = self.a.exe.w(m::LANDMARK_STAT + 2 * l) as usize;
            if idx == 0 && l == 2 {
                match (1..=15).find(|&k| self.qty(slot::WEAPON, k) == 0) {
                    None => effect = false,
                    Some(k) => {
                        self.add(slot::WEAPON + 2 * k, 1);
                        self.say(m::YOU_FIND_A);
                        let short = self.ptr(m::WEAPON_SHORT, k);
                        if short.starts_with(['A', 'E', 'I', 'O', 'U', 'a', 'e', 'i', 'o', 'u']) {
                            self.term.print("n");
                        }
                        self.term.print(" ");
                        let long = self.ptr(m::WEAPON_LONG, k);
                        self.term.print(&long);
                        self.say(m::EXCL);
                    }
                }
            } else if l != 6 {
                let v = self.stat(idx);
                if v == 99 {
                    effect = false;
                } else {
                    let mut gain = (v + 8) / 10;
                    if v + gain > 99 {
                        gain = 99 - v;
                    }
                    self.say(m::THOU_DOST_GAIN);
                    self.term.num(gain as u16, 1, ' ', TEXT, true);
                    self.term.print(" ");
                    let sn = self.ptr(m::STAT_NAMES, idx);
                    self.term.print(&sn);
                    self.set_stat(idx, v + gain);
                }
            }
        }
        if l == 6 {
            effect = false;
        }
        if effect {
            self.flash_frame(WHITE);
            self.snd(io, 5);
            self.flash_frame(BLUE);
        } else if l != 6 {
            self.say(m::NO_EFFECT_LM);
            self.snd(io, 6);
        }
        self.wait(io)?;
        if l != 6 {
            self.nl();
        }
        self.redraw();
        Ok(())
    }

    /// `2168`: the box frame in a colour.
    fn flash_frame(&mut self, c: u8) {
        self.term.line(34, 26, 284, 26, c);
        self.term.line(284, 26, 284, 124, c);
        self.term.line(284, 124, 34, 124, c);
        self.term.line(34, 124, 34, 26, c);
    }

    /// Attack `10c2` / `1072`.
    fn cmd_attack(&mut self, io: &mut dyn Io, dir: Option<Dir>) -> IoResult<()> {
        self.say(m::ATTACK_WITH);
        let wpn = self.rec.u16_at(off::READY_WEAPON) as usize;
        let name = self.ptr(m::WEAPON_SHORT, wpn);
        self.term.print(&name);
        let range = self.a.exe.w(m::WEAPON_RANGE + 2 * wpn) as i32;
        if range == 0 {
            self.err(io, m::QUESTION);
            return Ok(());
        }
        if self.mode == crate::dgame::DUNGEON {
            self.dungeon_attack(io, range, wpn);
            return Ok(());
        }
        self.say(m::COLON);
        let d = match dir {
            Some(d) => d,
            None => match self.key_timeout(io)? {
                keys::LEFT => Dir::West,
                keys::RIGHT => Dir::East,
                keys::UP => Dir::North,
                keys::DOWN => Dir::South,
                _ => {
                    self.sayln(m::NOTHING);
                    return Ok(());
                }
            },
        };
        let dn = self.a.t.direction_names[d as usize].clone();
        self.term.println(&dn);
        let hit = i32::from(self.stat(AGI)) + 50;
        let hi = i32::from(self.stat(STR)) + 8 * wpn as i32;
        let dmg = self.r(2, hi);
        if self.mode == OUTDOORS {
            self.outdoor_attack(io, d, 7, range, dmg, hit, SHOT_TILE)
        } else {
            self.town_attack(io, d, range, dmg, hit);
            Ok(())
        }
    }

    /// Outdoor fight (`cf3`): the shot tile flies cell by cell (sound
    /// `snd` on the first, two ticks between the others) until it meets a
    /// monster or a mountain or runs out of range.
    pub(crate) fn outdoor_attack(
        &mut self,
        io: &mut dyn Io,
        d: Dir,
        snd: u8,
        range: i32,
        dmg: i32,
        hit: i32,
        shot: u8,
    ) -> IoResult<()> {
        let (dx, dy) = d.delta();
        let mut k = 1;
        let (mut col, mut row, mut cell, mut t);
        loop {
            col = rules::PLAYER_COL + dx * k;
            row = rules::PLAYER_ROW + dy * k;
            self.draw_world_tile(col, row, shot);
            if k == 1 {
                self.snd(io, snd);
                io.pause(&self.term, 1)?;
            } else {
                io.pause(&self.term, 2)?;
            }
            cell = rules::wrap(self.ow.x + dx * k, self.ow.y + dy * k);
            t = self.cell_or_object(cell.0, cell.1);
            self.draw_world_cell(col, row);
            k += 1;
            if k > range || t >= 0x13 || t == 3 {
                break;
            }
        }
        if t < 0x13 {
            self.sayln(m::MISS);
            return Ok(());
        }
        let name = self.a.t.monster_names[rules::monster_kind(t)].clone();
        if self.r(1, 100) > hit {
            self.say(m::MISSED_THE);
            self.term.print(&name);
            self.sayln(m::EXCL);
            return Ok(());
        }
        self.draw_world_tile(col, row, shot);
        let i = self.ow.entity_at(cell.0, cell.1).expect("monster");
        let hp = (self.ow.entities[i].hp as i16).wrapping_sub(dmg as i16);
        self.ow.entities[i].hp = hp as u16;
        self.say(if hp <= 0 { m::KILLED_THE } else { m::HIT_THE });
        self.term.print(&name);
        let long = name.len() > 6;
        if hp > 0 {
            if long {
                self.sayln(m::EXCL);
            } else {
                self.say(m::EXCL_SP);
            }
            self.term.num(dmg as u16, 2, ' ', TEXT, true);
            self.sayln(m::SP_DAMAGE);
            self.snd(io, 2);
            self.draw_world_cell(col, row);
            return Ok(());
        }
        self.ow.entities.remove(i);
        let base = i32::from(self.a.t.monster_damage[rules::monster_kind(t)]);
        let gold = self.r(1, base * 4) + 10;
        let xp = self.r(2, base * 5);
        self.add(off::EXPERIENCE, xp as i16);
        self.snd(io, 2);
        self.draw_world_cell(col, row);
        self.loot(gold as i16, name.len());
        self.show_exp();
        Ok(())
    }

    /// `761`: coins found on a slain monster.
    pub(crate) fn loot(&mut self, n: i16, len: usize) {
        let mut n = n;
        let coin = self.coin();
        if coin.wrapping_add(n) > 9999 {
            n = 9999 - coin;
        }
        let (len, plain) = if len == 0 { (8, true) } else { (len, false) };
        if n == 0 {
            return;
        }
        let copper = n % 10;
        let silver = (n % 100 - copper) / 10;
        let gold = n / 1000;
        let mut open = if plain {
            true
        } else if len > 6 {
            self.sayln(m::EXCL);
            false
        } else {
            self.say(m::EXCL_SP);
            true
        };
        for (v, w, o) in [(copper, 1, m::COPPER_EX), (silver, 2, m::SILVER_EX), (gold, 2, m::GOLD_EX)] {
            if v != 0 {
                self.term.num(v as u16, w, ' ', TEXT, true);
                self.say(o);
                if open {
                    self.nl();
                    open = false;
                } else {
                    open = true;
                }
            }
        }
        if open {
            self.nl();
        }
        self.add(off::COIN, n);
        self.show_coin();
    }

    /// Fire: the frigate's cannons outdoors; elsewhere "Fire?".
    fn cmd_fire(&mut self, io: &mut dyn Io) -> IoResult<()> {
        if self.mode != OUTDOORS || self.transport() != 4 {
            self.err(io, m::FIRE_Q);
            return Ok(());
        }
        self.say(m::FIRE);
        self.term.print(" ");
        self.say(m::CANNONS);
        self.say(m::COLON);
        let d = match self.key_timeout(io)? {
            keys::LEFT => Dir::West,
            keys::RIGHT => Dir::East,
            keys::UP => Dir::North,
            keys::DOWN => Dir::South,
            _ => {
                self.sayln(m::NOTHING);
                return Ok(());
            }
        };
        let dn = self.a.t.direction_names[d as usize].clone();
        self.term.println(&dn);
        let dmg = self.r(1, i32::from(self.transport() as i16) * 10) + 30;
        self.outdoor_attack(io, d, 8, 3, dmg, 80, SHOT_TILE)
    }

    fn cmd_get(&mut self, io: &mut dyn Io) {
        self.say(m::GET);
        match self.mode {
            CASTLE => self.castle_get(io),
            TOWN => {
                self.say(m::WHAT);
                self.err(io, m::QUESTION);
            }
            _ => self.err(io, m::QUESTION),
        }
    }

    /// Inform and search `564b`.
    fn cmd_inform(&mut self) {
        self.sayln(m::INFORM);
        let p = self.a.locs.id_at(self.ow.x as u8, self.ow.y as u8);
        if p != 0 {
            if p < 0x21 {
                self.say(m::CITY_OF_LC);
            }
            let n = self.ptr(m::LOCATION_NAMES, p);
            self.term.println(&n);
            return;
        }
        match self.a.map.cell(self.ow.x as usize, self.ow.y as usize) {
            0 => self.sayln(m::AT_SEA),
            2 => self.sayln(m::IN_WOODS),
            _ => {
                self.sayln(m::IN_LANDS);
                let land = rules::land_of(self.ow.x, self.ow.y) as usize;
                let n = self.ptr(m::REGION_NAMES, land);
                self.term.println(&n);
            }
        }
    }

    fn cmd_noise(&mut self) {
        self.say(m::NOISE);
        let on = self.rec.u16_at(off::SOUND) == 0;
        self.rec.set_u16(off::SOUND, u16::from(on));
        self.sayln(if on { m::ON } else { m::OFF });
    }

    /// Quit `5b54`: save (outdoors only) and carry on playing.
    fn cmd_quit(&mut self, io: &mut dyn Io) {
        self.say(m::QUIT);
        if self.mode != OUTDOORS {
            self.err(io, m::ONLY_OUTDOORS);
            return;
        }
        self.say(m::SAVING);
        self.sync_record();
        if let Some(p) = self.save_to.clone() {
            if let Some(d) = p.parent() {
                let _ = std::fs::create_dir_all(d);
            }
            if std::fs::write(&p, self.rec.as_bytes()).is_err() {
                self.term.println("");
                return;
            }
        }
        self.sayln(m::SAVED);
    }

    // ------------------------------------------------------------ ready / ztats

    /// Ready `bc20`.
    pub(crate) fn cmd_ready(&mut self, io: &mut dyn Io) -> IoResult<()> {
        self.say(m::READY_MENU);
        let k = self.key_timeout(io)?;
        self.term.fill(56, 192, 230, 199, BLACK);
        self.term.cursor(7, 24);
        let (label, base, names, last, field) = match k {
            b'W' => (m::READY_WEAPON, slot::WEAPON, m::WEAPON_LONG, 15, off::READY_WEAPON),
            b'A' => (m::READY_ARMOUR, slot::ARMOUR, m::ARMOUR_NAMES, 5, off::READY_ARMOUR),
            b'S' => (m::READY_SPELL, slot::SPELL, m::SPELL_NAMES, 10, off::READY_SPELL),
            _ => {
                self.sayln(m::READY_NOTHING);
                return Ok(());
            }
        };
        self.say(label);
        let cur = self.rec.u16_at(field) as usize;
        let r = self.ready_menu(io, base, names, last, cur)?;
        self.rec.set_u16(field, r as u16);
        Ok(())
    }

    /// `ba63`: the list in the viewport; returns the chosen item or the
    /// current one.
    fn ready_menu(&mut self, io: &mut dyn Io, base: usize, names: usize, last: usize, cur: usize) -> IoResult<usize> {
        self.term.clear_view();
        let t = self.s(m::READY_TITLE);
        self.term.ornament_left(16, 0);
        self.term.print(&t);
        self.term.ornament_right(24, 0);
        let have: Vec<usize> = (0..=last).filter(|&i| self.qty(base, i) != 0).collect();
        let row0 = 10 - have.len() / 2;
        for (n, &i) in have.iter().enumerate() {
            let name = self.ptr(names, i);
            self.term.print_at(15, row0 + n, &format!("{}) {name}", (b'a' + i as u8) as char));
        }
        self.term.cursor(15, 24);
        let k = self.key(io)?.to_ascii_lowercase();
        let i = k.wrapping_sub(b'a') as usize;
        let res = if i <= last && self.qty(base, i) != 0 { i } else { cur };
        let name = self.ptr(names, res);
        self.term.println(&name);
        self.term.restore_border(127, 199);
        self.redraw();
        Ok(res)
    }

    /// Ztats `bfe6`.
    pub(crate) fn cmd_ztats(&mut self, io: &mut dyn Io) -> IoResult<()> {
        self.sayln(m::ZTATS);
        self.term.clear_view();
        self.term.ornament_left(13, 0);
        self.say(m::INVENTORY);
        self.term.ornament_right(25, 0);
        let name = self.rec.name();
        let pl = self.s(m::PLAYER);
        self.term.print_at_attr(2, 2, &format!("{pl}{name}"), WHITE);
        let lv = self.s(m::A_LEVEL);
        self.term.print_at_attr(2, 3, &lv, WHITE);
        let level = (self.exp() as u16) / 1000 + 1;
        self.term.num(level, 1, ' ', WHITE, true);
        let sex = if self.rec.sex() == 0 { m::MALE } else { m::FEMALE };
        let sx = self.s(sex);
        let race = self.ptr(m::RACE_NAMES, self.rec.race() as usize);
        let class = self.ptr(m::CLASS_NAMES, self.rec.class() as usize);
        self.term.print_attr(&format!("{sx}{race} {class}"), WHITE);
        let mut slot_n = 0usize;
        let mut entries: Vec<(String, i16, bool)> = Vec::new();
        for i in 0..=6 {
            entries.push((self.ptr(m::STAT_NAMES, i), self.stat(i), false));
        }
        let coin = self.coin();
        if coin % 10 != 0 {
            entries.push((self.s(m::COPPER), coin % 10, false));
        }
        if coin % 100 > 9 {
            entries.push((self.s(m::SILVER), (coin % 100) / 10, false));
        }
        if coin > 99 {
            entries.push((self.s(m::GOLD), coin / 100, false));
        }
        let en = self.w(off::ENEMIES);
        if en != 0 {
            entries.push((self.s(m::ENEMY_VESSELS), en, false));
        }
        let lists = [
            (slot::ARMOUR, m::ARMOUR_NAMES, 1, 5, self.rec.u16_at(off::READY_ARMOUR) as usize),
            (slot::WEAPON, m::WEAPON_LONG, 1, 15, self.rec.u16_at(off::READY_WEAPON) as usize),
            (slot::SPELL, m::SPELL_NAMES, 1, 10, self.rec.u16_at(off::READY_SPELL) as usize),
            (slot::VEHICLE, m::TRANSPORT_NAMES, 1, 7, self.transport() as usize),
            (slot::GEM, m::GEM_NAMES, 0, 3, usize::MAX),
        ];
        for (base, names, first, last, cur) in lists {
            for i in first..=last {
                let q = self.qty(base, i);
                if q != 0 {
                    entries.push((self.ptr(names, i), q, i == cur));
                }
            }
        }
        for (name, v, current) in entries {
            if v == 0 {
                continue;
            }
            if slot_n > 25 {
                self.term.ornament_left(16, 19);
                self.say(m::MORE);
                self.term.ornament_right(23, 19);
                self.press_space(io)?;
                self.term.fill(8, 39, 311, 151, BLACK);
                slot_n = 0;
            }
            let col = 2 + 19 * usize::from(slot_n > 12);
            let row = 5 + slot_n % 13;
            let attr = if current { WARN } else { TEXT };
            let dots = ".".repeat(13usize.saturating_sub(name.len()));
            self.term.print_at_attr(col, row, &format!("{name}{dots}"), attr);
            self.term.num(v as u16, 4, '.', attr, false);
            slot_n += 1;
        }
        self.press_space(io)?;
        self.term.restore_border(100, 220);
        Ok(())
    }
}

