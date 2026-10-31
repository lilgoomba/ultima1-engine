//! The engine's own input helpers (SPEC §23.3), kept apart from SDL so
//! they can be tested: the auto-repeat for a held arrow and the gamepad
//! mapping. The front end turns SDL events into the plain values here
//! and acts on what comes back.

use crate::io::keys;
use crate::keymap::{PadMap, PadOut};
use std::time::{Duration, Instant};

// ------------------------------------------------------------ repeat

/// The auto-repeat of a held arrow. A repeat is only ever given to the
/// game at its command prompt, with no other key waiting and no window
/// open, and never sooner than one step after the last, so a held arrow
/// can not outrun the game's turns or answer any other question.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Repeater {
    /// The arrow held, what holds it (a keycode, or a negative number for
    /// the pad) and when it may next repeat.
    held: Option<(u8, i32, Instant)>,
}

impl Repeater {
    /// Arrow `a` went down, held by `by`; the first repeat waits `delay`.
    pub fn hold(&mut self, a: u8, by: i32, now: Instant, delay: Duration) {
        self.held = Some((a, by, now + delay));
    }

    /// Whatever held `by` was let go.
    pub fn release(&mut self, by: i32) {
        if self.held.is_some_and(|(_, h, _)| h == by) {
            self.held = None;
        }
    }

    /// Stop repeating (another key, a window).
    pub fn clear(&mut self) {
        self.held = None;
    }

    pub fn holding(&self) -> Option<u8> {
        self.held.map(|h| h.0)
    }

    /// The arrow to give the game now, if one is due. `ready`: the game is
    /// at its command prompt with nothing else waiting and no window up.
    pub fn due(&mut self, now: Instant, ready: bool, step: Duration) -> Option<u8> {
        let (a, by, at) = self.held?;
        if !ready || now < at {
            return None;
        }
        // From now, not from when it was due: a slow turn never leads to a
        // burst of catching-up steps.
        self.held = Some((a, by, now + step));
        Some(a)
    }
}

// ------------------------------------------------------------ gamepad

/// The left stick's dead zone (about a third of its travel).
pub const STICK_DEAD: i16 = 12_000;
/// What holds an arrow given by the stick.
pub const BY_STICK: i32 = -100;

/// A controller event, by SDL's button names ("a", "dpup", ...).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PadEvent<'a> {
    Button(&'a str, bool),
    /// The left stick's x or y axis (`true`: x) moved to a value.
    Stick(bool, i16),
}

/// What the front end should do for a pad event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PadAction {
    /// A key for the open window or the game, with its key name.
    Key(u8, String),
    /// A command-prompt-only code (the engine's screens).
    Command(u8),
    /// Open the options menu.
    Options,
    /// Start repeating an arrow, held by the given id.
    Hold(u8, i32),
    /// Whatever the id held was let go.
    Release(i32),
}

/// The d-pad buttons and the arrow each is.
const DPAD: [(&str, u8); 4] = [("dpup", keys::UP), ("dpdown", keys::DOWN), ("dpleft", keys::LEFT), ("dpright", keys::RIGHT)];

/// A stable id for a button, for [`Repeater`] (always negative).
fn button_id(name: &str) -> i32 {
    let i = DPAD.iter().position(|d| d.0 == name).unwrap_or_else(|| 4 + crate::keymap::BUTTONS.iter().position(|&b| b == name).unwrap_or(99));
    -1 - i as i32
}

/// The gamepad layer: the d-pad and left stick are the arrows, the other
/// buttons as the [`PadMap`] says.
#[derive(Debug, Clone, Default)]
pub struct PadInput {
    pub map: PadMap,
    /// The stick's current arrow (0: centred).
    stick: u8,
}

impl PadInput {
    pub fn new(map: PadMap) -> Self {
        PadInput { map, stick: 0 }
    }

