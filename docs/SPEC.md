# Ultima I (DOS) — data file & format specification

Status: **draft 0.1 (2026-10-29)**. Compiled clean-room from public community
documentation only (see [Sources](#sources)). Nothing in this document was
obtained by disassembling, decompiling or debugging Origin's executables, and
none of it has yet been checked against a real copy of the game.

Legend used throughout:

* **CONFIRMED (docs)** — stated by at least one public source and consistent
  with the documented file sizes (arithmetic cross-check). Still not
  verified against real files by this project.
* **CONFLICT** — public sources disagree; both claims are recorded.
* **UNKNOWN / needs verification against a real copy** — not covered by any
  source we found, or only guessed.

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
| `ULTIMA.EXE` | 36,880 | Intro program | code — never read by engine |
| `GEN.EXE` | 20,112 | Main menu & character generation | code — never read |
| `OUT.EXE` | 69,858 | Sosaria: overworld, towns, dungeons | code — never read |
| `SPACE.EXE` | 39,824 | Space flight / combat | code — never read |
| `MONDAIN.EXE` | 31,650 | Final battle with Mondain | code — never read |
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
| `PLAYER1.U1` … `PLAYER4.U1` | 820 | Save slots (§7) | mostly CONFIRMED (docs) |
| `INUSE.U1` | 820 | Same layout; hand-off between the EXEs | CONFIRMED (docs) |

**Not present as data files** (hard-coded in the executables, therefore
UNKNOWN in byte form and to be re-implemented from documented *behaviour*):
monster stats, weapon/armour/spell/vehicle tables, shop prices, signpost
and NPC text, the location → town/castle/dungeon assignment, and the dungeon
generator (§6, §9). No compression is documented for any file; every format
below is raw and headerless [S3].

UNKNOWN / needs verification: whether the GOG build adds/omits any file
(e.g. a `SOUND`/config file, or `INSTALL.EXE`), and exact upper/lower case
of names. The engine matches names case-insensitively.

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
pixel per copy) appear to be for fast sub-byte horizontal placement. The
exact copy count/shift semantics are UNKNOWN / needs verification.

### 3.1 Encodings

**EGA row-planar** — for each pixel row, `W/8` bytes of the **blue** plane,
then `W/8` bytes green, `W/8` red, `W/8` intensity. 16-wide tiles are thus
`BB GG RR II` (8 bytes/row), 8-wide tiles `B G R I` (4 bytes/row). Within
each byte the most significant bit is the leftmost pixel (the reference
viewer described in [S3] walks bits 7→0). Pixel value
`= I<<3 | R<<2 | G<<1 | B`, which is directly an index into the default EGA
16-colour palette [S12]. [S5] describes the same data as four 16-bit words
per row, first word = least-significant colour bit (= blue).
CONFIRMED (docs); byte order of the two bytes inside a 16-bit plane word
(left 8 pixels first) is implied by [S3] — needs verification.

**CGA linear** — 4 pixels/byte, bits 7-6 = leftmost pixel; rows
left-to-right, top-to-bottom [S3]. Palette: UNKNOWN which CGA palette the
game programs; [S3]'s viewer uses palette 1 low intensity
(black/cyan/magenta/grey), which the engine uses for now.

**Tandy (T1K) linear** — 2 pixels/byte, high nibble = left pixel [S3].
Palette assumed to be the standard 16-colour IRGB one (needs verification).

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
| 0x12 | Time machine | 0x31 | Unused (dog head) |

0x32–0x33: UNKNOWN (52 tiles stored, 50 documented). Whether the paired
tiles are animation frames or e.g. facing directions, and the animation
rate: UNKNOWN / needs verification (the viewer alternates them every
500 ms as a placeholder).

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

## 4. Overworld map — `MAP.BIN`

Sources: [S1] (quoting Johnny Wood's notes), [S4] (ModdingWiki), [S7]
(Ultima Stones).

* Headerless, uncompressed. **4 bits per cell, 2 cells per byte, high
  nibble first.** Rows stored left-to-right, top-to-bottom; 168 cells =
  **84 bytes per row**. CONFIRMED (docs).
  * [S1]'s quoted prose says "the low 4 bits represent the first tile" but
    its own example ("a value of `10` = grass `1` followed by water `0`")
    and [S4]'s reference viewer both mean **high nibble first**; we follow
    the example. Needs verification.
* **CONFLICT — dimensions:** [S1] and [S7] give **168 x 156** (which
  matches the 13,104-byte size: 84 x 156); [S4] gives 168 x 168 (would be
  14,112 bytes). The engine trusts the file size: height = len / 84, and
  warns if it is not 156. [S8]'s save notes give player Y range 0–153.
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
  location [S4]; that table lives in `OUT.EXE` and is UNKNOWN in byte form.
  It must be rebuilt from public maps/walkthroughs (e.g. [S1] Geography,
  [S14] dungeon names) by coordinates.
* The world contains the four lands (Lord British, Feudal Lords, Dark
  Unknown, Danger & Despair) [S14]. Whether movement wraps at the map edges:
  UNKNOWN / needs verification by observation.
* On-screen viewport in the original: 19 x 9 cells of 16x16 px
  (304 x 144 px) [S4].

## 5. Towns and castles — `TCD.BIN`

Source: Ultima Stones [S6b]; size from [S1].

* 10 maps x 38 (W) x 18 (H) cells, 1 byte per cell = 684 bytes/map,
  6,840 bytes total ✓. CONFIRMED (docs).
* **Column-major**: stored "top to bottom, left to right", i.e. 18 bytes for
  column x=0, then column x=1, … ("at a right angle to its screen
  representation") [S6b]. Needs verification.
* Map order: 0–1 = castle one and castle two, 2–9 = town 1…8 [S6b].
* Cell values index `???TOWN.BIN` (§3.3). Whether high bits carry flags,
  and whether NPC positions are in the map or placed by code: UNKNOWN.
* The game has more towns/castles than 10 maps (multiple per land), so maps
  are reused across locations; the location→map table is in `OUT.EXE`
  (UNKNOWN).

## 6. Dungeons — procedurally generated, no data file

Sources: [S13], [S14], [S15], [S16] (all describe observed behaviour).

* There is **no dungeon data file.** Dungeons are generated from a
  **per-character seed stored in the save file** (word at 0x32, see §7):
  same seed → same layouts; layouts are fixed for the life of a character
  and change with each new character [S8][S13][S15].
* Each dungeon has **10 levels**; monsters change every 2 levels [S13][S14].
* Each level is a **9 x 9 grid** [S13]: five parallel north–south corridors
  separated by four north–south walls; walls may contain doors or secret
  doors; corridors may contain force fields / walls; an up and a down ladder
  sit in the 2nd and 4th corridors (top floor excepted) [S14]. Ladders can
  be walled in, hence Ladder Up/Down spells [S13].
* The dungeon code is described as largely the same as Garriott's
  *Akalabeth* [S13].
* The exact PRNG and generation algorithm: **UNKNOWN**. A clean-room engine
  will need its own generator honouring the documented structure; byte-exact
  reproduction of a given seed's layout is not achievable from public docs.

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
  objects) cannot fit; we treat 820 as correct (0xB4 + 40x16 = 0x334 = 820 ✓).

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
| 0x2C | u16 | Ready spell | 0 prayer, 1–10 |
| 0x2E | u16 | Worn armour | 0 none, 1–5 |
| 0x30 | u16 | Transport | 0 foot, 1 horse, 2 cart, 3 raft, 4 frigate, 5 aircar (6 shuttle, 7 time machine per [S8b]) |
| 0x32 | u16 | **Dungeon seed** | [S8b]; "unknown" in [S8] — needs verification |
| 0x34 | u16 | Player X | 0–167 |
| 0x36 | u16 | Player Y | 0–153 per [S8b] |
| 0x38–0x4B | | Quest flags etc. | [S8b]: 8 x u16 quest states (0 none, 1 active, 0xFFFF done); layout vs [S8]'s "unknown" words needs verification |
| 0x4C–0x52 | u16 x4 | Gems red, green, blue, white | |
| 0x54 | u16 | 0xFFFF | list terminator? |
| 0x56–0x5E | u16 x5 | Armour qty: leather, chain, plate, vacuum suit, reflect suit | |
| 0x60 | u16 | 0xFFFF | |
| 0x62–0x7E | u16 x15 | Weapon qty: dagger, mace, axe, rope & spikes, sword, great sword, bow & arrows, amulet, wand, staff, triangle, pistol, light sword, phazor, blaster | |
| 0x80 | u16 | 0xFFFF | |
| 0x82–0x94 | u16 x10 | Spell qty: open, unlock, magic missile, steal, ladder down, ladder up, blink, create, destroy, kill | |
| 0x96 | u16 | 0xFFFF | |
| 0x98–0xA4 | u16 x7 | Vehicle qty: horse, cart, raft, frigate, aircar, shuttle, time machine | |
| 0xA6 | u16 | UNKNOWN | |
| 0xA8 | u16 | Last signpost visited | default 0xFFFF |
| 0xAA | u16 | UNKNOWN | |
| 0xAC | u16 | Steps/moves | |
| 0xAE–0xB2 | u16 x3 | UNKNOWN | [S8b] lists enemy ships destroyed, object count, total moves, fuel, shields somewhere here — needs verification |
| 0xB4–0x333 | 40 x 16 B | Map objects | below |

Map object (16 bytes): `+0 u16` object tile (index into §3.2) · `+2 u16`
unknown (matches type for vehicles, 1–2 for monsters [S8]; "map tile the
object covers" per [S8b]) · `+4 u16` X · `+6 u16` Y · `+8 u16` hits (0 for
vehicles) · `+0A..+0F` 3 x u16, always 0 so far.

## 8. Full-screen images

* `CASTLE.4` (16,384 B): 320x200 CGA, 2 bpp, "odd scanlines, padding, even
  scanlines, padding" — i.e. a raw dump of CGA video memory (two 8 KiB
  banks of 8,000 bytes + 192 bytes padding each) [S1]. Bank order
  (even/odd first) needs verification.
* `CASTLE.16` (32,000 B): 320x200 x 4 bpp = 32,000 ✓, but the encoding
  (packed nibbles vs. 4 planes of 8,000 bytes) is **UNKNOWN** — "to be
  determined" in [S1].
* `NIF.BIN` (6,720 B): end-game text as a 320x168 1-bpp bitmap
  (320/8 x 168 = 6,720 ✓) [S1]. Bit order assumed MSB-left (needs
  verification).

## 9. Game data that lives in code (behavioural spec only)

These tables are compiled into the executables; we do **not** extract them.
The engine will carry its own tables written from public game documentation
(manual, wikis, guides), with values marked unverified until matched by
observation:

* Monster HP/damage: Ultima Codex monster table [S16] (overland monsters,
  sea monsters, dungeon monsters per level pair).
* Weapons, armour, spells, vehicles: names/order are fixed by the save
  layout (§7); prices and effects from [S1]-family guides and the manual.
* Quests (Lord British, Lost King, Barataria, Rondorin, Olympus, Black
  Dragon, White Dragon, Shamino) and gem rewards: [S8b].
* Character creation stat bonuses (race/class): [S8b].
* Space sequence, Mondain battle, text/dialogue, signpost messages:
  UNKNOWN beyond observed behaviour.
* Sound: PC speaker effects generated by code; no sound data files.

## 10. What the engine implements today

| Area | Module | Tests |
|---|---|---|
| All tile encodings + documented file specs | `u1-assets::tiles` | handcrafted bytes, round-trip for all 15 files, size checks |
| MAP.BIN | `u1-assets::map` | nibble order, 84-byte stride, 168x156/168x168 handling |
| TCD.BIN | `u1-assets::tcd` | column-major addressing |
| PLAYERn.U1 (documented fields) | `u1-assets::save` | offsets, 0F 27 = 9999, object table |
| Game dir discovery (case-insensitive) | `u1-assets::gamedir` | synthetic files |
| Software renderer | `u1-assets::render` | colours, off-map |

All test data is synthetic, built from this spec.

## 11. Verification checklist (needs a legal copy)

1. Confirm file list/sizes/case in the GOG install (and whether `game.gog`
   must be extracted).
2. View EGATILES with `u1view` — water/grass/forest/mountain recognisable at
   indices 0–3 confirms planar order & bit order.
3. Confirm MAP.BIN is 13,104 bytes and the four continents look right
   (nibble order, 156 rows).
4. Dump TCD map 0 as text and confirm column-major order.
5. Confirm save offsets incl. 0x32 seed and 0xAE–0xB2.
6. Identify tiles 0x32–0x33 and animation timing.
7. Determine CASTLE.16 encoding and CGA palette.

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
