//! Save slots and quicksave (an optional engine feature, SPEC §22.2). Every
//! slot is an ordinary 820-byte player record, the same bytes the
//! original's Quit command writes, so the original can read any of them
//! (renamed to `PLAYERn.U1`). Slot 1 *is* the character's own file, the one
//! Quit saves to; the others sit beside it.

use std::path::{Path, PathBuf};

use crate::io::keys;
use crate::options::{center, panel};
use crate::save::PlayerRecord;
use crate::term::{Term, TEXT, WARN};

/// Slots 1..=SLOTS, plus the quicksave slot.
pub const SLOTS: u8 = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Slot {
    N(u8),
    Quick,
}

impl Slot {
    /// The slots in the screen's order.
    pub fn all() -> Vec<Slot> {
        (1..=SLOTS).map(Slot::N).chain([Slot::Quick]).collect()
    }
    pub fn label(self) -> String {
        match self {
            Slot::N(n) => n.to_string(),
            Slot::Quick => "Q".into(),
        }
    }
}

/// What the player asked for (F2's screen, F5, F9).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveReq {
    Save(Slot),
    Load(Slot),
}

/// The file of `slot` for a character saved to `base`: `base` itself for
/// slot 1, `<stem>-slot<n>.<ext>` or `<stem>-quick.<ext>` beside it.
pub fn slot_path(base: &Path, slot: Slot) -> PathBuf {
    let stem = base.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "PLAYER".into());
    let ext = base.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
    let name = match slot {
        Slot::N(1) => return base.to_path_buf(),
        Slot::N(n) => format!("{stem}-slot{n}{ext}"),
        Slot::Quick => format!("{stem}-quick{ext}"),
    };
    base.with_file_name(name)
}

/// One line of the slot screen: the character in the file, if any.
pub fn summary(path: &Path) -> Option<String> {
    let b = std::fs::read(path).ok()?;
    let Ok(r) = PlayerRecord::parse(&b) else { return Some("(not a player file)".into()) };
    let name: String = r.name().chars().take(13).collect();
    Some(format!("{name:<13} H{:>4} F{:>4} C{:>4}", r.hits(), r.food(), r.coin()))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlotScreen {
    pub row: usize,
    pub slots: Vec<(Slot, Option<String>)>,
    /// A line under the list (the last result).
    pub note: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotKey {
    Stay,
    Close,
    Do(SaveReq),
}

impl SlotScreen {
    /// The slots of the character saved to `base`.
    pub fn new(base: &Path) -> Self {
        let slots = Slot::all().into_iter().map(|s| (s, summary(&slot_path(base, s)))).collect();
        SlotScreen { row: 0, slots, note: String::new() }
    }

    pub fn key(&mut self, k: u8) -> SlotKey {
        let n = self.slots.len();
        let slot = self.slots[self.row].0;
        match k.to_ascii_uppercase() {
            keys::UP => self.row = (self.row + n - 1) % n,
            keys::DOWN => self.row = (self.row + 1) % n,
            b'S' => return SlotKey::Do(SaveReq::Save(slot)),
            b'L' | keys::ENTER if self.slots[self.row].1.is_some() => return SlotKey::Do(SaveReq::Load(slot)),
            b'L' | keys::ENTER => self.note = "That slot is empty.".into(),
            keys::ESC => return SlotKey::Close,
            b'1'..=b'5' => self.row = usize::from(k - b'1'),
            b'Q' => self.row = n - 1,
            _ => {}
        }
        SlotKey::Stay
    }

    pub fn draw(&self, t: &mut Term) {
        panel(t);
        center(t, 2, "Save slots", TEXT);
        center(t, 3, "(the game is paused)", TEXT);
        for (i, (s, info)) in self.slots.iter().enumerate() {
            let row = 5 + i;
            let sel = i == self.row;
            t.print_at_attr(2, row, if sel { "\u{10}" } else { " " }, WARN);
            t.print_at_attr(3, row, &s.label(), TEXT);
            let text = info.clone().unwrap_or_else(|| "(empty)".into());
            t.print_at_attr(5, row, &text, if sel { WARN } else { TEXT });
        }
        center(t, 12, "1 is the character's own save", TEXT);
        center(t, 13, "(the one Quit writes)", TEXT);
        if !self.note.is_empty() {
            center(t, 14, &self.note, WARN);
        }
        center(t, 16, "S: save  L: load  Esc: back", TEXT);
        t.cursor(1, 24);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_beside_the_save() {
        let b = Path::new("/r/PLAYER2.U1");
        assert_eq!(slot_path(b, Slot::N(1)), b);
        assert_eq!(slot_path(b, Slot::N(3)), Path::new("/r/PLAYER2-slot3.U1"));
        assert_eq!(slot_path(b, Slot::Quick), Path::new("/r/PLAYER2-quick.U1"));
        assert_eq!(slot_path(Path::new("hero"), Slot::N(2)), Path::new("hero-slot2"));
        assert_eq!(Slot::all().len(), 6);
    }

    #[test]
    fn screen_keys_and_lines() {
        let dir = std::env::temp_dir().join(format!("u1slots-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let base = dir.join("PLAYER1.U1");
        let mut r = crate::game::Game::default_record("Iolo", 40, 40);
        r.set_u16(crate::save::off::COIN, 321);
        std::fs::write(slot_path(&base, Slot::N(2)), r.as_bytes()).unwrap();
        std::fs::write(slot_path(&base, Slot::Quick), b"short").unwrap();
        let mut s = SlotScreen::new(&base);
        assert_eq!(s.slots[0].1, None);
        assert!(s.slots[1].1.as_deref().unwrap().starts_with("Iolo") && s.slots[1].1.as_deref().unwrap().ends_with("C 321"));
        assert_eq!(s.slots[5].1.as_deref(), Some("(not a player file)"));
        assert_eq!(s.key(b'l'), SlotKey::Stay, "slot 1 is empty");
        assert_eq!(s.note, "That slot is empty.");
        assert_eq!(s.key(b's'), SlotKey::Do(SaveReq::Save(Slot::N(1))));
        s.key(keys::DOWN);
        assert_eq!(s.key(b'L'), SlotKey::Do(SaveReq::Load(Slot::N(2))));
        s.key(keys::UP);
        s.key(keys::UP);
        assert_eq!(s.row, 5, "wraps to the quicksave");
        assert_eq!(s.key(b'3'), SlotKey::Stay);
        assert_eq!(s.row, 2);
        assert_eq!(s.key(keys::ESC), SlotKey::Close);
        let mut t = Term::new();
        s.draw(&mut t);
        assert!(t.row_text(6).contains("Iolo"));
        assert!(t.row_text(5).contains("(empty)"));
        assert!(t.row_text(10).contains("Q"));
        assert_eq!(t.row_text(24)[..30].trim(), "", "the message window is untouched");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
