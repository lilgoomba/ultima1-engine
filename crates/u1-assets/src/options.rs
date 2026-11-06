//! The options menu (F1, a key the original never reads): the display and
//! the sound, switched while the game waits. It is drawn as the
//! original's own text window (the black box framed in blue that shops and
//! the king use) so it never breaks the faithful look. The front end runs
//! it with the game paused, over a copy of the screen.

use crate::config::VideoChoice;
use crate::io::keys;
use crate::qol::{self, Qol, Scaling};
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

/// The menu's settings.
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
    Food,
    Footsteps,
    Facing,
    Music,
    MusicVolume,
    Smoothing,
    Keys,
    MasterVolume,
    SfxVolume,
    ConfirmQuit,
    SignWait,
    DockFee,
    SteadyAim,
    EnemySpeed,
    SpaceMap,
    SpaceKills,
    WideHits,
    AceNeed,
}

/// The menu's pages (tabs), in order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Video,
    Sound,
    Gameplay,
    Controls,
    Space,
    Interface,
}

impl Page {
    pub const ALL: [Page; 6] = [Page::Video, Page::Sound, Page::Gameplay, Page::Controls, Page::Space, Page::Interface];

    pub fn label(self) -> &'static str {
        match self {
            Page::Video => "Video",
            Page::Sound => "Sound",
            Page::Gameplay => "Gameplay",
            Page::Controls => "Controls",
            Page::Space => "Space",
            Page::Interface => "Interface",
        }
    }

    /// The page's rows, top to bottom.
    pub fn rows(self) -> &'static [Row] {
        match self {
            Page::Video => &[Row::Video, Row::Fullscreen, Row::Scaling, Row::Smoothing, Row::Crt, Row::Facing],
            Page::Sound => &[Row::Sound, Row::MasterVolume, Row::SfxVolume, Row::Music, Row::MusicVolume, Row::Footsteps],
            Page::Gameplay => &[Row::Turbo, Row::Delay, Row::Space, Row::Timing, Row::Food, Row::Confirm, Row::SignWait, Row::DockFee],
            Page::Controls => &[Row::Repeat, Row::RepeatDelay, Row::RepeatRate, Row::Gamepad, Row::Keys],
            Page::Space => &[Row::SteadyAim, Row::EnemySpeed, Row::WideHits, Row::SpaceMap, Row::SpaceKills, Row::AceNeed],
            Page::Interface => &[Row::Slots, Row::Log, Row::Automap, Row::Minimap, Row::Character, Row::Help, Row::Hints, Row::ConfirmQuit],
        }
    }
}

/// Every row, page by page (the order `--menu-row` counts in).
pub const ROWS: [Row; 39] = [
    Row::Video,
    Row::Fullscreen,
    Row::Scaling,
    Row::Smoothing,
    Row::Crt,
    Row::Facing,
    Row::Sound,
    Row::MasterVolume,
    Row::SfxVolume,
    Row::Music,
    Row::MusicVolume,
    Row::Footsteps,
    Row::Turbo,
    Row::Delay,
    Row::Space,
    Row::Timing,
    Row::Food,
    Row::Confirm,
    Row::SignWait,
    Row::DockFee,
    Row::Repeat,
    Row::RepeatDelay,
    Row::RepeatRate,
    Row::Gamepad,
    Row::Keys,
    Row::SteadyAim,
    Row::EnemySpeed,
    Row::WideHits,
    Row::SpaceMap,
    Row::SpaceKills,
    Row::AceNeed,
    Row::Slots,
    Row::Log,
    Row::Automap,
    Row::Minimap,
    Row::Character,
    Row::Help,
    Row::Hints,
    Row::ConfirmQuit,
];

/// Where the values start: one blank after the longest label.
const VALUE_COL: usize = 17;

/// The most rows a page has.
pub const SHOWN: usize = 8;

