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
  directory, then in the source tree). On `main` it is empty, so every
  tile is the original art scaled up nearest-neighbour.
* A pack is a directory with `manifest.toml` and PNG files. The manifest
  declares `tile_size` (pixels per original 16-pixel tile: 32 gives a
  640×400 internal screen, 64 gives 1280×800), and maps tile ids
  (`world.N`, `town.N`, `mond.N`, `screen.nif`) to a `file` or a list of
  animation `frames`, with optional `scroll` (water) and `fps`, or to
  `variants` (alternatives chosen per map cell by a fixed hash, so large
  grass fields do not repeat). An
  optional `[palette]` table holds free-form notes per video mode. See
  SPEC §20 and the docs at the top of `crates/u1-assets/src/enhanced.rs`.
* Any id the pack does not list, and any tile the game has drawn over
  (text, cursor, effects), falls back to the original picture. Game
  logic, screen layout and the viewport are unchanged.
* The window is always a whole multiple of the internal resolution.
  `--screenshot` writes the internal resolution.

Pack art in this repository is original work, not derived from Origin's
tiles, and is licensed CC-BY-SA 4.0 (see `assets/enhanced/LICENSE` where
present); the engine code remains GPL-3.0-or-later.

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
assets/enhanced    default asset pack for --video enhanced
docs/demo.png      demo screenshot (synthetic art)
```

## Roadmap

1. Finish the remaining open items in SPEC §11.
2. Space and the Mondain battle.
3. Optional quality-of-life features, kept apart from the faithful mode.
4. Enhanced video mode art (first batch under review on the
   `enhanced-art-review` branch).

## License

GPL-3.0-or-later. See [LICENSE](LICENSE).
*Ultima* is a trademark of Electronic Arts. This project is not affiliated
with or endorsed by Electronic Arts or GOG. It ships no game content.
