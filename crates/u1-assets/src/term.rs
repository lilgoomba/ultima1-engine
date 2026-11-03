//! The 320x200 game screen as palette indices, with the text-mode helpers
//! the original program uses on top of its graphics (SPEC §9.2): BIOS-style
//! character cells (40x25), the four-line scrolling message window, right
//! justified numbers, the title ornaments and the shop/king box.
//! All positions and colours are "verified from original executable".

use crate::font::glyph;

pub const W: usize = 320;
pub const H: usize = 200;
pub const BLACK: u8 = 0;
pub const BLUE: u8 = 1;
pub const WARN: u8 = 12;
pub const TEXT: u8 = 11;
pub const WHITE: u8 = 15;
/// Flag on a colour: a device value used as is (no CGA mapping).
pub const RAW: u8 = 0x10;

/// Which original picture a drawn region shows, for the enhanced video
/// mode (`crate::enhanced`); the faithful renderer ignores it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArtId {
    /// Overworld tile (`???TILES.BIN`, SPEC §3.2).
    World(u16),
    /// Town/castle tile (`???TOWN.BIN`, SPEC §3.3).
    Town(u16),
    /// Mondain's lair tile (`???MOND.BIN`, SPEC §3.4).
    Mond(u16),
    /// A full-screen picture (`"nif"`).
    Screen(&'static str),
    /// A part of the first-person dungeon view ([`DUN_PARTS`]).
    Dun(u16),
    /// A dungeon monster seen in the first-person view (monster id).
    DMon(u16),
    /// A part of the space view ([`SPACE_PARTS`]).
    Space(u16),
}

/// Names of the [`ArtId::Space`] parts, by number (manifest `[space.NAME]`).
pub const SPACE_PARTS: [&str; 12] = ["frame", "cockpit", "starfield", "deep", "earth", "sun", "station", "ship0", "ship1", "ship2", "fighter", "reticle"];

/// [`ArtId::Space`] numbers.
pub mod space_part {
    /// The dotted frame around the top-down view.
    pub const FRAME: u16 = 0;
    /// The same frame in the front view.
    pub const COCKPIT: u16 = 1;
    /// What the black of the top-down view shows.
    pub const STARFIELD: u16 = 2;
    /// What the black of the front view shows.
    pub const DEEP: u16 = 3;
    pub const EARTH: u16 = 4;
    pub const SUN: u16 = 5;
    pub const STATION: u16 = 6;
    /// Ship types 0-2 (shuttle, ...), pictured nose up.
    pub const SHIP0: u16 = 7;
    pub const FIGHTER: u16 = 10;
    /// The front view's crosshair.
    pub const RETICLE: u16 = 11;
}

/// One picture drawn into the space view: which part, turned how, and
/// the pixels it lit (screen offsets).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SObj {
    pub part: u16,
    /// Quarter turns clockwise from nose up.
    pub rot: u8,
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
    pub lit: Vec<u32>,
}

impl SObj {
    /// Most of its pixels are still lit (stars and shots may cross it;
    /// an erased picture is gone).
    pub fn shown(&self, t: &Term) -> bool {
        let on = self.lit.iter().filter(|&&i| t.px[i as usize] != 0).count();
        !self.lit.is_empty() && on * 4 >= self.lit.len() * 3
    }
}

/// The space view as pictures, for the enhanced renderer (used while its
/// frame is intact).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SScene {
    /// Front view (else top-down).
    pub fp: bool,
    /// The top bar of the frame as drawn.
    pub frame: ArtOp,
    pub objs: Vec<SObj>,
}

/// Names of the [`ArtId::Dun`] parts, by number (manifest `[dungeon.NAME]`).
pub const DUN_PARTS: [&str; 12] = ["wall", "door", "secret", "floor", "ceiling", "ladder_up", "ladder_down", "hole_up", "hole_down", "chest", "coffin", "field"];

