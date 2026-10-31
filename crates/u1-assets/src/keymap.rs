//! Extra keys for the commands, and the gamepad's buttons (SPEC §23.4).
//!
//! The original's own keys always work and cannot be taken away: a
//! binding only adds a second key, and only keys the original does not
//! use at its command prompt can be bound (a letter it answers with
//! "Huh?", a spare digit, punctuation, Tab, some function keys). The
//! extra keys mean their command only at the command prompt; inside a
//! prompt (a shop list, a number) every key types what it always did.
//! Keys are named as SDL names them, in lower case ("j", "tab", "f7").

use crate::io::keys;

/// One thing a key can do at the command prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Action {
    /// Its name in the config file (`key_<id>`).
    pub id: &'static str,
    /// What it does, in plain words.
    pub label: &'static str,
    /// The key code the game receives.
    pub code: u8,
    /// The original's own key for it ("" for the engine's screens).
    pub own: &'static str,
    /// The extra key it has without a config file.
    pub default: &'static str,
}

const fn act(id: &'static str, label: &'static str, code: u8, own: &'static str) -> Action {
    Action { id, label, code, own, default: "" }
}

pub const ACTIONS: [Action; 28] = [
    act("north", "North (dungeon: forward)", keys::UP, "Up"),
    act("south", "South (dungeon: turn round)", keys::DOWN, "Down"),
    act("west", "West (dungeon: turn left)", keys::LEFT, "Left"),
    act("east", "East (dungeon: turn right)", keys::RIGHT, "Right"),
    act("pass", "Pass", b' ', "Space"),
    act("attack", "Attack", b'A', "A"),
    act("board", "Board", b'B', "B"),
    act("cast", "Cast", b'C', "C"),
    act("drop", "Drop", b'D', "D"),
    act("enter", "Enter", b'E', "E"),
    act("fire", "Fire", b'F', "F"),
    act("get", "Get", b'G', "G"),
    act("hyperjump", "Hyperjump", b'H', "H"),
    act("inform", "Inform and search", b'I', "I"),
    act("klimb", "Klimb", b'K', "K"),
    act("noise", "Noise", b'N', "N"),
    act("open", "Open", b'O', "O"),
    act("quit", "Quit and save", b'Q', "Q"),
    act("ready", "Ready", b'R', "R"),
    act("steal", "Steal", b'S', "S"),
    act("transact", "Transact", b'T', "T"),
    act("unlock", "Unlock", b'U', "U"),
    act("view", "View", b'V', "V"),
    act("xit", "X-it", b'X', "X"),
    act("ztats", "Ztats", b'Z', "Z"),
    Action { id: "map", label: "Map", code: keys::MAP, own: "", default: "tab" },
    Action { id: "help", label: "Command help", code: keys::HELP, own: "", default: "f3" },
    Action { id: "character", label: "Character screen", code: keys::CHARSHEET, own: "", default: "f6" },
];

/// Keys the original does nothing with at its command prompt, so they
/// can be bound without clashing with any of its controls.
pub const FREE: [&str; 31] = [
    "j", "l", "m", "p", "w", "y", "0", "1", "3", "5", "7", "9", ";", "'", ",", ".", "/", "[", "]", "\\", "-", "=", "`", "tab", "f3",
    "f6", "f7", "f8", "f10", "f11", "f12",
];

pub fn is_free(name: &str) -> bool {
    FREE.contains(&name)
}

