# ultima1-engine

An open-source re-implementation engine for Origin Systems' *Ultima I: The
First Age of Darkness* (the MS-DOS port of the 1986 remake, as sold on the
*Ultima 1-3* CD-ROM and by GOG.com). It is written independently in Rust
with SDL2, from a written specification. Linux first.

**Status: early.** Today it has:

* `u1-assets`: a library that parses the DOS data formats, verified
  against an original copy:
  * every tile file (EGA row-planar, CGA, Tandy and mono)
  * `MAP.BIN` (the overworld)
  * `TCD.BIN` (towns and castles)
  * `PLAYERn.U1` saves
  * the `CASTLE.16` / `CASTLE.4` title pictures and `NIF.BIN`
  * the location table, read at run time from your own `OUT.EXE`
  * the game's random number generator and its dungeon-level generator
  * the outdoor rules (movement, monsters, food, prices, shop stock),
    with their small tables read at run time from your own `OUT.EXE`
  * case-insensitive game-folder discovery
  * a small software renderer with the original animation timing
* `u1`: a playable overworld for your own game files. It has the
  original screen layout, movement and transports, food, wandering
  monsters, edge wrap and location names; entering places is still a
  stub.
* `u1view`: a scrollable overworld viewer, either for your own game files
  or for a fully synthetic `--demo` world.
* [`docs/SPEC.md`](docs/SPEC.md): the format and behaviour spec, with
  sources. Each item is marked as verified against original data,
  verified from the original executable, documented publicly, or unknown.