/// [`ArtId::Dun`] numbers.
pub mod dun {
    pub const WALL: u16 = 0;
    pub const DOOR: u16 = 1;
    pub const SECRET: u16 = 2;
    pub const FLOOR: u16 = 3;
    pub const CEILING: u16 = 4;
    pub const LADDER_UP: u16 = 5;
    pub const LADDER_DOWN: u16 = 6;
    pub const HOLE_UP: u16 = 7;
    pub const HOLE_DOWN: u16 = 8;
    pub const CHEST: u16 = 9;
    pub const COFFIN: u16 = 10;
    pub const FIELD: u16 = 11;
}

/// A region of the screen known to show one original picture: kept while
/// the pixels there are still the ones drawn (checked when composing).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtOp {
    pub id: ArtId,
    /// What the original shows behind a sprite there (outdoors: the
    /// terrain under the player or a monster).
    pub under: Option<ArtId>,
    /// Horizontal scroll of the tile (the water).
    pub shift: u16,
    /// Map cell the tile shows (outdoors: world map x,y; in towns: the
    /// town map cell), which picks among a picture's `variants`.
    pub pos: Option<(u16, u16)>,
    /// Draw the enhanced picture mirrored left to right (a vehicle facing
    /// the other way, SPEC §25); the original pixels are never mirrored.
    pub flip: bool,
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
    /// The pixels as drawn.
    pub px: Vec<u8>,
    /// Parts filled over since (a window's box over the edge of the map,
    /// inclusive 320x200 rectangles): there the original shows, the rest
    /// of the tile keeps its art.
    pub cut: Vec<(usize, usize, usize, usize)>,
}

impl ArtOp {
    /// Still on screen: nothing has been drawn over it since.
    pub fn intact(&self, t: &Term) -> bool {
        (0..self.h).all(|r| {
            let o = (self.y + r) * W + self.x;
            t.px[o..o + self.w] == self.px[r * self.w..(r + 1) * self.w]
        })
    }
}

/// Viewport rectangle (inclusive), cleared to black by the original.
pub const VIEW: (usize, usize, usize, usize) = (8, 8, 311, 151);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Term {
    pub px: Vec<u8>,
    pub col: usize,
    pub row: usize,
    /// The graphics pen of the line routines (`lineto` starts here).
    pub pen: (i32, i32),
    /// The EGA palette swapped end for end (MONDAIN.EXE's flash).
    pub inverted: bool,
    /// CGA mode 4: pixels hold 2-bit values. Colours passed to the drawing
    /// routines are the EGA ones; [`Term::dc`] turns them into what the
    /// original's CGA path draws.
    pub cga: bool,
    /// Cell holding the blinking cursor's last visible frame, if any.
    pub blink_at: Option<(usize, usize)>,
    /// Palette registers reprogrammed (the start-up castle picture):
    /// pixel value -> standard colour index.
    pub remap: Option<[u8; 16]>,
    /// Regions showing known pictures (newest last), for the enhanced
    /// renderer.
    pub art: Vec<ArtOp>,
    /// The last first-person dungeon view as surfaces and objects, for the
    /// enhanced renderer (shown while its `Screen("dungeon")` note is intact).
    pub dscene: Option<Box<crate::dview::DScene>>,
    /// The space view as pictures, for the enhanced renderer.
    pub sscene: Option<Box<SScene>>,
    /// A standing backdrop for the enhanced renderer: the pack's
    /// `behind` picture `id` shows wherever the rectangle (x, y, w, h) is
    /// black, whatever is drawn there, until it is cleared (the character
    /// program's screens, which redraw their insides between key waits).
    pub backdrop: Option<(ArtId, usize, usize, usize, usize)>,
    /// The message log (an optional engine feature, SPEC §22.3): when
    /// set, each line that scrolls off the top of the message window is
    /// kept here (newest last). Nothing on the screen changes.
    pub log: Option<Box<std::collections::VecDeque<String>>>,
}

impl Default for Term {
    fn default() -> Self {
        Self::new()
    }
}

