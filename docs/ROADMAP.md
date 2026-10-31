# Quality-of-life roadmap

Approved, **optional** features. Built (branch qol-review-1, SPEC §22):
fullscreen, whole-number scaling, CRT filter, save slots, quicksave and
quickload, the scrollback message log, turbo, adjustable message delay,
faster space flight and steady timing. Built (branch qol-review-2, SPEC
§23): the dungeon automap, the overworld mini-map, extra keys with a rebind
screen, gamepad support, arrow auto-repeat, the help screen and hints, the
character screen and confirm-before-selling or dropping. Everything on
this list is now built and merged. Still to do: testing the gamepad with a
real controller (the mapping is tested with synthetic SDL events only). The rule for every
one: the faithful behaviour stays the default. Each is off until the player
turns it on (in the options menu, F1, or `config.toml`; see SPEC §21), never
changes a save file's format, and never changes the game's rules or the
original's random sequence unless its entry says so. Screenshot runs and
the tests keep the original behaviour.

## Display

* **Fullscreen.** A window/fullscreen toggle (also Alt+Enter), remembered
  in the config file.
* **Whole-number scaling.** Scale the 320x200 screen (or the enhanced
  resolution) only by whole multiples, letterboxed, so pixels stay square
  and even; a choice between "largest whole multiple" and "fill the
  window".
* **CRT scanline filter.** An optional scanline/mask overlay drawn by the
  front end after the frame is composed; off by default and in
  screenshots only with `--crt`.

## Saving

* **Several save slots**, beside the original's single player file per
  character, each an ordinary 820-byte player record so the original can
  still read it.
* **Quicksave and quickload** keys (outside the original's key set) that
  write and read a dedicated slot. The original only saves on Quit; with
  this off, that stays the only way.

## Maps and information

* **Dungeon automap.** A map of the dungeon squares the player has seen,
  per level, shown on request in place of the first-person view. Kept
  beside the save, not in it.
* **Scrollback message log.** The last few hundred lines of the
  four-line message window, viewable on request; the window itself is
  unchanged.
* **Overworld mini-map or compass.** A small map or compass on request
  that marks the towns, castles, dungeons and landmarks the player has
  already visited (nothing they have not found: hidden things stay
  hidden).
* **Command help and tooltips.** A help screen listing the commands and
  what they do, and an optional one-line hint for the key just pressed
  or the prompt shown.
* **One character and inventory screen.** The Z-stats pages (attributes,
  weapons, armour, spells, vehicles, gems) gathered on one screen.

## Speed

* **Turbo toggle.** Run the waits and animations faster while held or
  toggled.
* **Adjustable message delay.** The pauses after messages, from the
  original's to none.
* **Faster space combat.** Shorter waits between space turns and
  animations.
* **Turn off the CPU-speed quirks.** Some of the original's timing
  depends on the processor (the idle Pass after a number of key polls,
  speaker-effect lengths, some space and Mondain waits; SPEC §3, §11,
  §18, §19); the engine reproduces them at one measured reference speed.
  An option replaces them with fixed timings chosen for comfort.

## Controls

* **Remappable keys**, kept in the config file (the original's keys stay
  the defaults and always work).
* **Gamepad support** through SDL's game controller API (d-pad or stick to
  move, buttons for common commands, one to open the options menu).
* **Movement auto-repeat**: holding a direction keeps moving at the
  original's pace.

## Safety

* **Confirm before selling or dropping valuable items** (weapons, armour,
  vehicles, gems and similar above a value threshold), as a yes/no prompt
  in the message window.

## Built on features-review-1

* **Enhanced art for vehicles.** Pack pictures for overworld tiles
  0x0B-0x12 (CC-BY-SA); the original tiles stay the fallback.
* **Area music** (F1, off by default; SPEC §24). Nine original chiptune
  scores rendered at run time; heard only with Enhanced sound (greyed in
  F1 otherwise, setting kept).
* **Smoothing** (F1, off by default; SPEC §24): linear, or our own
  Scale2x pixel-art scaler.

## Future ideas (not built)