![demo screenshot (procedural placeholder art, not Origin's)](docs/demo.png)

*The screenshot uses procedurally generated placeholder tiles and a
synthetic map. It contains no original game art.*

## Development policy: spec from reference, independent implementation

This is **not** a strict clean-room project. It works like this:

1. **Reference study stays outside the repository.** Maintainers may study
   a legally owned copy of the original game, both its data files and its
   executables (`ULTIMA.EXE`, `GEN.EXE`, `OUT.EXE`, `SPACE.EXE`,
   `MONDAIN.EXE`, `MOSLO.COM`). Study methods include running it in
   DOSBox, diffing save files, and disassembling or decompiling the
   executables, but only **for reference**. Disassembly listings,
   decompiled code, memory dumps and raw notes stay on the maintainer's
   machine and are never committed.
2. **Findings become a plain-language spec.** What was learned is restated
   in [`docs/SPEC.md`](docs/SPEC.md) in our own words: layouts, rules,
   and algorithms in prose or pseudocode, plus a few example values. Each
   item says how it was established ("verified against original data",
   "verified from original executable", public documentation, or
   unknown). Public community documentation is cited there as well.
3. **The engine is written fresh from the spec.** No original code is
   translated or pasted in. Do not copy decompiled code or large data
   tables from the executables into the repository. When the engine needs
   such a table (for example the location coordinates, monster stats,
   food rates or message strings), it reads it at run time from the
   user's own copy (`u1_assets::rules::ExeTables`, `::locations`).
4. **No original assets in the repository.** Never commit original game
   files (`.BIN`, `.EXE`, `.COM`, `.U1`, `CASTLE.*`, `NIF.BIN`), extracts
   from them, or screenshots that show original art. `.gitignore` blocks
   the usual extensions as a safety net. Do not use abandonware
   downloads.
5. **Third-party code is not copied.** Other projects' descriptions may be
   cited; their code may not be copied.
6. **Tests use synthetic data only.** Tests build their inputs from the
   spec and never read original files. One optional check runs against
   your own install when `U1_GAME_DIR` is set.

To run the engine with real data you need your own legally purchased copy.

## Building on Linux

Debian/Ubuntu:

```sh
sudo apt install build-essential pkg-config libsdl2-dev
# Rust toolchain: either the distro package...
sudo apt install cargo rustc          # Debian 13 ships Rust 1.85
# ...or rustup (https://rustup.rs):
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

cargo build --release
cargo test
```

Fedora: `sudo dnf install SDL2-devel gcc pkgconf`.
Arch: `sudo pacman -S sdl2 base-devel`.

The minimum Rust version is 1.75. The project is developed and tested
with 1.85.1.

## Running

### Demo mode (no game files needed)

```sh
cargo run --release -p u1view -- --demo
cargo run --release -p u1view -- --demo --seed 42 --scale 4
```

### With your own copy (CD-ROM or GOG)

From the *Ultima 1-3* CD-ROM, point `--game-dir` at the `ULTIMA1` folder.
For GOG:

1. Install *Ultima 1+2+3* from GOG.
2. Find the Ultima I folder, for example `~/GOG Games/Ultima 1+2+3/Ultima1`
   or the folder where you unpacked the Windows installer with
   `innoextract`.
3. Look for `MAP.BIN` and `EGATILES.BIN` in that folder. If you only see a
   `game.gog` file, it is a disc image, and you need to extract it first:
   `7z x game.gog -ogame-files`. This step is unverified for U1; see
   SPEC §1.
4. Run the viewer:

```sh
cargo run --release -p u1view -- --game-dir "/path/to/Ultima1"
cargo run --release -p u1view -- --game-dir "/path/to/Ultima1" --mode cga
```

`u1view` looks for files case-insensitively, in the folder itself and in
`ULTIMA1/` or `U1/` subfolders. It picks the best available tile set:
EGA first, then Tandy, then CGA.

### Controls and options

| Key | Action |
|---|---|
| Arrow keys | Scroll 1 cell (hold Shift to scroll 8 cells) |
| Home | Recentre |
| Esc | Quit |

Options:

* `--scale N`, `--cols N`, `--rows N`
* `--start X,Y`
* `--seed N` (demo only)
* `--mode ega|t1k|cga`
* `--screenshot out.png`: renders one frame and exits.
* `--full-map out.png`: renders the whole 168x156 world to one image and
  exits.
* `--frame N`: the animation tick to render with `--screenshot`. One tick
  is about 0.165 s; water scrolls each tick, castles change every 3 ticks
  and towns every 4.
* `--locations` (needs `--game-dir`): prints the 84-entry location table
  read from your `OUT.EXE` and checks it against `MAP.BIN`.
* `--dungeon X,Y,LEVEL,SEED` (needs `--game-dir`): prints an ASCII map of
  the dungeon level the original generator would build for the entrance
  at X,Y. SEED is save word 0x32.

Screenshots also work headless:

```sh
SDL_VIDEODRIVER=dummy cargo run -p u1view -- --demo --start 100,104 --screenshot docs/demo.png
```

### Playing the game: `u1`

```sh
cargo run --release -p u1 -- --game-dir "/path/to/Ultima1" --player 2
```

`u1` needs your own game folder. It reads `MAP.BIN`, `TCD.BIN`, the tile
sets, `OUT.EXE` (for its tables and texts) and a save. The game folder is
never written. Without `--player`/`--save` it starts a fresh character
at the default start, (49,40). It covers the overworld, towns and castles
(shops, pub, kings, jail and princess) and the ten-level dungeons with
their first-person view. Space and the time machine stop after their
entry message.

The keys are the original's:

| Key | Action |
|---|---|
| Arrow keys or keypad 8/2/4/6 | Move north/south/west/east (in a dungeon: forward, turn around, turn left, turn right) |
| Space | Pass |
| A | Attack (then a direction) |
| B / X | Board / exit a vehicle |
| C | Cast the ready spell |
| D | Drop (coin into a pond, in towns) |
| E | Enter a town, castle, dungeon or landmark |
| F | Fire (vehicles with cannons) |
| G | Get (castle storeroom, with the king's leave) |
| I / Z | Inform (in a dungeon: search for secret doors) / Ztats |
| K | Klimb a dungeon ladder (face north or south) |
| N | Noise on/off |
| O | Open a coffin (dungeon) |
| Q | Quit and save (outdoors only) |
| R | Ready a weapon, armour or spell |
| S | Steal (behind a shop counter) |
| T | Transact (shops, pub, king) |
| U | Unlock (castle jail doors, dungeon chests) |

If no key is pressed for 1,500 polls (about 3¼ minutes), the turn
passes by itself, as in the original. Options:

* `--player N` or `--save FILE` to load a save; `--pos X,Y` to override
  the position.
* `--save-to FILE`: where `Q` writes the 820-byte record. The default is
  `./PLAYER1.U1`. It must not be inside the game folder.
* `--mode ega|t1k|cga`, `--scale N` (1–6, default 3).
* Like the original, the random generator is seeded from the save's seed
  word plus the time of day, and dungeon items are reseeded from the clock
  on each level. For reproducible runs (tests, screenshots) the hidden
  option `--seed N` fixes both the generator seed and the clock value.
* `--screenshot out.png` renders one 640×400 frame and exits.
  `--keys TEXT` types those keys first. Use `{up}` `{down}` `{left}`
  `{right}` `{enter}` `{esc}` `{bs}` `{space}` for special keys and `~`
  to end a timed wait. `--counter N` sets the animation counter.
* `--video enhanced [--pack DIR]`: the optional enhanced video mode
  (below). `--video original` is the default.
* `--sound original|enhanced|mute`: the original PC-speaker beeps (the
  default), the optional enhanced sounds (below), or nothing. `--mute` is
  the same as `--sound mute`.

```sh
SDL_VIDEODRIVER=dummy cargo run -p u1 -- --game-dir "/path/to/Ultima1" \
    --player 2 --seed 2 --keys "{up}{up}e" --screenshot /tmp/u1.png
```

### Enhanced video mode (optional)

The faithful 320×200 picture stays the default. `--video enhanced` draws
the same game at a higher internal resolution and paints the art of an
*asset pack* over each original tile it recognises:

```sh
cargo run --release -p u1 -- --game-dir "/path/to/Ultima1" --video enhanced
cargo run --release -p u1 -- --game-dir "/path/to/Ultima1" --video enhanced --pack my-pack/
```

* The default pack is `assets/enhanced/` (looked up in the current
  directory, then in the source tree). Anything it has no picture for is
  the original art scaled up nearest-neighbour.
* A pack is a directory with `manifest.toml` and PNG files. The manifest
  declares `tile_size` (pixels per original 16-pixel tile: 32 gives a
  640×400 internal screen, 64 gives 1280×800), and maps tile ids
  (`world.N`, `town.N`, `mond.N`, `screen.nif`, `dungeon.PART`,
  `dmon.N`) to a `file` or a list of
  animation `frames`, with optional `scroll` (water) and `fps`, or to
  `variants` (alternatives chosen per map cell by a fixed hash, so large
  grass fields do not repeat). An
  optional `[palette]` table holds free-form notes per video mode. See
  SPEC §20 and the docs at the top of `crates/u1-assets/src/enhanced.rs`.
* In dungeons the first-person view keeps the original geometry but is
  redrawn with the pack's wall, door, floor and ceiling textures (shaded
  by depth) and its object and monster sprites, if the pack has
  `dungeon.wall`. Secret doors look like walls, as in the original.
* Any id the pack does not list, and any tile the game has drawn over
  (text, cursor, effects), falls back to the original picture. Game
  logic, screen layout and the viewport are unchanged.
* The window is always a whole multiple of the internal resolution.
  `--screenshot` writes the internal resolution.

Pack art in this repository is original work, not derived from Origin's
tiles, and is licensed CC-BY-SA 4.0 (see `assets/enhanced/LICENSE`); the
engine code remains GPL-3.0-or-later.

The current pack (batch 1) covers the overworld: water (four frames),
grass (four variants), forest, mountains (three variants), the castle
(two pennant frames), the town (two smoke frames), the signpost, the
dungeon entrance and the player on foot, as 32×32 tiles in a
16-bit-console style with one shared 64-colour palette. Grass and
mountain variants share an identical edge band, so any mix of them
tiles seamlessly; the engine picks one per map cell. The pictures were
made with an image generator from text descriptions only (no original
tiles as input), then cut to the pixel grid, made seamless, given a
shared palette and hand-finished (pennants, smoke, tufts, the
hand-placed town cottages) with Python/Pillow.

Batch 2 covers towns and castles: the castle wall, the
cobbled floor, water and its six shore shapes, bush, tree, the six
counter pieces, the grey brick wall and the people (guard, player,
jester, king, merchant, princess, wench) as 16×16 pictures; the letters
of shop signs stay original. It also covers the dungeon view: wall,
door, floor and ceiling textures, ladders, ceiling and floor holes,
chest, coffin, an animated force field and sprites for every monster but
the invisible seeker (the mimic uses the chest). Secret doors use the
wall on purpose. Town pictures were drawn procedurally or by hand; the
dungeon textures and monsters were made like batch 1 (generated from
text only, cut out, outlined and given their own 64-colour palette).

Batch 3 covers space flight: a riveted gunmetal frame for
the top-down view and a cockpit frame for the front view, a starfield
and a nebula behind them, the earth-like planet, the sun, the space
station, the three player ships, the enemy fighter and the crosshair.
The sector scan, shots, exhaust and explosions stay original.
It also covers Mondain's castle and the endgame: the stone floor and red
walls, the time machine, Mondain (two frames), his corpse, his gem and
its four glow frames, the demon he becomes (three frames), the six
frames of his fire spell, and a dim sunrise backdrop behind the closing
text. The text boxes, the victory box and the title castle stay
original. Everything else falls back to the original art.

### Enhanced sound mode (optional)

The original PC-speaker effects stay the default. `--sound enhanced`
plays the pack's sounds instead, one per game event:

```sh
cargo run --release -p u1 -- --game-dir "/path/to/Ultima1" --sound enhanced
cargo run --release -p u1 -- --sound-demo /tmp/demo.wav   # every sound after its original beep
```

* Sounds are `sounds/<event>.wav` in the pack folder (PCM WAV, any rate;
  mixed to mono 44.1 kHz by the engine through SDL audio, so they can
  overlap). Events are finer than the original's ten effects: a hit, a
  kill and taking damage share effect 2 in the original but have their
  own sounds here. The list is in `assets/enhanced/sounds/CREDITS`.
* An event with no file plays the original effect. A few events that are
  silent in the original (a miss, a dodged blow, launching, docking, the
  hyperspace jump) only sound in this mode, kept quiet and short.
* Levels match the beeps: each sound is about as loud as the effect it
  replaces; footsteps and the star field are soft; townspeople's steps
  are heard only within 4 squares of you (the original beeps for every
  one, and still does in the default mode).
* The game's timing is unchanged: the engine still waits as long as the
  original effect would have lasted; a longer enhanced sound plays on.
  The in-game Noise command (N) still turns all sound off and on.
* There is no music.

Every sound in `assets/enhanced/sounds/` was synthesized for this project
by a script (oscillators and noise; no recordings or samples, and no
original game audio) and is licensed CC-BY-SA 4.0 like the art.

Tests that use your own files are skipped unless you point them at your
install:

```sh
U1_GAME_DIR="/path/to/Ultima1" cargo test --workspace
```

### Developer helpers

```sh
# ASCII overview of a map (demo or real file)
cargo run -p u1-assets --example ascii_map -- --demo 1
cargo run -p u1-assets --example ascii_map -- /path/to/MAP.BIN
# Write a synthetic game dir (placeholder art) to test --game-dir
cargo run -p u1-assets --example write_synthetic_gamedir -- /tmp/synthgame
```

## Layout

```
crates/u1-assets   asset parsing and game-rules library (no dependencies)
crates/u1          SDL2 playable overworld
crates/u1view      SDL2 overworld viewer
docs/SPEC.md       data-format specification with sources
assets/enhanced    default asset pack for --video enhanced and --sound enhanced
docs/demo.png      demo screenshot (synthetic art)
```

## Roadmap

1. Finish the remaining open items in SPEC §11.
2. Space and the Mondain battle.
3. Optional quality-of-life features, kept apart from the faithful mode.
4. Enhanced video mode art: the title screens and an optional enhanced sound mode.

## License

GPL-3.0-or-later. See [LICENSE](LICENSE).
*Ultima* is a trademark of Electronic Arts. This project is not affiliated
with or endorsed by Electronic Arts or GOG. It ships no game content.
