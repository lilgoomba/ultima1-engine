//! The options menu (F1, a key the original never reads): the display and
//! the sound, switched while the game waits. It is drawn as the
//! original's own text window (the black box framed in blue that shops and
//! the king use) so it never breaks the faithful look. The front end runs
//! it with the game paused, over a copy of the screen.

use crate::config::VideoChoice;
use crate::qol::{self, Qol, Scaling};
use crate::io::keys;
use crate::soundpack::SoundMode;
use crate::term::{Term, BLACK, BLUE, TEXT, WARN};

pub const SOUNDS: [SoundMode; 3] = [SoundMode::Original, SoundMode::Enhanced, SoundMode::Mute];

pub fn sound_label(s: SoundMode) -> &'static str {
    match s {
        SoundMode::Original => "Original beeps",
        SoundMode::Enhanced => "Enhanced",
        SoundMode::Mute => "Mute",
    }
}

/// The menu's rows, top to bottom.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Row {
    Video,
    Sound,
    Fullscreen,
    Scaling,
    Crt,
    Slots,
    Log,
    Turbo,
    Delay,
    Space,
    Timing,
    Automap,
    Minimap,
    Character,
    Help,
    Hints,
    Confirm,
    Repeat,
    RepeatDelay,
    RepeatRate,
    Gamepad,
    Keys,
}

pub const ROWS: [Row; 22] = [
    Row::Video,
    Row::Sound,
    Row::Fullscreen,
    Row::Scaling,
    Row::Crt,
    Row::Slots,
    Row::Log,
    Row::Turbo,
    Row::Delay,
    Row::Space,
    Row::Timing,
    Row::Automap,
    Row::Minimap,
    Row::Character,
    Row::Help,
    Row::Hints,
    Row::Confirm,
    Row::Repeat,
    Row::RepeatDelay,
    Row::RepeatRate,
    Row::Gamepad,
    Row::Keys,
];

/// How many rows the menu shows at once (it scrolls).
pub const SHOWN: usize = 11;