impl Term {
    /// The static frame: blue border, white viewport frame, black viewport,
    /// the message box (left) and the stats box (right) with their labels.
    pub fn new() -> Self {
        Self::with_cga(false)
    }

    /// The same frame for a CGA (`cga`) or a 16-colour screen.
    pub fn with_cga(cga: bool) -> Self {
        let mut t =
            Term { px: vec![0; W * H], col: 1, row: 24, pen: (0, 0), inverted: false, cga, blink_at: None, remap: None, art: Vec::new(), dscene: None, sscene: None, backdrop: None, log: None };
        t.fill(0, 0, W - 1, H - 1, BLUE);
        t.fill(7, 7, 312, 152, WHITE);
        t.fill(VIEW.0, VIEW.1, VIEW.2, VIEW.3, BLACK);
        t.corners(0, 319);
        t.fill(0, 159, 241, 199, WHITE);
        t.fill(0, 160, 240, 199, BLACK);
        t.fill(246, 159, 319, 199, WHITE);
        t.fill(247, 160, 319, 199, BLACK);
        t.restore_labels();
        t
    }

    /// The four stat labels (cursor left at (1,24)).
    pub fn restore_labels(&mut self) {
        for (i, l) in ["Hits:", "Food:", "Exp.:", "Coin:"].iter().enumerate() {
            self.print_at_attr(31, 21 + i, l, TEXT);
        }
        self.col = 1;
        self.row = 24;
    }

    /// Device colour for an EGA colour number. The original keeps its four
    /// interface colours in variables, set at start-up to 15/1/12/11 on a
    /// 16-colour screen and 3/2/1/3 on CGA (`b2ae`); any other colour is
    /// written with only its low two bits on CGA (the plot routine `aa3c`
    /// masks with 3).
    pub fn dc(&self, c: u8) -> u8 {
        if c & RAW != 0 {
            return c & 15;
        }
        if !self.cga {
            return c;
        }
        match c {
            BLUE => 2,
            WARN => 1,
            _ => c & 3,
        }
    }

    pub fn at(&self, x: usize, y: usize) -> u8 {
        self.px[y * W + x]
    }

    /// Filled rectangle, both corners inclusive (clipped to the screen).
    /// Picture notes it overlaps are dropped: a black box over black
    /// tiles must not bring their pictures back.
    pub fn fill(&mut self, x0: usize, y0: usize, x1: usize, y1: usize, c: u8) {
        let c = self.dc(c);
        self.art.retain_mut(|o| {
            if o.x > x1 || x0 >= o.x + o.w || o.y > y1 || y0 >= o.y + o.h {
                return true;
            }
            let (cx0, cy0, cx1, cy1) = (x0.max(o.x), y0.max(o.y), x1.min(o.x + o.w - 1), y1.min(o.y + o.h - 1));
            if (cx0, cy0, cx1, cy1) == (o.x, o.y, o.x + o.w - 1, o.y + o.h - 1) {
                return false;
            }
            for y in cy0..=cy1 {
                for x in cx0..=cx1 {
                    o.px[(y - o.y) * o.w + x - o.x] = c;
                }
            }
            o.cut.push((cx0, cy0, cx1, cy1));
            true
        });
        for y in y0..=y1.min(H - 1) {
            for x in x0..=x1.min(W - 1) {
                self.px[y * W + x] = c;
            }
        }
    }

