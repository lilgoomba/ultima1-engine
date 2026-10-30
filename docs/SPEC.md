# Ultima I (DOS) — data file & format specification

Status: **draft 0.3 (2026-10-29)**. Version 0.1 was compiled from public
community documentation only. Version 0.2 checks it against a legally owned
original copy (the DOS release from the *Ultima 1-3* CD-ROM) and adds facts
learned by studying that copy. Version 0.3 adds the outdoor game rules
(§9.1), edge wrap (§4), timing (§3.5) and the remaining save fields (§7).

**How this spec is produced (spec-from-reference).** The original files are
used *only as a reference* on the developer's machine, outside this
repository. Findings are restated here in plain language: data layouts,
table contents described as facts, and algorithms in prose or pseudocode.
The engine is then written fresh in Rust from this text. This repository
contains no original game files, no disassembly or decompiled code, and no
screenshots of original art. Where the engine needs a table that lives in an
original executable, it reads it at run time from the user's own copy.

Legend used throughout:

* **VERIFIED (original data)**: checked against the original data files,
  either by decoding them and comparing with the game running in DOSBox
  0.74-3 pixel by pixel, or by diffing save files before and after known
  in-game actions. The method is named next to each marker.
* **VERIFIED (original executable)**: read from a reference study
  (disassembly) of the original `OUT.EXE` and restated in our own words.
  Where the *meaning* of a value is inferred from how the code uses it, the
  text says "inferred".
* **CONFIRMED (docs)**: stated by at least one public source and consistent
  with the documented file sizes. Not yet checked against real files.
* **CONFLICT**: public sources disagree; both claims are recorded.
* **UNKNOWN**: not resolved yet.

Verification methods referenced below:

* **[M-DOSBox]**: the game is run in DOSBox (machine=ega or machine=cga,
  cycles=20000) on a *copy* of the files. Its window is captured, and the
  2x-scaled 19x9 viewport is compared cell by cell with the engine's
  render of the same map window.
* **[M-save]**: the three shipped save slots and `INUSE.U1` are diffed,
  values are compared with the in-game Ztats screens, and saves are diffed
  before and after the Q(uit and save) command.
* **[M-exe]**: a reference study of `OUT.EXE` (69,858 bytes) with a 16-bit
  x86 disassembler. The data segment of `OUT.EXE` starts at file offset
  0xD680, so "DS:0x18C8" means file offset 0xEF48.

All multi-byte integers are little-endian (x86) unless stated otherwise.

---

## 1. Target release

* Origin Systems' MS-DOS port of the 1986 *Ultima I* remake (PCGamingWiki
  dates the DOS release to 1987; DOS port credited to John Fachini) [S9].
* Sold by GOG.com in the *Ultima 1+2+3* bundle, running under a bundled
  DOSBox [S9][S10].
* Video modes supported by the shipped data: **CGA** (4-colour),
  **EGA** (16-colour) and **Tandy 1000** ("T1K", 16-colour). Each has its own
  set of tile files [S1][S3]. **There is no VGA mode or VGA data** in the
  original release; "VGA" tile sets that exist are later fan creations
  (e.g. the Ultima I Unity project [S11]) and are out of scope. EGA is
  therefore the highest-fidelity original target.
* UNKNOWN / needs verification: whether the GOG install exposes the data
  files directly in its `Ultima1` folder or packs them inside a `game.gog`
  disc image (GOG does this for some other DOSBox titles [S10]). If the
  latter, the user must extract it (e.g. `7z x game.gog`) and point the
  engine at the extracted folder. GOG also redirects writes (saves) into a
  `cloud_saves` folder [S9].

## 2. File inventory

Sizes are from Daniel D'Agostino's file list [S1]; descriptions merge [S1],
[S3], [S7], [S8].

| File | Size (bytes) | Contents | Status |
|---|---:|---|---|
| `ULTIMA.EXE` | 36,880 | Intro program | code; not read by engine |
| `GEN.EXE` | 20,112 | Main menu & character generation | code; not read |
| `OUT.EXE` | 69,858 | Sosaria: overworld, towns, dungeons | code; engine reads two small tables from it at run time (§4.1, §6) |
| `SPACE.EXE` | 39,824 | Space flight / combat | strings, tables read at run time (§12) |
| `MONDAIN.EXE` | 31,650 | Final battle with Mondain | strings, arena, tables read at run time (§13) |
| `MAP.BIN` | 13,104 | Overworld map (§4) | CONFIRMED (docs), height CONFLICT |
| `TCD.BIN` | 6,840 | 10 town/castle maps (§5) | CONFIRMED (docs) |
| `EGATILES.BIN` | 6,656 | Overworld tiles, EGA (§3) | CONFIRMED (docs) |
| `CGATILES.BIN` | 3,328 | Overworld tiles, CGA | CONFIRMED (docs) |
| `T1KTILES.BIN` | 6,656 | Overworld tiles, Tandy | CONFIRMED (docs) |
| `EGATOWN.BIN` | 1,632 | Town tiles 8x8, EGA | CONFIRMED (docs) |
| `CGATOWN.BIN` | 816 | Town tiles 8x8, CGA | CONFIRMED (docs) |
| `T1KTOWN.BIN` | 1,632 | Town tiles 8x8, Tandy | CONFIRMED (docs) |
| `EGAMOND.BIN` | 2,432 | Mondain-battle tiles, EGA | CONFIRMED (docs) |
| `CGAMOND.BIN` | 1,216 | Mondain-battle tiles, CGA | CONFIRMED (docs) |
| `T1KMOND.BIN` | 2,432 | Mondain-battle tiles, Tandy | CONFIRMED (docs) |
| `EGAFIGHT.BIN` | 3,192 | Space-combat enemy fighters, EGA | partly (encoding "?" in source) |
| `CGAFIGHT.BIN` | 3,192 | Space-combat fighters, CGA | partly |
| `T1KFIGHT.BIN` | 3,192 | Space-combat fighters, Tandy | partly |
| `EGASPACE.BIN` | 7,296 | Spaceships (docking), EGA | partly |
| `CGASPACE.BIN` | 7,296 | Spaceships, CGA | partly |
| `T1KSPACE.BIN` | 7,296 | Spaceships, Tandy | partly |
| `CASTLE.16` | 32,000 | Intro castle picture, 320x200, 16-colour | encoding UNKNOWN |
| `CASTLE.4` | 16,384 | Intro castle picture, 320x200, CGA interleaved | partly (§8) |
| `NIF.BIN` | 6,720 | End-game text as 320x168 1-bpp bitmap | partly (§8) |
| `PLAYER1.U1` … `PLAYER4.U1` | 820 | Save slots (§7) | VERIFIED (original data) [M-save] |
| `INUSE.U1` | 820 | Same layout; hand-off between the EXEs | layout VERIFIED [M-save] |
| `MOSLO.COM` | 2,378 | Slow-down TSR shipped on the CD-ROM | code; not read |
| `ULTIMA1.TXT` | 45,721 | Text manual / command reference (CD-ROM) | not read by engine |

**VERIFIED (original data):** every size above matches the CD-ROM copy
exactly. That copy has no `PLAYER4.U1` (slots are created on demand). The
names are upper case.

**Not present as data files** (hard-coded in the executables, therefore
UNKNOWN in byte form and to be re-implemented from documented *behaviour*):
monster stats, weapon/armour/spell/vehicle tables, shop prices, signpost
and NPC text, the location → town/castle/dungeon assignment, and the dungeon
generator (§6, §9). No compression is documented for any file; every format
below is raw and headerless [S3].

Still UNKNOWN: whether the GOG build differs from the CD-ROM copy. The
engine matches names case-insensitively.

## 3. Tile graphics (`???TILES`, `???TOWN`, `???MOND`, `???FIGHT`, `???SPACE`)

Source: ModdingWiki *Ultima I Tile Graphic Format* (TheAlmightyGuru) [S3],
Ultima Stones EGATILES/EGATOWN pages [S5][S6], D'Agostino [S1].

No header, no compression, no tile count stored: tile size, count and
encoding are fixed by the executable [S3].

| File | Encoding | Tile size | Tiles in file | Bytes/tile | Check |
|---|---|---|---:|---:|---|
| EGATILES | EGA row-planar | 16x16 | 52 | 128 | 52x128 = 6,656 ✓ |
| CGATILES | CGA linear | 16x16 | 52 | 64 | 3,328 ✓ |
| T1KTILES | Tandy linear | 16x16 | 52 | 128 | 6,656 ✓ |
| EGATOWN | EGA row-planar | 8x8 | 51 | 32 | 1,632 ✓ |
| CGATOWN | CGA linear | 8x8 | 51 | 16 | 816 ✓ |
| T1KTOWN | Tandy linear | 8x8 | 51 | 32 | 1,632 ✓ |
| EGAMOND | EGA row-planar | 16x16 | 19 | 128 | 2,432 ✓ |
| CGAMOND | CGA linear | 16x16 | 19 | 64 | 1,216 ✓ |
| T1KMOND | Tandy linear | 16x16 | 19 | 128 | 2,432 ✓ |
| EGAFIGHT | mono linear (?) | 24x19 | 56 (14 x 4 shifts) | 57 | 3,192 ✓ |
| CGAFIGHT | CGA linear (?) | 24x19 | 28 (14 x 2) | 114 | 3,192 ✓ |
| T1KFIGHT | Tandy linear | 24x19 | 14 | 228 | 3,192 ✓ |
| EGASPACE | mono linear (?) | 32x19 | 96 (24 x 4) | 76 | 7,296 ✓ |
| CGASPACE | CGA linear (?) | 32x19 | 48 (24 x 2) | 152 | 7,296 ✓ |
| T1KSPACE | Tandy linear | 32x19 | 24 | 304 | 7,296 ✓ |

CONFLICT (minor): [S3]'s prose says EGAFIGHT/EGASPACE replicate each tile
"eight times" while its table lists 56/96 tiles (= 4 copies of 14/24); and
[S1] lists EGAFIGHT as "14 tiles" and EGASPACE "24 tiles", i.e. counting
logical sprites. The pre-shifted copies (pixel data nudged right by one
pixel per copy) appear to be for fast sub-byte horizontal placement.

**VERIFIED (original data, by decoding):** the copies of each sprite are
stored *consecutively*. In EGAFIGHT and EGASPACE (1 bpp), sprite *n*
occupies tiles 4n..4n+3, and copy *s* equals copy 0 shifted right by *s*
pixels. In CGAFIGHT and CGASPACE (2 bpp), sprite *n* occupies tiles
2n..2n+1, and copy 1 is copy 0 shifted right by one pixel (2 bits). This
holds for all 14 fighters and 24 ships. The mono files use only values
{0, 1}. Decoded as Tandy 4 bpp, T1KFIGHT and T1KSPACE use only colours
{0, 15}.

### 3.1 Encodings

**EGA row-planar** — for each pixel row, `W/8` bytes of the **blue** plane,
then `W/8` bytes green, `W/8` red, `W/8` intensity. 16-wide tiles are thus
`BB GG RR II` (8 bytes/row), 8-wide tiles `B G R I` (4 bytes/row). Within
each byte the most significant bit is the leftmost pixel (the reference
viewer described in [S3] walks bits 7→0). Pixel value
`= I<<3 | R<<2 | G<<1 | B`, which is directly an index into the default EGA
16-colour palette [S12]. [S5] describes the same data as four 16-bit words
per row, first word = least-significant colour bit (= blue).
**VERIFIED (original data) [M-DOSBox]:** the engine's EGA decode of
EGATILES matches the game's EGA screen pixel for pixel, including plane
order and left-byte-first order.