impl Row {
    pub fn label(self) -> &'static str {
        match self {
            Row::Video => "Video",
            Row::Sound => "Sound",
            Row::Fullscreen => "Fullscreen",
            Row::Scaling => "Scaling",
            Row::Crt => "CRT filter",
            Row::Slots => "Save slots",
            Row::Log => "Message log",
            Row::Turbo => "Turbo",
            Row::Delay => "Msg delay",
            Row::Space => "Space speed",
            Row::Timing => "CPU timing",
            Row::Automap => "Dungeon map",
            Row::Minimap => "World map",
            Row::Character => "Character",
            Row::Help => "Help screen",
            Row::Hints => "Hints",
            Row::Confirm => "Confirm",
            Row::Repeat => "Arrow repeat",
            Row::RepeatDelay => "Repeat delay",
            Row::RepeatRate => "Repeat rate",
            Row::Gamepad => "Gamepad",
            Row::Keys => "Keys",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionsMenu {
    /// Index into [`ROWS`].
    pub row: usize,
    pub video: VideoChoice,
    pub sound: SoundMode,
    /// Which of [`VideoChoice::ALL`] the game files (and pack) allow.
    pub avail: [bool; 4],
    /// The optional engine features (SPEC §22).
    pub qol: Qol,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuKey {
    Stay,
    /// The video or sound value changed (apply it now).
    Changed,
    Close,
    /// Open the key rebinding window.
    Rebind,
}

/// The engine's own windows (options, save slots, message log): black
/// over the whole map view, framed in blue like the original's shop box.
pub const PANEL: (usize, usize, usize, usize) = (8, 8, 311, 151);

/// Draw an empty panel over `t`.
pub fn panel(t: &mut Term) {
    let (x0, y0, x1, y1) = PANEL;
    t.fill(x0, y0, x1, y1, BLACK);
    let (a, b, c, d) = (x0 as i32 + 3, y0 as i32 + 3, x1 as i32 - 3, y1 as i32 - 3);
    t.line(a, b, c, b, BLUE);
    t.line(c, b, c, d, BLUE);
    t.line(c, d, a, d, BLUE);
    t.line(a, d, a, b, BLUE);
}

/// `s` centred on text row `row` of the panel.
pub fn center(t: &mut Term, row: usize, s: &str, attr: u8) {
    let n = s.chars().count().min(36);
    t.print_at_attr(2 + (36 - n) / 2, row, s, attr);
}

impl OptionsMenu {
    /// The box's rectangle on the 320x200 screen (inclusive).
    pub const BOX: (usize, usize, usize, usize) = PANEL;

    pub fn new(video: VideoChoice, sound: SoundMode, avail: [bool; 4]) -> Self {
        OptionsMenu { row: 0, video, sound, avail, qol: Qol::default() }
    }

    pub fn with_qol(mut self, qol: Qol) -> Self {
        self.qol = qol;
        self
    }

    /// The value shown for `r`.
    pub fn value(&self, r: Row) -> String {
        let on = |b: bool| if b { "On" } else { "Off" }.to_string();
        let q = &self.qol;
        match r {
            Row::Video => self.video.label().into(),
            Row::Sound => sound_label(self.sound).into(),
            Row::Fullscreen => on(q.fullscreen),
            Row::Scaling => if q.scaling == Scaling::Whole { "Whole pixels" } else { "Fill window" }.into(),
            Row::Crt => on(q.crt),
            Row::Slots => if q.slots { "On (F2 F5 F9)" } else { "Off" }.into(),
            Row::Log => if q.log { "On (PgUp)" } else { "Off" }.into(),
            Row::Turbo => if q.turbo { "On (F4)" } else { "Off (F4)" }.into(),
            Row::Delay if q.message_delay == 0 => "None".into(),
            Row::Delay if q.message_delay == 100 => "Original".into(),
            Row::Delay => format!("{}%", q.message_delay),
            Row::Space => if q.fast_space { "Fast" } else { "Original" }.into(),
            Row::Timing => if q.steady { "Steady" } else { "Original" }.into(),
            Row::Automap => if q.automap { "On (Tab)" } else { "Off" }.into(),
            Row::Minimap => if q.minimap { "On (Tab)" } else { "Off" }.into(),
            Row::Character => if q.charsheet { "On (F6)" } else { "Off" }.into(),
            Row::Help => if q.help { "On (F3)" } else { "Off" }.into(),
            Row::Hints => on(q.hints),
            Row::Confirm if q.confirm == 0 => "Off".into(),
            Row::Confirm => format!("Worth {}+", q.confirm),
            Row::Repeat => if q.repeat { "Own" } else { "System" }.into(),
            Row::RepeatDelay => format!("{} ms", q.repeat_delay),
            Row::RepeatRate if q.repeat_rate == 0 => "Game pace".to_string(),
            Row::RepeatRate => format!("{} a second", q.repeat_rate),
            Row::Gamepad => on(q.gamepad),
            Row::Keys => "Enter: change".into(),
        }
    }

    pub fn key(&mut self, k: u8) -> MenuKey {
        match k {
            keys::UP => {
                self.row = (self.row + ROWS.len() - 1) % ROWS.len();
                MenuKey::Stay
            }
            keys::DOWN => {
                self.row = (self.row + 1) % ROWS.len();
                MenuKey::Stay
            }
            keys::LEFT | keys::RIGHT | b' ' => {
                let step = if k == keys::LEFT { 3 } else { 1 };
                let q = &mut self.qol;
                match ROWS[self.row] {
                    Row::Video | Row::Sound => {}
                    Row::Fullscreen => q.fullscreen ^= true,
                    Row::Scaling => q.scaling = if q.scaling == Scaling::Whole { Scaling::Fill } else { Scaling::Whole },
                    Row::Crt => q.crt ^= true,
                    Row::Slots => q.slots ^= true,
                    Row::Log => q.log ^= true,
                    Row::Turbo => q.turbo ^= true,
                    Row::Delay => q.cycle_delay(k == keys::LEFT),
                    Row::Space => q.fast_space ^= true,
                    Row::Timing => q.steady ^= true,
                    Row::Automap => q.automap ^= true,
                    Row::Minimap => q.minimap ^= true,
                    Row::Character => q.charsheet ^= true,
                    Row::Help => q.help ^= true,
                    Row::Hints => q.hints ^= true,
                    Row::Confirm => q.confirm = qol::cycle(&qol::CONFIRM_VALUES, q.confirm, k == keys::LEFT),
                    Row::Repeat => q.repeat ^= true,
                    Row::RepeatDelay => q.repeat_delay = qol::cycle(&qol::REPEAT_DELAYS, q.repeat_delay, k == keys::LEFT),
                    Row::RepeatRate => q.repeat_rate = qol::cycle(&qol::REPEAT_RATES, q.repeat_rate, k == keys::LEFT),
                    Row::Gamepad => q.gamepad ^= true,
                    Row::Keys => return MenuKey::Rebind,
                }
                if ROWS[self.row] == Row::Video {
                    let mut i = VideoChoice::ALL.iter().position(|&v| v == self.video).unwrap_or(0);
                    for _ in 0..4 {
                        i = (i + step) % 4;
                        if self.avail[i] {
                            break;
                        }
                    }
                    let old = self.video;
                    self.video = VideoChoice::ALL[i];
                    if old == self.video { MenuKey::Stay } else { MenuKey::Changed }
                } else if ROWS[self.row] == Row::Sound {
                    let i = SOUNDS.iter().position(|&s| s == self.sound).unwrap_or(0);
                    self.sound = SOUNDS[(i + step.min(2)) % 3];
                    MenuKey::Changed
                } else {
                    MenuKey::Changed
                }
            }
            keys::ENTER if ROWS[self.row] == Row::Keys => MenuKey::Rebind,
            keys::ENTER | keys::ESC => MenuKey::Close,
            _ => MenuKey::Stay,
        }
    }

    /// The first row shown (the menu scrolls to keep the choice in view).
    pub fn top(&self) -> usize {
        self.row.saturating_sub(SHOWN - 1)
    }

    /// Draw over `t` (a copy of the game's screen).
    pub fn draw(&self, t: &mut Term) {
        panel(t);
        center(t, 2, "Options  (the game is paused)", TEXT);
        let top = self.top();
        if top > 0 {
            t.print_at_attr(36, 4, "\u{18}", TEXT);
        }
        if top + SHOWN < ROWS.len() {
            t.print_at_attr(36, 3 + SHOWN, "\u{19}", TEXT);
        }
        for (i, &r) in ROWS.iter().enumerate().skip(top).take(SHOWN) {
            let row = 4 + i - top;
            let sel = i == self.row;
            t.print_at_attr(2, row, if sel { "\u{10}" } else { " " }, WARN);
            t.print_at_attr(3, row, &format!("{}:", r.label()), TEXT);
            t.print_at_attr(16, row, &format!("<{}>", self.value(r)), if sel { WARN } else { TEXT });
        }
        center(t, 16, "Up/Down: choose  Left/Right: change", TEXT);
        center(t, 17, "F1 or Enter: resume", TEXT);
        t.cursor(1, 24);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycles_skip_missing_displays() {
        let mut m = OptionsMenu::new(VideoChoice::Ega, SoundMode::Original, [true, true, false, true]);
        assert_eq!(m.key(keys::RIGHT), MenuKey::Changed);
        assert_eq!(m.video, VideoChoice::Tandy, "CGA is skipped");
        m.key(keys::RIGHT);
        assert_eq!(m.video, VideoChoice::Enhanced);
        m.key(keys::LEFT);
        assert_eq!(m.video, VideoChoice::Tandy);
        let mut only = OptionsMenu::new(VideoChoice::Ega, SoundMode::Original, [false, true, false, false]);
        assert_eq!(only.key(keys::RIGHT), MenuKey::Stay);
        assert_eq!(m.key(keys::DOWN), MenuKey::Stay);
        m.key(keys::RIGHT);
        assert_eq!(m.sound, SoundMode::Enhanced);
        m.key(keys::RIGHT);
        assert_eq!(m.sound, SoundMode::Mute);
        m.key(keys::LEFT);
        assert_eq!(m.sound, SoundMode::Enhanced);
        assert_eq!(m.key(keys::ENTER), MenuKey::Close);
        assert_eq!(m.key(b'q'), MenuKey::Stay);
    }

    #[test]
    fn qol_rows_toggle_and_default_off() {
        let mut m = OptionsMenu::new(VideoChoice::Ega, SoundMode::Original, [true; 4]);
        assert_eq!(m.qol, Qol::default());
        m.key(keys::UP);
        assert_eq!(ROWS[m.row], Row::Keys, "wraps to the last row");
        assert_eq!(m.key(keys::ENTER), MenuKey::Rebind);
        assert_eq!(m.key(keys::RIGHT), MenuKey::Rebind);
        m.row = 10;
        assert_eq!(m.key(keys::RIGHT), MenuKey::Changed);
        assert!(m.qol.steady);
        for _ in 0..3 {
            m.key(keys::UP);
        }
        assert_eq!(ROWS[m.row], Row::Turbo);
        m.key(b' ');
        assert!(m.qol.turbo && m.value(Row::Turbo) == "On (F4)");
        m.key(keys::DOWN);
        m.key(keys::LEFT);
        assert_eq!((m.qol.message_delay, m.value(Row::Delay)), (0, "None".to_string()));
        m.row = 3;
        m.key(keys::RIGHT);
        assert_eq!(m.qol.scaling, Scaling::Fill);
        for r in [2, 4, 5, 6, 9] {
            m.row = r;
            m.key(keys::RIGHT);
        }
        assert!(m.qol.fullscreen && m.qol.crt && m.qol.slots && m.qol.log && m.qol.fast_space);
        assert_eq!((m.video, m.sound), (VideoChoice::Ega, SoundMode::Original), "the other rows are untouched");
    }

    #[test]
    fn drawn_as_a_text_window() {
        let mut t = Term::new();
        let m = OptionsMenu::new(VideoChoice::Cga, SoundMode::Mute, [true; 4]);
        m.draw(&mut t);
        assert_eq!(t.at(11, 11), BLUE, "the blue frame");
        let (x0, y0, x1, y1) = OptionsMenu::BOX;
        let f = Term::new();
        for (x, y) in [(x0 - 1, y0), (x0, y0 - 1), (x1 + 1, y1), (x1, y1 + 1)] {
            assert_eq!(t.at(x, y), f.at(x, y), "nothing outside the box");
        }
        assert!(t.row_text(2).contains("Options"));
        assert!(t.row_text(4).contains("Video:") && t.row_text(4).contains("<CGA>"));
        assert!(t.row_text(5).contains("<Mute>"));
        assert!(t.row_text(6).contains("Fullscreen:") && t.row_text(6).contains("<Off>"));
        assert!(t.row_text(14).contains("CPU timing:") && t.row_text(14).contains("<Original>"));
        assert_eq!(t.row_text(24)[..30].trim(), "", "the message window is untouched");
    }

    #[test]
    fn second_group_rows_and_scrolling() {
        let mut m = OptionsMenu::new(VideoChoice::Ega, SoundMode::Original, [true; 4]);
        for r in [Row::Automap, Row::Minimap, Row::Character, Row::Help, Row::Hints, Row::Repeat, Row::Gamepad] {
            m.row = ROWS.iter().position(|&x| x == r).unwrap();
            assert_eq!(m.key(b' '), MenuKey::Changed);
        }
        let q = m.qol;
        assert!(q.automap && q.minimap && q.charsheet && q.help && q.hints && q.repeat && q.gamepad);
        m.row = ROWS.iter().position(|&x| x == Row::Confirm).unwrap();
        m.key(keys::RIGHT);
        assert_eq!((m.qol.confirm, m.value(Row::Confirm)), (10, "Worth 10+".to_string()));
        m.key(keys::LEFT);
        m.key(keys::LEFT);
        assert_eq!(m.qol.confirm, 500);
        let mut t = Term::new();
        m.draw(&mut t);
        assert!(t.row_text(4 + m.row - m.top()).contains("Confirm:"));
        assert!(!t.row_text(4).contains("Video:"), "scrolled");
        m.row = 0;
        let mut t = Term::new();
        m.draw(&mut t);
        assert!(t.row_text(4).contains("Video:") && !(4..15).any(|y| t.row_text(y).contains("Keys:")));
    }
}
