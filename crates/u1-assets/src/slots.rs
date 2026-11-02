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

/// The thumbnail's size: the 320x200 screen, one pixel in four each way.
pub const THUMB_W: usize = 80;
pub const THUMB_H: usize = 50;

/// What the slot screen shows of a save beyond its record: our own text
/// file beside it (`<save>.info`). The 820-byte record is never touched;
/// a save without the file still lists (its details come from the record).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SaveInfo {
    /// Local date and time, "YYYY-MM-DD HH:MM".
    pub when: String,
    pub name: String,
    pub level: u16,
    pub location: String,
    /// THUMB_W x THUMB_H palette indices (empty: none).
    pub thumb: Vec<u8>,
}

/// A save's details: its sidecar, else what its record says.
pub fn details(save: &Path) -> Option<SaveInfo> {
    let r = PlayerRecord::parse(&std::fs::read(save).ok()?).ok()?;
    let mut i = SaveInfo::load(save).unwrap_or_default();
    if i.name.is_empty() {
        let (x, y) = r.position();
        i.name = r.name().to_string();
        i.level = level(&r);
        i.location = format!("Outdoors {x},{y}");
    }
    Some(i)
}

/// The sidecar of `save`.
pub fn info_path(save: &Path) -> PathBuf {
    let mut n = save.file_name().map(|s| s.to_os_string()).unwrap_or_default();
    n.push(".info");
    save.with_file_name(n)
}

/// The level shown for a record (as the character screen: 1 per 1000 exp).
pub fn level(r: &PlayerRecord) -> u16 {
    r.experience() / 1000 + 1
}

/// Every fourth pixel of the screen.
pub fn thumbnail(t: &Term) -> Vec<u8> {
    let mut v = Vec::with_capacity(THUMB_W * THUMB_H);
    for y in 0..THUMB_H {
        for x in 0..THUMB_W {
            v.push(t.at(x * 4, y * 4) & 15);
        }
    }
    v
}

impl SaveInfo {
    /// The details of a save of `r` made now (`when`), with the screen `t`.
    pub fn of(r: &PlayerRecord, t: &Term, when: &str) -> Self {
        let (x, y) = r.position();
        SaveInfo { when: when.into(), name: r.name().to_string(), level: level(r), location: format!("Outdoors {x},{y}"), thumb: thumbnail(t) }
    }

    pub fn to_text(&self) -> String {
        let hex: String = self.thumb.iter().map(|&c| char::from_digit(u32::from(c & 15), 16).unwrap()).collect();
        let mut s = format!("# ultima1-engine save details (not part of the save)\nwhen = {}\nname = {}\nlevel = {}\nlocation = {}\n", self.when, self.name, self.level, self.location);
        for row in hex.as_bytes().chunks(THUMB_W) {
            s.push_str("thumb = ");
            s.push_str(std::str::from_utf8(row).unwrap());
            s.push('\n');
        }
        s
    }

    pub fn parse(text: &str) -> Self {
        let mut i = SaveInfo::default();
        for line in text.lines() {
            let Some((k, v)) = line.split_once(" = ") else { continue };
            match k {
                "when" => i.when = v.into(),
                "name" => i.name = v.into(),
                "level" => i.level = v.parse().unwrap_or(0),
                "location" => i.location = v.into(),
                "thumb" => i.thumb.extend(v.chars().map(|c| c.to_digit(16).unwrap_or(0) as u8)),
                _ => {}
            }
        }
        if i.thumb.len() != THUMB_W * THUMB_H {
            i.thumb.clear();
        }
        i
    }

    pub fn load(save: &Path) -> Option<Self> {
        std::fs::read_to_string(info_path(save)).ok().map(|s| Self::parse(&s))
    }

