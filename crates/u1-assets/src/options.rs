//! The options menu (F1, a key the original never reads): the display and
//! the sound, switched while the game waits. It is drawn as the
//! original's own text window (the black box framed in blue that shops and
//! the king use) so it never breaks the faithful look. The front end runs
//! it with the game paused, over a copy of the screen.

use crate::config::VideoChoice;
use crate::io::keys;
use crate::soundpack::SoundMode;
use crate::term::{Term, BLUE, TEXT, WARN};

pub const SOUNDS: [SoundMode; 3] = [SoundMode::Original, SoundMode::Enhanced, SoundMode::Mute];

pub fn sound_label(s: SoundMode) -> &'static str {
    match s {
        SoundMode::Original => "Original beeps",
        SoundMode::Enhanced => "Enhanced",
        SoundMode::Mute => "Mute",
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionsMenu {
    /// 0: video, 1: sound.
    pub row: usize,
    pub video: VideoChoice,
    pub sound: SoundMode,
    /// Which of [`VideoChoice::ALL`] the game files (and pack) allow.
    pub avail: [bool; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuKey {
    Stay,
    /// The video or sound value changed (apply it now).
    Changed,
    Close,
}

impl OptionsMenu {
    pub fn new(video: VideoChoice, sound: SoundMode, avail: [bool; 4]) -> Self {
        OptionsMenu { row: 0, video, sound, avail }
    }

    pub fn key(&mut self, k: u8) -> MenuKey {
        match k {
            keys::UP | keys::DOWN => {
                self.row ^= 1;
                MenuKey::Stay
            }
            keys::LEFT | keys::RIGHT | b' ' => {
                let step = if k == keys::LEFT { 3 } else { 1 };
                if self.row == 0 {
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
                } else {
                    let i = SOUNDS.iter().position(|&s| s == self.sound).unwrap_or(0);
                    self.sound = SOUNDS[(i + step.min(2)) % 3];
                    MenuKey::Changed
                }
            }
            keys::ENTER | keys::ESC => MenuKey::Close,
            _ => MenuKey::Stay,
        }
    }

    /// Draw over `t` (a copy of the game's screen).
    pub fn draw(&self, t: &mut Term) {
        t.boxed(BLUE);
        let center = |t: &mut Term, row: usize, s: &str, attr: u8| {
            t.print_at_attr(5 + (30 - s.len().min(30)) / 2, row, s, attr);
        };
        center(t, 4, "Options", TEXT);
        center(t, 5, "(the game is paused)", TEXT);
        let lines = [("Video", self.video.label()), ("Sound", sound_label(self.sound))];
        for (i, (name, val)) in lines.iter().enumerate() {
            let row = 7 + i * 2;
            let sel = i == self.row;
            t.print_at_attr(6, row, if sel { "\u{10}" } else { " " }, WARN);
            t.print_at_attr(8, row, &format!("{name}:"), TEXT);
            t.print_at_attr(15, row, &format!("<{val}>"), if sel { WARN } else { TEXT });
        }
        center(t, 12, "Up/Down: choose", TEXT);
        center(t, 13, "Left/Right: change", TEXT);
        center(t, 14, "F1 or Enter: resume", TEXT);
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
    fn drawn_as_a_text_window() {
        let mut t = Term::new();
        let m = OptionsMenu::new(VideoChoice::Cga, SoundMode::Mute, [true; 4]);
        m.draw(&mut t);
        assert_eq!(t.at(34, 26), BLUE, "the shops' frame");
        assert!(t.row_text(4).contains("Options"));
        assert!(t.row_text(7).contains("Video:") && t.row_text(7).contains("<CGA>"));
        assert!(t.row_text(9).contains("<Mute>"));
        assert_eq!(t.row_text(24)[..30].trim(), "", "the message window is untouched");
    }
}
