//! The on-screen controls (Android, or `U1_TOUCH=1` on the desktop): where
//! the buttons go for a screen of a given size, and which one a touch hits.
//! Drawing and the touch events themselves are the program's (crates/u1).
//!
//! Left of the picture: a row of engine buttons (options, map, log,
//! quicksave and quickload, Enter, Esc, the system keyboard, the number pad,
//! hide) above a d-pad with Space (pass) in its middle. Right of it: the
//! letter commands, or the number pad. Hidden, only a small "Show" button
//! stays. Nothing here changes what a key does: every button only types the
//! key (or opens the screen) its desktop key would.

use crate::io::keys;

/// What a button does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Act {
    /// Types this key (a letter, a digit, Space, Enter, Esc, Backspace).
    Key(u8),
    /// An arrow: typed at once, then repeated while held.
    Arrow(u8),
    Options,
    Map,
    Log,
    QuickSave,
    QuickLoad,
    /// The system's on-screen keyboard on or off.
    Keyboard,
    /// The number pad instead of the letters, or back.
    Numbers,
    Hide,
    Show,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Button {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub label: String,
    pub act: Act,
}

impl Button {
    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.w && y < self.y + self.h
    }
}

/// The letter commands on the right, three columns of eight.
pub const LETTERS: &[u8; 24] = b"ABCDEFGHIKMNOPQRSTUVWXYZ";

/// The overlay's state: shown or not, and letters or numbers.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Overlay {
    pub hidden: bool,
    pub numbers: bool,
}

/// The size of one button for a `ww` x `wh` window with `dpi` dots per
/// inch: about 3/8 inch (big enough for a thumb), never more than an
/// eighth of the height (eight rows of letters must fit).
pub fn unit(wh: u32, dpi: f32) -> i32 {
    let want = (dpi.max(80.0) * 0.38) as i32;
    want.min(wh as i32 / 8 - 2).max(16)
}

/// The width of each side panel (three buttons and the gaps).
pub fn panel_width(wh: u32, dpi: f32) -> i32 {
    let u = unit(wh, dpi);
    3 * u + 4 * gap(u)
}

fn gap(u: i32) -> i32 {
    (u / 12).max(2)
}

/// Where the game picture goes: between the panels when that keeps at
/// least 85% of the size it would have over the whole window, else over
/// the whole window (the buttons are drawn over it, see-through).
pub fn game_area(ww: u32, wh: u32, dpi: f32, o: Overlay) -> (i32, i32, u32, u32) {
    if o.hidden {
        return (0, 0, ww, wh);
    }
    let p = panel_width(wh, dpi);
    let inner = ww as i32 - 2 * p;
    let full = (ww as f32 / 320.0).min(wh as f32 / 200.0);
    let between = (inner.max(0) as f32 / 320.0).min(wh as f32 / 200.0);
    if between >= 0.85 * full {
        (p, 0, inner as u32, wh)
    } else {
        (0, 0, ww, wh)
    }
}

/// The buttons for a `ww` x `wh` window.
pub fn layout(ww: u32, wh: u32, dpi: f32, o: Overlay) -> Vec<Button> {
    let u = unit(wh, dpi);
    let g = gap(u);
    let (ww, wh) = (ww as i32, wh as i32);
    let mut v = Vec::new();
    let mut b = |x: i32, y: i32, w: i32, h: i32, label: &str, act: Act| v.push(Button { x, y, w, h, label: label.to_string(), act });
    if o.hidden {
        b(g, g, u, u * 2 / 3, "Show", Act::Show);
        return v;
    }
    let cell = |i: i32| g + i * (u + g);
    // Left: three rows of engine buttons at the top.
    let top = [
        ("F1", Act::Options),
        ("Map", Act::Map),
        ("Log", Act::Log),
        ("QSav", Act::QuickSave),
        ("QLd", Act::QuickLoad),
        ("Kbd", Act::Keyboard),
        ("Ent", Act::Key(keys::ENTER)),
        ("Esc", Act::Key(keys::ESC)),
        ("Hide", Act::Hide),
    ];
    let sh = u * 3 / 4;
    for (i, (l, a)) in top.iter().enumerate() {
        let (c, r) = (i as i32 % 3, i as i32 / 3);
        b(cell(c), g + r * (sh + g), u, sh, l, *a);
    }
    // Left: the d-pad at the bottom, Space in its middle.
    let y0 = wh - 3 * (u + g);
    let pad = [(1, 0, "^", Act::Arrow(keys::UP)), (0, 1, "<", Act::Arrow(keys::LEFT)), (1, 1, "Spc", Act::Key(b' ')), (2, 1, ">", Act::Arrow(keys::RIGHT)), (1, 2, "v", Act::Arrow(keys::DOWN))];
    for (c, r, l, a) in pad {
        b(cell(c), y0 + r * (u + g), u, u, l, a);
    }
    // Right: the letters (eight rows of three) or the number pad.
    let x0 = ww - 3 * (u + g);
    if o.numbers {
        let nums = ["7", "8", "9", "4", "5", "6", "1", "2", "3", "Bks", "0", "Ent"];
        let h = ((wh - 6 * g) / 5).min(u * 3 / 2);
        for (i, l) in nums.iter().enumerate() {
            let (c, r) = (i as i32 % 3, i as i32 / 3);
            let a = match *l {
                "Bks" => Act::Key(keys::BACKSPACE),
                "Ent" => Act::Key(keys::ENTER),
                d => Act::Key(d.as_bytes()[0]),
            };
            b(x0 + c * (u + g), g + r * (h + g), u, h, l, a);
        }
        b(x0, g + 4 * (h + g), 3 * u + 2 * g, h, "ABC", Act::Numbers);
    } else {
        let h = (wh - 10 * g) / 9;
        for (i, &c) in LETTERS.iter().enumerate() {
            let (col, row) = (i as i32 % 3, i as i32 / 3);
            b(x0 + col * (u + g), g + row * (h + g), u, h, &(c as char).to_string(), Act::Key(c));
        }
        b(x0, g + 8 * (h + g), 3 * u + 2 * g, h, "123", Act::Numbers);
    }
    v
}

