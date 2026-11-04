# Playing Ultima I with ultima1-engine (Linux AppImage)

ultima1-engine plays *Ultima I* (the 1986 MS-DOS release) from **your own
copy** of the game: the CD-ROM *Ultima 1-3* collection or the GOG.com
edition. The AppImage contains the engine, SDL2 and our own CC-BY-SA
enhanced art and sounds, but **no game files**.

## 1. Download

Take `ultima1-engine-<version>-x86_64.AppImage` from the
[Releases page](https://github.com/lilgoomba/ultima1-engine/releases) and
make it executable:

```sh
chmod +x ultima1-engine-*-x86_64.AppImage
./ultima1-engine-*-x86_64.AppImage
```

It is built on Ubuntu 22.04 and runs on x86-64 distributions with glibc
2.35 or newer (Ubuntu 22.04+, Debian 12+, Fedora 36+, and so on). AppImages
need FUSE 2 (`libfuse2`, called `libfuse2t64` on Ubuntu 24.04). Without it,
run it as `./ultima1-engine-*.AppImage --appimage-extract-and-run` or set
`APPIMAGE_EXTRACT_AND_RUN=1`.

## 2. Point it at your game files

The engine needs these files from your copy (any case of the name):
`OUT.EXE`, `MAP.BIN`, `TCD.BIN`, `ULTIMA.EXE`, `GEN.EXE` and one display's
tiles (`EGATILES.BIN` + `EGATOWN.BIN`, or the `T1K` or `CGA` pair). The rest
(`SPACE.EXE`, `MONDAIN.EXE`, `NIF.BIN`, `CASTLE.16`/`CASTLE.4`, the
`*FIGHT`, `*SPACE` and `*MOND` tiles) are used when present; copy the whole
folder to be safe. On the CD and in GOG installs they are in a folder
called `ULTIMA1`.

Any one of these works. Without `--game-dir` the places are tried in this
order and the first complete one wins:

1. `--game-dir /path/to/ULTIMA1` on the command line.
2. `$U1_GAME_DIR`.
3. Put the AppImage **in the game folder, or next to the `ULTIMA1`
   folder** (each place is also searched for an `ULTIMA1`/`U1` folder
   inside it).
4. The folder you start it from.
5. `~/.local/share/ultima1-engine/game` (`$XDG_DATA_HOME/ultima1-engine/game`),
   e.g. `mkdir -p ~/.local/share/ultima1-engine/game && cp /path/to/ULTIMA1/* ~/.local/share/ultima1-engine/game/`.

If no place has the files, the engine says which places it tried and which
files each one lacks (in the terminal, and in a message window when
started from a file manager).

The game folder is only ever read.

## 3. Starting

With no options the game starts like the original: the title sequence,
then the character menu (create a character, or continue one). Other
starts: `--player N` (character N of your roster, saved back there;
the game's own `PLAYERn.U1` the first time), `--save FILE` (saved
back to FILE; to the roster when FILE is one of the game's files), `--gen`
(straight to the character menu). `--help` lists every option.

## 4. Keys

The original's keys all work as they did (arrows move; A Attack, B Board,
C Cast, D Drop, E Enter, F Fire, G Get, I Inform, K Klimb, N Noise, O Open,
Q Quit and save, R Ready, S Steal, T Transact, U Unlock, X eXit, Z Ztats;
Space passes; the README has the full table). The engine adds keys the original
never reads:

| Key | What it does |
|---|---|
| F1 | Options, in five pages (Video, Sound, Gameplay, Controls, Interface): Tab or PgUp/PgDn turn pages, arrows change, R resets the page, O resets everything. Each setting is explained under the list and applies at once. Everything optional is **off** by default, except save slots, the quit confirmation and (enhanced video only) vehicle turning. The game is paused while it is open. |
| F2 | Save slots (on by default; slot 1 is your character's own file, the one Q writes) |
| F3 | Command help screen (turn on "Help screen" in F1) |
| F4 | Turbo: every wait 4x shorter (always available) |
| F5 / F9 | Quicksave / quickload |
| Tab | Dungeon automap / world map (turn on the maps in F1) |
| PgUp / PgDn | Message log (turn on "Message log" in F1) |
| Z | Character screen instead of the original Ztats (turn on "Character" in F1) |
| F12 | Screenshot, saved to `~/Pictures/ultima1-engine/` (or `~/.local/share/ultima1-engine/screenshots/`); change the key in F1 → Controls → Keys (last row) |
| Alt+Enter | Fullscreen on/off |

A sign post's or pillar's words stay up until you press a key (F1,
Gameplay: Sign wait; on by default, off for the original's short pause).

Docking at a space station costs 500 coin to refuel, as in the original;
F1, Gameplay: Docking fees Off refuels free. Either way, ready a vacuum
or reflect suit before docking, or the open space doors kill you.
At "Choose thy ship:" type the port number, or press the arrow toward
the ship you want (it shows in yellow) and Enter, Space or pad A.

The game also pauses when its window loses focus. Closing the window with
progress not yet saved asks first (F1, Interface: Confirm quit). The
window's size and place are remembered.

In dungeons, as in the original, G (Get) does nothing (it answers "Get?").
Step onto a chest's square and press U (Unlock) to open it (it may be
trapped); a coffin is opened with O. The name of what you can see ahead
("Chest") is printed in the message window each time the view is drawn.

The enhanced mode (F1, Video: Enhanced) draws our own CC-BY-SA art over
the same game; the original EGA picture is the default. There, vehicles
face left or right the way they last moved sideways (F1, Video: Vehicle
turn; up and down keep the facing, and a vehicle you leave keeps its
facing, saved beside your save in its `.map` file).

## 5. Where saves and settings live

* **Saves and characters**: `~/.local/share/ultima1-engine/saves/`
  (`$XDG_DATA_HOME/ultima1-engine/saves`). The roster files
  (`PLAYER1.U1` ... `PLAYER4.U1`) are original 820-byte records; save slots
  and maps sit beside them, and each save made in the engine gets a small
  `.info` file (date, place, thumbnail for the slot screen) - the save itself
  stays an original record. The first time, the characters that came with
  the game are copied there. Saves never go into the game folder, so your
  original files are never changed and a read-only game folder (a CD) is
  fine. `--save-to FILE` or `--roster DIR` choose another place.
* **Settings**: `~/.config/ultima1-engine/config.toml`
  (`$XDG_CONFIG_HOME/ultima1-engine/config.toml`), written by the F1 menu.

## 6. Bugs

Please report problems as [GitHub issues](https://github.com/lilgoomba/ultima1-engine/issues)
(the repository is private: ask for access if you cannot open it). Say
which release, your distribution, what you did and what happened; the
terminal output (start the AppImage from a terminal) and a screenshot
help. Please do not attach game files or saves made from them.
