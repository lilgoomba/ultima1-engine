//! Optional quality-of-life settings (docs/ROADMAP.md, SPEC §22). Every one
//! is off by default, and off means the engine behaves exactly as it does
//! without them: the same screens, the same timings, the same saves. None
//! of them changes the game's rules, its random sequence or the save
//! format. The front end applies them; this module holds the settings,
//! their timing arithmetic and the pure parts (the CRT filter) so they can
//! be tested without a window.

/// How the screen fills the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Scaling {
    /// Whole multiples only, letterboxed: every pixel the same size (the
    /// engine's behaviour without the option).
    #[default]
    Whole,
    /// As large as the window allows at the right shape (pixels may differ
    /// by one in size).
    Fill,
}

impl Scaling {
    pub fn as_str(self) -> &'static str {
        match self {
            Scaling::Whole => "whole",
            Scaling::Fill => "fill",
        }
    }
    pub fn parse(s: &str) -> Option<Scaling> {
        match s.trim().to_ascii_lowercase().as_str() {
            "whole" | "integer" => Some(Scaling::Whole),
            "fill" => Some(Scaling::Fill),
            _ => None,
        }
    }
}

/// The message-delay choices, in per cent of the original's.
pub const MESSAGE_DELAYS: [u8; 4] = [100, 50, 25, 0];

/// The settings. `Qol::default()` is the faithful engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Qol {
    /// Fullscreen (also Alt+Enter).
    pub fullscreen: bool,
    pub scaling: Scaling,
    /// The CRT scanline filter (never in screenshots unless asked for).
    pub crt: bool,
    /// Save slots (F2) and quicksave/quickload (F5/F9).
    pub slots: bool,
    /// The message log (PgUp).
    pub log: bool,
    /// Everything runs four times as fast (also F4).
    pub turbo: bool,
    /// The waits after messages, in per cent of the original's.
    pub message_delay: u8,
    /// Space flight and combat wait half as long.
    pub fast_space: bool,
    /// The original's processor-bound waits replaced by comfortable fixed
    /// ones (a speaker effect no longer freezes the game).
    pub steady: bool,
    // The second group (SPEC §23). The game-side screens open at the
    // command prompt only and are drawn over a copy of the screen.
    /// The dungeon automap (Tab in a dungeon).
    pub automap: bool,
    /// The overworld mini-map (Tab outdoors and in towns).
    pub minimap: bool,
    /// The character screen (F6).
    pub charsheet: bool,
    /// The command reference (F3).
    pub help: bool,
    /// Short hints in the message window when a command prompts.
    pub hints: bool,
    /// Ask before selling or dropping something worth at least this much
    /// (0: never ask).
    pub confirm: u16,
    /// Our own auto-repeat for a held arrow key (and pad direction).
    pub repeat: bool,
    /// Milliseconds before the repeat starts.
    pub repeat_delay: u16,
    /// Steps per second while it repeats (0: the original's pace, one
    /// step per pass of its main loop).
    pub repeat_rate: u8,
    /// Gamepads (SDL game controllers).
    pub gamepad: bool,
}

impl Default for Qol {
    fn default() -> Self {
        Qol {
            fullscreen: false,
            scaling: Scaling::Whole,
            crt: false,
            slots: false,
            log: false,
            turbo: false,
            message_delay: 100,
            fast_space: false,
            steady: false,
            automap: false,
            minimap: false,
            charsheet: false,
            help: false,
            hints: false,
            confirm: 0,
            repeat: false,
            repeat_delay: 250,
            repeat_rate: 0,
            gamepad: false,
        }
    }
}

/// The confirmation thresholds the options menu cycles through (0: off).
pub const CONFIRM_VALUES: [u16; 6] = [0, 10, 50, 100, 250, 500];
/// The auto-repeat delays (ms) and rates (steps per second) offered.
pub const REPEAT_DELAYS: [u16; 4] = [250, 400, 600, 800];
/// 0 is the game's own pace: one step per pass of the original main loop.
pub const REPEAT_RATES: [u8; 5] = [0, 4, 8, 12, 16];

/// The next of `list` after `v` (`back`: before), the first if `v` is
/// not in it.
pub fn cycle<T: Copy + PartialEq>(list: &[T], v: T, back: bool) -> T {
    let n = list.len();
    match list.iter().position(|&x| x == v) {
        Some(i) => list[if back { (i + n - 1) % n } else { (i + 1) % n }],
        None => list[0],
    }
}

/// What kind of wait the game asks the front end for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wait {
    /// A main-loop key poll (130 ms) or a timed pause in general.
    Poll,
    /// A pause after a message (`Io::message_pause`, `wait_or_key`).
    Message,
    /// A speaker effect holding the game while it plays.
    Sound,
}

