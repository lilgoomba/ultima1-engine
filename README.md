# ultima1-engine

An open-source, **clean-room** re-implementation engine for Origin Systems'
*Ultima I: The First Age of Darkness* (the MS-DOS port of the 1986 remake,
as sold by GOG.com). Written in Rust with SDL2. Linux first.

**Status: early.** Today it has:

* `u1-assets`: a library that parses the documented DOS data formats:
  * every tile file (EGA row-planar, CGA, Tandy and mono)
  * `MAP.BIN` (the overworld)
  * `TCD.BIN` (towns and castles)
  * `PLAYERn.U1` saves
  * case-insensitive game-folder discovery
  * a small software renderer
* `u1view`: a scrollable overworld viewer, either for your own game files
  or for a fully synthetic `--demo` world.
* [`docs/SPEC.md`](docs/SPEC.md): the file-format spec, with sources. It
  marks what is confirmed by public docs and what is still unknown.

![demo screenshot (procedural placeholder art, not Origin's)](docs/demo.png)

*The screenshot uses procedurally generated placeholder tiles and a
synthetic map. It contains no original game art.*

## Clean-room policy

This project must stay legally clean. Contributors agree to the following:

1. **Use only public documentation.** Format knowledge comes from public
   community documentation (ModdingWiki, Ultima Stones, Dino's U1 guide,
   the Ultima Codex and similar) and from observing the game while it
   runs. Every claim is cited in `docs/SPEC.md`.
2. **No reverse engineering of the executables.** Do not disassemble,
   decompile or debug Origin's executables. Do not paste code or tables
   taken from them. Do not use projects that say they derive from
   decompilation (for example Ultima I Unity) as a source.
3. **Do not copy third-party code verbatim.** Descriptions in other
   projects' docs are fine. Their code is not.
4. **Never commit original game files.** That includes `.BIN`, `.EXE`,
   `.U1`, `CASTLE.*` and any data or screenshots taken from them. Do not
   use abandonware downloads. `.gitignore` blocks the usual extensions as a
   safety net.
5. **Use synthetic test data only.** Tests build their inputs from the
   spec.

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

### With your own copy (GOG)

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

Options: `--scale N`, `--cols N`, `--rows N`, `--start X,Y`, `--seed N`
(demo only), `--mode ega|t1k|cga`, and `--screenshot out.png`. The
screenshot option renders one frame and exits. It also works headless:

```sh
SDL_VIDEODRIVER=dummy cargo run -p u1view -- --demo --start 100,104 --screenshot docs/demo.png
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
crates/u1-assets   asset parsing library (no dependencies)
crates/u1view      SDL2 overworld viewer
docs/SPEC.md       data-format specification with sources
docs/demo.png      demo screenshot (synthetic art)
```

## Roadmap

1. Verify the spec against a real copy (see the checklist in SPEC §11).
2. Town/castle viewer from `TCD.BIN` and `EGATOWN.BIN`.
3. Save-file inspector.
4. Movement, transports and the overworld rules.
5. A dungeon generator that follows the documented 9x9 / 10-level
   structure.
6. Combat, shops, space and the Mondain battle, rebuilt from documented
   behaviour.

## License

GPL-3.0-or-later. See [LICENSE](LICENSE).
*Ultima* is a trademark of Electronic Arts. This project is not affiliated
with or endorsed by Electronic Arts or GOG. It ships no game content.