    pub fn write(&self, save: &Path) -> std::io::Result<()> {
        std::fs::write(info_path(save), self.to_text())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlotScreen {
    pub row: usize,
    pub slots: Vec<(Slot, Option<String>)>,
    /// The details of each slot (from its sidecar, or its record).
    pub info: Vec<Option<SaveInfo>>,
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
        let slots: Vec<_> = Slot::all().into_iter().map(|s| (s, summary(&slot_path(base, s)))).collect();
        let info = slots.iter().map(|&(s, _)| details(&slot_path(base, s))).collect();
        SlotScreen { row: 0, slots, info, note: String::new() }
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
            let text = match (info, self.info.get(i).and_then(|d| d.as_ref())) {
                (_, Some(d)) => format!("{:<10} Lv{:>2}", d.name.chars().take(10).collect::<String>(), d.level),
                (Some(s), None) => s.chars().take(15).collect(),
                (None, None) => "(empty)".into(),
            };
            t.print_at_attr(5, row, &text, if sel { WARN } else { TEXT });
        }
        // The chosen slot's picture and details, right of the list.
        let (tx, ty) = (27 * 8, 5 * 8);
        t.fill(tx - 1, ty - 1, tx + THUMB_W, ty + THUMB_H, 8);
        t.fill(tx, ty, tx + THUMB_W - 1, ty + THUMB_H - 1, 0);
        let d = self.info.get(self.row).and_then(|d| d.as_ref());
        if let Some(th) = d.map(|d| &d.thumb).filter(|th| th.len() == THUMB_W * THUMB_H) {
            for (k, &c) in th.iter().enumerate() {
                t.plot((tx + k % THUMB_W) as i32, (ty + k / THUMB_W) as i32, c);
            }
        } else {
            t.print_at_attr(28, 7, if d.is_some() { "no" } else { "" }, TEXT);
            t.print_at_attr(28, 8, if d.is_some() { "picture" } else { "" }, TEXT);
        }
        if let Some(d) = d {
            let when = if d.when.is_empty() { "Saved: (unknown)".to_string() } else { format!("Saved {}", d.when) };
            t.print_at_attr(3, 12, &when, TEXT);
            t.print_at_attr(3, 13, &format!("{}, level {}, {}", d.name.chars().take(10).collect::<String>(), d.level, d.location), TEXT);
        }
        center(t, 15, "1 is the save Quit writes", TEXT);
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
        assert_eq!(s.info[1].as_ref().unwrap().name, "Iolo", "details from the record alone");
        assert_eq!(s.info[5], None);
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
        assert!(t.row_text(6).contains("Iolo") && t.row_text(6).contains("Lv 1"));
        assert!(t.row_text(5).contains("(empty)"));
        assert!(t.row_text(10).contains("Q"));
        assert_eq!(t.row_text(24)[..30].trim(), "", "the message window is untouched");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn sidecar_round_trip_leaves_the_record() {
        let dir = std::env::temp_dir().join(format!("u1info-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let base = dir.join("PLAYER1.U1");
        let mut r = crate::game::Game::default_record("Shamino", 40, 40);
        r.set_u16(crate::save::off::EXPERIENCE, 2500);
        std::fs::write(&base, r.as_bytes()).unwrap();
        let mut t = Term::new();
        t.fill(0, 0, 7, 7, 4);
        let i = SaveInfo::of(&r, &t, "2026-11-02 14:50");
        assert_eq!(i.level, 3);
        assert_eq!(i.thumb.len(), THUMB_W * THUMB_H);
        assert_eq!((i.thumb[0], i.thumb[1], i.thumb[2]), (4, 4, t.at(8, 0)));
        i.write(&base).unwrap();
        assert_eq!(info_path(&base), dir.join("PLAYER1.U1.info"));
        assert_eq!(SaveInfo::load(&base).unwrap(), i);
        assert_eq!(std::fs::read(&base).unwrap(), r.as_bytes(), "the record is untouched");
        let s = SlotScreen::new(&base);
        let mut t = Term::new();
        s.draw(&mut t);
        assert!(t.row_text(12).contains("2026-11-02 14:50"));
        assert!(t.row_text(13).contains("level 3"));
        assert_eq!(t.at(27 * 8, 5 * 8), 4, "the thumbnail is drawn");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