/// How much a turbo wait is shortened by.
pub const TURBO: f64 = 4.0;

impl Qol {
    /// The factor a wait of kind `w` is multiplied by (`in_space`: during
    /// space flight). 1.0 for every wait with the settings off.
    pub fn wait_factor(&self, w: Wait, in_space: bool) -> f64 {
        let mut f = if self.turbo { 1.0 / TURBO } else { 1.0 };
        match w {
            Wait::Message => f *= f64::from(self.message_delay.min(100)) / 100.0,
            Wait::Sound if self.steady => f = 0.0,
            _ => {}
        }
        if in_space && self.fast_space && w != Wait::Sound {
            f *= 0.5;
        }
        f
    }

    /// Whether a thing worth `v` needs a yes before it is sold or dropped.
    pub fn must_confirm(&self, v: i32) -> bool {
        self.confirm > 0 && v >= i32::from(self.confirm)
    }

    /// The auto-repeat's first wait and its step. `poll` is one pass of
    /// the original main loop at the current speed (the original moved a
    /// held key at most once a pass); the step is never shorter.
    pub fn repeat_timing(&self, poll: std::time::Duration) -> (std::time::Duration, std::time::Duration) {
        use std::time::Duration;
        let rate = if self.repeat_rate == 0 { poll } else { Duration::from_millis(1000 / u64::from(self.repeat_rate)) };
        (Duration::from_millis(u64::from(self.repeat_delay)), rate.max(poll))
    }

    /// The next message delay in [`MESSAGE_DELAYS`] (`back`: the one before).
    pub fn cycle_delay(&mut self, back: bool) {
        let i = MESSAGE_DELAYS.iter().position(|&d| d == self.message_delay).unwrap_or(0);
        let n = MESSAGE_DELAYS.len();
        self.message_delay = MESSAGE_DELAYS[if back { (i + n - 1) % n } else { (i + 1) % n }];
    }
}

/// How many output rows (and columns) a source pixel becomes under the CRT
/// filter: a 320x200 frame is drawn at 3x, the enhanced 640x400 at 2x.
pub fn crt_factor(w: usize) -> usize {
    if w <= 320 {
        3
    } else {
        2
    }
}

/// The CRT filter: `rgb` (`w` x `h`) enlarged `k` times with dark gaps
/// between the scanlines and a slight horizontal glow. A pure picture
/// effect; the frame it is given is not changed.
pub fn crt(rgb: &[u8], w: usize, h: usize, k: usize) -> Vec<u8> {
    let k = k.max(2);
    let ow = w * k;
    let mut out = vec![0u8; ow * h * k * 3];
    // Brightness of the k output rows of one scanline (per 256).
    let rows: &[u32] = if k == 2 { &[256, 112] } else { &[256, 236, 96] };
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) * 3;
            let l = if x > 0 { i - 3 } else { i };
            let r = if x + 1 < w { i + 3 } else { i };
            for sx in 0..k {
                // The pixel, a little of its neighbour on that side.
                let n = if sx == 0 {
                    l
                } else if sx == k - 1 {
                    r
                } else {
                    i
                };
                let px: [u32; 3] = std::array::from_fn(|c| (u32::from(rgb[i + c]) * 7 + u32::from(rgb[n + c])) / 8);
                for (sy, &b) in rows.iter().enumerate().take(k) {
                    let o = ((y * k + sy) * ow + x * k + sx) * 3;
                    for c in 0..3 {
                        out[o + c] = (px[c] * b / 256).min(255) as u8;
                    }
                }
            }
        }
    }
    out
}

/// Where a `pw`x`ph` picture goes in a `ww`x`wh` window: centred, black
/// around it. `Whole` uses the largest whole multiple that fits (or, in a
/// window smaller than the picture, the largest size at the right shape);
/// `Fill` the largest size at the right shape. Returns (x, y, w, h).
pub fn fit(ww: u32, wh: u32, pw: u32, ph: u32, s: Scaling) -> (i32, i32, u32, u32) {
    let (ww, wh, pw, ph) = (ww.max(1), wh.max(1), pw.max(1), ph.max(1));
    let n = (ww / pw).min(wh / ph);
    let (w, h) = if s == Scaling::Whole && n >= 1 {
        (pw * n, ph * n)
    } else if ww as u64 * ph as u64 <= wh as u64 * pw as u64 {
        // Width-limited.
        (ww, ((ww as u64 * ph as u64) / pw as u64).max(1) as u32)
    } else {
        (((wh as u64 * pw as u64) / ph as u64).max(1) as u32, wh)
    };
    (((ww - w) / 2) as i32, ((wh - h) / 2) as i32, w, h)
}

