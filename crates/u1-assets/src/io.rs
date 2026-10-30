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
    /// Script-only: `{p}` stops the script at the next timed pause, so a
    /// screenshot can catch a picture that the pause would show.
    pub const STOP_AT_PAUSE: u8 = 0xFD;
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

    /// A display chosen in the options menu, with its files, for the game
    /// to take at its next safe point (the main loop's key poll).
    fn video_switch(&mut self) -> Option<crate::game::VideoSet> {
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

    /// Drop keys typed ahead (front ends with a key buffer).
    fn flush(&mut self) {}

    /// Wait `n` vertical retraces (70 Hz).
    fn retrace(&mut self, _t: &Term, _n: u32) -> IoResult<()> {
        Ok(())
    }
}

/// Keys from a list; `Halt` once they run out.
#[derive(Debug, Default, Clone)]
pub struct ScriptIo {
    pub keys: VecDeque<u8>,
    pub sounds: Vec<u8>,
    /// The events of `sound_ev` and `cue`, in order.
    pub events: Vec<crate::sound::Sfx>,
}

impl ScriptIo {
    pub fn new(keys: &[u8]) -> Self {
        ScriptIo { keys: keys.iter().copied().collect(), sounds: Vec::new(), events: Vec::new() }
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
    fn poll(&mut self, _t: &Term) -> IoResult<Option<u8>> {
        loop {
            match self.keys.pop_front() {
                // A stray wait token where a key is expected is skipped.
                Some(keys::WAIT | keys::STOP_AT_PAUSE) => continue,
                Some(keys::IDLE) => return Ok(None),
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
}