    /// Straight line between two points (both inclusive).
    pub fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, c: u8) {
        let c = self.dc(c);
        let (dx, dy) = ((x1 - x0).abs(), -(y1 - y0).abs());
        let (sx, sy) = (if x0 < x1 { 1 } else { -1 }, if y0 < y1 { 1 } else { -1 });
        let (mut x, mut y, mut err) = (x0, y0, dx + dy);
        loop {
            if (0..W as i32).contains(&x) && (0..H as i32).contains(&y) {
                self.px[y as usize * W + x as usize] = c;
            }
            if x == x1 && y == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    }

    /// One pixel, set (not mixed), clipped to the screen.
    pub fn plot(&mut self, x: i32, y: i32, c: u8) {
        let c = self.dc(c);
        if (0..W as i32).contains(&x) && (0..H as i32).contains(&y) {
            self.px[y as usize * W + x as usize] = c;
        }
    }

    /// The original's line routine (SPEC §6.6): a Bresenham walk along
    /// the longer axis (x wins ties) with error 2m-n, plotting n+1 pixels.
    /// The pen ends at (x1,y1). Used by the dungeon view, where the exact
    /// pixels matter.
    pub fn line_u1(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, c: u8) {
        self.pen = (x1, y1);
        let (mut sx, mut sy) = (1, 1);
        let mut dy = y1 - y0;
        if dy < 0 {
            sy = -1;
            dy = -dy;
        }
        let mut dx = x1 - x0;
        if dx < 0 {
            sx = -1;
            dx = -dx;
        }
        // (major, minor) deltas and the straight step along the major axis.
        let (n, m, ax, ay) = if dx >= dy { (dx, dy, sx, 0) } else { (dy, dx, 0, sy) };
        let (mut x, mut y) = (x0, y0);
        let mut err = 2 * m - n;
        for _ in 0..=n {
            self.plot(x, y, c);
            if err < 0 {
                x += ax;
                y += ay;
                err += 2 * m;
            } else {
                x += sx;
                y += sy;
                err += 2 * m - 2 * n;
            }
        }
    }

    /// `lineto`: a line from the pen to (x,y).
    pub fn lineto(&mut self, x: i32, y: i32, c: u8) {
        let (x0, y0) = self.pen;
        self.line_u1(x0, y0, x, y, c);
    }

    /// Move the pen and plot one pixel there.
    pub fn moveto_plot(&mut self, x: i32, y: i32, c: u8) {
        self.pen = (x, y);
        self.plot(x, y, c);
    }

    pub fn cursor(&mut self, col: usize, row: usize) {
        self.col = col;
        self.row = row;
    }

    /// One character cell: glyph in `attr` on black.
    pub fn put_char(&mut self, col: usize, row: usize, ch: char, attr: u8) {
        if col >= 40 || row >= 25 {
            return;
        }
        let (x, y) = (col * 8, row * 8);
        let attr = self.dc(attr);
        let g = glyph(ch);
        for (r, bits) in g.iter().enumerate() {
            for b in 0..8 {
                let on = bits & (0x80 >> b) != 0;
                self.px[(y + r) * W + x + b] = if on { attr } else { BLACK };
            }
        }
    }

    /// Print at the cursor, advancing it (BIOS teletype: wraps at column 40).
    pub fn print_attr(&mut self, s: &str, attr: u8) {
        for ch in s.chars() {
            if self.col >= 40 {
                self.col = 0;
                self.row = (self.row + 1).min(24);
            }
            self.put_char(self.col, self.row, ch, attr);
            self.col += 1;
        }
    }

    /// `a043`: print in the text colour at the cursor.
    pub fn print(&mut self, s: &str) {
        self.print_attr(s, TEXT);
    }

    /// `21e7`: move the cursor to (col,row) and print in the text colour.
    pub fn print_at(&mut self, col: usize, row: usize, s: &str) {
        self.print_at_attr(col, row, s, TEXT);
    }

    pub fn print_at_attr(&mut self, col: usize, row: usize, s: &str, attr: u8) {
        self.cursor(col, row);
        self.print_attr(s, attr);
    }

    /// `ac0a`: print, then scroll the message window (rows 21..=24, columns
    /// 0..=29) up one line and put the cursor at (1,24).
    pub fn println(&mut self, s: &str) {
        self.print(s);
        self.scroll();
    }

    pub fn scroll(&mut self) {
        if self.log.is_some() {
            let gone = self.msg_text(21);
            if let Some(l) = self.log.as_mut() {
                if !gone.trim().is_empty() {
                    if l.len() >= crate::msglog::KEEP {
                        l.pop_front();
                    }
                    l.push_back(crate::msglog::tidy(&gone));
                }
            }
        }
        let (x0, x1) = (0, 30 * 8);
        for y in 21 * 8..24 * 8 {
            for x in x0..x1 {
                self.px[y * W + x] = self.px[(y + 8) * W + x];
            }
        }
        for y in 24 * 8..25 * 8 {
            self.px[y * W + x0..y * W + x1].fill(BLACK);
        }
        self.cursor(1, 24);
    }

    /// `a966`: unsigned number right-justified in `width` with `pad`.
    /// With `advance == false` the cursor stays on the last digit.
    pub fn num(&mut self, v: u16, width: usize, pad: char, attr: u8, advance: bool) {
        let s = v.to_string();
        let mut out = String::new();
        for _ in s.len()..width {
            out.push(pad);
        }
        out.push_str(&s);
        self.print_attr(&out, attr);
        if !advance {
            self.col -= 1;
        }
    }

    /// `a3ea`: the left title ornament at a character cell.
    pub fn ornament_left(&mut self, col: usize, row: usize) {
        self.ornament(col, row, false);
    }

    /// `a48c`: the right (mirrored) title ornament.
    pub fn ornament_right(&mut self, col: usize, row: usize) {
        self.ornament(col, row, true);
    }

    fn ornament(&mut self, col: usize, row: usize, mirror: bool) {
        let (x, y) = ((col * 8) as i32, (row * 8) as i32);
        // The font's arrow glyph (CP437 0x10 / 0x11) in the border colour ...
        self.put_char(col, row, if mirror { '\u{11}' } else { '\u{10}' }, BLUE);
        // ... outlined in white, with a blue upright on the flat side.
        if mirror {
            self.line(x + 7, y, x, y + 3, WHITE);
            self.line(x, y + 3, x + 7, y + 7, WHITE);
            self.line(x + 7, y + 1, x + 7, y + 6, BLUE);
        } else {
            self.line(x, y, x + 7, y + 3, WHITE);
            self.line(x + 7, y + 3, x, y + 7, WHITE);
            self.line(x, y + 1, x, y + 6, BLUE);
        }
        self.cursor(col + 1, row);
    }

    /// A title in the top border between two ornaments, e.g. " Ready ".
    pub fn title(&mut self, left: usize, text: &str) {
        self.ornament_left(left, 0);
        self.print(text);
        let right = self.col;
        self.ornament_right(right, 0);
    }

    /// Restore the top border after a title (fill blue, white line at y=7).
    pub fn restore_border(&mut self, x0: usize, x1: usize) {
        self.fill(x0, 0, x1, 6, BLUE);
        self.fill(x0, 7, x1, 7, WHITE);
        self.corners(x0, x1);
    }

    /// The rounded top corners of the screen (black, 6/4/3/2/1/1 pixels
    /// wide on rows 0..=5) and the four missing corner pixels of the
    /// viewport frame, both as the original's frame shows them; only the
    /// parts inside columns `x0..=x1` are drawn.
    fn corners(&mut self, x0: usize, x1: usize) {
        let inside = |x: usize| x >= x0 && x <= x1;
        for (y, n) in [6usize, 4, 3, 2, 1, 1].into_iter().enumerate() {
            for i in 0..n {
                for x in [i, W - 1 - i] {
                    if inside(x) {
                        self.px[y * W + x] = self.dc(BLACK);
                    }
                }
            }
        }
        for (x, y) in [(7, 7), (312, 7), (7, 152), (312, 152)] {
            if inside(x) {
                self.px[y * W + x] = self.dc(BLUE);
            }
        }
    }

    /// Clear the viewport to black.
    pub fn clear_view(&mut self) {
        self.fill(VIEW.0, VIEW.1, VIEW.2, VIEW.3, BLACK);
    }

    /// Switch between the CGA and a 16-colour screen while playing (the
    /// options menu): the pixels already drawn are converted to the other
    /// screen's colours (the four interface colours swap places; on the way
    /// back the text windows' colour 3 is the text colour, elsewhere white).
    /// The caller redraws the view, whose tiles change with the display.
    pub fn set_cga(&mut self, cga: bool) {
        if cga == self.cga {
            return;
        }
        for y in 0..H {
            for x in 0..W {
                let c = self.px[y * W + x];
                self.px[y * W + x] = if cga {
                    match c {
                        WHITE | TEXT => 3,
                        BLUE => 2,
                        WARN => 1,
                        _ => c & 3,
                    }
                } else {
                    // (Column 0 holds the white-outlined prompt ornaments.)
                    let text_window = y >= 160 && ((8..=240).contains(&x) || x >= 247);
                    match c & 3 {
                        3 if text_window => TEXT,
                        3 => WHITE,
                        2 => BLUE,
                        1 => WARN,
                        _ => BLACK,
                    }
                };
            }
        }
        self.cga = cga;
    }

    /// `21c2`: the black box used by shops and the king, framed in
    /// `frame`; cursor (5,4).
    pub fn boxed(&mut self, frame: u8) {
        self.fill(31, 23, 287, 127, BLACK);
        self.line(34, 26, 284, 26, frame);
        self.line(284, 26, 284, 124, frame);
        self.line(284, 124, 34, 124, frame);
        self.line(34, 124, 34, 26, frame);
        self.cursor(5, 4);
    }

    /// One stat value at column 36: clamped to 0..=9999, warning colour
    /// below 100, cursor left at (1,24).
    /// A status number; hits and food (`warn`) turn red below 100.
    pub fn stat(&mut self, row: usize, v: i32, warn: bool) {
        let v = v.clamp(0, 9999) as u16;
        let attr = if warn && v < 100 { WARN } else { TEXT };
        self.cursor(36, row);
        self.num(v, 4, ' ', attr, false);
        self.cursor(1, 24);
    }

    /// Blit an indexed `w`x`h` image at (x,y), colours masked to 4 bits.
    /// `c655`: one frame of the blinking cursor at the text cursor; `rows`
    /// as stored in OUT.EXE (bit 0 is the leftmost pixel), unset pixels black.
    pub fn cursor_frame(&mut self, rows: [u8; 8], colour: u8) {
        if self.col >= 40 || self.row >= 25 {
            return;
        }
        let (x, y, c) = (self.col * 8, self.row * 8, self.dc(colour));
        self.blink_at = if rows == [0; 8] { None } else { Some((self.col, self.row)) };
        for (r, bits) in rows.iter().enumerate() {
            for b in 0..8 {
                self.px[(y + r) * W + x + b] = if bits >> b & 1 != 0 { c } else { BLACK };
            }
        }
    }

    /// Tile pixels are device values already (2-bit on CGA).
    pub fn blit(&mut self, x: usize, y: usize, w: usize, h: usize, pixels: &[u8], shift: usize) {
        for py in 0..h {
            for px in 0..w {
                if x + px < W && y + py < H {
                    self.px[(y + py) * W + x + px] = pixels[py * w + (px + shift) % w] & 0x0F;
                }
            }
        }
    }

    /// [`Term::blit`] that also notes which tile was drawn there.
    #[allow(clippy::too_many_arguments)]
    pub fn blit_art(&mut self, id: ArtId, under: Option<ArtId>, x: usize, y: usize, w: usize, h: usize, pixels: &[u8], shift: usize) {
        self.blit(x, y, w, h, pixels, shift);
        self.mark(id, under, shift, x, y, w, h);
    }

    /// Note that the region now shows picture `id`; older notes it
    /// overlaps are dropped.
    #[allow(clippy::too_many_arguments)]
    pub fn mark(&mut self, id: ArtId, under: Option<ArtId>, shift: usize, x: usize, y: usize, w: usize, h: usize) {
        let (w, h) = (w.min(W.saturating_sub(x)), h.min(H.saturating_sub(y)));
        if w == 0 || h == 0 {
            return;
        }
        self.art.retain(|o| o.x >= x + w || x >= o.x + o.w || o.y >= y + h || y >= o.y + o.h);
        let mut px = Vec::with_capacity(w * h);
        for r in y..y + h {
            px.extend_from_slice(&self.px[r * W + x..r * W + x + w]);
        }
        self.art.push(ArtOp { id, under, shift: shift as u16, pos: None, flip: false, x, y, w, h, px, cut: Vec::new() });
    }

    /// Note the map cell shown by the newest region (see [`ArtOp::pos`]).
    pub fn art_pos(&mut self, pos: (u16, u16)) {
        if let Some(o) = self.art.last_mut() {
            o.pos = Some(pos);
        }
    }

    /// Mirror the newest region's enhanced picture (see [`ArtOp::flip`]).
    pub fn art_flip(&mut self, flip: bool) {
        if let Some(o) = self.art.last_mut() {
            o.flip = flip;
        }
    }

    /// Start a new space scene once its frame is drawn (`fp`: front view).
    pub fn space_begin(&mut self, fp: bool) {
        let (x, y, w, h) = (8, 8, 304, 8);
        let mut px = Vec::with_capacity(w * h);
        for r in y..y + h {
            px.extend_from_slice(&self.px[r * W + x..r * W + x + w]);
        }
        let frame = ArtOp { id: ArtId::Space(space_part::FRAME), under: None, shift: 0, pos: None, flip: false, x, y, w, h, px, cut: Vec::new() };
        self.sscene = Some(Box::new(SScene { fp, frame, objs: Vec::new() }));
    }

    /// Note a space picture drawn over the pixels `lit`. With `toggle`
    /// (XOR drawing) the same picture drawn again is its erasure; `only`
    /// keeps one picture of this part.
    pub fn space_obj(&mut self, part: u16, rot: u8, lit: Vec<u32>, toggle: bool, only: bool) {
        let Some(sc) = self.sscene.as_mut() else { return };
        if lit.is_empty() {
            return;
        }
        let (mut x0, mut y0, mut x1, mut y1) = (W, H, 0, 0);
        for &i in &lit {
            let (x, y) = (i as usize % W, i as usize / W);
            (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x), y1.max(y));
        }
        let o = SObj { part, rot, x: x0, y: y0, w: x1 - x0 + 1, h: y1 - y0 + 1, lit };
        if toggle {
            if let Some(i) = sc.objs.iter().position(|p| p.part == part && p.rot == rot && p.lit == o.lit) {
                sc.objs.remove(i);
                return;
            }
        }
        if only {
            sc.objs.retain(|p| p.part != part);
        }
        sc.objs.push(o);
    }

    /// Forget space pictures of `part` (erased).
    pub fn space_drop(&mut self, part: u16) {
        if let Some(sc) = self.sscene.as_mut() {
            sc.objs.retain(|p| p.part != part);
        }
    }

    /// Text currently shown in a character row (glyphs matched back to
    /// characters; for tests and debugging).
    pub fn row_text(&self, row: usize) -> String {
        (0..40)
            .map(|col| {
                let mut cell = [0u8; 8];
                for (r, c) in cell.iter_mut().enumerate() {
                    for b in 0..8 {
                        let p = self.px[(row * 8 + r) * W + col * 8 + b];
                        if p != BLACK && p != self.dc(BLUE) {
                            *c |= 0x80 >> b;
                        }
                    }
                }
                if let Some(ch) = (0x20u8..0x7F).map(char::from).find(|&ch| glyph(ch) == cell) {
                    ch
                } else if self.blink_at == Some((col, row)) {
                    ' '
                } else if cell == Self::ornament_cell() {
                    '>'
                } else {
                    '#'
                }
            })
            .collect::<String>()
    }

    /// The non-blue pixels of the left ornament (read back as '>').
    fn ornament_cell() -> [u8; 8] {
        let mut t = Term::new();
        t.ornament_left(0, 24);
        let mut cell = [0u8; 8];
        for (r, c) in cell.iter_mut().enumerate() {
            for b in 0..8 {
                let p = t.px[(192 + r) * W + b];
                if p != BLACK && p != BLUE {
                    *c |= 0x80 >> b;
                }
            }
        }
        cell
    }

    /// Text of a message-window row (columns 0..30), trailing blanks cut.
    pub fn msg_text(&self, row: usize) -> String {
        self.row_text(row)[..30].trim_end().to_string()
    }

    /// Message window as lines (rows 21..=24).
    pub fn messages(&self) -> Vec<String> {
        (21..25).map(|r| self.msg_text(r)).collect()
    }

    pub fn to_rgb(&self, palette: &[[u8; 3]; 16], scale: usize) -> Vec<u8> {
        let mut out = vec![0u8; W * H * 3 * scale * scale];
        for y in 0..H * scale {
            for x in 0..W * scale {
                let mut i = (self.px[(y / scale) * W + x / scale] & 15) as usize;
                if self.inverted && self.cga {
                    let c = crate::palette::CGA_FLASH[i & 3];
                    let o = (y * W * scale + x) * 3;
                    out[o..o + 3].copy_from_slice(&c);
                    continue;
                }
                if self.inverted {
                    i = 15 - i;
                }
                if let Some(r) = &self.remap {
                    i = r[i] as usize;
                }
                let c = palette[i];
                let o = (y * W * scale + x) * 3;
                out[o..o + 3].copy_from_slice(&c);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn println_scrolls_the_message_window_only() {
        let mut t = Term::new();
        t.cursor(1, 24);
        t.println("North");
        assert_eq!(t.msg_text(23), " North");
        assert_eq!((t.col, t.row), (1, 24));
        assert!(t.row_text(21).starts_with("      "));
        // stats labels untouched
        assert!(t.row_text(21).contains("Hits:"));
    }

    #[test]
    fn numbers_are_right_justified() {
        let mut t = Term::new();
        t.cursor(0, 10);
        t.num(42, 4, ' ', TEXT, true);
        assert_eq!(&t.row_text(10)[..4], "  42");
        assert_eq!(t.col, 4);
        t.cursor(0, 11);
        t.num(7, 3, '.', TEXT, false);
        assert_eq!(&t.row_text(11)[..3], "..7");
        assert_eq!(t.col, 2);
    }

    #[test]
    fn a_box_drops_the_black_tiles_under_it() {
        let mut t = Term::new();
        t.blit_art(ArtId::Mond(0), None, 40, 40, 16, 16, &[0; 256], 0);
        t.blit_art(ArtId::Mond(0), None, 8, 136, 16, 16, &[0; 256], 0);
        t.boxed(BLUE);
        assert_eq!(t.art.len(), 1);
        assert_eq!((t.art[0].x, t.art[0].y), (8, 136));
    }

    #[test]
    fn cga_switch_keeps_the_frame() {
        let ega = Term::with_cga(false);
        let mut t = ega.clone();
        t.set_cga(true);
        assert_eq!(t.px, Term::with_cga(true).px);
        t.set_cga(false);
        assert_eq!(t.px, ega.px);
        assert!(!t.cga);
    }

    #[test]
    fn box_and_stats() {
        let mut t = Term::new();
        t.boxed(BLUE);
        assert_eq!(t.at(34, 26), BLUE);
        assert_eq!(t.at(40, 40), BLACK);
        t.stat(21, 50, true);
        assert_eq!(&t.row_text(21)[31..40], "Hits:  50");
        assert!((168..176).any(|y| (36 * 8..40 * 8).any(|x| t.at(x, y) == WARN)));
    }
}