#[cfg(test)]
mod tests {
    #[test]
    fn fit_centres_and_letterboxes() {
        // Whole multiples, centred, at odd sizes.
        assert_eq!(fit(1024, 557, 320, 200, Scaling::Whole), (192, 78, 640, 400));
        assert_eq!(fit(960, 600, 320, 200, Scaling::Whole), (0, 0, 960, 600));
        assert_eq!(fit(1001, 999, 640, 400, Scaling::Whole), (180, 299, 640, 400));
        // Smaller than the picture: shrunk at the right shape.
        assert_eq!(fit(500, 600, 640, 400, Scaling::Whole), (0, 144, 500, 312));
        // Fill: as large as fits.
        assert_eq!(fit(1024, 557, 320, 200, Scaling::Fill), (66, 0, 891, 557));
        assert_eq!(fit(800, 900, 320, 200, Scaling::Fill), (0, 200, 800, 500));
    }

    use super::*;

    #[test]
    fn off_changes_no_wait() {
        let q = Qol::default();
        for w in [Wait::Poll, Wait::Message, Wait::Sound] {
            for s in [false, true] {
                assert_eq!(q.wait_factor(w, s), 1.0, "{w:?} in space {s}");
            }
        }
    }

    #[test]
    fn speed_settings() {
        let q = Qol { turbo: true, ..Default::default() };
        assert_eq!(q.wait_factor(Wait::Poll, false), 0.25);
        assert_eq!(q.wait_factor(Wait::Sound, false), 0.25);
        let q = Qol { message_delay: 25, ..Default::default() };
        assert_eq!(q.wait_factor(Wait::Message, false), 0.25);
        assert_eq!(q.wait_factor(Wait::Poll, false), 1.0, "only the message waits");
        let q = Qol { message_delay: 0, turbo: true, ..Default::default() };
        assert_eq!(q.wait_factor(Wait::Message, false), 0.0);
        let q = Qol { fast_space: true, ..Default::default() };
        assert_eq!((q.wait_factor(Wait::Poll, true), q.wait_factor(Wait::Poll, false)), (0.5, 1.0));
        let q = Qol { steady: true, ..Default::default() };
        assert_eq!(q.wait_factor(Wait::Sound, false), 0.0, "an effect no longer holds the game");
        assert_eq!(q.wait_factor(Wait::Poll, false), 1.0);
        let mut q = Qol::default();
        let seen: Vec<u8> = (0..4)
            .map(|_| {
                q.cycle_delay(false);
                q.message_delay
            })
            .collect();
        assert_eq!(seen, [50, 25, 0, 100]);
        q.cycle_delay(true);
        assert_eq!(q.message_delay, 0);
    }

    #[test]
    fn second_group_defaults_off() {
        let q = Qol::default();
        assert!(!q.automap && !q.minimap && !q.charsheet && !q.help && !q.hints && !q.repeat && !q.gamepad);
        assert!(!q.must_confirm(30000), "never asks with the setting off");
        let q = Qol { confirm: 100, ..q };
        assert!(q.must_confirm(100) && !q.must_confirm(99));
        assert_eq!(cycle(&CONFIRM_VALUES, 0, false), 10);
        assert_eq!(cycle(&CONFIRM_VALUES, 0, true), 500);
        assert_eq!(cycle(&REPEAT_RATES, 3, false), 0, "unknown value: the first");
        let (d, s) = Qol { repeat_delay: 400, repeat_rate: 8, ..q }.repeat_timing(std::time::Duration::from_millis(100));
        assert_eq!((d.as_millis(), s.as_millis()), (400, 125));
    }

    #[test]
    fn crt_scanlines() {
        // A flat grey 2x2 picture: bright first rows, dark gap rows.
        let rgb = vec![200u8; 2 * 2 * 3];
        let out = crt(&rgb, 2, 2, 3);
        assert_eq!(out.len(), 6 * 6 * 3);
        let at = |x: usize, y: usize| out[(y * 6 + x) * 3];
        assert_eq!(at(0, 0), 200);
        assert!(at(0, 2) < 90 && at(0, 2) > 60, "{}", at(0, 2));
        assert_eq!(at(4, 3), 200, "the next scanline is bright again");
        assert_eq!(crt_factor(320), 3);
        assert_eq!(crt_factor(640), 2);
        // A lone white pixel glows a little into its dark neighbours'
        // edges, never beyond them.
        let mut rgb = vec![0u8; 3 * 1 * 3];
        rgb[3..6].fill(255);
        let out = crt(&rgb, 3, 1, 2);
        assert!(out[(1) * 3] > 0 && out[0] == 0);
    }
}
