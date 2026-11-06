//! Input/output seam between the game logic and a front end.
//!
//! The original is a single-threaded DOS program that polls the keyboard
//! from inside its commands (shops ask for a letter, the king for an
//! amount, ...). The game code here is written the same way and calls an
//! `Io` whenever it waits; the SDL front end implements it with a window,
//! tests and the screenshot mode with a list of keys.

use std::collections::VecDeque;

use crate::term::Term;

/// Returned when a scripted run has no more keys: the game stops where it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Halt;

pub type IoResult<T> = std::result::Result<T, Halt>;

/// Key codes as the original's keyboard routine reports them
/// (verified from original executable): arrows become 1..=4 in the same
/// order as its direction numbers, everything else is ASCII.
pub mod keys {
    pub const LEFT: u8 = 1;
    pub const RIGHT: u8 = 2;
    pub const UP: u8 = 3;
    pub const DOWN: u8 = 4;
    pub const BACKSPACE: u8 = 8;
    pub const ENTER: u8 = 13;
    pub const ESC: u8 = 27;
    /// Script-only token: the key that ends a "wait for a key or a while"
    /// pause (the original swallows it).
    pub const WAIT: u8 = 0xFF;
    /// Script-only: one poll that finds no key (lets time pass).
    pub const IDLE: u8 = 0xFE;
    /// Script-only: `{p}` stops at the next timed pause (or at `any_key` when
    /// `skip_pauses` skipped those), so a
    /// screenshot can catch a picture that the pause would show.
    pub const STOP_AT_PAUSE: u8 = 0xFD;
    /// Scripts: `{saveN}` / `{loadN}` (N 1..5, or q) ask for a save-slot
    /// action at that poll (an idle poll otherwise), as F2/F5/F9 do.
    pub const SAVE_REQ: u8 = 0xD0;
    pub const LOAD_REQ: u8 = 0xD8;
    /// The engine's screens (SPEC §23), sent only at the command prompt
    /// (an extra key or a script's `{map}`, `{help}`). The character
    /// screen is Ztats (Z) with that option on.
    pub const MAP: u8 = 0xC1;
    pub const HELP: u8 = 0xC2;
}

pub trait Io {
    /// One keyboard poll of the main loop (about 130 ms in the original):
    /// show `t` and return a key if one was pressed.
    fn poll(&mut self, t: &Term) -> IoResult<Option<u8>>;

    /// Block until a key is pressed.
    fn key(&mut self, t: &Term) -> IoResult<u8> {
        loop {
            if let Some(k) = self.poll(t)? {
                return Ok(k);
            }
        }
    }

    /// Up to 1500 polls; 0 if no key came (the original's prompt helper).
    fn key_timeout(&mut self, t: &Term) -> IoResult<u8> {
        for _ in 0..1500 {
            if let Some(k) = self.poll(t)? {
                return Ok(k);
            }
        }
        Ok(0)
    }

    /// How many polls a "key or timeout" read waits (1500 in the original;
    /// a script's `{w}` stands for the whole timeout).
    fn timeout_polls(&self) -> u32 {
        1500
    }

    /// Wait `ticks` of the 18.2 Hz timer.
    fn pause(&mut self, t: &Term, ticks: u32) -> IoResult<()>;

    /// Wait for a key or a short while, whichever comes first (the key is
    /// swallowed).
    fn wait_or_key(&mut self, t: &Term) -> IoResult<()>;

    /// Play one of the original's sound effects (numbers as in the
    /// original). Front ends may ignore it.
    fn sound(&mut self, _n: u8) {}

    /// Effect `n` played for the game event `ev` (the enhanced sound mode
    /// may give the event its own sound). By default just effect `n`.
    fn sound_ev(&mut self, n: u8, _ev: crate::sound::Sfx) {
        self.sound(n);
    }

    /// An event the original leaves silent; only the enhanced sound mode
    /// may play something. Never waits.
    fn cue(&mut self, _ev: crate::sound::Sfx) {}

    /// The area the game is in (or a moment such as death), for the
    /// engine's optional music. The original has none; by default nothing.
    fn music(&mut self, _t: crate::music::Track) {}

    /// A display chosen in the options menu, with its files, for the game
    /// to take at its next safe point (the main loop's key poll).
    fn video_switch(&mut self) -> Option<crate::game::VideoSet> {
        None
    }

    /// The title or character program stopped (a [`Halt`]) because a
    /// display was chosen in the options menu on its screen: that display,
    /// now shown, to start the program again in. `None` for a real halt.
    fn display_restart(&mut self) -> Option<crate::gamedir::VideoMode> {
        None
    }

    /// The game now draws in `mode` (the front end changes its palette).
    fn video_shown(&mut self, _mode: crate::gamedir::VideoMode) {}

