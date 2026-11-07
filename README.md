# ultima1-engine

**0.1.0-beta.1 — a desktop beta, not a final release.** Play *Ultima I:
The First Age of Darkness* using your own original DOS game files, with a
modern Rust/SDL2 engine. No installer or development tools are needed for
the downloads below.

This is an independent, unofficial project, not affiliated with or endorsed
by Origin Systems, Electronic Arts, Richard Garriott, or GOG. You must
supply your own legally obtained game files. **No original game files are
included in the repository or downloads.**

## Download and play

| Platform | Ready-to-run beta |
| --- | --- |
| Linux x86-64 | [AppImage](https://github.com/lilgoomba/ultima1-engine/releases/download/v0.1.0-beta.1/ultima1-engine-0.1.0-beta.1-x86_64.AppImage) |
| Windows x64 | [Portable ZIP](https://github.com/lilgoomba/ultima1-engine/releases/download/v0.1.0-beta.1/ultima1-engine-0.1.0-beta.1-windows-x86_64.zip) |

[Release notes and checksums](https://github.com/lilgoomba/ultima1-engine/releases/tag/v0.1.0-beta.1)
· [Playing guide](PLAYING.md) · [Beta testing scope / known limitations](docs/BETA.md)

### Windows: extract, add your files, double-click

1. Extract the **whole ZIP** to a folder you can write to (Documents,
   Desktop, or a USB drive). Do not run it inside the ZIP or install into
   `Program Files`.
2. Copy the files from your own Ultima I installation into the supplied
   **`ULTIMA1`** subfolder. Put `OUT.EXE` directly inside that folder, not
   inside another nested folder.
3. Double-click **`ultima1-engine.exe`**. No administrator rights or
   installer are required.

```text
ultima1-engine-0.1.0-beta.1-windows-x86_64/
  ultima1-engine.exe
  vcruntime140.dll
  portable.txt             ← keep this marker
  assets/enhanced/          ← keep the bundled assets
  ULTIMA1/
    OUT.EXE                ← your game files go here
    MAP.BIN
    ...
  config.toml              ← created when options are saved
  saves/                   ← your characters and save slots
```

With `portable.txt` present, **settings and saves stay beside the exe**, not
in AppData and not in the original game folder. Preserve `config.toml`,
`saves/`, and your `ULTIMA1/` when updating. The executable is unsigned;
Windows may show an unknown-publisher/SmartScreen warning. Only run a copy
you trust. Windows 10/11 x64 are the recommended beta targets.

### Linux: put the AppImage next to your ULTIMA1 folder

```text
Ultima1Engine/
  ultima1-engine-0.1.0-beta.1-x86_64.AppImage
  ULTIMA1/
    OUT.EXE
    MAP.BIN
    ...
```

Make the AppImage executable in your file manager, or run:

```sh
chmod +x ultima1-engine-0.1.0-beta.1-x86_64.AppImage
./ultima1-engine-0.1.0-beta.1-x86_64.AppImage
```

You can instead point it to an existing installation without copying it:

```sh
./ultima1-engine-0.1.0-beta.1-x86_64.AppImage --game-dir "/absolute/path/to/ULTIMA1"
```

Linux requires **x86-64 and glibc 2.35 or newer** (for example Ubuntu
22.04+ or Debian 12+). SDL2 is bundled. The AppImage runtime may need FUSE 2
(`libfuse2`, or `libfuse2t64` on Ubuntu 24.04). If FUSE is unavailable, try:

```sh
./ultima1-engine-0.1.0-beta.1-x86_64.AppImage --appimage-extract-and-run
```

If needed, extract with `--appimage-extract` and run `./squashfs-root/AppRun
--game-dir "/absolute/path/to/ULTIMA1"`. Linux saves normally live in
`~/.local/share/ultima1-engine/saves/` and settings in
`~/.config/ultima1-engine/config.toml` (XDG overrides apply). Neither is
stored inside the AppImage. Back up saves/settings before updating.

## Which original game files?

The target is **the MS-DOS port of the 1986 remake (DOS release 1987)**,
not the Apple II original or a fan remake. Local validation used the
*Ultima 1–3* CD-ROM DOS file set. The DOS edition sold by GOG is an intended
source of compatible files, but installer layout and differences from the
CD-ROM data have not been comprehensively verified; find the actual DOS
files in your own installation, not an installer or DOSBox shortcut.

The startup loader checks for these names (case-insensitive):

- `OUT.EXE`, `MAP.BIN`, `TCD.BIN`, `ULTIMA.EXE`, `GEN.EXE`
- One matching display pair: `EGATILES.BIN` + `EGATOWN.BIN`,
  `T1KTILES.BIN` + `T1KTOWN.BIN`, or `CGATILES.BIN` + `CGATOWN.BIN`

**For the full game, copy the complete DOS game folder**, not just that
minimum. Later scenes also use `SPACE.EXE`, `MONDAIN.EXE`, `NIF.BIN`,
`CASTLE.16` (or `CASTLE.4` for CGA) and the matching display's `*FIGHT.BIN`,
`*SPACE.BIN`, and `*MOND.BIN` files. Existing `PLAYER1.U1`–`PLAYER4.U1`
characters can be imported as copies on the first run; you can also create
a character. The engine reads the game data; normal saves use a separate
roster. You do not run the old DOS executables themselves.

If files are missing, the engine lists the folders searched and missing
names. See [the playing guide](PLAYING.md) for alternate game locations,
command-line overrides and troubleshooting. Please do not upload original
game files, extracted original art, ROM fonts, or original-game screenshots
to this repository or issues.

## Controls and enhancements

- **Arrows** move. Original letter commands remain; **F1** opens options.
- **Q outdoors** saves and quits. **Escape at a gameplay command prompt**
  quits without saving, like closing the window. Confirm quit is **on by
  default**, including just after loading/saving: **Y** exits; **N**, Enter
  or a fresh Escape cancels. In menus/questions, Escape keeps its normal
  close/cancel behavior.
- With save slots enabled: **F2** slots, **F5** quicksave, **F9** quickload.
- **F3** command help, **Tab** maps and **PgUp** message log depend on their
  options being enabled. [Full controls](PLAYING.md#4-keys).

The engine supports the original CGA/EGA/Tandy data plus optional enhanced
art, synthesized effects and original music. F1 exposes the existing
settings; enhancements are not all enabled by default. Music starts off;
God mode is a clearly labeled testing cheat and starts off. See the
[playing guide](PLAYING.md) for defaults, space-combat options and details.
This is not a claim of perfect DOS fidelity or a bug-free final release.

## Beta feedback and development

Beta work focuses on fixes and regressions. When reporting a bug, include
version, OS, steps and relevant F1 settings. See [BETA.md](docs/BETA.md) for
what was tested and remaining gaps. **macOS is not a beta target; Android
work is parked.** Old review branches are retained, not separate supported
releases.

For source builds, architecture and optional developer tools:
[Developer guide](docs/DEVELOPING.md). The [behavior specification](docs/SPEC.md)
and [roadmap](docs/ROADMAP.md) retain research and historical progress.
Reference study is not strict clean-room development; original executable
listings and data stay outside the repository and runtime tables/text are
read from the user's copy. See the developer policy before contributing.

## License and credits

Engine code: [GPL-3.0-or-later](LICENSE). Enhanced art/effects and original
music scores: [CC BY-SA 4.0](assets/enhanced/LICENSE). See [CREDITS](CREDITS),
[font provenance](crates/u1-assets/fonts/README.md), and
[third-party notices](THIRD_PARTY.md) for separate components and licenses.
Ultima names and original content belong to their respective owners;
these project licenses do not grant rights to the original game.