**CGA linear** — 4 pixels/byte, bits 7-6 = leftmost pixel; rows
left-to-right, top-to-bottom [S3]. **VERIFIED (original data)
[M-DOSBox, machine=cga]:** the game uses CGA palette 1, low intensity:
0 black, 1 cyan (00AAAA), 2 magenta (AA00AA), 3 light grey (AAAAAA). The
engine's CGA render of the overworld matches DOSBox for every terrain cell.

**Tandy (T1K) linear** — 2 pixels/byte, high nibble = left pixel [S3].
**VERIFIED (original data, by decoding):** T1KTILES, T1KTOWN and T1KMOND
decode to exactly the same pixel values as the EGA sets. The Tandy palette
is the standard IRGB one: overworld, town, dungeon, castle, space and
Mondain screens rendered by the engine match DOSBox `machine=tandy` apart
from the blinking cursor's phase (Phase 7).

**Monochrome linear** — 8 pixels/byte, MSB = leftmost, set bits drawn white
[S3]. Colour used in-game: UNKNOWN.

[S1] describes the EGA data as "BGRA channels, alpha = intensity multiplier"
— the same thing expressed differently.

### 3.2 Overworld tile indices (`???TILES.BIN`, 52 tiles) [S3]

| Index | Tile | Index | Tile |
|---|---|---|---|
| 0x00 | Water | 0x13–0x14 | Ness monster (2 frames) |
| 0x01 | Grass | 0x15–0x16 | Giant squid |
| 0x02 | Forest | 0x17–0x18 | Dragon turtle |
| 0x03 | Mountain | 0x19–0x1A | Pirate ship |
| 0x04–0x05 | Castle (2 frames) | 0x1B–0x1C | Hood |
| 0x06 | Signpost | 0x1D–0x1E | Bear |
| 0x07–0x08 | Town (2 frames) | 0x1F–0x20 | Hidden archer |
| 0x09 | Dungeon entrance | 0x21–0x22 | Dark knight |
| 0x0A | Player | 0x23–0x24 | Evil trent |
| 0x0B | Horse | 0x25–0x26 | Thief |
| 0x0C | Cart | 0x27–0x28 | Orc |
| 0x0D | Raft | 0x29–0x2A | Knight |
| 0x0E–0x0F | Frigate | 0x2B–0x2C | Necromancer |
| 0x10 | Aircar | 0x2D–0x2E | Evil ranger |
| 0x11 | Shuttle | 0x2F–0x30 | Wandering warlock |
| 0x12 | Time machine | 0x31 | blue disc (see below) |
| | | 0x32 | red disc |
| | | 0x33 | dog head |

**VERIFIED (original data, decoded sheets in EGA/CGA/T1K):** [S3] calls
0x31 the "unused dog head", but in the shipped files the dog head is
**0x33**. 0x31 is a small blue disc and 0x32 a red disc. **VERIFIED (original
executable):** the red disc 0x32 is drawn as the flying missile of the
player's ranged weapons and vehicle guns. The blue disc 0x31 is the spell
effect: it flashes over monsters removed by a spell and is the magic
missile.
The monster names in this table also appear, in this order, in `OUT.EXE`'s
string data [M-exe].

### 3.3 Town tile indices (`???TOWN.BIN`, 51 tiles) [S3]

0x00 solid wall · 0x01 floor · 0x02 water · 0x03–0x06 water corners
(TL, BL, TR, BR) · 0x07–0x08 water diagonals (BR, BL) · 0x09 small tree ·
0x0A big tree · 0x0B–0x10 counters (L-R, T-B, top, bottom, left, right) ·
0x11 guard · 0x12 player · 0x13 jester 1 · 0x14 king · 0x15 merchant ·
0x16 prisoner · 0x17 brick wall · 0x18–0x31 letters A–Z · 0x32 jester 2.

### 3.4 Other tile sets [S3]

* MOND: 0x00 time machine · 0x01–0x02 Mondain · 0x03 Mondain slain ·
  0x04 wall · 0x05–0x09 Gem of Immortality · 0x0A–0x0C bat ·
  0x0D–0x12 explosion.
* FIGHT: pairs 0x00–0x0B = enemy fighter at zoom levels x1…x6.
* SPACE: 0x00–0x07 shuttle · 0x08–0x0F ship (left view) ·
  0x10–0x17 ship (bottom view). Earth and Sun are not listed in this
  set; how they are drawn is UNKNOWN.

### 3.5 Animation timing

**VERIFIED (original data) [M-DOSBox]**, by sampling map cells in the
captured screen over time (cycles=20000):

* The game advances a global animation tick about every **0.165 s**
  (about 3 BIOS timer ticks).
* **VERIFIED (original executable + DOSBox), Phase 3:** the tick is the
  keyboard poll. Each poll waits for 3 ticks of the 18.2 Hz timer
  interrupt. The first tick arrives at a random phase, so a wait lasts
  2–3 ticks; drawing time is added. The rate is therefore timer-driven,
  with a small CPU-dependent part. After 1,500 polls with no key the game
  plays "Pass" by itself. Measured in DOSBox: 195.7 s to the automatic
  Pass at cycles=20000 (130 ms per poll), 236.0 s at cycles=3000 (157 ms
  per poll). The engine uses 130 ms per poll.
* **Water** (tile 0) is not frame-animated. Each tick, the whole tile is
  redrawn scrolled horizontally by 4 pixels: at tick *k*, displayed column
  *x* shows source column `(x + 4·(k mod 4)) mod 16`. The pattern therefore
  drifts left and repeats every 4 ticks. All water cells share the same
  phase.
* **Castle** alternates tile 4 and tile 5, holding each for 3 ticks
  (period about 1 s).
* **Town** shows tile 7 for one tick out of every four (the tick where the
  water phase is 0) and tile 8 for the other three.
* Monster tiles: the frigate (0x0E/0x0F) and tile 0x19 alternate frames
  with the poll counter. Every other monster tile (0x13..=0x2F) draws its
  base tile plus `R(1,100) mod 2`, taken from the game's own random
  generator, so each visible monster costs one roll per poll (VERIFIED
  (original executable)).

**Who animates, and when (VERIFIED (original executable)).** The prompt
loop polls the keyboard, adds one to a per-prompt counter *p* (1 on the
first poll), and outdoors calls the view animation with *p* after every
poll, including the poll that returns the key. The animation scrolls the
water one step (the water phase is global and never reset), then walks the
view column by column, top to bottom: water is redrawn; a castle shows
tile 4 when `p mod 6 = 0`, tile 5 when `p mod 3 = 0`, and is left as it is
otherwise; a town shows 7 when `p mod 4 = 0`, else 8; objects as above.
A full view redraw (after a move or a turn) draws every cell in its base
tile, with no random rolls. Because the number of polls per prompt
depends on how long the player waits, these frames and the random
numbers that follow them are, in the original, effectively real-time
dependent. The engine does exactly the same per poll (`Game::world_tick`).

## 4. Overworld map — `MAP.BIN`

Sources: [S1] (quoting Johnny Wood's notes), [S4] (ModdingWiki), [S7]
(Ultima Stones).

* Headerless, uncompressed. **4 bits per cell, 2 cells per byte, high
  nibble first.** Rows stored left-to-right, top-to-bottom; 168 cells =
  **84 bytes per row**. CONFIRMED (docs).
  * [S1]'s quoted prose says "the low 4 bits represent the first tile", but
    its own example and [S4]'s viewer mean **high nibble first**.
    **VERIFIED (original data) [M-DOSBox]:** high nibble = even (left)
    cell. Engine renders of the windows around (14,14) in EGA and (43,38)
    in CGA match the game for every terrain cell.
* **Dimensions VERIFIED (original data): 168 x 156.** The 13,104-byte file
  is exactly 84 x 156, the four continents are complete in a full-map
  render, and all 84 location coordinates (§4.1) fall on matching terrain.
  [S4]'s 168 x 168 is wrong for this release.
* Cell values ([S4]):

  | Value | Terrain | Tile index |
  |---:|---|---:|
  | 0 | Water | 0x00 |
  | 1 | Grass | 0x01 |
  | 2 | Forest | 0x02 |
  | 3 | Mountain | 0x03 |
  | 4 | Castle | 0x04 (+0x05) |
  | 5 | Signpost | 0x06 |
  | 6 | Town | 0x07 (+0x08) |
  | 7 | Dungeon entrance | 0x09 |
  | 8–15 | UNKNOWN (not expected) | — |

* The map does **not** store *which* castle/town/dungeon/signpost is at a
  location [S4]; see §4.1.
* The world contains the four lands (Lord British, Feudal Lords, Dark
  Unknown, Danger & Despair) [S14]. **Land index VERIFIED (original
  executable):** `(y > 77)·2 + (x > 83)`.
* **Edge wrap VERIFIED (original executable + DOSBox):**
  * The drawn view wraps like a plain torus: x mod 168, y mod 156.
  * Player movement north/south wraps plainly (y 0 ↔ 155).
  * Crossing the east or west edge (x 167 ↔ 0) also shifts the player
    77 rows: y − 77 if y ≥ 77, otherwise y + 77. The east/west edge
    therefore joins the north half of the world to the south half.
  * DOSBox check: a frigate at (3,89) sailed West 8 times ends at
    (163,12); the engine shows the identical view
    (`u1-shots/phase3/*_wrap_west8.png`).
* On-screen viewport in the original: 19 x 9 cells of 16x16 px
  (304 x 144 px) [S4]. **VERIFIED [M-DOSBox]:** the player is drawn in
  cell (9,4) of the viewport.

### 4.1 Location table: VERIFIED (original executable) [M-exe]

* There are **84 numbered locations**, with ids 1..84 in this order:
  * 1-32: towns
  * 33-40: castles
  * 41-48: landmarks (signposts, pillars, tower, grave)
  * 49-84: dungeons

  The display names are stored in `OUT.EXE` in the same order. Each of
  the four lands has 8 towns, 2 castles, 2 landmarks and 9 dungeons.
* The coordinates are kept in two separate arrays of 84 bytes each: X
  values at DS:0x18C8 (file 0xEF48) and Y values at DS:0x191C (file
  0xEF9C). This is why searching for interleaved (x,y) pairs found
  nothing.
* The names are found through a table of 16-bit DS-relative pointers. For
  id *n*, the pointer is at DS:0x307C + 2*n (file 0x106FE for id 1).
* **Lookup rule:** to find what is at (x, y), the game scans ids 1..84 in
  order and takes the first id whose X and Y bytes both match. Id 0 means
  "nothing here".
* **Check (original data):** the terrain nibble in `MAP.BIN` under every
  entry matches its kind: castle 4, landmark 5, town 6, dungeon 7. All 84
  match, and the map has exactly 8/8/32/36 cells of those values.
* **Which town/castle map is used:** a town with id *n* uses `TCD.BIN` map
  `((n-1) mod 8) + 2`, so each town map serves one town in each land. A
  castle with id *n* uses map `(n-1) mod 2`. The first castle of each land
  pair (Lord British, Barataria, …) uses map 0. The castle index *n*-33
  (0..7) selects the castle's quest.
* **Entry position:** on entering a town, the player starts at local
  (19, 17), bottom centre. In a castle the player starts at (0, 9), the
  left edge. On entering a castle, the game also rewrites two cells to
  town tile 11 (counter): (31, y) and (35, y), with y = 14 in map 0 and
  y = 4 in map 1. This is probably the prison doors (inferred).
* The engine does **not** embed this table. `u1_assets::locations` reads
  it from the user's `OUT.EXE` (and refuses any file that is not 69,858
  bytes). `u1view --game-dir … --locations` prints the table and checks it
  against `MAP.BIN`.

## 5. Towns and castles — `TCD.BIN`