    /// Effect `n` for an event too far away for the enhanced sound mode
    /// to play (a distant townsperson's step): the original effect as
    /// usual, nothing in the enhanced mode (which still waits as long).
    fn sound_distant(&mut self, n: u8) {
        self.sound(n);
    }

    /// A keyboard check that does not wait (the space program's polls,
    /// which are paced by explicit delays instead).
    fn poll_now(&mut self, t: &Term) -> IoResult<Option<u8>> {
        self.poll(t)
    }

    /// Wait for a fresh key press (a mouse click or pad button counts
    /// too in front ends that have them); keys typed ahead and held-key
    /// repeats do not end it.
    fn any_key(&mut self, t: &Term) -> IoResult<()> {
        self.flush();
        self.key(t).map(drop)
    }

    /// Drop keys typed ahead (front ends with a key buffer).
    fn flush(&mut self) {}

    /// Wait `n` vertical retraces (70 Hz).
    fn retrace(&mut self, _t: &Term, _n: u32) -> IoResult<()> {
        Ok(())
    }

    // The optional engine features (SPEC §22); off, each is a no-op.

    /// A pause that only lets a message be read (the message-delay
    /// setting may shorten it); the original's `pause` otherwise.
    fn message_pause(&mut self, t: &Term, ticks: u32) -> IoResult<()> {
        self.pause(t, ticks)
    }

    /// Space flight starts (`true`) or ends (the fast space setting).
    fn in_space(&mut self, _on: bool) {}

    /// A save-slot action asked for since the last poll (F2, F5, F9 with
    /// save slots on). Taken by the main loop's key poll.
    fn save_request(&mut self) -> Option<crate::slots::SaveReq> {
        None
    }

    /// The game wrote `rec` to `path` (Quit or a save slot); `t` is the
    /// screen. The front end may keep its own details beside it (the save
    /// itself is never changed).
    fn saved(&mut self, _path: &std::path::Path, _rec: &crate::save::PlayerRecord, _t: &Term) {}

    /// The result of a save-slot action, to show the player.
    fn save_done(&mut self, _msg: &str) {}

    /// The optional features the game itself applies (the screens, hints,
    /// confirmations, the explored map). All off by default.
    fn features(&self) -> crate::qol::Qol {
        crate::qol::Qol::default()
    }

    /// The game waits at its command prompt (`true`) or has left it: extra
    /// keys mean their command only there.
    /// An engine screen (map, help, character) is up: its own key should
    /// reach it so the same key closes it.
    fn view_open(&mut self, _key: Option<u8>) {}

    fn command_prompt(&mut self, _on: bool) {}

    /// Whether the message log should be kept ([`crate::term::Term::log`]).
    fn wants_log(&self) -> bool {
        false
    }
}

/// Keys from a list; `Halt` once they run out.
#[derive(Debug, Default, Clone)]
pub struct ScriptIo {
    pub keys: VecDeque<u8>,
    pub sounds: Vec<u8>,
    /// The events of `sound_ev` and `cue`, in order.
    pub events: Vec<crate::sound::Sfx>,
    /// A `{saveN}`/`{loadN}` token seen, for `save_request`.
    pub request: Option<crate::slots::SaveReq>,
    /// What `save_done` reported.
    pub done: Vec<String>,
    /// `wants_log`.
    pub log: bool,
    /// `features`.
    pub qol: crate::qol::Qol,
    /// The music asked for, a repeat of the last one left out.
    pub music: Vec<crate::music::Track>,
    /// Each `retrace(n)` call's n (for cadence tests).
    pub retrace_slices: std::cell::RefCell<Vec<u32>>,
    /// Let this many timed `pause`/`message_pause` calls ignore a leading
    /// `{p}` so a later [`any_key`] can freeze on "Press any key".
    pub skip_pauses: std::cell::Cell<u32>,
}

impl ScriptIo {
    pub fn new(keys: &[u8]) -> Self {
        // Scripts replay the original's flow: the engine's wait on signs
        // is off unless a test turns it on.
        let qol = crate::qol::Qol { sign_wait: false, ..Default::default() };
        ScriptIo { keys: keys.iter().copied().collect(), qol, ..Default::default() }
    }