/// The button under (x, y), if any.
pub fn hit(buttons: &[Button], x: i32, y: i32) -> Option<&Button> {
    buttons.iter().find(|b| b.contains(x, y))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn overlap(a: &Button, b: &Button) -> bool {
        a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h
    }

    #[test]
    fn buttons_fit_and_never_overlap() {
        for (w, h, dpi) in [(2400, 1080, 400.0), (1920, 1080, 160.0), (2048, 1536, 264.0), (1280, 800, 213.0), (800, 480, 160.0)] {
            for o in [Overlay::default(), Overlay { numbers: true, ..Default::default() }, Overlay { hidden: true, ..Default::default() }] {
                let v = layout(w, h, dpi, o);
                for (i, a) in v.iter().enumerate() {
                    assert!(a.x >= 0 && a.y >= 0 && a.x + a.w <= w as i32 && a.y + a.h <= h as i32, "{w}x{h} {o:?} {a:?}");
                    assert!(a.w >= 16 && a.h >= 12, "{a:?}");
                    for b in &v[i + 1..] {
                        assert!(!overlap(a, b), "{w}x{h} {o:?}: {a:?} / {b:?}");
                    }
                }
            }
        }
    }

    #[test]
    fn every_command_letter_and_engine_button_is_there() {
        let v = layout(2400, 1080, 400.0, Overlay::default());
        for &c in b"ABCDEFGHIKMNOPQRSTUVWXZ " {
            assert!(v.iter().any(|b| b.act == Act::Key(c)), "{}", c as char);
        }
        for a in [Act::Options, Act::Map, Act::Log, Act::QuickSave, Act::QuickLoad, Act::Keyboard, Act::Numbers, Act::Hide] {
            assert!(v.iter().any(|b| b.act == a), "{a:?}");
        }
        for k in [keys::UP, keys::DOWN, keys::LEFT, keys::RIGHT] {
            assert!(v.iter().any(|b| b.act == Act::Arrow(k)));
        }
        let n = layout(2400, 1080, 400.0, Overlay { numbers: true, ..Default::default() });
        for &c in b"0123456789" {
            assert!(n.iter().any(|b| b.act == Act::Key(c)), "{}", c as char);
        }
        assert!(n.iter().any(|b| b.act == Act::Key(keys::BACKSPACE)) && n.iter().any(|b| b.act == Act::Key(keys::ENTER)));
    }

    #[test]
    fn a_wide_phone_keeps_the_picture_between_the_panels() {
        let o = Overlay::default();
        let (x, _, w, h) = game_area(2400, 1080, 400.0, o);
        assert_eq!(h, 1080);
        assert!(x > 0 && w >= 1400, "{x} {w}");
        // No button lies over the picture's middle.
        let v = layout(2400, 1080, 400.0, o);
        assert!(hit(&v, 1200, 540).is_none());
        // A 4:3 tablet draws it over the whole screen.
        assert_eq!(game_area(2048, 1536, 264.0, o), (0, 0, 2048, 1536));
        assert_eq!(game_area(2400, 1080, 400.0, Overlay { hidden: true, ..o }), (0, 0, 2400, 1080));
    }
}
