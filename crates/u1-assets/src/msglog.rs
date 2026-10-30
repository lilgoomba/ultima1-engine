//! The message log viewer (an optional engine feature, SPEC §22.3): the
//! lines that scrolled through the four-line message window, newest at the
//! bottom, paged with PgUp/PgDn. The game's own text flow is unchanged;
//! [`Term::log`] only collects what scrolls off.

use crate::options::{center, panel};
use crate::term::{Term, TEXT, WARN};

/// How many lines are kept.
pub const KEEP: usize = 500;
/// Lines shown per page.
pub const PAGE: usize = 12;

/// Front-end key codes for the viewer (no key of the original's).
pub mod keys {
    pub const PGUP: u8 = 0xF0;
    pub const PGDN: u8 = 0xF1;
    pub const HOME: u8 = 0xF2;
    pub const END: u8 = 0xF3;
}

/// Start keeping the log on `t` (no-op if it already is).
pub fn enable(t: &mut Term) {
    if t.log.is_none() {
        t.log = Some(Box::default());
    }
}

/// A message-window row as kept: without column 0 (the original starts
/// its lines at column 1) and trailing blanks.
pub fn tidy(row: &str) -> String {
    row.strip_prefix(' ').unwrap_or(row).trim_end().to_string()
}

/// Every line: the kept ones, then what the window shows now.
pub fn lines(t: &Term) -> Vec<String> {
    let mut v: Vec<String> = t.log.as_deref().map(|l| l.iter().cloned().collect()).unwrap_or_default();
    v.extend(t.messages().iter().filter(|l| !l.trim().is_empty()).map(|l| tidy(l)));
    v
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogView {
    pub lines: Vec<String>,
    /// Lines scrolled back from the newest.
    pub back: usize,
}

impl LogView {
    /// Opens on the newest page (more lines than the message window
    /// holds; PgUp goes further back).
    pub fn new(t: &Term) -> Self {
        LogView { lines: lines(t), back: 0 }
    }

    fn max_back(&self) -> usize {
        self.lines.len().saturating_sub(PAGE)
    }

    /// False when the viewer closes.
    pub fn key(&mut self, k: u8) -> bool {
        match k {
            keys::PGUP | crate::io::keys::UP => {
                let step = if k == keys::PGUP { PAGE } else { 1 };
                self.back = (self.back + step).min(self.max_back());
            }
            keys::PGDN | crate::io::keys::DOWN => {
                if self.back == 0 && k == keys::PGDN {
                    return false;
                }
                let step = if k == keys::PGDN { PAGE } else { 1 };
                self.back = self.back.saturating_sub(step);
            }
            keys::HOME => self.back = self.max_back(),
            keys::END => self.back = 0,
            crate::io::keys::ESC | crate::io::keys::ENTER => return false,
            _ => {}
        }
        true
    }

    /// The lines on screen.
    pub fn page(&self) -> &[String] {
        let end = self.lines.len() - self.back.min(self.lines.len());
        &self.lines[end.saturating_sub(PAGE)..end]
    }

    pub fn draw(&self, t: &mut Term) {
        panel(t);
        let total = self.lines.len();
        let end = total - self.back.min(total);
        center(t, 2, &format!("Message log  {}-{} of {}", end.saturating_sub(PAGE).min(end) + usize::from(end > 0), end, total), TEXT);
        for (i, l) in self.page().iter().enumerate() {
            t.print_at_attr(4, 4 + i, l, TEXT);
        }
        if self.lines.is_empty() {
            center(t, 9, "(nothing yet)", TEXT);
        }
        center(t, 17, "PgUp/PgDn, Up/Down: scroll  Esc: back", if self.back > 0 { WARN } else { TEXT });
        t.cursor(1, 24);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collects_only_when_on_and_never_changes_the_screen() {
        let mut off = Term::new();
        let mut on = Term::new();
        enable(&mut on);
        for i in 0..30 {
            for t in [&mut off, &mut on] {
                t.println(&format!("line {i}"));
            }
        }
        assert!(off.log.is_none());
        assert_eq!(off.px, on.px, "the window is drawn the same");
        let l = lines(&on);
        assert_eq!(l.len(), 30);
        assert_eq!((l[0].as_str(), l[29].as_str()), ("line 0", "line 29"));
        let mut t = Term::new();
        enable(&mut t);
        for i in 0..KEEP + 7 {
            t.println(&format!("{i}"));
        }
        assert_eq!(t.log.as_ref().unwrap().len(), KEEP);
    }

    #[test]
    fn paging() {
        let mut t = Term::new();
        enable(&mut t);
        for i in 0..40 {
            t.println(&format!("line {i}"));
        }
        let mut v = LogView::new(&t);
        assert_eq!(v.page().len(), PAGE);
        assert_eq!(v.page().last().unwrap(), "line 39", "opens on the newest");
        assert!(v.key(keys::PGUP));
        assert_eq!(v.page()[0], "line 16");
        assert!(v.key(keys::PGUP));
        assert_eq!(v.page()[0], "line 4");
        assert!(v.key(keys::PGUP));
        assert_eq!(v.page()[0], "line 0", "stops at the oldest");
        assert!(v.key(keys::END));
        assert_eq!(v.page().last().unwrap(), "line 39");
        assert!(!v.key(keys::PGDN), "PgDn at the newest closes");
        assert!(v.key(crate::io::keys::UP));
        assert_eq!(v.page().last().unwrap(), "line 38");
        let mut c = t.clone();
        v.draw(&mut c);
        assert!(c.row_text(2).contains("of 40"));
        assert!(c.row_text(15).contains("line 38"));
        assert_eq!(c.messages(), t.messages(), "the message window is untouched");
        assert!(!v.key(crate::io::keys::ESC));
    }
}
