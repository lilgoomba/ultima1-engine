# Changelog

No tagged releases yet; entries are by branch.

## windows-portable (unreleased)

### Added
- Portable Windows x86-64 ZIP (`packaging/windows`, Actions workflow
  `windows-portable.yml`): extract anywhere, put your own game files in
  `ULTIMA1`, run `ultima1-engine.exe`; no installer or admin rights.
- Portable mode (`portable.txt` beside the program, or `--portable`):
  `config.toml` and `saves/` beside the program, its own
  `assets/enhanced`, game files looked for only there. Without the marker
  every path is unchanged.
- `u1/windows-gui` Cargo feature (used by the ZIP only): no console window.
- Escape at the command prompt (outdoors, towns, castles, dungeons, space,
  Mondain's lair) quits like closing the window: the Confirm quit question
  when progress is unsaved (on by default), nothing saved. Windows and
  other prompts keep Escape as close/cancel.

## space-aim-alt (unreleased)

### Added
- Steady aim option (F1 → Controls, `steady_aim`, off by default): in the
  space front view the sights glide while an arrow or pad direction is
  held, driven by elapsed time (not OS key-repeat), and stop on release,
  focus loss or the options menu. 2 fuel per newly held direction.
- Enemy speed option (F1 → Controls, `enemy_speed`, Original by default;
  Half slows only front-view fighter flee/grow).
- Steady aim advances once per vertical retrace (with a one-frame dt cap)
  so held movement no longer jumps once per speed wait.
- Space map option (F1 → Controls, `space_map`, Original by default):
  Enhanced draws a clearer Inform scan with grid, labels, legend and a
  steady you-are-here mark; same sector intel as the original icons.
- Kill counter option (F1 → Controls, `space_kills`, off by default):
  space HUD shows fighter kills toward Space Ace (`K:n/20`, then `Ace!`).
  Enhanced map layout tightened so the grid and prompt no longer overlap.
- Enhanced Inform no longer clears the status panel (Shld/Fuel/Exp/Coin/Kills).
- F1 page digits derive from page count (1–6 after Space); key 6 opens Interface again.
- New original ending-narrative music "Sosaria Restored" (48 s loop), using existing Music/volume controls. Other tracks and victory sting unchanged.
- Ending: fresh-key return to the actual title/roster in-process, replacing the reboot prompt; preserves roster/config and clears transient input/start overrides.
- Optional Attack gem (F1 Controls, off; `attack_gem`): successful in-range attacks share G's instant gem destruction and explosion cost. No God mode required.
- Princess time-machine reward honors the selected Ace need (5/10/20), matching space HUD/award; 7000 pre-reward XP and rescue are still required. No save changes.
- Fix held-arrow repeat with Steady aim enabled: edge-only filtering is confined to front-view space, not walking or top-down flight. Engine repeat also runs on nonblocking space polls; fresh-key messages remain fresh-key-only.
- Correct startup dwell target: first title now 2.8 s with Message pauses on; restore loading-screen 9 ticks.
- God mode kills keep death side-effects (castle jester key, etc.).
- God mode (F1 Controls, off): Hits never fall; hits kill instantly; `--god`.
- Message pauses (was Sign wait): signs + princess rescue; config `wait_on_signs` kept.
- Enhanced map: cyan Home corners at sector (3,3); distinct from yellow You.
- Steady aim: original drift + free counter-steer stop; second press after release starts that way; no hold-to-move.
- Space options page: Hit box (forgiving +2 px, off by default) and Ace need
  (20/10/5, default 20) for Space Ace HUD and award; no save-format change.

## polish-1 (merged into main)

### Added
- F1 options menu in six pages (Video, Sound, Gameplay, Controls, Space,
  Interface) with a description line and per-page / all resets.
- Pause while the F1 menu is open or the window loses focus.
- Live settings: every option applies at once, no restart.
- Rich save slots, on by default: a `<save>.info` sidecar (time, name,
  level, place, thumbnail) shown in the slot screen; the original
  820-byte record is unchanged.
- Screenshot key (F12, rebindable on the Keys screen as `key_screenshot`;
  the older `screenshot_key` is still read).
- Window place and size remembered between runs.
- Master, effects and music volumes.
- Quit confirmation when closing with unsaved progress (on by default).
- AppImage desktop entry and a new icon (our own art, CC-BY-SA 4.0).
- Vehicle facing in enhanced video (on by default), kept in the `.map`
  sidecar.
- Sign wait option (on by default): "Press any key" after a sign.
- Docking fees option (on by default; off refuels free).
- Ship choice by arrow keys at "Choose thy ship" (Enter/Space/pad A take it).
- F3 in space shows the space commands (with the Help screen option).
- `--player N` plays and saves roster character N.

### Changed
- Hyperspace jump: the "Lightspeed!" streaks are paced so they are seen.
- Whole-pixel scaling keeps the original's multiple for the enhanced art.

### Fixed
- `--save FILE` saves back to FILE instead of over roster character 1.
- F3 and rebound keys were ignored at the space prompt.
- Fire: the aircar fires lasers; on foot "Fire what?" (as the original).
- Enhanced video: tiles partly under a window keep their art.
