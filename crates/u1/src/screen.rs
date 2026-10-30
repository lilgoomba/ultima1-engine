//! Composes the 320x200 outdoor screen as palette indices (SPEC §8.9):
//! a 19x9-tile viewport at (8,8) framed in white on a blue border, the
//! message area bottom-left and the stats panel bottom-right.

use u1_assets::map::{water_scroll, Terrain, WorldMap};
use u1_assets::overworld::Overworld;
use u1_assets::rules::{self, VIEW_COLS, VIEW_ROWS};
use u1_assets::tiles::TileSet;

use crate::font::glyph;

pub const W: usize = 320;
pub const H: usize = 200;
const BLUE: u8 = 1;
const CYAN: u8 = 11;
const WHITE: u8 = 15;
const BLACK: u8 = 0;
pub const VIEW_X: usize = 8;
pub const VIEW_Y: usize = 8;
/// Text rows of the bottom area (y of each 8-pixel line).
const TEXT_Y: [usize; 4] = [168, 176, 184, 192];

pub struct Screen {
    pub px: Vec<u8>,
}

impl Screen {
    pub fn new() -> Self {
        Screen { px: vec![BLUE; W * H] }
    }

    fn fill(&mut self, x0: usize, y0: usize, x1: usize, y1: usize, c: u8) {
        for y in y0..y1.min(H) {
            self.px[y * W + x0..y * W + x1.min(W)].fill(c);
        }
    }

    pub fn text(&mut self, x: usize, y: usize, s: &str, c: u8) {
        for (i, ch) in s.chars().enumerate() {
            let gx = x + 8 * i;
            if gx + 8 > W {
                break;
            }
            for (r, bits) in glyph(ch).iter().enumerate() {
                for b in 0..8 {
                    if bits & (0x80 >> b) != 0 && y + r < H {
                        self.px[(y + r) * W + gx + b] = c;
                    }
                }
            }
        }
    }

    fn tile(&mut self, tiles: &TileSet, t: usize, x: usize, y: usize, shift: usize) {
        let Some(tile) = tiles.tiles.get(t) else { return };
        let (tw, th) = (tiles.width, tiles.height);
        for py in 0..th {
            for px in 0..tw {
                self.px[(y + py) * W + x + px] = tile[py * tw + (px + shift) % tw] & 0x0F;
            }
        }
    }

    /// Whole frame. `counter` is the animation counter (one step per
    /// keyboard poll in the original); `cursor_on` blinks the prompt.
    pub fn compose(&mut self, o: &Overworld, map: &WorldMap, tiles: &TileSet, counter: u32, cursor_on: bool) {
        self.px.fill(BLUE);
        // Viewport frame and contents.
        let (vw, vh) = (VIEW_COLS as usize * 16, VIEW_ROWS as usize * 16);
        self.fill(VIEW_X - 1, VIEW_Y - 1, VIEW_X + vw + 1, VIEW_Y + vh + 1, WHITE);
        let shift = water_scroll(counter as usize, tiles.width);
        for row in 0..VIEW_ROWS {
            for col in 0..VIEW_COLS {
                let (mx, my) = rules::view_cell(o.x, o.y, col, row);
                let (sx, sy) = (VIEW_X + col as usize * 16, VIEW_Y + row as usize * 16);
                let terr = map.cell(mx as usize, my as usize);
                let player = col == rules::PLAYER_COL && row == rules::PLAYER_ROW;
                let ent = o.entity_at(mx, my).map(|i| o.entities[i]);
                let t = if player {
                    Some(o.player_tile())
                } else {
                    ent.map(|e| e.tile)
                };
                match t {
                    Some(t) => {
                        let frame = animate_object(t, counter, (mx * 31 + my * 17) as u32);
                        self.tile(tiles, frame as usize, sx, sy, 0);
                    }
                    None => {
                        let frame = Terrain::from_cell(terr).tile_for_frame(counter as usize);
                        let s = if terr == 0 { shift } else { 0 };
                        self.tile(tiles, frame, sx, sy, s);
                    }
                }
            }
        }
        // Bottom boxes: messages (left) and stats (right).
        self.fill(0, 159, 242, H, WHITE);
        self.fill(0, 160, 241, H, BLACK);
        self.fill(246, 159, W, H, WHITE);
        self.fill(247, 160, W, H, BLACK);
        let shown: Vec<&String> = o.messages.iter().rev().take(3).collect();
        for (i, m) in shown.iter().rev().enumerate() {
            let line = TEXT_Y[3 - shown.len() + i];
            self.text(0, line, &m.chars().take(30).collect::<String>(), CYAN);
        }
        self.text(0, TEXT_Y[3], ">", CYAN);
        if cursor_on && !o.dead {
            self.fill(8, TEXT_Y[3] + 6, 15, TEXT_Y[3] + 8, WHITE);
        }
        let stats = [("Hits:", o.hits), ("Food:", o.food), ("Exp.:", o.exp), ("Coin:", o.coin)];
        for (i, (label, v)) in stats.iter().enumerate() {
            self.text(248, TEXT_Y[i], &format!("{label}{v:>4}"), CYAN);
        }
    }