    /// The actions for one event. `in_menu`: a window is open (it gets
    /// keys but no screens or repeat); `repeat`: the auto-repeat is on.
    pub fn event(&mut self, e: PadEvent, in_menu: bool, repeat: bool) -> Vec<PadAction> {
        let hold = repeat && !in_menu;
        match e {
            PadEvent::Button(name, down) => {
                let by = button_id(name);
                if !down {
                    return vec![PadAction::Release(by)];
                }
                if let Some(&(_, a)) = DPAD.iter().find(|d| d.0 == name) {
                    let mut v = vec![PadAction::Key(a, String::new())];
                    if hold {
                        v.push(PadAction::Hold(a, by));
                    }
                    return v;
                }
                match self.map.out(name) {
                    PadOut::Key(k) => {
                        let n = match k {
                            keys::ESC => "escape".to_string(),
                            keys::ENTER => "return".to_string(),
                            b' ' => "space".to_string(),
                            k => (k as char).to_ascii_lowercase().to_string(),
                        };
                        vec![PadAction::Key(k, n)]
                    }
                    PadOut::Command(c) if !in_menu => vec![PadAction::Command(c)],
                    PadOut::Options => vec![PadAction::Options],
                    _ => vec![],
                }
            }
            PadEvent::Stick(horiz, v) => {
                let a = match (horiz, v) {
                    (true, v) if v < -STICK_DEAD => keys::LEFT,
                    (true, v) if v > STICK_DEAD => keys::RIGHT,
                    (false, v) if v < -STICK_DEAD => keys::UP,
                    (false, v) if v > STICK_DEAD => keys::DOWN,
                    _ => 0,
                };
                let same_axis = |s: u8| if horiz { matches!(s, keys::LEFT | keys::RIGHT) } else { matches!(s, keys::UP | keys::DOWN) };
                if a == 0 {
                    if same_axis(self.stick) {
                        self.stick = 0;
                        return vec![PadAction::Release(BY_STICK)];
                    }
                    return vec![];
                }
                if a == self.stick {
                    return vec![];
                }
                self.stick = a;
                let mut v = vec![PadAction::Key(a, String::new())];
                if hold {
                    v.push(PadAction::Hold(a, BY_STICK));
                }
                v
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MS: fn(u64) -> Duration = Duration::from_millis;

    #[test]
    fn repeat_waits_then_steps_and_only_when_the_game_is_ready() {
        let t0 = Instant::now();
        let mut r = Repeater::default();
        assert_eq!(r.due(t0, true, MS(130)), None, "nothing held");
        r.hold(keys::RIGHT, 7, t0, MS(250));
        assert_eq!(r.due(t0 + MS(249), true, MS(130)), None, "not before the delay");
        assert_eq!(r.due(t0 + MS(250), true, MS(130)), Some(keys::RIGHT));
        assert_eq!(r.due(t0 + MS(300), true, MS(130)), None, "one step at a time");
        // The game is busy (a message, a question, a key waiting) for a
        // long while: nothing goes in, and afterwards only one step.
        assert_eq!(r.due(t0 + MS(900), false, MS(130)), None);
        assert_eq!(r.due(t0 + MS(2000), true, MS(130)), Some(keys::RIGHT));
        assert_eq!(r.due(t0 + MS(2001), true, MS(130)), None, "no burst to catch up");
        assert_eq!(r.due(t0 + MS(2130), true, MS(130)), Some(keys::RIGHT));
        r.release(8);
        assert_eq!(r.holding(), Some(keys::RIGHT), "another key's release changes nothing");
        r.release(7);
        assert_eq!(r.due(t0 + MS(5000), true, MS(130)), None);
    }

    #[test]
    fn default_timing_is_a_quarter_second_then_the_original_pace() {
        let q = crate::qol::Qol::default();
        let poll = MS(130);
        let (d, s) = q.repeat_timing(poll);
        assert_eq!((d, s), (MS(250), poll), "250 ms, then one step per main-loop pass");
        let fast = crate::qol::Qol { repeat_rate: 16, ..q };
        assert_eq!(fast.repeat_timing(poll).1, poll, "never quicker than the game's pace");
        assert_eq!(fast.repeat_timing(MS(13)).1, MS(62), "turbo: the rate chosen");
        let slow = crate::qol::Qol { repeat_rate: 4, ..q };
        assert_eq!(slow.repeat_timing(poll).1, MS(250));
    }

    fn keys_of(v: &[PadAction]) -> Vec<u8> {
        v.iter().filter_map(|a| if let PadAction::Key(k, _) = a { Some(*k) } else { None }).collect()
    }

    #[test]
    fn pad_buttons_follow_the_defaults_and_the_config() {
        let mut p = PadInput::default();
        let press = |p: &mut PadInput, b: &str| p.event(PadEvent::Button(b, true), false, false);
        assert_eq!(press(&mut p, "a"), vec![PadAction::Key(b' ', "space".into())]);
        assert_eq!(press(&mut p, "b"), vec![PadAction::Key(keys::ESC, "escape".into())]);
        assert_eq!(keys_of(&press(&mut p, "x")), vec![b'A']);
        assert_eq!(keys_of(&press(&mut p, "y")), vec![b'E']);
        assert_eq!(keys_of(&press(&mut p, "leftshoulder")), vec![b'K']);
        assert_eq!(keys_of(&press(&mut p, "rightshoulder")), vec![b'F']);
        assert_eq!(keys_of(&press(&mut p, "rightstick")), vec![b'Z']);
        assert_eq!(press(&mut p, "back"), vec![PadAction::Command(keys::MAP)]);
        assert_eq!(press(&mut p, "guide"), vec![PadAction::Command(keys::HELP)]);
        assert_eq!(press(&mut p, "leftstick"), vec![PadAction::Command(keys::CHARSHEET)]);
        assert_eq!(press(&mut p, "start"), vec![PadAction::Options]);
        assert_eq!(p.event(PadEvent::Button("back", true), true, false), vec![], "no screens over a window");
        assert_eq!(press(&mut p, "misc1"), vec![], "unknown buttons do nothing");
        assert_eq!(p.event(PadEvent::Button("a", false), false, false).len(), 1, "a release is only a release");
        p.map.config_key("pad_y", "map").unwrap().unwrap();
        p.map.config_key("pad_a", "enter").unwrap().unwrap();
        assert_eq!(press(&mut p, "y"), vec![PadAction::Command(keys::MAP)]);
        assert_eq!(keys_of(&press(&mut p, "a")), vec![keys::ENTER]);
    }

    #[test]
    fn dpad_and_stick_are_the_arrows_and_hold_for_repeat() {
        let mut p = PadInput::default();
        for (b, a) in DPAD {
            assert_eq!(keys_of(&p.event(PadEvent::Button(b, true), false, false)), vec![a]);
        }
        let v = p.event(PadEvent::Button("dpleft", true), false, true);
        let by = button_id("dpleft");
        assert_eq!(v, vec![PadAction::Key(keys::LEFT, String::new()), PadAction::Hold(keys::LEFT, by)]);
        assert_eq!(p.event(PadEvent::Button("dpleft", false), false, true), vec![PadAction::Release(by)]);
        assert_eq!(keys_of(&p.event(PadEvent::Button("dpleft", true), true, true)), vec![keys::LEFT], "windows get arrows");
        assert!(!p.event(PadEvent::Button("dpleft", true), true, true).iter().any(|a| matches!(a, PadAction::Hold(..))), "but no repeat");
        // The stick: inside the dead zone nothing; past it one arrow until
        // it comes back.
        assert_eq!(p.event(PadEvent::Stick(true, 11_000), false, true), vec![]);
        assert_eq!(p.event(PadEvent::Stick(true, 20_000), false, true), vec![PadAction::Key(keys::RIGHT, String::new()), PadAction::Hold(keys::RIGHT, BY_STICK)]);
        assert_eq!(p.event(PadEvent::Stick(true, 30_000), false, true), vec![], "still right: no second key");
        assert_eq!(p.event(PadEvent::Stick(false, 500), false, true), vec![], "the other axis centred changes nothing");
        assert_eq!(p.event(PadEvent::Stick(true, 0), false, true), vec![PadAction::Release(BY_STICK)]);
        assert_eq!(keys_of(&p.event(PadEvent::Stick(false, -32768), false, false)), vec![keys::UP]);
        assert_eq!(keys_of(&p.event(PadEvent::Stick(false, 32767), false, false)), vec![keys::DOWN]);
        assert_eq!(keys_of(&p.event(PadEvent::Stick(true, -20_000), false, false)), vec![keys::LEFT]);
        // Distinct ids for every button.
        let mut ids: Vec<i32> = DPAD.iter().map(|d| d.0).chain(crate::keymap::BUTTONS).map(button_id).collect();
        ids.push(BY_STICK);
        let n = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), n);
    }
}
