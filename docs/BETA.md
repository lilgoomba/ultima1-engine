# 0.1.0-beta.1

Beta targets: Linux x86-64 (AppImage, glibc 2.35 or newer) and Windows x64
(portable ZIP; Windows 10/11 recommended). The repository remains private.
Beta work prioritizes bug fixes and regressions rather than new features.

## Try it

- Back up your existing saves and config before updating. Do not overwrite
  your own game files. Both downloads require a legally owned copy of
  Ultima I; neither contains those files.
- Linux: make the AppImage executable and run it. See PLAYING.md for game
  discovery and normal per-user save/config locations.
- Windows: extract the entire ZIP to a writable folder, copy your own game
  files into ULTIMA1, run ultima1-engine.exe. Keep portable.txt: config.toml
  and saves/ then stay beside the exe, independent of the working folder.
  Existing portable users should preserve config.toml, saves/ and ULTIMA1
  when updating. No installer or administrator access is needed.
- F1 controls the enhancements. Confirm quit is on by default: Escape at
  a gameplay command prompt or window close asks first, including just
  after loading/saving. Y quits without saving; N/Enter/fresh Escape
  cancels. Q outdoors and explicit save slots retain their existing saves.
- Report the version, OS, steps to reproduce, settings, and (if useful) a
  copy of the affected save. The program supports --version; BUILD-INFO.txt
  in each package records the version and exact source commit.

## Tested and remaining gaps

The reviewed game was completed by the tester before beta; the tester also
confirmed the portable Windows build and corrected Escape confirmation on
Windows and Linux. Automated tests cover game logic with owned game files
locally, real SDL quit event/modal handling, and Windows native tests plus
packaging/path smoke checks. CI does not have original game files and skips
its game-file scenarios. None of this is a comprehensive hardware matrix.

- Windows executable is unsigned; SmartScreen/unknown-publisher warnings
  are possible, not a confirmed failure. Linux AppImage may need FUSE or
  --appimage-extract-and-run depending on the host.
- Native Windows CI smoke checks do not constitute full interactive game,
  controller, display, audio-device or long-session tests. Linux headless
  SDL quit checks likewise do not cover all desktop/window managers.
- Fresh beta packages are rebuilt from the promoted source, not a new
  complete playthrough on both systems.
- macOS is not a beta deliverable or supported beta target; Android work is
  parked and excluded. Neither is being claimed as release-tested.
- Non-portable Windows config discovery still uses the existing HOME/XDG
  policy, which can lack a config path when HOME is absent. Use the supplied
  portable distribution (its config path does not depend on HOME).
- Music/art and optional enhancements retain their existing defaults;
  this is a beta, not a claim of perfect DOS fidelity or zero remaining bugs.