    /// Keys from text; `~` stands for the key that ends a timed wait and
    /// `{...}` names special keys: `{left}`, `{right}`, `{up}`, `{down}`,
    /// `{esc}`, `{enter}`, `{bs}`.
    pub fn from_text(s: &str) -> Self {
        let mut out = Vec::new();
        let mut it = s.chars().peekable();
        while let Some(c) = it.next() {
            match c {
                '~' => out.push(keys::WAIT),
                '{' => {
                    let name: String = it.by_ref().take_while(|&c| c != '}').collect();
                    // `{w}` / `{wN}`: N polls without a key.
                    if let Some(n) = name.strip_prefix('w') {
                        if n.is_empty() || n.bytes().all(|b| b.is_ascii_digit()) {
                            let n = n.parse::<usize>().unwrap_or(1);
                            out.extend(std::iter::repeat(keys::IDLE).take(n));
                            continue;
                        }
                    }
                    let slot = |n: &str| match n {
                        "q" => Some(6u8),
                        d if d.len() == 1 && (b'1'..=b'5').contains(&d.as_bytes()[0]) => Some(d.as_bytes()[0] - b'0'),
                        _ => None,
                    };
                    if let Some(n) = name.strip_prefix("save").and_then(slot) {
                        out.push(keys::SAVE_REQ + n);
                        continue;
                    }
                    if let Some(n) = name.strip_prefix("load").and_then(slot) {
                        out.push(keys::LOAD_REQ + n);
                        continue;
                    }
                    out.push(match name.as_str() {
                        "left" => keys::LEFT,
                        "right" => keys::RIGHT,
                        "up" => keys::UP,
                        "down" => keys::DOWN,
                        "esc" => keys::ESC,
                        "enter" => keys::ENTER,
                        "bs" => keys::BACKSPACE,
                        "space" => b' ',
                        "p" => keys::STOP_AT_PAUSE,
                        "map" => keys::MAP,
                        "help" => keys::HELP,
                        _ => b'?',
                    });
                }
                c => out.push(c as u8),
            }
        }
        ScriptIo::new(&out)
    }
}

impl Io for ScriptIo {
    fn retrace(&mut self, _t: &Term, n: u32) -> IoResult<()> {
        self.retrace_slices.borrow_mut().push(n);
        Ok(())
    }

    fn music(&mut self, t: crate::music::Track) {
        if self.music.last() != Some(&t) {
            self.music.push(t);
        }
    }

    fn poll(&mut self, _t: &Term) -> IoResult<Option<u8>> {
        loop {
            match self.keys.pop_front() {
                // A stray wait token where a key is expected is skipped.
                Some(keys::WAIT | keys::STOP_AT_PAUSE) => continue,
                Some(keys::IDLE) => return Ok(None),
                Some(k) if (keys::SAVE_REQ + 1..keys::SAVE_REQ + 7).contains(&k) || (keys::LOAD_REQ + 1..keys::LOAD_REQ + 7).contains(&k) => {
                    use crate::slots::{SaveReq, Slot};
                    let n = k & 7;
                    let slot = if n == 6 { Slot::Quick } else { Slot::N(n) };
                    self.request = Some(if k < keys::LOAD_REQ { SaveReq::Save(slot) } else { SaveReq::Load(slot) });
                    return Ok(None);
                }
                Some(k) => return Ok(Some(k)),
                None => return Err(Halt),
            }
        }
    }

    fn key_timeout(&mut self, t: &Term) -> IoResult<u8> {
        Ok(self.poll(t)?.unwrap_or(0))
    }

    fn timeout_polls(&self) -> u32 {
        1
    }

    fn pause(&mut self, _t: &Term, _ticks: u32) -> IoResult<()> {
        if self.keys.front() == Some(&keys::STOP_AT_PAUSE) {
            let n = self.skip_pauses.get();
            if n > 0 {
                self.skip_pauses.set(n - 1);
                return Ok(());
            }
            return Err(Halt);
        }
        Ok(())
    }

    fn wait_or_key(&mut self, _t: &Term) -> IoResult<()> {
        if self.keys.front() == Some(&keys::WAIT) {
            self.keys.pop_front();
        }
        Ok(())
    }

    fn any_key(&mut self, _t: &Term) -> IoResult<()> {
        // `{p}` freezes on the "Press any key" prompt for fixtures/tests.
        if self.keys.front() == Some(&keys::STOP_AT_PAUSE) {
            return Err(Halt);
        }
        // Skip queued wait/idle tokens so typed-ahead pacing keys cannot
        // dismiss the prompt (mirrors the SDL fresh-key path).
        loop {
            match self.keys.pop_front() {
                Some(keys::WAIT) | Some(keys::IDLE) => continue,
                Some(_) => return Ok(()),
                None => return Err(Halt),
            }
        }
    }

    fn sound(&mut self, n: u8) {
        self.sounds.push(n);
    }

    fn sound_ev(&mut self, n: u8, ev: crate::sound::Sfx) {
        self.events.push(ev);
        self.sound(n);
    }

    fn cue(&mut self, ev: crate::sound::Sfx) {
        self.events.push(ev);
    }

    fn save_request(&mut self) -> Option<crate::slots::SaveReq> {
        self.request.take()
    }

    fn save_done(&mut self, msg: &str) {
        self.done.push(msg.to_string());
    }

    fn wants_log(&self) -> bool {
        self.log
    }

    fn features(&self) -> crate::qol::Qol {
        self.qol
    }
}
