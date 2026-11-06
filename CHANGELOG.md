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