/// Keys the menu reads besides the game's: Tab (next page) and the
/// message log's page keys (`msglog::keys`).
pub const TAB: u8 = 9;

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
            Row::Food => "Food use",
            Row::Footsteps => "Footsteps",
            Row::Facing => "Vehicle turn",
            Row::Music => "Music",
            Row::MusicVolume => "Music volume",
            Row::Smoothing => "Smoothing",
            Row::Keys => "Keys",
            Row::MasterVolume => "Master vol.",
            Row::SfxVolume => "Effects vol.",
            Row::ConfirmQuit => "Confirm quit",
            Row::SignWait => "Sign wait",
            Row::DockFee => "Docking fees",
            Row::SteadyAim => "Steady aim",
            Row::EnemySpeed => "Enemy speed",
            Row::SpaceMap => "Space map",
            Row::SpaceKills => "Kill counter",
            Row::WideHits => "Hit box",
            Row::AceNeed => "Ace need",
        }
    }

    /// One line on what the setting does (at most two lines of the box).
    pub fn describe(self) -> &'static str {
        match self {
            Row::Video => "The display: the original's EGA, CGA or Tandy, or the enhanced art.",
            Row::Sound => "The original's speaker beeps, the enhanced sounds, or silence.",
            Row::Fullscreen => "Fill the whole screen (also Alt+Enter).",
            Row::Scaling => "Whole pixels keep every pixel the same size; Fill uses the window.",
            Row::Crt => "Scanlines and a soft glow, like an old monitor.",
            Row::Slots => "Five save slots (F2), quicksave F5 and quickload F9.",
            Row::Log => "PgUp shows the messages that scrolled away.",
            Row::Turbo => "Everything runs four times as fast (also F4).",
            Row::Delay => "How long the game waits after a message.",
            Row::Space => "Space flight and combat wait half as long.",
            Row::Timing => "Speaker effects no longer hold the game up.",
            Row::Automap => "Tab in a dungeon shows the levels you have seen.",
            Row::Minimap => "Tab outdoors shows the land you have seen.",
            Row::Character => "Z shows a full character screen.",
            Row::Help => "F3 shows a list of the commands.",
            Row::Hints => "A short line on what a command asks for.",
            Row::Confirm => "Ask before selling or dropping something worth this much.",
            Row::Repeat => "Our own repeat for held arrows, or the system's.",
            Row::RepeatDelay => "How long an arrow is held before it repeats.",
            Row::RepeatRate => "Steps a second while an arrow repeats.",
            Row::Gamepad => "Play with a game controller.",
            Row::Food => "How fast food runs out (a rule change).",
            Row::Footsteps => "Footsteps outdoors (with the enhanced sounds).",
            Row::Facing => "Vehicles face the way they move (enhanced video).",
            Row::Music => "Our own music for each area, separate from the effects.",
            Row::MusicVolume => "How loud the music is.",
            Row::Smoothing => "How the enlarged picture is smoothed.",
            Row::Keys => "Extra keys for commands and screens.",
            Row::MasterVolume => "How loud everything is.",
            Row::SfxVolume => "How loud the beeps and sound effects are.",
            Row::ConfirmQuit => "Ask before closing the window with progress not saved.",
            Row::SignWait => "Keep a sign's words up until a key.",
            Row::DockFee => "Pay 500 coin to refuel at a station.",
            Row::SteadyAim => "Original drift; opposite press stops free. Press again to go that way.",
            Row::EnemySpeed => "How fast front-view fighters flee and grow (aim and fire unchanged).",
            Row::SpaceMap => "I in space: original scan icons, or a clearer labelled grid.",
            Row::SpaceKills => "Show fighter kills toward Space Ace on the space HUD.",
            Row::WideHits => "Slightly larger fighter hit box (aim only; damage unchanged).",
            Row::AceNeed => "Fighter kills required for Space Ace (HUD and award).",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionsMenu {
    /// Index into [`Page::ALL`].
    pub page: usize,
    /// Index into the page's rows.
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
        OptionsMenu { page: 0, row: 0, video, sound, avail, qol: Qol::default() }
    }

    /// The page shown.
    pub fn page(&self) -> Page {
        Page::ALL[self.page]
    }

    /// The row chosen.
    pub fn current(&self) -> Row {
        self.page().rows()[self.row]
    }

    /// Show `r`'s page with `r` chosen.
    pub fn select(&mut self, r: Row) {
        for (p, page) in Page::ALL.iter().enumerate() {
            if let Some(i) = page.rows().iter().position(|&x| x == r) {
                (self.page, self.row) = (p, i);
            }
        }
    }

    fn turn_page(&mut self, back: bool) {
        let n = Page::ALL.len();
        self.page = if back { (self.page + n - 1) % n } else { (self.page + 1) % n };
        self.row = 0;
    }

    /// Put `rows` back to the faithful engine's values (the display keeps
    /// its original mode, without the enhanced art).
    pub fn reset(&mut self, rows: &[Row]) {
        let d = Qol::default();
        let q = &mut self.qol;
        for &r in rows {
            match r {
                Row::Video => self.video = VideoChoice::from_parts(false, self.video.mode()),
                Row::Sound => self.sound = SoundMode::Original,
                Row::Fullscreen => q.fullscreen = d.fullscreen,
                Row::Scaling => q.scaling = d.scaling,
                Row::Crt => q.crt = d.crt,
                Row::Slots => q.slots = d.slots,
                Row::Log => q.log = d.log,
                Row::Turbo => q.turbo = d.turbo,
                Row::Delay => q.message_delay = d.message_delay,
                Row::Space => q.fast_space = d.fast_space,
                Row::Timing => q.steady = d.steady,
                Row::Automap => q.automap = d.automap,
                Row::Minimap => q.minimap = d.minimap,
                Row::Character => q.charsheet = d.charsheet,
                Row::Help => q.help = d.help,
                Row::Hints => q.hints = d.hints,
                Row::Confirm => q.confirm = d.confirm,
                Row::Repeat => q.repeat = d.repeat,
                Row::RepeatDelay => q.repeat_delay = d.repeat_delay,
                Row::RepeatRate => q.repeat_rate = d.repeat_rate,
                Row::Gamepad => q.gamepad = d.gamepad,
                Row::Food => q.food_use = d.food_use,
                Row::Footsteps => q.footsteps = d.footsteps,
                Row::Facing => q.vehicle_facing = d.vehicle_facing,
                Row::Music => q.music = d.music,
                Row::MusicVolume => q.music_volume = d.music_volume,
                Row::Smoothing => q.smoothing = d.smoothing,
                Row::Keys => {}
                Row::MasterVolume => q.master_volume = d.master_volume,
                Row::SfxVolume => q.sfx_volume = d.sfx_volume,
                Row::ConfirmQuit => q.confirm_quit = d.confirm_quit,
                Row::SignWait => q.sign_wait = d.sign_wait,
                Row::DockFee => q.docking_fee = d.docking_fee,
                Row::SteadyAim => q.steady_aim = d.steady_aim,
                Row::EnemySpeed => q.enemy_speed = d.enemy_speed,
                Row::SpaceMap => q.space_map = d.space_map,
                Row::SpaceKills => q.space_kills = d.space_kills,
                Row::WideHits => q.wide_hits = d.wide_hits,
                Row::AceNeed => q.space_ace_need = d.space_ace_need,
            }
        }
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
            Row::Character => if q.charsheet { "On (Z)" } else { "Off" }.into(),
            Row::Help => if q.help { "On (F3)" } else { "Off" }.into(),
            Row::Hints => on(q.hints),
            Row::Confirm if q.confirm == 0 => "Off".into(),
            Row::Confirm => format!("Worth {}+", q.confirm),
            Row::Repeat => if q.repeat { "Own" } else { "System" }.into(),
            Row::RepeatDelay => format!("{} ms", q.repeat_delay),
            Row::RepeatRate if q.repeat_rate == 0 => "Game pace".to_string(),
            Row::RepeatRate => format!("{} a second", q.repeat_rate),
            Row::Gamepad => on(q.gamepad),
            Row::Food => match q.food_use {
                qol::FoodUse::Original => "Original",
                qol::FoodUse::Half => "Half",
                qol::FoodUse::Off => "Off",
            }
            .into(),
            Row::Footsteps if self.sound != SoundMode::Enhanced => if q.footsteps { "On (enh. sound)" } else { "Off" }.into(),
            Row::Footsteps => on(q.footsteps),
            Row::Facing if self.video != VideoChoice::Enhanced => if q.vehicle_facing { "On (enhanced)" } else { "Off" }.into(),
            Row::Facing => on(q.vehicle_facing),
            Row::Music => on(q.music),
            Row::MusicVolume => format!("{}%", q.music_volume),
            Row::Smoothing => match q.smoothing {
                qol::Smoothing::Off => "Off",
                qol::Smoothing::Linear => "Linear",
                qol::Smoothing::PixelArt => "Pixel art",
            }
            .into(),
            Row::Keys => "Enter: change".into(),
            Row::MasterVolume => format!("{}%", q.master_volume),
            Row::SfxVolume => format!("{}%", q.sfx_volume),
            Row::ConfirmQuit => on(q.confirm_quit),
            Row::SignWait => on(q.sign_wait),
            Row::DockFee => on(q.docking_fee),
            Row::SteadyAim => on(q.steady_aim),
            Row::EnemySpeed => q.enemy_speed.label().into(),
            Row::SpaceMap => q.space_map.label().into(),
            Row::SpaceKills => on(q.space_kills),
            Row::WideHits => on(q.wide_hits),
            Row::AceNeed => q.space_ace_need.label().into(),
        }
    }

    pub fn key(&mut self, k: u8) -> MenuKey {
        let n = self.page().rows().len();
        match k {
            keys::UP => {
                self.row = (self.row + n - 1) % n;
                MenuKey::Stay
            }
            keys::DOWN => {
                self.row = (self.row + 1) % n;
                MenuKey::Stay
            }
            TAB | crate::msglog::keys::PGDN | b']' => {
                self.turn_page(false);
                MenuKey::Stay
            }
            crate::msglog::keys::PGUP | b'[' => {
                self.turn_page(true);
                MenuKey::Stay
            }
            // Digits jump to a page; count comes from Page::ALL (not a fixed 1..5).
            k if (b'1'..).contains(&k) && usize::from(k - b'1') < Page::ALL.len() => {
                (self.page, self.row) = (usize::from(k - b'1'), 0);
                MenuKey::Stay
            }
            b'r' | b'R' => {
                self.reset(self.page().rows());
                MenuKey::Changed
            }
            b'o' | b'O' => {
                self.reset(&ROWS);
                MenuKey::Changed
            }
            keys::LEFT | keys::RIGHT | b' ' => {
                let step = if k == keys::LEFT { 3 } else { 1 };
                let cur = self.current();
                let q = &mut self.qol;
                match cur {
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
                    Row::Food => q.food_use = qol::cycle(&qol::FoodUse::ALL, q.food_use, k == keys::LEFT),
                    Row::Footsteps => q.footsteps ^= true,
                    Row::Facing => q.vehicle_facing ^= true,
                    Row::Music => q.music ^= true,
                    Row::MusicVolume => q.music_volume = qol::cycle(&qol::MUSIC_VOLUMES, q.music_volume, k == keys::LEFT),
                    Row::Smoothing => q.smoothing = qol::cycle(&qol::Smoothing::ALL, q.smoothing, k == keys::LEFT),
                    Row::Keys => return MenuKey::Rebind,
                    Row::MasterVolume => q.master_volume = qol::cycle(&qol::VOLUMES, q.master_volume, k == keys::LEFT),
                    Row::SfxVolume => q.sfx_volume = qol::cycle(&qol::VOLUMES, q.sfx_volume, k == keys::LEFT),
                    Row::ConfirmQuit => q.confirm_quit ^= true,
                    Row::SignWait => q.sign_wait ^= true,
                    Row::DockFee => q.docking_fee ^= true,
                    Row::SteadyAim => q.steady_aim ^= true,
                    Row::EnemySpeed => {
                        q.enemy_speed = match q.enemy_speed {
                            qol::EnemySpeed::Original => qol::EnemySpeed::Half,
                            qol::EnemySpeed::Half => qol::EnemySpeed::Original,
                        }
                    }
                    Row::SpaceMap => {
                        q.space_map = match q.space_map {
                            qol::SpaceMap::Original => qol::SpaceMap::Enhanced,
                            qol::SpaceMap::Enhanced => qol::SpaceMap::Original,
                        }
                    }
                    Row::SpaceKills => q.space_kills ^= true,
                    Row::WideHits => q.wide_hits ^= true,
                    Row::AceNeed => q.space_ace_need = qol::cycle(&qol::SpaceAceNeed::ALL, q.space_ace_need, k == keys::LEFT),
                }
                if self.current() == Row::Video {
                    let mut i = VideoChoice::ALL.iter().position(|&v| v == self.video).unwrap_or(0);
                    for _ in 0..4 {
                        i = (i + step) % 4;
                        if self.avail[i] {
                            break;
                        }
                    }
                    let old = self.video;
                    self.video = VideoChoice::ALL[i];
                    if old == self.video {
                        MenuKey::Stay
                    } else {
                        MenuKey::Changed
                    }
                } else if self.current() == Row::Sound {
                    let i = SOUNDS.iter().position(|&s| s == self.sound).unwrap_or(0);
                    self.sound = SOUNDS[(i + step.min(2)) % 3];
                    MenuKey::Changed
                } else {
                    MenuKey::Changed
                }
            }
            keys::ENTER if self.current() == Row::Keys => MenuKey::Rebind,
            keys::ENTER | keys::ESC => MenuKey::Close,
            _ => MenuKey::Stay,
        }
    }

    /// Draw over `t` (a copy of the game's screen).
    pub fn draw(&self, t: &mut Term) {
        panel(t);
        center(t, 2, "Options  (the game is paused)", TEXT);
        // The tabs, on two lines; the page shown in the warning colour.
        for (line, pages) in [(3, 0..3), (4, 3..6)] {
            let text: Vec<String> = pages.clone().map(|i| format!("{} {}", i + 1, Page::ALL[i].label())).collect();
            let width = text.iter().map(|s| s.len()).sum::<usize>() + 2 * (text.len() - 1);
            let mut col = 2 + (36 - width) / 2;
            for (i, s) in pages.zip(&text) {
                t.print_at_attr(col, line, s, if i == self.page { WARN } else { TEXT });
                col += s.len() + 2;
            }
        }
        let rows = self.page().rows();
        for (i, &r) in rows.iter().enumerate() {
            let row = 6 + i;
            let sel = i == self.row;
            t.print_at_attr(2, row, if sel { "\u{10}" } else { " " }, WARN);
            t.print_at_attr(3, row, &format!("{}:", r.label()), TEXT);
            t.print_at_attr(VALUE_COL, row, &format!("<{}>", self.value(r)), if sel { WARN } else { TEXT });
        }
        // Description and help sit under the last setting (8 or 9 rows).
        let under = 6 + rows.len();
        for (i, l) in wrap(self.current().describe(), 36).iter().take(2).enumerate() {
            t.print_at_attr(2, under + i, l, TEXT);
        }
        center(t, under + 2, &format!("Tab/1-{}: page  R: reset  O: all", Page::ALL.len()), TEXT);
        center(t, under + 3, "Arrows: change  F1/Enter: resume", TEXT);
        t.cursor(1, 24);
    }
}

