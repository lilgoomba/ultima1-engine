# Playing on Android (alpha)

The engine runs on Android phones and tablets (Android 5.0, API 21, or
newer; 64-bit ARM or x86_64). As on the desktop, **you need your own copy
of Ultima I**: the app holds none of the game, and plays only with files you
give it.

This is an alpha: it is not on the Play Store and may have rough edges
(see *Limits* below).

## Installing

1. Get `ultima1-engine-android-alpha1.apk` onto the device (download it,
   or copy it over USB).
2. Open it from the Files app. Android asks to allow installing from that
   source ("Install unknown apps"); allow it for that app, then *Install*.
   Turn the permission off again afterwards if you like.
3. Or from a computer with the platform tools: `adb install ultima1-engine-android-alpha1.apk`.

An update installs over the old version, keeping your saves, if both are
signed with the same key.

## Bringing in the game files

Copy your Ultima I folder (the one with `ULTIMA.EXE`, `OUT.EXE`,
`MAP.BIN`, `TCD.BIN`, `GEN.EXE` and the tile files) to the device, for
example to `Download/ULTIMA1`, or as a `.zip` of that folder.

On first start the app asks for it: *Choose the game folder* opens
Android's folder picker (go to the folder and tap *Use this folder*,
then *Allow*), or *Choose a .zip*. The files are copied into the app's own
storage and checked the same way the desktop program checks a game folder;
if something is missing it says what. The folder you picked is only read.

To use other files later: F1 (options), the last row *Game files: import
again*. Saves are kept.

Saves, the roster of characters and the options live in the app's private
storage. Uninstalling the app deletes them.

## Touch controls

The screen is landscape. The game picture sits in the middle (whole
multiples, or filled, per the *Scaling* option); the buttons go to its left
and right, or over it, see-through, when the screen is too square for both.

Left side, top:

| Button | Does |
|---|---|
| F1 | the options (as F1 on the desktop) |
| Map | the world or dungeon map (needs *World map* or *Dungeon map* on) |
| Log | the message log (needs *Message log* on) |
| Sav / Lod | quicksave / quickload (needs *Save slots* on) |
| Kbd | the system keyboard on or off (for names, or typing anything) |
| Ent / Esc | Enter / Esc |
| Hid | hides all the buttons; a small *Show* button brings them back |

Left side, bottom: the arrows. Hold one to keep going (the engine's key
repeat, at the speed set in the options). The middle button is Space
(pass a turn).

Right side: the command letters (A, B, C ... Z, as on the keyboard). *123*
swaps them for a number pad (0-9, backspace, Enter) for amounts and
directions that want numbers; *ABC* swaps back.

Android's Back button is Esc.

A physical keyboard (USB or Bluetooth) works as on the desktop, F-keys
included, and so does a gamepad when the *Gamepad* option is on.

## Sound

The original beeps, the enhanced sounds and the music all play, as chosen
in the options. Switching to another app pauses the game and its sound;
coming back carries on.

## Limits (alpha)

- Not on the Play Store; updates are by installing the new APK.
- The import copies files one by one from the folder picker; a folder far
  bigger than the game (64 MB) is refused.
- Choosing *import again* ends the current game: save first.
- The touch layout is fixed (no moving or resizing buttons yet).
- The Android build is tested in the emulator; real devices vary.

Building the APK yourself: see [docs/ANDROID.md](docs/ANDROID.md).
