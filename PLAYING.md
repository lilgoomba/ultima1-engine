# Playing Ultima I with ultima1-engine

**0.1.0-beta.1**, not a final release. This unofficial engine uses your own
MS-DOS Ultima I remake files (the tested CD-ROM DOS set). No original game
files are distributed. See the [README](README.md) for supported files,
ready-to-run downloads, and compatibility limits.

## 1. Download

Use the [beta release](https://github.com/lilgoomba/ultima1-engine/releases/tag/v0.1.0-beta.1):

- **Windows x64:** extract the entire portable ZIP to a writable folder,
  copy your own game files into `ULTIMA1/`, and double-click
  `ultima1-engine.exe`. Keep the DLL, assets and `portable.txt` beside it.
  Settings (`config.toml`) and `saves/` stay beside the program. No installer
  or admin access is needed; the unsigned exe may trigger SmartScreen.
- **Linux x86-64:** make the AppImage executable and run it next to your
  `ULTIMA1/` folder, or pass `--game-dir`. Requires glibc 2.35+. SDL2 is
  bundled; if FUSE 2 is unavailable, use `--appimage-extract-and-run`.
  See [Linux setup](README.md#linux-put-the-appimage-next-to-your-ultima1-folder)
  for commands, extraction and dependencies.

Back up settings/saves before updates. For existing Windows portable
installs, preserve `config.toml`, `saves/` and your own `ULTIMA1/` folder.

## 2. Point it at your game files

The engine needs these files from your copy (any case of the name):
`OUT.EXE`, `MAP.BIN`, `TCD.BIN`, `ULTIMA.EXE`, `GEN.EXE` and one display's
tiles (`EGATILES.BIN` + `EGATOWN.BIN`, or the `T1K` or `CGA` pair). The rest
(`SPACE.EXE`, `MONDAIN.EXE`, `NIF.BIN`, `CASTLE.16`/`CASTLE.4`, the
`*FIGHT`, `*SPACE` and `*MOND` tiles) are used when present; copy the whole
folder to be safe. The tested CD-ROM set is in `ULTIMA1`. GOG installation layouts may vary;
locate the actual DOS files, not the shortcut or installer.

For a normal Linux run the places below are tried in order. `--game-dir`
explicitly selects a folder; otherwise the first complete one wins. Windows
portable mode uses only `U1_GAME_DIR` (if set) and the exe folder / its
`ULTIMA1` subfolder, not the working folder or per-user game directory:

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
| F1 | Options, in six pages (Video, Sound, Gameplay, Controls, Space, Interface): Tab or PgUp/PgDn turn pages, arrows change, R resets the page, O resets everything. Each setting is explained under the list and applies at once. Everything optional is **off** by default, except save slots, the quit confirmation and (enhanced video only) vehicle turning. The game is paused while it is open. |
| F2 | Save slots (on by default; slot 1 is your character's own file, the one Q writes) |
| F3 | Command help screen (turn on "Help screen" in F1); in space it lists the space commands |
| F4 | Turbo: every wait 4x shorter (always available) |
| F5 / F9 | Quicksave / quickload |
| Tab | Dungeon automap / world map (turn on the maps in F1) |
| PgUp / PgDn | Message log (turn on "Message log" in F1) |
| Z | Character screen instead of the original Ztats (turn on "Character" in F1) |
| F12 | Screenshot, saved to `~/Pictures/ultima1-engine/` (or `~/.local/share/ultima1-engine/screenshots/`); change the key in F1 → Controls → Keys (last row) |
| Alt+Enter | Fullscreen on/off |

A sign post's or pillar's words, and the princess rescue reward text,
stay up until you press a key (F1, Gameplay: Message pauses; on by
default, off for the original's short pause). Pubs, shops and the time
machine are unchanged.

Docking at a space station costs 500 coin to refuel, as in the original;
F1, Gameplay: Docking fees Off refuels free. Either way, ready a vacuum
or reflect suit before docking, or the open space doors kill you
(that story death still ends the flight even with God mode on).
F1, Controls: God mode (off by default; also `--god`) is a testing cheat:
Hits never fall, and any successful hit of yours kills at once. It is not
stored in the save.
At "Choose thy ship:" type the port number, or press the arrow toward
the ship you want (it shows in yellow) and Enter, Space or pad A.

In the space front view an arrow sets the sights drifting until another
arrow or Space, as in the original. F1, Space: Steady aim keeps that
drift, but pressing the opposite direction stops free (no fuel); after
you release, press again to go that way (usual 2 fuel). Enemy
speed Half keeps fighters in view longer; it does not change your aim,
lasers or damage. Space map Enhanced makes I show a labelled galaxy
grid with a legend; Original keeps the classic scan. Hit box (off by
default) slightly enlarges the fighter hit rectangle (+2 px; damage
unchanged). Ace need is 20, 10 or 5 (default 20) for the Space Ace
award, kill HUD and princess time-machine reward. Kill counter (off by default) shows `K:n/N` on
the space status panel, or `Ace!` once you have reached the need.

The game also pauses when its window loses focus. With Confirm quit on
(the default, F1 → Interface), closing the window during gameplay always
asks first, even just after loading or saving. Escape at the command prompt
(outdoors, towns and castles, dungeons, space and Mondain's lair) quits the
same way: Y quits, N, Enter or a fresh Escape keeps playing. Nothing is
saved by either method; Q (Quit) outdoors still saves. With Confirm quit
off, both methods close at once. A held/repeated key or a key already queued
when the question opens cannot answer it. In windows, the title and
character menus, and a command's own questions, Escape keeps its usual
meaning (close / cancel). The window's size and place are remembered.

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
* **Portable mode** (the Windows portable ZIP, or `--portable`): when a
  `portable.txt` file sits beside the program, settings are `config.toml`
  and saves are `saves/` in the program's own folder (not the current
  folder), the enhanced assets are its `assets/enhanced`, and the game files
  are looked for only in that folder or its `ULTIMA1` folder (`--game-dir`
  and `$U1_GAME_DIR` still work). The game files are still only read.

## 6. Bugs

Please report problems as [GitHub issues](https://github.com/lilgoomba/ultima1-engine/issues)
Say which release, OS/distribution, settings, what you did and what
happened. Terminal output (start the AppImage from a terminal) can help;
review logs for personal details before posting. Do not attach original
game files, extracted assets, ROM fonts, original-art screenshots or
saves made from the supplied game data.

First title (Ultima I / The First Age of Darkness / 1987 copyright): Message pauses on keeps it visible for 2.8 seconds after presentation, unaffected by turbo/message speed or held keys; no key required. Off retains the previous engine 12-tick pause. Character-menu loading retains its 9-tick pause.

Steady aim only suppresses held-arrow repeat in front-view space. Walking
(overworld, towns and dungeons) and top-down space retain normal arrow
repeat. F1 Controls Repeat selects the engine timing; when off, keyboard
OS repeat remains as before. After focus changes, menus or a fresh-key
message, release and press again to resume held movement.

For the time machine, lead a freed princess out of a castle with her
adjacent to you (any of the eight neighboring cells). Before her +500 XP
reward, you need at least 7000 XP and the selected Ace need in actual
fighter kills. She then says a time machine is far to the northwest and
places one in the northwest forest region of your current land. Gems
are not checked at rescue; all four are needed to use the machine later.
If an earlier rescue only gave 500 of each, enter a castle and rescue a
princess again after meeting the requirements. Changing Ace need uses
your existing raw kill count; raising it may make a future rescue
ineligible but does not remove an already placed machine.

In Mondain's lair, **G (Get)** next to the gem already destroys it, with
the original explosion costing three quarters of your current Hits
(God mode prevents that damage). Optional **F1 → Controls → Attack gem**
(`attack_gem = true`, off by default) also lets a successful in-range
weapon/projectile attack destroy it immediately, using the same effect
and cost. Misses and out-of-range attacks do not destroy it; God mode
is not required. Defeating Mondain is still necessary to win.

After the victory picture: **Press any key to return to title screen**.
Use a fresh press (held/queued keys do not dismiss it). The same window
returns to the actual title sequence and character roster; you can
create a new character or select an existing one. This does not save
over your character or change your options. No reboot keys are needed.

With Music enabled, the closing narrative plays "Sosaria Restored", a
new original 48-second instrumental loop (warm lead, plucked arpeggios,
strings and bass). The victory fanfare remains separate. Music stays
off by default and follows its own volume/master volume; effects may be
Original or Enhanced independently (global Mute still silences all).
Returning to title changes to the title theme through the usual crossfade.