Source: Ultima Stones [S6b]; size from [S1].

* 10 maps x 38 (W) x 18 (H) cells, 1 byte per cell = 684 bytes/map,
  6,840 bytes total ✓.
* **Column-major VERIFIED (original data):** cell (x, y) is at byte
  `map*684 + x*18 + y`. Decoding the maps this way gives coherent towns,
  with walls, counters, ponds and lettered signs readable left to right.
  Row-major decoding gives noise.
* **Map order VERIFIED:** maps 0 and 1 are castles (outer border value 0,
  wall). Their lettered labels include PRISON, MESS HALL and STABLES.
  Maps 2-9 are towns. §4.1 gives which location uses which map.
* **Cell values VERIFIED (original data):** 0..50 index `???TOWN.BIN`
  (§3.3). Values **51..63** do not exist as tiles. They are floor cells
  that also carry a **region code**, and draw as floor (tile 1):

  | Value | Where it appears |
  |---:|---|
  | 51 | floor around water and ponds |
  | 52, 53 | around the ponds in castles |
  | 54, 55 | armour shop (the two sides of the counter) |
  | 56, 57 | food shop |
  | 58, 59 | weapon shop |
  | 60 | magic shop; also the castle prison area |
  | 61 | pub; also the castle prison |
  | 62 | transport shop |
  | 63 | gaps and doors in walls |

  The shop pairing fits `ULTIMA1.TXT`: "Steal: may be used behind the
  unwatched counters of shoppes". How the code uses each code (which side
  allows stealing) is UNKNOWN.
* NPCs are not stored in `TCD.BIN`; the code places them (UNKNOWN details).

## 6. Dungeons — procedurally generated, no data file

Public sources ([S13]-[S16]) describe the behaviour. The algorithm below is
**VERIFIED (original executable) [M-exe]** and restated in our own words.

### 6.1 Random number generator: VERIFIED (original executable)

* The state is one 16-bit word. "Seed" just stores a value into it.
* One draw adds 0x9248 (wrapping), rotates the 16-bit result right by 3
  bits, stores that as the new state, and returns its low 10 bits *r*
  (0..1023).
* A ranged draw `R(lo, hi)` clamps *hi* to 255 and sets span = *hi* - *lo*.
  While *r* > span it subtracts span from *r*. It returns `(lo + r) & 0xFF`.
  The result is in lo..hi inclusive, with a slight bias.
* A separate "time seed" is built from the DOS time of day (INT 21h
  AH=2Ch). Each part is shifted within its own byte: hours·2, minutes·4
  and seconds·8, each mod 256. The game then forms the 16-bit sum
  `(seconds·8)·256 + hundredths + (hours·2)·256 + minutes·4`, XORs it
  with 0x91EB, and keeps the low 12 bits (0..4095).
* When the overworld starts, the game seeds with
  `save[0x32] + (time seed mod 999)`. Outdoor events are therefore not
  reproducible; dungeon structure is.

### 6.2 Level generation: VERIFIED (original executable)

* The level grid is **11 x 11 cells** indexed `[a][b]`, with 0..10 on both
  axes. *a* is the player's X axis and *b* the Y axis. The outer ring is
  wall, so the interior is the documented **9 x 9**. Each cell holds a
  structure code, an item code and two monster fields.