/// How a key name is shown.
pub fn show(name: &str) -> String {
    match name {
        "" => "-".into(),
        n if n.len() == 1 => n.to_ascii_uppercase(),
        n => {
            let mut c = n.chars();
            c.next().map_or(String::new(), |f| f.to_ascii_uppercase().to_string() + c.as_str())
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyMap {
    /// The extra key of each of [`ACTIONS`] ("" for none).
    pub extra: Vec<String>,
}

impl Default for KeyMap {
    fn default() -> Self {
        KeyMap { extra: ACTIONS.iter().map(|a| a.default.to_string()).collect() }
    }
}

impl KeyMap {
    /// The game's code for an extra key, if one is bound to it.
    pub fn code_for(&self, name: &str) -> Option<u8> {
        if name.is_empty() {
            return None;
        }
        self.extra.iter().position(|k| k == name).map(|i| ACTIONS[i].code)
    }

    /// Bind `name` to action `i`. A key the original uses is refused (with
    /// the reason); a key bound elsewhere moves here. Returns a note.
    pub fn bind(&mut self, i: usize, name: &str) -> Result<String, String> {
        if let Some(a) = ACTIONS.iter().find(|a| a.own.eq_ignore_ascii_case(name)) {
            return Err(format!("{} is the original's key for {}", show(name), a.label));
        }
        if !is_free(name) {
            return Err(format!("{} cannot be bound", show(name)));
        }
        let mut note = format!("{} now {}", show(name), ACTIONS[i].label);
        for (j, k) in self.extra.iter_mut().enumerate() {
            if j != i && k == name {
                k.clear();
                note = format!("{} moved from {}", show(name), ACTIONS[j].label);
            }
        }
        self.extra[i] = name.to_string();
        Ok(note)
    }

    pub fn unbind(&mut self, i: usize) {
        self.extra[i].clear();
    }

    /// Read one `key_<id>` config line: `None` if `k` is not one.
    pub fn config_key(&mut self, k: &str, v: &str) -> Option<Result<(), &'static str>> {
        let i = ACTIONS.iter().position(|a| k.strip_prefix("key_") == Some(a.id))?;
        let v = v.trim().to_ascii_lowercase();
        Some(if v.is_empty() || v == "none" {
            self.unbind(i);
            Ok(())
        } else {
            self.bind(i, &v).map(|_| ()).map_err(|_| "a free key (j l m p w y, 0 1 3 5 7 9, punctuation, tab, f3 f6 f7 f8 f10 f11 f12) or none")
        })
    }

    /// The `key_<id> = "<key>"` lines that differ from the defaults.
    pub fn config_lines(&self) -> Vec<String> {
        let d = KeyMap::default();
        let mut out = Vec::new();
        for (i, a) in ACTIONS.iter().enumerate() {
            if self.extra[i] != d.extra[i] {
                let v = if self.extra[i].is_empty() { "none" } else { &self.extra[i] };
                out.push(format!("key_{} = \"{}\"", a.id, v));
            }
        }
        out
    }
}

/// The gamepad buttons that can be given a key (SDL's names).
pub const BUTTONS: [&str; 11] = ["a", "b", "x", "y", "back", "start", "guide", "leftshoulder", "rightshoulder", "leftstick", "rightstick"];

/// What each button does without a config file.
pub const PAD_DEFAULTS: [&str; 11] = ["space", "esc", "a", "e", "map", "options", "help", "k", "f", "character", "z"];

/// What a pad button sends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PadOut {
    /// A key, everywhere.
    Key(u8),
    /// A command-prompt-only code (the engine's screens).
    Command(u8),
    /// The options menu (F1).
    Options,
    None,
}

/// The meaning of a pad value: a letter or digit, space, enter, esc, an
/// arrow, options, map, help, character or none.
pub fn pad_out(v: &str) -> Option<PadOut> {
    Some(match v {
        "none" | "" => PadOut::None,
        "space" => PadOut::Key(b' '),
        "enter" => PadOut::Key(keys::ENTER),
        "esc" => PadOut::Key(keys::ESC),
        "up" => PadOut::Key(keys::UP),
        "down" => PadOut::Key(keys::DOWN),
        "left" => PadOut::Key(keys::LEFT),
        "right" => PadOut::Key(keys::RIGHT),
        "options" | "f1" => PadOut::Options,
        "map" => PadOut::Command(keys::MAP),
        "help" => PadOut::Command(keys::HELP),
        "character" => PadOut::Command(keys::CHARSHEET),
        v if v.len() == 1 && v.as_bytes()[0].is_ascii_alphanumeric() => PadOut::Key(v.as_bytes()[0].to_ascii_uppercase()),
        _ => return None,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PadMap {
    /// The value of each of [`BUTTONS`].
    pub buttons: Vec<String>,
}

impl Default for PadMap {
    fn default() -> Self {
        PadMap { buttons: PAD_DEFAULTS.iter().map(|s| s.to_string()).collect() }
    }
}

impl PadMap {
    pub fn out(&self, button: &str) -> PadOut {
        BUTTONS.iter().position(|&b| b == button).and_then(|i| pad_out(&self.buttons[i])).unwrap_or(PadOut::None)
    }

    pub fn config_key(&mut self, k: &str, v: &str) -> Option<Result<(), &'static str>> {
        let i = BUTTONS.iter().position(|&b| k.strip_prefix("pad_") == Some(b))?;
        let v = v.trim().to_ascii_lowercase();
        Some(match pad_out(&v) {
            Some(_) => {
                self.buttons[i] = v;
                Ok(())
            }
            None => Err("a letter, space, enter, esc, up, down, left, right, options, map, help, character or none"),
        })
    }

    pub fn config_lines(&self) -> Vec<String> {
        BUTTONS
            .iter()
            .enumerate()
            .filter(|&(i, _)| self.buttons[i] != PAD_DEFAULTS[i])
            .map(|(i, b)| format!("pad_{b} = \"{}\"", self.buttons[i]))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults() {
        let k = KeyMap::default();
        assert_eq!(k.code_for("tab"), Some(keys::MAP));
        assert_eq!(k.code_for("f3"), Some(keys::HELP));
        assert_eq!(k.code_for("f6"), Some(keys::CHARSHEET));
        assert_eq!(k.code_for("j"), None);
        assert!(k.config_lines().is_empty());
        // Every default extra key is free, and no original key is.
        for a in ACTIONS {
            assert!(a.default.is_empty() || is_free(a.default), "{}", a.id);
            assert!(!is_free(&a.own.to_ascii_lowercase()), "{}", a.own);
        }
        for own in ["a", "k", "z", "2", "4", "6", "8", "space", "up", "f1", "f2", "f4", "f5", "f9", "enter"] {
            assert!(!is_free(own), "{own}");
        }
    }

    #[test]
    fn binding() {
        let mut k = KeyMap::default();
        assert!(k.bind(5, "k").unwrap_err().contains("Klimb"), "an original key is refused");
        assert!(k.bind(5, "f1").is_err());
        assert!(k.bind(5, "j").is_ok());
        assert_eq!(k.code_for("j"), Some(b'A'));
        // Moving Tab from the map to attack.
        let map = ACTIONS.iter().position(|a| a.id == "map").unwrap();
        assert!(k.bind(5, "tab").unwrap().contains("moved"));
        assert_eq!(k.code_for("tab"), Some(b'A'));
        assert_eq!(k.extra[map], "");
        k.unbind(5);
        assert_eq!(k.code_for("tab"), None);
    }

    #[test]
    fn pad() {
        let p = PadMap::default();
        assert_eq!(p.out("a"), PadOut::Key(b' '));
        assert_eq!(p.out("start"), PadOut::Options);
        assert_eq!(p.out("back"), PadOut::Command(keys::MAP));
        assert_eq!(p.out("x"), PadOut::Key(b'A'));
        assert_eq!(p.out("nonsense"), PadOut::None);
        let mut p = p;
        assert_eq!(p.config_key("pad_x", "Q"), Some(Ok(())));
        assert_eq!(p.out("x"), PadOut::Key(b'Q'));
        assert!(matches!(p.config_key("pad_x", "jump"), Some(Err(_))));
        assert_eq!(p.config_lines(), ["pad_x = \"q\""]);
    }
}