    pub fn to_rgb(&self, palette: &[[u8; 3]; 16], scale: usize) -> Vec<u8> {
        let mut out = vec![0u8; W * H * 3 * scale * scale];
        for y in 0..H * scale {
            for x in 0..W * scale {
                let c = palette[(self.px[(y / scale) * W + x / scale] & 15) as usize];
                let o = (y * W * scale + x) * 3;
                out[o..o + 3].copy_from_slice(&c);
            }
        }
        out
    }
}

impl Default for Screen {
    fn default() -> Self {
        Self::new()
    }
}

/// Animation frame of an object tile: the frigate and one sea monster
/// alternate with the counter; other monsters pick a frame "at random"
/// (here a cheap hash of the counter and position, so screenshots are
/// reproducible).
pub fn animate_object(tile: u8, counter: u32, salt: u32) -> u8 {
    match tile {
        0x0E | 0x19 => tile + (counter & 1) as u8,
        t if rules::is_monster_tile(t) => {
            let h = counter.wrapping_mul(2_654_435_761).wrapping_add(salt.wrapping_mul(40_503));
            t + ((h >> 16) & 1) as u8
        }
        t => t,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use u1_assets::overworld::Entity;

    fn tiles() -> TileSet {
        // Tile n is filled with colour n % 16.
        TileSet { width: 16, height: 16, tiles: (0..52).map(|n| vec![(n % 16) as u8; 256]).collect() }
    }

    #[test]
    fn layout_and_player_cell() {
        let map = WorldMap { width: 168, height: 156, cells: vec![1; 168 * 156] };
        let mut o = Overworld::new_at(50, 50, 1);
        o.entities.push(Entity { tile: 0x0B, x: 51, y: 50, hp: 0 });
        o.say(">North");
        let mut s = Screen::new();
        s.compose(&o, &map, &tiles(), 0, true);
        let at = |x: usize, y: usize| s.px[y * W + x];
        assert_eq!(at(0, 0), BLUE);
        assert_eq!(at(7, 7), WHITE);
        assert_eq!(at(VIEW_X, VIEW_Y), 1); // grass = tile 1
        assert_eq!(at(VIEW_X + 9 * 16 + 1, VIEW_Y + 4 * 16 + 1), 0x0A); // player
        assert_eq!(at(VIEW_X + 10 * 16 + 1, VIEW_Y + 4 * 16 + 1), 0x0B); // horse
        assert_eq!(at(242, 180), BLUE);
        assert!(s.px[168 * W..176 * W].iter().take(240).all(|&c| c == BLACK));
        // stats text present in the right panel
        assert!((168..176).any(|y| (248..320).any(|x| at(x, y) == CYAN)));
    }

    #[test]
    fn object_animation() {
        assert_eq!(animate_object(0x0E, 1, 0), 0x0F);
        assert_eq!(animate_object(0x0E, 2, 0), 0x0E);
        assert_eq!(animate_object(0x0B, 1, 0), 0x0B);
        let f = animate_object(0x21, 5, 9);
        assert!(f == 0x21 || f == 0x22);
    }
}