* **Structure codes:**
  * 0 empty
  * 1 wall
  * 2 secret door. The Search command turns it into 3 ("Thou dost find a
    secret door!").
  * 3 door
  * 6 ladder down (the Ladder Down spell writes 6)
  * 7 ladder up
  * 8 force field. The spells write and clear 8 ("Field created!" /
    "Field destroyed!").
* **Item codes:** 4 chest, 5 coffin.
* **Structure seed:** `5·X + 3·Y + 17·level + save[0x32]`, with 16-bit
  wrapping. X and Y are the dungeon entrance on the overworld and level is
  1..10. Save word 0x32 is therefore a per-character dungeon seed, which
  confirms [S8b].
* **Steps:**
  1. Clear every cell, then set the outer ring (a or b equal to 0 or 10)
     to wall.
  2. For each *a* in {2, 4, 6, 8}, and each *b* in 1..9 in increasing
     order, set the structure to `R(1,4) - 1`: empty, wall, secret door or
     door. These are the four wall lines with doors that separate five
     corridors.
  3. Repeat 2·level + 1 times:
     * draw `R(1,255)`; the feature is a force field if the value is above
       160, otherwise a wall.
     * draw `i = R(1,16) - 1` and take spot *i* from a fixed 16-entry list
       of (p, q) positions stored in `OUT.EXE`. The list's values are 1..9.
     * on odd levels write the feature at [p][q], on even levels at [q][p].
     
     The engine reads the list from the user's own `OUT.EXE`: two arrays
     of 16 words, at file offsets 0x10EA0 and 0x10EC0.
  4. Reseed with `time seed + 0x6F1`, so items differ on each visit. Then
     repeat level + 1 times:
     * draw the item: coffin if `R(1,100)` is odd, otherwise chest.
     * draw `a = R(10,99) div 10` and `b = R(10,99) div 10`.
     * place the item unless that cell is a wall, secret door or force
       field.
  5. Ladders:
     * odd levels: ladder down at [3][7], ladder up at [7][3].
     * even levels: ladder down at [7][3], ladder up at [3][7].
     
     Going down therefore lands on the next level's up ladder. Level 10
     has no ladder down. Level 1 has no up ladder at [7][3]; instead its
     up ladder (the exit) is at [1][1].
  6. Three monsters are placed (§6.3), still drawing from the generator
     reseeded in step 4.
* The engine re-implements this in `u1_assets::dungeon` and `::rng`.
  `u1view --dungeon X,Y,LEVEL,SEED` prints a level. **Compared in DOSBox
  (Phase 5):** levels 1–3 of the Mines of Mt. Drash II match cell for
  cell, as do their items and monsters once the clock values are known.

### 6.3 Monster placement: VERIFIED (original executable, DOSBox)

* One placement tries up to 500 random interior cells. Each try draws
  `a = R(0,242) mod 9 + 1`, then `b` the same way. A cell is accepted when it
  has no monster, is not wall, secret door or force field, and holds no
  chest. If no cell is found, or the cell is the player's own, nothing is
  placed.
* The kind is `R(0,49) div 10` (0..4), redrawn until it names a kind not
  yet on the level. The monster number is `5·((level-1) div 2) + kind`, so
  each level pair has its own five kinds, in the order of the list below
  (0 ranger, 1 skeleton, 2 thief, ... 24 balron). Its hit points are
  `R(1, level² + 1) + 10 + number`.
* A level gets three placements when it is set up. Each kill frees that
  kind again (`number mod 5`) and makes one new placement at once. An
  opened coffin can release a monster into the cell ahead (§6.5).
* Because the item reseed in §6.2 step 4 uses the time of day, and the
  monsters continue from that state, a level's contents depend only on
  the clock reading at the moment it is set up (entering, or each
  K-limb) and on the keys pressed since. The original's overworld seed
  is also clock-based (§6.1); the engine now reads the local clock the
  same way. A hidden `--seed N [--times t1,t2,...]` option fixes the
  start seed and the successive clock values for tests and side-by-side
  runs.

### 6.4 Moving, the view and the screen

* The player starts on level 1 at [1][1], facing south, on the up ladder.
  Facing codes: 1 west, 2 east, 3 north, 4 south. ↑ goes forward, ←/→
  turn, ↓ turns around ("Forward", "Turn left", "Turn right", "Turn
  around"). Forward is refused ("Forward - path blocked!" with a sound)
  by a wall, a secret door, a force field, a monster, or a door when the
  player already stands in a door.
* The view replaces the map window. The top border shows "Level n"
  between two ornaments, and the border under the view shows the compass
  word ("North", "South", "East", "West") in the same way. Both borders
  are restored on leaving, and during Ztats.
* Sight runs ahead until a wall, secret door, door or a monster that
  blocks the eye, up to 5 cells. The invisible seeker, the mimic and the
  gelatinous cube do not block it. The view is drawn from the far end
  towards the player. For each depth it draws the left and right side
  (open, wall, or wall with a door outline; a secret door looks like
  wall), then a vertical edge wherever the side changes between solid
  and open, then the cell contents: a front wall or door, a ladder (the
  outline depends on up or down, and on whether the player faces
  north/south or east/west), a
  force field in light red, a chest or coffin outline, or a monster
  outline. When a monster stands on a ladder, only the hole is drawn.
  Standing in a doorway draws the door frame around the view.
* All coordinate tables and outlines (walls per depth, ladder, chest,
  coffin and all monster figures) are read from the user's `OUT.EXE` at
  run time. The name of the nearest thing seen is printed in the message
  area (e.g. "Coffin", "Thief").
* **Compared in DOSBox:** the view interior (x 9–310, y 9–150 in
  320×200) is pixel-identical in every frame checked (about 50 frames on
  levels 1–3, including doors, secret doors, both ladders, coffins, walls
  at every depth and several monster figures). Text is identical in
  content; the engine draws it with its own font (§3).

### 6.5 Commands in the dungeon

* **K-limb**: only on a ladder ("K-limb what?" otherwise), and only when
  facing north or south ("Thou must face the ladder!"). Up prints
  "K-limb up to level n", down "K-limb down to level n"; each sets up the
  new level (§6.2). Up to level 0 returns to the overworld at the
  dungeon's tile. On the way out, the hit point bonus earned below is
  added ("Thou dost gain n hit points", capped so hits stay ≤ 9999).
* **Open**: "Open coffin". On a coffin, `R(1,255)` below 104 releases a
  monster into the cell ahead when that cell is open floor or a ladder
  with no monster; otherwise the coffin is emptied and holds
  `R(3, level²+9)` coins. "Open coffin - none here!" otherwise.
* **Unlock**: "Unlock chest". On a chest, if `R(1,75)` exceeds Agility and
  the character is not a thief, a trap costs hits equal to the level;
  otherwise the chest is emptied for `R(3, level²+9)` coins.
* **Inform and search**: "Thou dost find a secret door!" turns the secret
  door ahead into a door (not while standing in a door); otherwise
  "Thou dost find nothing".
* **Attack** uses the ready weapon without asking a direction. It looks
  ahead up to the weapon's range for the first monster, stopping at
  walls and doors; with none, "Nothing". The hit chance is `AGI + 50` out
  of `R(1,100)`. Damage is `R(2, STR + 8·weapon)`. Blows from or into a
  doorway only land on open cells.
* A blow that does not kill prints "Hit! n damage!". A kill prints
  "<name> killed!" (the Kill spell prints a "destroyed" message instead), then
  "Thou dost find: " and `R(2, 3·level + 10 + number)` coins, adds
  experience `R(2, level² + 10)`, and adds twice that to the climb-out hit
  bonus. Killing a gelatinous cube, carrion creeper, lich or balron
  completes the matching open castle quest ("Quest completed!").
* A quirk kept on purpose: when a monster moves, its hit points stay in
  the cell it left. Attacking a monster that has just stepped up may
  report "Nothing", and a monster that walks into a cell where another
  was wounded inherits the lower count. DOSBox shows the same.
* Get, Steal, Drop, Fire, Board, X-it, Transact and Quit give their
  dungeon refusals as in the original.

### 6.6 Monsters' turn

* After each player turn (no food is eaten below ground) the grid is
  scanned a = 1..9, b = 1..9. Each monster that has not already moved
  this turn either attacks, if it is orthogonally next to the player,
  or, within 6 steps (city-block distance), tries one step towards the
  player along a first and then along b. It cannot step into wall,
  secret door, force field or another monster.
* No attack happens while the player stands in a force field, or across
  a door from a wall or door cell.
* An attack prints "Attacked by <name>!". It misses ("Missed!") when
  `R(1,255)` ≤ `STA/2 + 8·armour + 56`. Otherwise the damage is
  `number² + level`, or `R(number+1, 255)` when that exceeds 255:
  "Hit! n damage!".
* Special attacks replace the damage:
  * Gelatinous cube with armour ready: a message that the armour is
    destroyed. One of that armour is lost and none is ready.
  * Gremlin: a message that food was stolen, and food is halved.
  * Mind whipper, half the time: a mental-attack message, and
    Intelligence becomes `INT/2 + 5`.
  * Thief: steals one weapon other than the ready one, the first in
    inventory order, names it with "a" or "an", and still deals damage.
* The mimic's printed name comes from its own string ("Mimic").
* All of these strings are read from the user's `OUT.EXE`.

### 6.7 Spells

* **Cast** uses the ready spell. In towns, castles and space: "Cast -- no
  effect!". Each cast uses one; with none left: "Used up!". The
  incantation is printed first.
* Prayer (never used up): in the dungeon it acts as a random spell,
  `R(1,10)` with 4 (Magic missile is excluded and 4 becomes 5). Outdoors it
  raises hits or food to 15 if below; otherwise "no effect".
* Open (1) / Unlock (2): open the coffin/chest under the player for
  coins. Magic missile (3): first monster within 5 cells, always hits,
  damage `R(1, INT)` ×2 with a wand, ×3/2 with an amulet, ×3 with a staff
  or triangle. Steal (4): no effect beyond being cast. Ladder down (5):
  not on level 10, not in a field, and only on a cell with an odd
  coordinate: "Ladder created!". Ladder up (6): likewise. Blink (7):
  teleport to a random free interior cell. Create (8): force field ahead
  if the cell is empty floor with nothing in it. Destroy (9): removes a
  force field ahead. Kill (10): destroys the monster ahead outright.
  Failure prints "Failed!". Outside the dungeon only Magic missile and Kill
  are allowed; other spells print the "dungeon only" refusal.
* Outdoors (and in the frigate's and the player's outdoor fights) a shot
  travels cell by cell: the shot tile (world tile 0x32 for weapons and
  cannons, 0x31 for spells) is drawn on the next cell, the first cell
  plays the attack sound (7 weapons and spells, 8 cannons), later cells
  wait two timer ticks, then the cell is redrawn. It stops on a monster,
  a mountain or at its range. On a hit the shot tile stays on the monster
  while the messages print and is replaced by the monster (survived) or
  the terrain (slain).
* Outdoor Magic missile / Kill: after the spell name, ": " and a
  direction (any other key: "Nothing", and the spell is given back). The
  incantation ("DELCIO-ERE-UI" or "INTERFICIO-NUNC!") follows on the same
  line; damage is computed (Kill: 9999), then the cast succeeds always
  for a Cleric, otherwise when `R(1,100) < WIS + 50`; failure appends
  "Failed!". On success a sound plays and the shot flies 3 cells and
  always hits.
* Outdoor prayer with hits and food at 15 or more: when `R(1,100) < 25`,
  the first object (in slot order) that is a monster and stands within its
  own attack reach (Chebyshev 3 for the four ranged kinds, else 1) of the
  player is flashed with tile 0x31 and removed: "Monster removed!". The
  original does not lower its monster counter here, so the removed monster
  still counts toward the spawn limit until OUT.EXE is restarted; the
  engine keeps the same count. Otherwise "Hmmmm... no effect!".
  Side by side in DOSBox: `u1-shots/phase7/play/sbs_ega_prayer_removed.png`
  (4 px, player animation) and `sbs_ega_magic_missile.png` (damage differs
  only because OUT.EXE seeds from the clock).

Dungeon inhabitants by level pair [S13][S16][S15]:
1–2 ranger, skeleton, thief, giant rat, bat · 3–4 spider, viper, orc,
cyclops, gelatinous cube · 5–6 ettin, mimic ("chest"), lizard man,
minotaur ("Minatour" in-game), carrion creeper · 7–8 tangler, gremlin,
wandering eyes, wraith, lich · 9–10 invisible seeker, mind whipper, zorn,
daemon, balron.

## 7. Save file — `PLAYERn.U1` / `INUSE.U1`

Source: ModdingWiki *Ultima I Save Game Format* (TheAlmightyGuru, Daniel
D'Agostino) [S8]; cross-checked with Ultima Stones [S8b] and a public cheat
note [S17]; method in [S18].

* **820 bytes**, mostly 16-bit little-endian integers [S1][S8].
  CONFLICT: [S8b] states 334 bytes, but its own field list (incl. 40 x 16-byte
  objects) cannot fit. **VERIFIED (original data):** all shipped saves are
  820 bytes (0xB4 + 40 x 16 = 0x334 ✓).
* **VERIFIED [M-save]:** the decoded fields of the three shipped saves
  match the game's Ztats screens. Checked fields: name, race, class, sex,
  hits, the six attributes, coin, experience, food, ready items, transport
  and position. The player is drawn at the stored X/Y.
* **VERIFIED (original executable) [M-exe]:** `OUT.EXE` keeps a copy of
  the whole 0xB4-byte header in its data segment. Every field access below
  was located in the code.
* Coin is stored in copper. The game shows it as gold, silver and copper,
  with 1 gold = 100 copper and 1 silver = 10 copper. The level shown on
  Ztats is `experience div 1000 + 1`.

| Offset | Type | Field | Notes |
|---|---|---|---|
| 0x00 | char[15] | Name | ASCIIZ, max 14 chars |
| 0x0F | u8 | 0 | |
| 0x10 | u16 | Race | 0 human, 1 elf, 2 dwarf, 3 bobbit |
| 0x12 | u16 | Class | 0 fighter, 1 cleric, 2 wizard, 3 thief |
| 0x14 | u16 | Sex | 0 male, 1 female |
| 0x16 | u16 | Hits | 0–9999 |
| 0x18–0x22 | u16 x6 | Str, Agi, Sta, Cha, Wis, Int | |
| 0x24 | u16 | Coin (copper) | 0–9999 |
| 0x26 | u16 | Experience | 0–9999 |
| 0x28 | u16 | Food | 0–9999 |
| 0x2A | u16 | Ready weapon | 0 hands, 1–15 (list below) |
| 0x2C | u16 | Ready spell | 0 prayer, 1–10 (VERIFIED) |
| 0x2E | u16 | Worn armour | 0 none, 1–5 (5 = reflect suit, VERIFIED) |
| 0x30 | u16 | Transport | 0 foot, 1 horse, 2 cart, 3 raft, 4 frigate, 5 aircar, 6 shuttle, 7 time machine (VERIFIED (exe)) |
| 0x32 | u16 | **Seed** | VERIFIED (exe): added into the dungeon structure seed and the outdoor RNG seed (§6) |
| 0x34 | u16 | Player X | 0–167 |
| 0x36 | u16 | Player Y | 0–155 (VERIFIED (exe): wrap at 155) |
| 0x38 | u16 | Sound on/off | VERIFIED (exe): toggled by the Noise command |
| 0x3A | u16 | Quest slot 0 | VERIFIED (exe): never used |
| 0x3C–0x4A | u16 x8 | Quest slots k = 1..8 at 0x3A + 2k | VERIFIED (exe), see §9.1 |
| 0x4C–0x52 | u16 x4 | Gems red, green, blue, white | |
| 0x54 | u16 | 0xFFFF | list terminator? |
| 0x56–0x5E | u16 x5 | Armour qty: leather, chain, plate, vacuum suit, reflect suit | |
| 0x60 | u16 | 0xFFFF | |
| 0x62–0x7E | u16 x15 | Weapon qty: dagger, mace, axe, rope & spikes, sword, great sword, bow & arrows, amulet, wand, staff, triangle, pistol, light sword, phazor, blaster | |
| 0x80 | u16 | 0xFFFF | |
| 0x82–0x94 | u16 x10 | Spell qty: open, unlock, magic missile, steal, ladder down, ladder up, blink, create, destroy, kill | |
| 0x96 | u16 | 0xFFFF | |
| 0x98–0xA4 | u16 x7 | Vehicle qty: horse, cart, raft, frigate, aircar, shuttle, time machine | |
| 0xA6 | u16 | Enemy vessels destroyed (space) | VERIFIED (exe) |
| 0xA8 | u16 | Last signpost visited | default 0xFFFF |
| 0xAA | u16 | Number of used map objects − 1 | VERIFIED [M-save]: matches the non-empty object slots |
| 0xAC | u16 | Move counter | VERIFIED [M-save]: increments per turn |
| 0xAE | u16 | Move counter, high word | VERIFIED (exe): 0xAC/0xAE form one 32-bit counter |
| 0xB0, 0xB2 | u16 x2 | Shuttle fuel, shield | VERIFIED (exe, Phase 6): both set to 1000 when a shuttle is bought; read at launch, written back on a safe landing, zeroed when the shuttle burns up (§12) |
| 0xB4–0x333 | 40 x 16 B | Map objects | below |

Map object (16 bytes): `+0 u16` object tile (index into §3.2) · `+2 u16`
the terrain value under the object for monsters and the time machine.
This is VERIFIED [M-save] against `MAP.BIN`, so [S8b] is right. For the
other vehicles the value varies and its meaning is UNKNOWN. · `+4 u16` X ·
`+6 u16` Y · `+8 u16` hits (monster hit points; 0 for vehicles) ·
`+0A..+0F` 3 x u16, always 0 in the shipped saves. Object slot 0 is the
player.

## 8. Full-screen images: VERIFIED (original data) [M-DOSBox]

Each decode below was compared with the game's own title screen in DOSBox
(EGA and CGA machines):

* `CASTLE.16` (32,000 B): 320x200, **packed 4 bpp**, 160 bytes per row,
  rows top to bottom, high nibble = left pixel. The nibble values are
  **not** EGA colour numbers. When the program displays the picture it
  maps them through this 16-entry table, giving default-palette EGA
  colours: 0→0, 1→8, 2→7, 3→15, 4→15, 5→2, 6→10, 7→6, 8→12, 9→7, 10→13,
  11→11, 12→1, 13→9, 14→7, 15→15. The table was derived by matching every
  pixel of the decoded image against the DOSBox capture. With it the
  match is exact.
* `CASTLE.4` (16,384 B): a CGA video memory dump. The **even** scanlines
  are in the first 8 KiB bank (8,000 bytes + 192 padding) and the **odd**
  scanlines in the second bank, starting at byte 8,192. Each row is 80
  bytes, 2 bpp, with bits 7-6 the leftmost pixel. It uses CGA palette 1
  low intensity (§3.1).
* `NIF.BIN` (6,720 B): 320x168, 1 bpp, 40 bytes per row, MSB = left
  pixel. It decodes to legible end-game text.

The engine implements these in `u1_assets::images`.

## 9. Game data that lives in code

The engine does not copy code tables into this repository. Small facts are
restated here; larger tables are read from the user's `OUT.EXE` at run time
(§4.1, §6.2).

**VERIFIED (original executable) [M-exe]:**

* **Food price:** 10 packs cost `5 - (save[0x22] div 20)` pence, so a
  higher sixth attribute (Intelligence) makes food cheaper.
* **Melee/ranged attack (overworld):**
  * damage = `R(2, Strength + 8·weapon)`, where weapon is the ready weapon
    number (0..15) and *R* is the generator in §6.1.
  * The hit test uses `Agility + 50`.
  * Each weapon has a reach in cells: 1 for hands, dagger, mace, axe,
    sword, great sword, triangle and light sword; 3 for bow, pistol,
    phazor and blaster; 0 (cannot attack) for rope & spikes, amulet, wand
    and staff.

  These semantics are inferred from how the code uses them.
* **Magic damage:** `R(1, Intelligence)` (save 0x22). It is doubled if
  the ready weapon is the wand, ×1.5 (integer) for the amulet, and tripled
  for the staff or triangle. The use of this value as spell damage is
  inferred.
* **Quests:** the castle quest strings ask the player to kill a
  gelatinous cube, carrion creeper, lich or balron, or to "find" a named
  landmark. Rewards are a stat point or hint, and the four gems.

**From public documentation (not yet verified):**

* Character creation bonuses: [S8b]. A new character starts at
  (49,40) on foot with Hits 150, Food 200 and Coin 100: VERIFIED (GEN.EXE +
  DOSBox).
* Space and Mondain's lair: see §12 and §13. Sound is PC-speaker code;
  there are no sound data files.

### 9.1 Outdoor rules and run-time tables: VERIFIED (original executable), Phase 3

The engine does not store these tables. `u1_assets::rules::ExeTables` reads
them from the user's `OUT.EXE` (69,858 bytes; the data segment starts at
file offset 0xD680). The DS offsets are:

| DS offset | Contents |
|---|---|
| 0x16CC | Monster appearance chance, 10 words, indexed by `Exp div 1000` (capped at 9) |
| 0x368C | Monster damage base, 15 words read. The last nine words are the first nine words of the next table; the original reads them that way and so does the engine |
| 0x3698 | Monster hit-point base, 15 words |
| 0x36E8 | Monster move chance (percent), 8 words, by the player's transport |
| 0x36F8 | Food rate (moves per food unit), 8 words, by transport |
| 0x2AB8 | 15 pointers to monster names |
| 0x37AE | 7 pointers to move-refusal messages |
| 0x37D4 | 5 pointers to direction names (index 1..4 = West, East, North, South) |
| 0x277E / 0x2968 / 0x2910 / 0x29AE / 0x30CE | name pointers: weapons, armour, spells, transports, landmarks |

In the shipped executable the appearance chances rise from 35 to 205 out
of 255. The food rates are 2, 3, 3, 4, 5, 6, 11, 0 (foot … time machine).
The move chances are 100, 50, 33, 100, 50, 25, 10, 10.

**Turn order (outdoors).** For each move or Pass:

1. The command is echoed: `>West`, `>Pass` and so on.
2. For a move, the target cell is checked. Refused moves print a message
   from 0x37AE: water; mountains; an aircar in woods; a frigate or a raft
   on land; "Blocked by a(n) <monster>"; or "can't move on land" for the
   shuttle and time machine. Water needs a raft, frigate or aircar.
   Mountains are never passable.
3. When the player enters a different land, the monsters of the old land
   are dropped.
4. A monster may appear (step 5), the move counter goes up by one, the
   monsters act (step 6), and food is eaten.
5. **Appearance.** This happens only while fewer than 11 monsters exist,
   and only if `R(1,255) < chance[Exp div 1000]`.
   * The type is `R(0,149) div 10`.
   * The place is the player's position plus `±R(10,15)` columns and
     `±R(5,15)` rows. Each sign is negative when `R(1,16)` is odd.
     Coordinates wrap.
   * The cell must hold the type's terrain: types 0–3 need water, 4–8
     forest, 9–14 grass. It must also be at least 9 columns and 6 rows
     inside the land's north-west corner.
   * The original retries until it finds a place; the engine gives up
     after 32 tries.
   * A new monster's hit points are `10·base + Exp div 1000 + 2`.
6. **Monsters act.** Each monster within reach attacks. The reach is 3
   cells for the ranged tiles 0x17, 0x19, 0x1F and 0x2F, and 1 for the
   rest. Otherwise, with the transport's move chance, a monster steps one
   cell toward the player. Sea monsters stay on water. Tiles
   0x1F/0x20/0x23/0x24 stay in forest. The rest use grass, forest and
   location cells.
   * An attack prints "<Name> attacks!".
   * It hits if `R(1,255) < 200 − Stamina/2 − 8·armour`.
   * Damage is `R(1, 2·damage_base + 1)`.
7. **Food.** The food counter starts at 1. A unit is eaten when it
   reaches the rate for the current transport. A rate of 0 (time machine)
   means food is eaten every move.

**Shops.** Prices are in coin units; *Int*, *Wis* and *Cha* are the
player's attributes.

| Item | Buy price | Shop pays |
|---|---|---|
| Weapon *n* = 1..15 | `(255−Int)·n² div 256 + 5` | `(Cha+40)·n² div 256 + 1` |
| Armour *n* = 1..5 | `((200−Int) div 4)·n` | `(Cha div 4)·n` |
| Spell *n* = 1..10 | `((200−Wis) div 32)·n` | not bought |
| Transport *n* = 1..7 | `((200−Int) div 5)·n²` | not bought |
| 10 food | `5 − Int div 20` | — |

* **Era** = 3 once the move counter passes 3000; before that it is
  `(moves mod 32767) div 1500`, capped at 3.
* **Weapon stock** in town *t* (1..32): `era + 2` items, every second
  weapon, starting at 1 if `(t−1) mod 8` is even and at 2 if it is odd.
* **Armour stock:** items 1–3, plus 4–5 after 3000 moves.
* **Magic shops:**
  * With *p* = `(t−1) mod 2`, the shop sells every second spell from
    `1+p` up to `5+p`, skipping spell 4 (steal).
  * Wizards (class 2) are offered spells up to `9+p`.
* **Transport shops:**
  * Horses and carts need a free grass cell next to the shop; rafts and
    frigates need water.
  * Aircars and shuttles are sold only after 3000 moves.
  * A player may own at most 15 vehicles.

**Quests.** The quest slot for castle quest *k* = 1..8 is the save word
at `0x3A + 2k`. Its values are 0 (none), 0xFFFF (given, not yet done)
and 1 (done).
* Even *k* are kill quests: the gelatinous cube, carrion creeper, lich
  and balron for k = 2, 4, 6, 8.
* Odd *k* are visit quests: the landmark `8 − k`.
* Rewards: odd quests raise Strength, even quests give the red, green,
  blue and white gems.
* The last landmark visited is kept at save 0xA8, so a landmark does not
  reward twice in a row.

**Castle entry.**
* The castle id gives the layout parity and the quest slot.
* A variable is set at random to 60 or 61 (0x3C/0x3D) on each entry. It
  selects which of the two jail-door tiles the key opens.
* It is not stored in the save.

**Leaving a dungeon** prints "Thou dost gain N hit points", where N is
the dungeon's accumulated bonus, capped so Hits stays ≤ 9999.

**Screen layout (EGA, 320×200).**
* The border is blue.
* The 19×9 viewport is at (8,8), 304×144 px, with a 1-px white frame.
* Below it, from y = 159, are a message box (x 0..241) and a stats box
  (from x 246), both white-framed.
* The text rows are at y 168, 176, 184 and 192, in light cyan.
* The last row holds the `>` prompt with a blinking cursor.
* The stats box shows `Hits:`, `Food:`, `Exp.:` and `Coin:` with
  4-character right-aligned values.
* The engine draws this with an 8×8 font loaded at run time (§16).

### 9.2 Towns, castles, shops, kings and saving: Phase 4

All items below are **VERIFIED (original executable)** unless marked
otherwise. Items marked [DOSBox] were also compared side by side with the
original running in DOSBox. Every message, shop title and name is read
from the user's `OUT.EXE` at run time, and maps come from `TCD.BIN`.

**Entering and leaving**

* `E` on a location tile: ids 1..32 are towns, using TCD map
  `(id-1) mod 8 + 2`. Ids 33..40 are castles, using map `(id-1) mod 2`.
  [DOSBox] Entered and compared: Britain, Moon, Fawn, Lord British's
  castle and the Castle of the Lost King.
* Start cell: (19,17) in a town and (0,9) in a castle [DOSBox].
* Walking off any edge of the 38x18 map returns to the overworld. The
  player stands on the location's own tile [DOSBox].
* NPCs are not in `TCD.BIN`. Each map's starting NPC list (tile, x, y,
  hits) is a table in the executable, which the engine reads at run time.
* On entry the castle's two jail-door cells are reset to the locked-door
  tile. The castle then picks, at random, which region code (60 or 61,
  §5) the key opens. `U` with the key opens only that one; the other
  stays shut. The key is found by killing the castle's jester. A freed
  princess follows the player once the castle is roused. Stepping off the
  map with her in one of the eight cells around the player adds 500 coin,
  500 hits and 500 experience [DOSBox, Phase 6]. If at least
  20 enemy ships were destroyed and experience is 7000 or more, a time
  machine is also placed on land near the castle.

**Commands inside** (original keys): arrows move; `A` attack; `D` drop
coin into the pond; `E`; `G` get (in a castle, from the storeroom, only
with the king's leave); `I`; `N` noise; `Q` (refused unless outdoors)
[DOSBox]; `R` ready; `S` steal at a shop counter; `T` transact; `U`
unlock; `X`; Space passes. Commands that only make sense outdoors print
the original's refusal.

**NPCs:** bards/jesters (they may sing, or steal a weapon), wenches,
merchants, guards, the king and princesses each have their own turn
routine. Guards walk toward the player once the town is hostile, and
strike when next to the player. The town turns hostile when the player
attacks someone or a theft is noticed. A kill gives 1 experience (a
guard gives 15). Only the first point is redrawn at once; the guard's
extra 14 show up at the next stats redraw [DOSBox].

**Shops** (`T` next to a counter; a counter's region code picks the
shop, §5; "not by a counter" otherwise [DOSBox]):

* Each shop has its own name box and a `letter) item - price` list.
  Buy and sell prices come from §9.1. Stock depends on the town and on
  the move counter.
* Weapon and armour shops sell and buy back. The magic shop sells
  spells, and the wizard class gets the longer list. Food is sold in packs
  of 10. The transport shop places the bought vehicle next to the town
  and closes when 15 vehicle objects exist.
* A purchase you cannot afford prints the item as "nothing" [DOSBox].
  Observed: Moon armour shop with 35 copper; Moon dagger at 5; Britain
  food.
* Pub: each ale costs 1 coin [DOSBox, Fawn]. After more ales than STA/4,
  a wench at the counter can take half the coin and 1 WIS. About 75 times
  in 255, the barkeep gives one of eight tips.

**Kings:** `T` in a castle works only next to the king ("He is not
here!" elsewhere) [DOSBox]. It offers pence or service:

* Pence: paying *n* copper gives `n*3/2` hits [DOSBox].
* Service: gives the castle's quest when its save slot (§7, 0x3A+2k) is
  0. It shows the quest again while the slot is 0xFFFF (active). It
  rewards the player and clears the slot when the slot is 1 (done).
* Odd quest slots reward Strength: `(99 - STR) div 8` points, at least
  1 unless STR is 99. Even slots give the gem of the current land; the
  fourth land's gem also opens the storeroom for `G`.
* Exact wording is shown from the user's `OUT.EXE` [DOSBox: quest offer
  and pence].

**Saving:** `Q` outdoors writes the 820-byte record (§7) to the chosen
file. The engine refuses to write into the game directory. Object slots
not changed in play are kept byte for byte. With the same seed and keys,
a quit after visiting a castle and a town produced a file byte-equal to
the original's [DOSBox]. Differences in other runs come only from random
rolls (NPC steps, monster spawns), because the original seeds its
generator from the clock.

**Status line colours:** Hits and Food show in the warning colour below
100. Experience and Coin always show in the text colour [DOSBox].

**UNKNOWN / approximate:** the town NPC frame choice; the duration of
some waits (see §18).

## 10. What the engine implements today

| Area | Module | Tests |
|---|---|---|
| All tile encodings + documented file specs | `u1-assets::tiles` | handcrafted bytes, round-trip for all 15 files, size checks |
| MAP.BIN | `u1-assets::map` | nibble order, 84-byte stride, 168x156/168x168 handling |
| TCD.BIN | `u1-assets::tcd` | column-major addressing |
| PLAYERn.U1 (documented fields) | `u1-assets::save` | offsets, 0F 27 = 9999, object table |
| Game dir discovery (case-insensitive) | `u1-assets::gamedir` | synthetic files |
| Software renderer (incl. water scroll, castle/town cadence) | `u1-assets::render`, `::map` | colours, off-map, animation timing |
| Location table loader (reads user's OUT.EXE) | `u1-assets::locations` | synthetic EXE image, id ranges, TCD map rule |
| RNG | `u1-assets::rng` | hand-computed sequences, range bounds |
| Dungeon generator and monster placement | `u1-assets::dungeon` | borders, ladders, determinism, synthetic spot list |
| Dungeon view, movement, monsters, combat, spells | `u1-assets::dview`, `::dgame` | turns, time seed; end-to-end DOSBox replay over three levels in `tests/with_game_files.rs` |
| CASTLE.16 / CASTLE.4 / NIF.BIN | `u1-assets::images` | handcrafted bytes |
| Outdoor rules, prices, stock, run-time tables from OUT.EXE | `u1-assets::rules` | synthetic EXE, wrap, passability, food, prices, monsters |
| Overworld turn logic (movement, monsters, food, messages) | `u1-assets::overworld` | synthetic map/tables |
| Towns, castles, NPCs, shops, pub, kings, princess, quit-save | `u1-assets::game`, `::town`, `::shops`, `::term` | stat colours; end-to-end scripts in `tests/with_game_files.rs` (need `U1_GAME_DIR`) |
| Space: launch, galaxy, views, fighters, docking, landing | `u1-assets::space` | Space Ace, small fighters, docking, landing (need `U1_GAME_DIR`) |
| Mondain's lair and the ending | `u1-assets::mondain` | intro, arena, gem, death, victory (need `U1_GAME_DIR`) |
| Death picture and resurrection | `u1-assets::game` | starving run (need `U1_GAME_DIR`) |
| Playable game binary | `u1` (SDL2) | screen layout, animation, font |

All unit-test data is synthetic, built from this spec. Tests that read
original files (one location check, plus `crates/u1-assets/tests/with_game_files.rs`)
run only when `U1_GAME_DIR` points at the user's own install; otherwise they pass
without doing anything.

## 11. Verification status (v0.6)

Done against the CD-ROM copy:

1. File list and sizes: ✓ (§2).
2. EGA/CGA/T1K tile decoding: ✓ pixel-exact against DOSBox (EGA, CGA);
   T1K equals EGA by value (§3.1).
3. MAP.BIN: nibble order and 168x156 ✓ (§4).
4. TCD.BIN: column-major order and map order ✓; values 51..63 are region
   codes (§5).
5. Save offsets: mostly ✓ (§7). 0x32 is the seed. 0xAE–0xB2 and the quest
   words are still open.
6. Tiles 0x31–0x33 identified (§3.2); animation timing measured (§3.5).
7. CASTLE.16, CASTLE.4, NIF.BIN encodings and CGA palette ✓ (§8, §3.1).
8. Location table, entry rules, RNG and dungeon generator ✓ from the
   executable (§4.1, §6).

Done in Phase 3 (v0.3):

9. Outdoor monster stats, appearance and movement; prices and per-town
   stock; quest slots; save 0x38, 0xAE, 0xB0/0xB2; the 60/61 castle
   variable; discs 0x31/0x32 (§9.1, §7, §3.2).
10. Poll timing and idle Pass measured in DOSBox at two CPU speeds
    (§3.5).
11. Edge wrap, including the 77-row shift on east/west crossings,
    observed in DOSBox (§4).
12. `u1` overworld compared side by side with DOSBox (start position,
    five moves, and the west-edge crossing).

Done in Phase 4 (v0.4):

13. Towns and castles (§9.2) compared side by side with DOSBox: Britain,
    Moon, Fawn, Lord British's castle, Castle of the Lost King; food,
    armour, weapon and pub transactions; king pence and quest; exits;
    quit-save byte-equal.

Done in Phase 5 (v0.5):

14. Dungeons (§6.3–6.7) compared side by side with DOSBox on levels 1–3 of
    the Mines of Mt. Drash II. Clock values recovered from the DOSBox
    frames (one per level set-up) reproduce every frame: view
    pixel-identical, monster movement, attacks, kills, coins, hits and
    experience.
15. Lord British's castle checked in DOSBox: killing the jester gives the
    key and the same three messages; the guards then turn hostile, close
    in and attack with the same messages. With the key, one jail door
    answered that it was not the correct key and the other opened, so the
    door that the key opens is chosen at random on each entry. In that
    run the princess was behind the door that would not open.

Done in Phase 6 (v0.6):

16. Space (§12): launch, top-down view, landing after a crunch and wrap
    compared in DOSBox; Space Ace, untouchable small fighters, docking and
    the ship change checked by tests against the user's `SPACE.EXE`.
17. Mondain (§13): intro panels and arena pixel-identical in DOSBox; the
    fight, gem, transformation and ending reproduced from the executable.
18. Death (§14): skull picture pixel-identical; resurrection compared.
19. Princess rescue reached in DOSBox: on a re-entry the key matched her
    door; leading her out added 500 hits, coin and experience.
20. Victory reached in DOSBox (edited save copy, 9999 hits): taking the
    gem cost 9999 -> 2500 hits as in the engine; "Mondain is dead!" is
    followed by an empty line, then the victory panel and text; the
    `NIF.BIN` picture and closing text match pixel for pixel (only the
    restart line's font differs without a ROM font).
21. Fighters firing back compared in DOSBox with a 9999 shield: the
    "Alien fires!" message, the light-red beam from the fighter to the
    bottom middle of the viewer on a hit, "You've been hit!" and a loss of
    321 shield per hit (9999, 9678, ... 8715) all match.

Still open:

* Dungeon levels 4–10 not yet compared in DOSBox; same code paths.
* Exact monster frame choice (engine uses its own pseudo-random pick).
* A DOSBox machine=tandy comparison.

## 12. Space (`SPACE.EXE`): Phase 6

VERIFIED (original executable) unless marked; [DOSBox] = compared side by
side. Strings, the galaxy layout tables and the ship and fighter sprites
(`EGASPACE.BIN`, `EGAFIGHT.BIN`) are read from the user's files at run time.

* **Launch.** Boarding a shuttle outdoors hands over to the space program.
  The shuttle's fuel and shield come from save 0xB0/0xB2; other ships
  start full. A countdown is shown, then the home sector [DOSBox].
* **Galaxy.** A 7x7 grid of sectors, home at (3,3); each holds a few stations, enemy
  fighters and possibly a star. The sector scan draws the whole grid.
  Hyperjump moves to a neighbouring sector.
* **Views.** Top-down (turn, thrust, retro; an idle ship keeps drifting
  and the prompt prints Pass by itself after 1500 key polls, counted from
  zero at each prompt, two ticks per poll in the top-down view: about
  165 s, measured in DOSBox) and the first-person front view (speed 1-8,
  crosshair, streaming stars). The top-down view matched DOSBox pixel for
  pixel; front-view star positions are random in both.
* **Fire** works only in front view, costs 10 fuel and draws two laser
  lines to the crosshair. A fighter inside the hit box of its current size
  is destroyed: +100 experience and +1 to the kill counter (save 0xA6).
  The smallest (size 0) fighter's box is empty, so a distant fighter
  cannot be hit, as in the original. Fighters sometimes shoot back.
* **Space Ace.** The 20th kill shows a three-line panel; each later kill
  prints a short "still an ace" line.
* **Touching something** is checked in this order: a star (the ship
  melts, death); the planet, only in sector (3,3) within 25 units of
  (60,75): a shuttle lands safely and its fuel and shield are written to
  0xB0/0xB2, any other ship burns up (0xB0/0xB2 zeroed, death); otherwise
  "Crunch!": the shield loses a quarter plus 5, the velocity reverses and
  an empty shield kills. An idle shuttle drifting into the home station
  crunches, bounces back, wraps at the sector edge and lands [DOSBox].
* **Docking** at a station: without vacuum-proof armour (ready armour
  below 4) the pilot dies. Otherwise 500 coin refuels and recharges the
  ship (a short message if there is not enough). With more than one ship
  docked the player picks a port 1-4 and can leave in that ship.
* **Back to OUT.** Landing or death restarts the outdoor game from the
  record; death leaves hits and food at 0, so the death flow (§14) runs.

## 13. The time machine and Mondain's lair (`MONDAIN.EXE`): Phase 6

* A time machine is outdoor object 0x12. It is placed near a castle when
  a space ace (20+ kills, 7000+ experience) rescues a princess (§9.2),
  up to nine times. Boarding it without all four gems shows a short
  panel and nothing happens; with all four gems the lair starts.
* Four intro panels, each closed with "Press Space to continue", then the
  arena: a fixed grid read from the executable, tiles from `EGAMOND.BIN`
  and `EGATILES.BIN` [DOSBox: intro and arena pixel-identical].
* Mondain starts with 1000 hit points. Until the player acts he only
  chants now and then. Attacks use the ready weapon's range (walls do not
  stop a shot). Spells: Magic Missile, Blink, Create, Destroy; Kill only
  doubles his hit points (up to 15000).
* His turn: next to the player he strikes (chance from Agility and
  armour; damage hits/32 + 1..20); within 7 cells, half the time he
  casts one of three spells (a missile, a blast that lowers all six
  stats, or a shock with a palette flash); otherwise he steps *away*
  from the player (the step is the sign of his position minus the
  player's, one axis tried first at random, then the other). As a demon
  a zero axis is replaced by a random sign, so a cornered demon stays
  put and regains 5 [DOSBox: he retreats to the arena corners].
* **The gem.** `G` next to it takes it for three quarters of the current
  hits. Fire bursts then start appearing on random empty cells. Below 501
  hit points he becomes a demon that wanders and regains 5 when stuck.
  At 0 he falls; without the gem taken he rises again (+25 per turn).
* **Ending.** With the gem taken and Mondain at 0, the victory panel is
  shown, then the `NIF.BIN` picture and the closing text with the
  restart prompt. The original stops there; so does the engine.
* **Death in the lair** returns to OUT with hits and food at 0 (§14).

## 14. Death and resurrection: Phase 6

VERIFIED (original executable) [DOSBox].

* Triggered when hits or food reach 0 (outdoors, in a dungeon, or on the
  way back from space or the lair).
* The view is cleared and a 64x95 one-bit picture (a skull, stored in
  `OUT.EXE`'s data segment) is drawn at (128,31) in white; three death
  sounds; "<name>, thou art dead", a pause, then "Attempting
  resurrection!". Hits count up to 99, every outdoor monster in all four
  lands is removed, food counts up to 99, and coin, transport, weapons and
  armour are cleared before the player is placed back on land. The
  picture matched DOSBox pixel for pixel.

## 15. Video modes: Phase 7

The original picks one display when it starts and only ever reads that
mode's tile files. The engine does the same: it uses the first of EGA,
Tandy, CGA whose `???TILES.BIN` is present in the game directory, or
`--mode ega|t1k|cga` (`gamedir::VideoMode`).

* **EGA** (320x200, 16 colours, shown doubled): tile files are decoded as
  in §3.1. Pixel-identical to DOSBox `machine=ega` in all screens checked.
* **CGA** (palette 1, low intensity): the 16 text and frame colours of the
  EGA screens are mapped to the four CGA colours with the same table the
  original uses (read from OUT.EXE at run time); the frame corners and the
  blinking prompt cursor are drawn from OUT.EXE's own CGA bitmaps. Space
  fighters and ships use the two-tiles-per-sprite CGA layouts (§3).
  Compared with DOSBox `machine=cga`: castle, credits and town 0 px
  different, overworld and dungeon differ only by the cursor phase.
* **Tandy 1000** (16 colours, IRGB): values decode exactly like EGA, so
  every screen equals the EGA one; compared with DOSBox `machine=tandy`
  for overworld, town, dungeon and castle (cursor phase only).

Shots: `/workspace/u1-shots/phase7/sbs_*` and `/workspace/u1-shots/phase7/play/sbs_*`.

## 16. Text font: Phase 7

The original prints text through the PC BIOS, so its glyphs are the
machine's ROM font. Neither the game files nor this repository contain
it. The engine loads an 8x8 font at run time from `--font FILE` or the
`U1_FONT` variable: a raw dump of 8-byte glyphs (128 to 256 of them), a
PSF1/PSF2 font with 8x8 glyphs, or a Unifont-style `.hex` file. Without
one it uses a subset of the public-domain unscii-8 font shipped in
`crates/u1-assets/fonts/`, which looks close but not identical. With the
machine's own ROM font supplied, text rows match DOSBox pixel for pixel.

## 17. Start-up programs: title sequence (`ULTIMA.EXE`) and character program (`GEN.EXE`): Phase 7

Run with `--intro` (title sequence, then the character program) or `--gen`
(character program only). Text, name tables and the cursor shapes are read
from the user's `ULTIMA.EXE` / `GEN.EXE` at run time; nothing is copied.
Characters live in a roster directory (`--roster DIR`, default the
`--save-to` directory; the game directory is refused). The chosen
character's file is loaded and the game saves back into that same file,
as the original does.

**Title sequence**: verified pixel-identical in EGA, CGA and Tandy (castle
with flag frames, pages, credits). The title text screen stays up for 12
ticks, which stands in for the original's file-load time (a guess).

**Character program** (`crates/u1-assets/src/gen.rs`), in the original's
drawing order:

- Border: blue bands around a white rectangle, rounded black corners.
  Menu: title and three lines in white, options and "Thy choice:" in the
  text colour (EGA/Tandy light cyan, CGA colour 3). A, B or Q is accepted;
  Q only redraws the menu.
- Keys are read with an animated cursor: four 8x8 shapes from `GEN.EXE`
  drawn at the text cursor, the next one every 3 timer ticks while no key
  is waiting; after a key the cell is blanked. Point allocation shows no
  cursor.
- Generation: 30 points over six attributes starting at 10 (range 10..25);
  up/down move the ornamented markers (wrapping), right/left spend or
  refund a point; Space continues only at 0 points (otherwise a beep),
  Escape returns to the menu. Then race (Human +5 Int, Elf +5 Agi,
  Dwarf +5 Str, Bobbit +10 Wis -5 Str), sex, class (Fighter +10 Str and
  Agi, Cleric +10 Wis, Wizard +10 Int, Thief +10 Agi), name (up to 14
  characters; no leading space; Backspace/left moves back without
  erasing), and "Save this character? (Y-N)": N starts over.
- Saved record (820 bytes, race/class/sex as 0-based indexes): hits 150,
  coin 100, food 200, weapon/armour 1 ready, position (49,40) in both
  places, the fixed object-table defaults, and seed = clock value + Wisdom
  + class, where the clock value mixes the DOS time
  (`((sec<<3 & 0xff)<<8 | hundredths) + ((hour<<1 & 0xff)<<8 | (min<<2 & 0xff))`,
  xor 0x91eb, low 12 bits). The rest of the buffer carries over from an
  earlier character in the same session, as in the original. With
  `--seed` the clock reads 00:00:00.00.
- Full roster: "Character Replacement" lists the four names; 1-4 picks a
  slot, Escape returns. Selection: an empty roster offers Enter to
  generate into slot 1; otherwise 1-4 picks an existing character and the
  "Please wait whilst thy game loads" screen is shown before the game.

Verified against DOSBox (EGA, CGA, `machine=tandy`) with the same keys:
menu, empty selection, points, race, sex, class, name, save prompt,
menu after saving and the selection list are pixel-identical once the
cursor is at the same animation frame; the written PLAYER1.U1 matched the
original's byte for byte apart from the clock-dependent seed. Shots:
`/workspace/u1-shots/phase7/gen/`.

Approximations: the "please wait" screen is held for 9 ticks (the original
shows it for as long as DOS takes to load OUT.EXE, too short to capture in
DOSBox); the cursor's phase depends on real time, as in the original.

## 18. Remaining differences from the original (end of Phase 7)

Each item has been compared with the original in DOSBox or read from its
code; nothing else is known to differ.

1. **Random numbers.** The original seeds its generator from the clock;
   the engine takes a seed (`--seed`, default from the clock). Monster
   moves, hits, spawns, prayer results and monster frames therefore match
   only in kind, not roll for roll. Same seed and same keys give
   byte-equal saves (§9).
2. **Real-time phases.** Water scroll, castle/town flags, monster frames
   and the prompt cursor advance once per keyboard poll, and the number
   of polls depends on how long the player waits. Side-by-sides differ
   only in these phases.
3. **Sound.** The ten effects are played (§19); half-wave counts and the
   pitch ratio of effects 0 and 1 were checked against a DOSBox capture.
   Pitch and length in the original depend on the CPU; the engine fixes
   them to a 4.77 MHz 8088 model.
4. **Poll speed.** One poll is taken as 130 ms outdoors (DOSBox
   cycles=20000); the original's speed depends on the CPU. Space waits 2
   timer ticks per poll, so its automatic Pass after 1500 polls is
   ~165 s in both. The very first Pass in one DOSBox run came after
   ~231 s; this was not reproduced and is unexplained.
5. **Mondain ending timing.** Compared in DOSBox (§11 item 20); panel,
   picture and closing text match. Two waits depend on the CPU: the
   original polls the keyboard up to 10,000 times after the victory text
   (engine: ~5.5 s), and it plots `NIF.BIN` one pixel at a time, so the
   picture appears from the top down (about 5.5 s at cycles=4000, about
   1.1 s at the cycles=20000 speed the engine is matched to; the engine
   draws eight rows per timer tick, ~1.1 s).
6. **Load-time waits.** The title text (12 ticks) and the "please wait"
   screen (9 ticks) stand in for DOS loading the next program.
7. **Town NPC frames.** Which of an NPC's two frames is shown follows the
   engine's own rule; not yet traced in the original.
8. **Prayer flash in towns.** The outdoor prayer flash is drawn; the
   town/castle one is not.
9. **Font.** Without a user-supplied ROM font, text uses the unscii-8
   lookalike (§16).

## 19. PC-speaker sound: Phase 8

VERIFIED (original executables, reference reading).

* There is no sound data. `OUT.EXE`, `SPACE.EXE`, `MONDAIN.EXE` and
  `GEN.EXE` each carry the same ten effect routines (byte-identical, same
  numbering through a ten-entry table); `ULTIMA.EXE` has none. A wrapper
  plays effect *n* only while the save's sound flag (offset 0x38, the
  Noise command) is on; `GEN.EXE` calls the routine directly.
* The routines do not program the timer. They clear the timer-gate and
  speaker bits of port 0x61 and then flip the speaker bit by hand,
  waiting between flips in counted busy loops. Pitch and length therefore
  scale with CPU speed; the game is frozen while an effect plays. Every
  effect ends by writing back the port value it found.
* A *pass* below is one turn of a two-instruction "decrement, jump if not
  zero" loop. A wait of 0 on an 8-bit counter means 256 passes. Each
  half-wave is the wait plus a few instructions of overhead.

| # | Shape | Used for (examples) |
|---|---|---|
| 0 | 16 half-waves, each a 1000-pass wait (16-bit counter): a short low buzz | bumping a wall, "Crunch!", stepping into fire |
| 1 | 48 half-waves, each 355 passes: a higher buzz | errors, "Huh?", can't afford |
| 2 | 200 half-waves; the 8-bit wait starts at 0x83 and grows by 5 each half-wave, wrapping past 255, so the pitch falls, jumps back up and falls again | a hit, damage taken |
| 3 | as 2 with 255 half-waves | a monster attacks |
| 4 | as 2 with 8 half-waves: a click | a step in towns and the lair |
| 5 | 26 steps; each step is 60 pulses of "high for A passes, low for B passes", A starting at 0xC0 and dropping by one per step, B going 1, 2, … 26 (8-bit waits): a long buzzing warble | spells and magic that work, quests, resurrection |
| 6 | 160 half-waves whose 16-bit wait falls from 160 to 1 passes: a rising whistle | a spell that fails |
| 7 | as 5 but A starts at 5 (so it wraps through 0 = 256 to 255, 254, …) and 15 pulses per step | attacking (weapons and spells) |
| 8 | 251 half-waves whose 8-bit wait rises from 5 to 255 passes (counted up to wrap): a falling whistle | space lasers |
| 9 | 4 half-waves of 5, 10, 15, 20 passes: a tick | each star-field frame in space |

* **Timing model used by the engine.** The loops are costed with 8088
  instruction timings (decrement 2 or 3 cycles, taken jump 16, not taken
  4, port write 10, …) at the IBM PC's 4.77 MHz. That gives effect 0
  about 132 Hz for 60 ms, effect 1 about 372 Hz for 65 ms, effect 2 104 ms,
  3 132 ms, 4 5 ms, 5 1.2 s, 6 50 ms, 7 0.33 s, 8 131 ms and 9 0.2 ms.
  The engine renders the exact sequence of speaker levels at those times
  as a square wave (box-filtered to the output rate) through SDL audio,
  and waits until it ends, as the original does. `--mute` keeps the
  waits but plays nothing; script and screenshot runs only record which
  effects were asked for.
* **DOSBox capture** [M-DOSBox] (`pcspeaker=true`, wave capture,
  cycles=4000): "Huh?" gave two bursts of 48 half-waves, bumping into
  water three bursts of 16, matching effects 1 and 0. Their half-waves
  were 0.18 ms and 0.50 ms, a ratio of 2.75 (the model gives 2.8). At
  cycles=4000 DOSBox runs these loops about 7.6 times faster than an
  8088, so its pitches (about 2.8 kHz and 1 kHz) are higher in the same
  ratio; the model's 372 Hz and 132 Hz correspond to roughly
  cycles=530. Effect 4 (the step click) is too short to resolve at
  44.1 kHz in the capture.

## 20. Enhanced video mode (engine feature, not original behaviour)

This section describes an optional feature of this engine. It is not part
of the original game; the faithful 320×200 output (§3) stays the default.

* **Asset pack.** A directory with `manifest.toml`, a small TOML subset
  (top-level keys, `[section]` headers, strings, integers including `0x`,
  booleans, lists of strings, `#` comments). Top-level keys: `name`,
  `license`, `tile_size` (16–128, a multiple of 16; default 32) and a
  `[palette]` table of free-form notes per video mode. Each other section
  names an original picture: `world.N` (overworld tile set, §3.2),
  `town.N` (§3.3), `mond.N` (§3.4), a full screen `screen.NAME`
  (`nif`, `title`, `intro`, `castle`, `menu`, `loading`), a part of the
  first-person dungeon view `dungeon.NAME` (`wall`, `door`, `secret`,
  `floor`, `ceiling`, `ladder_up`, `ladder_down`, `hole_up`, `hole_down`,
  `chest`, `coffin`, `field`), a dungeon monster `dmon.N` (N = the
  monster kind 0–24, §6) or a part of the space view `space.NAME`
  (`frame`, `cockpit`, `starfield`, `deep`, `earth`, `sun`, `station`,
  `ship0`–`ship2`, `fighter`, `reticle`). It holds `file`,
  `frames` or `variants` (PNG paths relative to the pack), and optionally
  `scroll`, `fps` and `behind`. An unknown section is skipped with a warning, and a wrong-sized
  PNG is resized nearest-neighbour with a warning. A malformed line, an
  unknown key in a picture section or a missing or unreadable PNG is an
  error, and the pack is not loaded.
* **Sizes.** With S = tile_size / 16 the internal screen is 320S×200S.
  World and Mondain tiles are (16S)², town tiles (8S)². Screens:
  `nif` 320S×168S; `title`, `intro` and `castle` 320S×200S; `menu`
  304S×184S and `loading` 304S×144S (the insides of the character
  program's borders). Dungeon and space pictures may have any size; they are
  stretched over the view's geometry.
* **Recording.** The game draws exactly as before. Every tile blit also
  records an *art op*: the picture's id, the screen rectangle, the
  pixels it drew, the map cell it shows (world map x,y outdoors, town
  map cell in towns and castles) and, for a sprite, the ground under it
  (outdoors the terrain under a tile > 9; in towns the town map tile
  under the player or a person; in Mondain's arena the floor tile 0 under
  every tile but the floor and the wall). A later blit that overlaps an op
  removes it.
* **Variants.** `variants = [...]` lists alternative pictures for one id
  (with `file`, the file is the first). Each map cell shows variant
  h(x, y) mod n, where h is a fixed integer hash of the cell
  (`enhanced::variant_index`), so the choice never changes while
  playing and large fields of one terrain do not repeat. A sprite's
  ground uses the same cell. Ops with no map cell use the first
  variant. `variants` cannot be combined with a multi-picture `frames`
  list.
* **Composition.** The 320×200 screen is scaled up by S
  nearest-neighbour. Then each op whose rectangle still holds exactly the
  pixels it drew, and whose id is in the pack, is replaced by the pack's
  picture. A sprite PNG with alpha is drawn over the pack's picture of the
  terrain under it, or over black if the pack lacks that terrain. So
  anything that has been drawn over (text, the cursor, hit flashes, the
  map reveal) automatically shows the original art. A filled rectangle
  (a text box, a cleared view) also drops the ops under it, since black
  over a black tile (Mondain's floor) would leave its pixels unchanged.
* **Backdrops.** With `behind = true` the picture only shows where the
  original region is black (colour 0). The original's lit pixels stay as
  they are, and a backdrop pixel diagonally down-right of a lit one is
  darkened to a quarter, a drop shadow. This is how a screen of text
  (the ending, `screen.nif`) gets a picture without changing its words.
* **Animation.** With `scroll = true` (water) the frame is chosen from
  the original scroll offset (frame = offset · n / 16 for n frames) and
  the picture is shifted by offset·S pixels, so the motion keeps the
  original timing. Otherwise, a `frames` list cycles at `fps` by the wall
  clock. Castle (4/5) and town (7/8) frames are separate ids, so they
  keep the original's frame choice (§3.5).
* **Palette effects.** While the screen is drawn with a changed palette
  (the inverted flash, palette remaps), no overlays are drawn, with one
  exception: the start-up castle (below).
* **Start-up screens.** The title sequence (§17) records whole-screen ops
  as it draws: `title` for the text title, `intro` for the logo screen
  (renewed on every dissolve frame, after "Presents", after each text
  page and on the credits page) and `castle` for the castle picture. The
  castle op is renewed after each flag the original draws, so it stays
  intact, and carries which of the three flag frames the original chose
  for the left and the right flag (its `pos`). With it, the pack's
  picture replaces the whole screen even though the castle uses a
  palette remap: the pack's frame N shows both pennants in the
  original's flag frame N, and the left half of the screen (x < 160) is
  taken from the left flag's frame, the right half from the right
  flag's, so each pennant changes exactly when the original's does (no
  clock). Only the EGA castle is noted: the pack's picture is an EGA
  picture and, like every picture the pack has no version of, the CGA
  and Tandy castles (their own pictures and colours) stay original. The
  character program (§17) sets a standing *backdrop* (`menu` over the
  inside of its border from its first screen; `loading` inside the
  "please wait" box): unlike an op it is not dropped by drawing, and
  shows wherever the region is black until replaced, so screens being
  drawn between key waits (a stat being rolled, a roster being written)
  keep it too. `title`, `intro`, `menu` and `loading` are backdrops
  (`behind = true`), so every original word, line and the logo stay
  exactly as drawn.
* **Dungeon view.** The first-person view (§6) is still drawn as the
  original's lines. While drawing, the engine also notes each surface
  between two of the view's vertical edges (wall faces ahead and at the
  sides, the far wall of a side passage, doors, the door frame when
  standing in a doorway, force fields), each object with the box its
  outline covers (monsters, chests, coffins, ladders and their holes),
  and a painter's order (farther first; an object after the walls of its
  square). If the view still shows exactly what was drawn and the pack
  has `dungeon.wall`, the view is redrawn: floor and ceiling pictures
  repeat once per square, placed by the corridor tables and fading
  toward the horizon; each surface is filled with its picture,
  perspective-correct across side walls, darkened by depth (light
  0.95 · 0.78^depth) and given a dark edge; force fields use the alpha of
  their picture; sprites keep their proportions and stand on the bottom
  of their box (chests, coffins and the mimic, low wide boxes in the
  original, fill the box's width and may rise to twice its height;
  ladders are stretched to it, a ladder seen edge-on gets a rail's
  width). A hole in the floor or ceiling keeps the original's
  four-sided outline (shapes 0x1a and 0x1b: level near and far edges,
  the far one narrower): each row of the picture is stretched between
  the outline's slanted sides, top edge to bottom edge. The outer walls are extended to the
  edges of the view. A surface or object without a picture keeps its
  original lines. A secret door without `dungeon.secret` uses the wall
  picture, so it stays hidden as in the original; traps are not drawn
  in the original view and get no picture. The geometry is unchanged.
* **Space view.** Space flight (§ space) still XORs its pictures into
  the screen. Each redraw of the view starts a scene (top-down or front
  view) that keeps the frame's top bar as drawn; every picture plotted
  into it is noted with the pixels it lit: Earth, the sun and the
  station; each ship with its type and heading (an XOR redraw of the
  same pixels erases the note); the front view's fighter and crosshair.
  The scan of `Inform`, `Ztats` and leaving space drop the scene. While
  the frame bar is unchanged, the frame (`frame`, or `cockpit` in the
  front view if present) is stretched over the frame ring; the
  background (`starfield`, or `deep` in the front view, else
  `starfield`) is stretched over the view wherever the original is
  black and under the noted pictures the pack has; a lone white pixel
  (a star) becomes a soft point; and each noted picture still at least
  three-quarters lit is fitted, keeping its proportions, into the box
  of the pixels it lit — ships are drawn nose up and turned a quarter
  turn per heading (west, east, north, south: 3, 1, 0, 2 turns
  clockwise), the crosshair's picture is 3 pixels larger on each side,
  and the fighter's may be up to 1.5 times its box's height (centred on
  it) so a tall picture is not squeezed into the original's flat H.
  Shots, exhaust, explosions and anything without a picture keep their
  original pixels, over the new background.
* **Limitations.** The NIF reveal shows the original art until the
  picture is complete.

## Sources

Accessed 2026-10-29. Prose/tables were read; no third-party code was copied.

* [S1] Daniel D'Agostino, *Dino's Complete Guide to Ultima 1* — Game Files
  <https://gigi.nullneuron.net/ultima/u1/files.php> and File Formats
  <https://gigi.nullneuron.net/ultima/u1/formats.php> (MAP.BIN notes quoted
  from Johnny Wood).
* [S2] (reserved) same site also links "Executable Flow" and "Palettes" pages; not yet read, not used for any claim here.
* [S3] ModdingWiki, *Ultima I Tile Graphic Format*
  <https://moddingwiki.shikadi.net/wiki/Ultima_I_Tile_Graphic_Format>
* [S4] ModdingWiki, *Ultima I Map Format*
  <https://moddingwiki.shikadi.net/wiki/Ultima_I_Map_Format>
* [S5] Ultima Stones (mithyr.com), *EGA Global Tiles Data Type*
  <https://mithyr.com/Ultima/1.0_I_GemOfImmortality/devDataTypeEGATILES_BIN.htm>
* [S6] Ultima Stones, *EGA Local Tiles Data Type*
  <https://mithyr.com/Ultima/1.0_I_GemOfImmortality/devDataTypeEGATOWN_BIN.htm>
* [S6b] Ultima Stones, *Place Map Data Type* (TCD.BIN)
  <https://mithyr.com/Ultima/1.0_I_GemOfImmortality/devDataTypeTCD_BIN.htm>
* [S7] Ultima Stones, *World Map Data Type* (MAP.BIN)
  <https://mithyr.com/Ultima/1.0_I_GemOfImmortality/devDataTypeMAP_BIN.htm>
* [S8] ModdingWiki, *Ultima I Save Game Format*
  <https://moddingwiki.shikadi.net/wiki/Ultima_I_Save_Game_Format>
* [S8b] Ultima Stones, *Player Saved Game Data Type*
  <https://mithyr.com/Ultima/1.0_I_GemOfImmortality/devDataTypePlayerU1.htm>
* [S9] PCGamingWiki, *Ultima I: The First Age of Darkness*
  <https://www.pcgamingwiki.com/wiki/Ultima_I:_The_First_Age_of_Darkness>
* [S10] GOG.com, *Ultima 1+2+3* <https://www.gog.com/en/game/ultima_1_2_3>;
  GOG forum on `game.gog` images
  <https://www.gog.com/forum/ultima_series/using_ultima_underworld_with_dosbox_android>
* [S11] Ultima I Unity <https://u1unity.archiveofmyth.com/> — cited **only**
  to note that VGA sets are fan-made. This project states it works from
  decompiled executables, so it is **not** used as a format source.
* [S12] Wikipedia, *Enhanced Graphics Adapter* (default 16-colour palette)
  <https://en.wikipedia.org/wiki/Enhanced_Graphics_Adapter>
* [S13] Ultima Stones, *Dungeons*
  <https://mithyr.com/Ultima/1.0_I_GemOfImmortality/atlasDungeons.htm>
* [S14] StrategyWiki, *Ultima I — Monster slayer quests* (dungeon structure,
  names) <https://strategywiki.org/wiki/Ultima_I:_The_First_Age_of_Darkness/Monster_slayer_quests>
* [S15] RPGClassics Shrine, *Ultima 1 — Dungeons*
  <https://shrines.rpgclassics.com/pc/ultima1/dungeons.shtml>
* [S16] Codex of Ultima Wisdom, *Ultima I monster data*
  <https://wiki.ultimacodex.com/wiki/Ultima_I_monster_data>; Gamer Corner,
  *Dungeon Levels* <https://guides.gamercorner.net/ultimai/dungeon-levels/>
* [S17] Cheatbook, *Ultima I FAQ* (save offsets 0x16–0x28)
  <https://www.cheatbook.de/wfiles/ultimaia.htm>
* [S18] Gigi Labs, *Ultima 1 Reverse Engineering: Decoding Savegame Files*
  (save-diffing method, no code analysis)
  <https://gigi.nullneuron.net/gigilabs/ultima-1-reverse-engineering-decoding-savegame-files/>