/// `s` in lines of at most `w` characters, broken at spaces.
pub fn wrap(s: &str, w: usize) -> Vec<String> {
    let mut out = vec![String::new()];
    for word in s.split(' ') {
        let cur = out.last_mut().expect("one line");
        if !cur.is_empty() && cur.len() + 1 + word.len() > w {
            out.push(word.to_string());
        } else {
            if !cur.is_empty() {
                cur.push(' ');
            }
            cur.push_str(word);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(m: &mut OptionsMenu, r: Row) -> &mut OptionsMenu {
        m.select(r);
        m
    }

    #[test]
    fn labels_and_values_never_touch() {
        for r in ROWS {
            assert!(3 + r.label().len() + 1 < VALUE_COL, "{r:?}");
            assert!(wrap(r.describe(), 36).len() <= 2, "{r:?} describes in two lines");
        }
    }

    #[test]
    fn every_row_on_one_page() {
        let all: Vec<Row> = Page::ALL.iter().flat_map(|p| p.rows().iter().copied()).collect();
        assert_eq!(all, ROWS.to_vec(), "ROWS is the pages in order");
        assert!(Page::ALL.iter().all(|p| p.rows().len() <= SHOWN));
        for (i, r) in ROWS.iter().enumerate() {
            assert!(!ROWS[i + 1..].contains(r), "{r:?} twice");
        }
    }

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
        assert_eq!(m.key(TAB), MenuKey::Stay);
        assert_eq!((m.page(), m.current()), (Page::Sound, Row::Sound));
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
    fn pages_turn_and_wrap() {
        let mut m = OptionsMenu::new(VideoChoice::Ega, SoundMode::Original, [true; 4]);
        m.key(keys::UP);
        assert_eq!(m.current(), Row::Facing, "up wraps within the page");
        m.key(crate::msglog::keys::PGUP);
        assert_eq!((m.page(), m.row), (Page::Interface, 0), "back from the first page");
        m.key(b']');
        assert_eq!(m.page(), Page::Video);
        m.key(TAB);
        assert_eq!(m.page(), Page::Sound, "Tab forward");
        m.key(crate::msglog::keys::PGUP);
        assert_eq!(m.page(), Page::Video, "PgUp back");
        // Wrap forward: Interface → Video
        m.key(b'6');
        assert_eq!(m.page(), Page::Interface);
        m.key(TAB);
        assert_eq!(m.page(), Page::Video, "Tab wraps past last page");
        m.key(b'4');
        assert_eq!(m.page(), Page::Controls);
        m.select(Row::Keys);
        assert_eq!(m.key(keys::ENTER), MenuKey::Rebind);
        assert_eq!(m.key(keys::RIGHT), MenuKey::Rebind);
    }

    #[test]
    fn digit_keys_select_every_page_including_sixth() {
        assert_eq!(Page::ALL.len(), 6, "Space page added a sixth tab");
        let mut m = OptionsMenu::new(VideoChoice::Ega, SoundMode::Original, [true; 4]);
        let expect = [(b'1', Page::Video), (b'2', Page::Sound), (b'3', Page::Gameplay), (b'4', Page::Controls), (b'5', Page::Space), (b'6', Page::Interface)];
        for (digit, page) in expect {
            assert_eq!(m.key(digit), MenuKey::Stay, "digit {}", digit as char);
            assert_eq!((m.page(), m.row), (page, 0), "key {} → {:?}", digit as char, page);
        }
        let on_iface = m.page();
        assert_eq!(m.key(b'7'), MenuKey::Stay, "past last page is a no-op");
        assert_eq!(m.page(), on_iface, "7 must not change page");
        assert_eq!(m.key(b'0'), MenuKey::Stay);
        assert_eq!(m.page(), on_iface);
    }
    #[test]
    fn interface_via_digit_six_screenshot() {
        let mut m = OptionsMenu::new(VideoChoice::Ega, SoundMode::Original, [true; 4]);
        m.key(b'6');
        assert_eq!(m.page(), Page::Interface);
        let mut t = Term::new();
        m.draw(&mut t);
        // Header tabs mark Interface selected; first Interface row is Slots.
        assert_eq!(m.current(), Row::Slots);
        let rgb = t.to_rgb(&crate::palette::EGA_PALETTE, 2);
        let path = std::path::Path::new("/workspace/u1-shots/space-aim/f1-interface-via-6.png");
        let _ = std::fs::create_dir_all(path.parent().unwrap());
        let f = std::fs::File::create(path).unwrap();
        let mut enc = png::Encoder::new(std::io::BufWriter::new(f), 640, 400);
        enc.set_color(png::ColorType::Rgb);
        enc.set_depth(png::BitDepth::Eight);
        enc.write_header().unwrap().write_image_data(&rgb).unwrap();
        assert!(path.is_file());
    }

    #[test]
    fn qol_rows_toggle_and_default_off() {
        let mut m = OptionsMenu::new(VideoChoice::Ega, SoundMode::Original, [true; 4]);
        assert_eq!(m.qol, Qol::default());
        assert_eq!(at(&mut m, Row::Timing).key(keys::RIGHT), MenuKey::Changed);
        assert!(m.qol.steady);
        at(&mut m, Row::Turbo).key(b' ');
        assert!(m.qol.turbo && m.value(Row::Turbo) == "On (F4)");
        at(&mut m, Row::Delay).key(keys::LEFT);
        assert_eq!((m.qol.message_delay, m.value(Row::Delay)), (0, "None".to_string()));
        at(&mut m, Row::Scaling).key(keys::RIGHT);
        assert_eq!(m.qol.scaling, Scaling::Fill);
        assert!(m.qol.slots && m.value(Row::Slots) == "On (F2 F5 F9)", "save slots default on");
        for r in [Row::Fullscreen, Row::Crt, Row::Slots, Row::Log, Row::Space] {
            at(&mut m, r).key(keys::RIGHT);
        }
        assert!(m.qol.fullscreen && m.qol.crt && !m.qol.slots && m.qol.log && m.qol.fast_space);
        at(&mut m, Row::Slots).key(b'r');
        assert!(m.qol.slots, "R puts the page back to slots on");
        assert_eq!((m.video, m.sound), (VideoChoice::Ega, SoundMode::Original), "the other rows are untouched");
        for r in [Row::Automap, Row::Minimap, Row::Character, Row::Help, Row::Hints, Row::Repeat, Row::Gamepad] {
            assert_eq!(at(&mut m, r).key(b' '), MenuKey::Changed);
        }
        let q = m.qol;
        assert!(q.automap && q.minimap && q.charsheet && q.help && q.hints && q.repeat && q.gamepad);
        at(&mut m, Row::Confirm).key(keys::RIGHT);
        assert_eq!((m.qol.confirm, m.value(Row::Confirm)), (10, "Worth 10+".to_string()));
        m.key(keys::LEFT);
        m.key(keys::LEFT);
        assert_eq!(m.qol.confirm, 500);
    }

    #[test]
    fn volumes_and_resets() {
        let mut m = OptionsMenu::new(VideoChoice::Enhanced, SoundMode::Mute, [true; 4]);
        at(&mut m, Row::SfxVolume).key(keys::LEFT);
        assert_eq!((m.qol.sfx_volume, m.value(Row::SfxVolume)), (90, "90%".to_string()));
        at(&mut m, Row::MasterVolume).key(keys::RIGHT);
        assert_eq!(m.qol.master_volume, 0, "100 wraps to silence");
        at(&mut m, Row::ConfirmQuit).key(b' ');
        at(&mut m, Row::Turbo).key(b' ');
        assert!(!m.qol.confirm_quit && m.qol.turbo);
        m.select(Row::Music);
        assert_eq!(m.key(b'r'), MenuKey::Changed, "reset the Sound page");
        assert_eq!((m.sound, m.qol.sfx_volume, m.qol.master_volume), (SoundMode::Original, 100, 100));
        assert!(m.qol.turbo && !m.qol.confirm_quit, "other pages kept");
        assert_eq!(m.video, VideoChoice::Enhanced);
        m.key(b'O');
        assert_eq!(m.qol, Qol::default());
        assert_eq!(m.video, VideoChoice::Ega, "the original display under the art");
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
        assert!(t.row_text(3).contains("1 Video") && t.row_text(4).contains("6 Interface"));
        assert!(t.row_text(6).contains("Video:") && t.row_text(6).contains("<CGA>"));
        assert!(t.row_text(7).contains("Fullscreen:") && t.row_text(7).contains("<Off>"));
        assert!(t.row_text(12).contains("The display"), "the description under the last Video row");
        assert_eq!(t.row_text(24)[..30].trim(), "", "the message window is untouched");
        let mut m = m;
        m.key(b'2');
        let mut t = Term::new();
        m.draw(&mut t);
        assert!(t.row_text(6).contains("<Mute>") && t.row_text(7).contains("Master vol."));
    }
}
