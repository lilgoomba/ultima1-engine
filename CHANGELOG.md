# Changelog

No tagged releases yet; entries are by branch.

## space-aim-alt (unreleased)

### Added
- Steady aim option (F1 → Controls, `steady_aim`, off by default): in the
  space front view the sights glide while an arrow or pad direction is
  held, driven by elapsed time (not OS key-repeat), and stop on release,
  focus loss or the options menu. 2 fuel per newly held direction.
- Enemy speed option (F1 → Controls, `enemy_speed`, Original by default;
  Half slows only front-view fighter flee/grow).

## polish-1 (merged into main)

### Added
- F1 options menu in five pages (Video, Sound, Gameplay, Controls,
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
