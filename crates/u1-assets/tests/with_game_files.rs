//! End-to-end checks of towns, castles, shops and saving that need the
//! user's own Ultima I files. Set `U1_GAME_DIR` to the directory holding
//! OUT.EXE, TCD.BIN, MAP.BIN, the tile files and PLAYER2.U1; without it
//! every test here is skipped. Expected values were observed side by side
//! in DOSBox (see SPEC.md); no game data is stored in this repository.

use u1_assets::exe::Exe;
use u1_assets::game::{Assets, Game};
use u1_assets::gamedir::GameDir;
use u1_assets::io::{keys, Halt, Io, IoResult, ScriptIo};
use u1_assets::locations::LocationTable;
use u1_assets::rules::ExeTables;
use u1_assets::save::{off, MapObject, PlayerRecord};
use u1_assets::space::SpaceFiles;
use u1_assets::tcd::TownCastleMaps;
use u1_assets::term::Term;

/// The game files named by `U1_GAME_DIR`. Without them each test here
/// returns early and passes (CI has no game files), saying so on stderr.
fn game_dir() -> Option<GameDir> {
    let Some(d) = std::env::var_os("U1_GAME_DIR") else {
        eprintln!("skipped: U1_GAME_DIR is not set");
        return None;
    };
    GameDir::new(d).ok()
}

/// PLAYER2.U1 moved to (x, y), run with seed 2 and the typed keys.
fn play(x: u16, y: u16, keys: &str) -> Option<Game> {
    play_with(x, y, keys, 2, &[], |_| {})
}

/// As `play`, with a chosen seed, recorded clock values for the time seeds
/// and a hook to adjust the save before starting.
fn play_with(x: u16, y: u16, keys: &str, seed: u16, times: &[u16], adjust: impl FnOnce(&mut PlayerRecord)) -> Option<Game> {
    play_io(x, y, &mut ScriptIo::from_text(keys), seed, times, adjust)
}

/// As `play_with`, driven by any `Io`.
fn play_io(x: u16, y: u16, io: &mut dyn Io, seed: u16, times: &[u16], adjust: impl FnOnce(&mut PlayerRecord)) -> Option<Game> {
    let g = game_dir()?;
    let mode = g.detect_video_mode()?;
    let exe_bytes = g.read("OUT.EXE").ok()?;
    let assets = Assets {
        t: ExeTables::from_out_exe(&exe_bytes).unwrap(),
        locs: LocationTable::from_out_exe(&exe_bytes).unwrap(),
        exe: Exe::new(exe_bytes).unwrap(),
        map: g.load_world_map().unwrap(),
        world_tiles: g.load_world_tiles(mode).unwrap(),
        town_tiles: g.load_town_tiles(mode).unwrap(),
        tcd: TownCastleMaps::parse(&g.read("TCD.BIN").unwrap()).unwrap(),
        video: mode,
    };
    let mut rec = PlayerRecord::parse(&g.read("PLAYER2.U1").ok()?).unwrap();
    rec.set_u16(off::POS_X, x);
    rec.set_u16(off::POS_Y, y);
    let mut o = rec.objects()[0];
    o.x = x;
    o.y = y;
    rec.set_object(0, &o);
    // The coin of the DOSBox runs these values were observed in.
    rec.set_u16(off::COIN, 35);
    adjust(&mut rec);
    let mut game = Game::new(assets, rec, seed);
    game.space = SpaceFiles::load(&g, game.a.video).ok();
    game.mondain = u1_assets::mondain::MondainFiles::load(&g, game.a.video).ok();
    game.fixed_times = times.iter().copied().collect();
    if let Some(p) = SAVE_TO.with(|s| s.borrow_mut().take()) {
        game.save_to = Some(p);
    }
    let _ = game.run(io);
    Some(game)
}

thread_local! {
    /// A save path for the next `play_io` (Quit writes there).
    static SAVE_TO: std::cell::RefCell<Option<std::path::PathBuf>> = const { std::cell::RefCell::new(None) };
}

fn said(g: &Game, s: &str) -> bool {
    g.term.messages().iter().any(|l| l.contains(s))
}

/// Weapon number of the dagger (slot 0 is bare hands).
const DAGGER: usize = 1;
const UP8: &str = "{up}{up}{up}{up}{up}{up}{up}{up}";

#[test]
fn entering_towns_and_castles() {
    let Some(g) = play(66, 41, "e") else { return };
    assert_eq!((g.mode, g.loc, g.px, g.py), (1, 2, 19, 17));
    assert!(said(&g, "Moon"));
    let g = play(25, 61, "e").unwrap();
    assert_eq!((g.loc, g.px, g.py), (3, 19, 17));
    let g = play(32, 27, "e").unwrap();
    assert_eq!((g.mode, g.loc, g.px, g.py), (2, 34, 0, 9));
    let g = play(40, 38, "e").unwrap();
    assert_eq!((g.loc, g.px, g.py), (33, 0, 9));
}

#[test]
fn moon_weapon_shop_sells_a_dagger_for_5() {
    let keys = format!("e{UP8}{{left}}{{left}}{{left}}{{left}}{{left}}{{left}}{{up}}{{up}}{{up}}{{left}}{{left}}{{left}}{{up}}{{up}}{{up}}{{left}}{{left}}{{left}}{{left}}{{right}}{{right}}{{right}}{{right}}{{right}}");
    let Some(g) = play(66, 41, &keys) else { return };
    assert_eq!((g.px, g.py), (11, 5));
    let before = g.coin();
    let daggers = g.rec.u16_at(u1_assets::game::slot::WEAPON + 2 * DAGGER);
    let g = play(66, 41, &format!("{keys}tbb~~")).unwrap();
    assert!(said(&g, "Transact-Buy: Dagger"));
    assert_eq!(g.coin(), before - 5);
    assert_eq!(g.rec.u16_at(u1_assets::game::slot::WEAPON + 2 * DAGGER), daggers + 1);
}

#[test]
fn moon_armour_too_dear_buys_nothing() {
    let keys = format!("e{UP8}{{left}}{{left}}{{left}}{{left}}{{left}}{{left}}{{up}}{{up}}{{up}}{{left}}{{left}}{{left}}{{up}}{{up}}{{up}}{{left}}{{left}}{{left}}{{left}}tbb~~");
    let Some(g) = play(66, 41, &keys) else { return };
    assert!(said(&g, "Transact-Buy: nothing"));
    assert_eq!(g.coin(), 35);
}

#[test]
fn fawn_pub_ale_costs_one_coin() {
    let keys = format!("e{UP8}{{up}}{{left}}{{left}}{{left}}{{left}}{{left}}{{right}}{{right}}{{right}}{{up}}{{up}}{{up}}{{up}}{{up}}{{up}}tb~~~~");
    let Some(g) = play(25, 61, &keys) else { return };
    assert_eq!((g.px, g.py), (17, 3));
    assert!(said(&g, "Transact-Buy: ale"));
    assert_eq!(g.coin(), 34);
}

#[test]
fn transact_away_from_a_counter() {
    let Some(g) = play(66, 41, "et") else { return };
    assert!(said(&g, "not by a counter"));
}

#[test]
fn leaving_a_town_returns_to_its_tile_and_quit_saves_820_bytes() {
    let dir = std::env::temp_dir().join(format!("u1test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let out = dir.join("OUT.U1");
    let Some(g) = game_dir() else { return };
    let _ = g;
    // Out of Moon by walking down off the bottom edge.
    let g = play(66, 41, "e{down}").unwrap();
    assert_eq!(g.mode, 0);
    assert_eq!((g.ow.x, g.ow.y), (66, 41));
    // Quit outdoors writes a copy, never the original.
    let mut game = play(66, 41, "").unwrap();
    game.save_to = Some(out.clone());
    let _ = game.run(&mut ScriptIo::from_text("q~"));
    let bytes = std::fs::read(&out).unwrap();
    assert_eq!(bytes.len(), 820);
    let back = PlayerRecord::parse(&bytes).unwrap();
    assert_eq!(back.position(), (66, 41));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn entering_a_dungeon_and_climbing_out() {
    let Some(g) = play(39, 60, "e") else { return };
    assert_eq!(g.mode, 3);
    let d = g.dungeon.as_ref().unwrap();
    assert_eq!((d.depth, d.a, d.b), (1, 1, 1));
    assert!(said(&g, "Entering..."));
    // Facing south on the up ladder: K climbs out to the overworld.
    let Some(g) = play(39, 60, "ek") else { return };
    assert!(said(&g, " up to level 0"));
    assert_eq!((g.mode, g.dungeon.is_none()), (0, true));
    // Facing east the ladder must be faced first.
    let Some(g) = play(39, 60, "e{left}k") else { return };
    assert!(said(&g, "Thou must face the ladder!"));
    assert_eq!(g.mode, 3);
}

#[test]
fn dungeon_turns_and_blocked_path() {
    // Level 1 of this dungeon has a wall two squares east of the ladder.
    let Some(g) = play(39, 60, "e{right}") else { return };
    assert!(said(&g, "Turn right"));
    assert_eq!(g.dungeon.as_ref().unwrap().facing, 1);
    let Some(g) = play(39, 60, "e{right}{up}") else { return };
    assert!(said(&g, "Forward - path blocked!"));
}

/// Replays a DOSBox session through three dungeon levels. The original seeds
/// its generator from the clock; the clock values below (one per level
/// set-up) were recovered from the DOSBox frames, after which every frame of
/// the run matched: monster positions, hits, kills and loot.
#[test]
fn dungeon_run_replays_dosbox_session() {
    let adjust = |r: &mut PlayerRecord| {
        r.set_u16(off::HITS, 225);
        r.set_u16(off::FOOD, 223);
        r.set_u16(off::QUESTS + 2, 0xFFFF);
        r.set_u16(off::STEPS, 158);
    };
    let run = |keys: &str| play_with(39, 60, keys, 2332, &[2332, 2248, 403], adjust);
    let l1 = "e{left}{up}{up}{right}{up}{up}{up}{up}{up}{up}";
    let Some(g) = run(&format!("{l1}a")) else { return };
    assert!(said(&g, "Ranger killed!"));
    assert!(said(&g, "Thou dost find: 5 copper!"));
    assert_eq!(g.rec.u16_at(off::COIN), 40);
    let l2 = format!(
        "{l1}aaaa{{up}}{{up}}{{up}}{{up}}{{up}}k{{left}}{{up}}{{left}}{{up}}{{up}}{{up}}{{up}}{{up}}{{up}}\
         {{right}}{{up}}{{up}}{{right}}{{up}}{{left}}{{up}}{{up}}{{right}}{{up}}{{down}}aaa"
    );
    let Some(g) = run(&l2) else { return };
    assert_eq!(g.dungeon.as_ref().unwrap().depth, 2);
    assert!(said(&g, "Ranger killed!"));
    assert_eq!((g.rec.u16_at(off::HITS), g.rec.u16_at(off::COIN)), (170, 61));
    let l3 = format!("{l2}{{up}}{{right}}{{up}}{{up}}{{right}}{{up}}k{{up}}{{up}}{{left}}{{up}}{{up}}{{down}}{{up}}{{right}}{{up}}");
    let Some(g) = run(&l3) else { return };
    assert_eq!(g.dungeon.as_ref().unwrap().depth, 3);
    assert!(said(&g, "Attacked by Cyclops!"));
    assert_eq!(g.rec.u16_at(off::HITS), 36);
}

/// PLAYER2.U1 standing on a shuttle at (40,40).
fn on_shuttle(r: &mut PlayerRecord) {
    r.set_object(1, &MapObject { kind: 0x11, unknown_02: 1, x: 40, y: 40, hits: 0 });
    r.set_u16(off::LAST_OBJECT, 1);
    // As the transport shop leaves them when it sells a shuttle.
    r.set_u16(off::SHUTTLE_0, 1000);
    r.set_u16(off::SHUTTLE_1, 1000);
    r.set_u16(off::HITS, 225);
}

#[test]
fn an_unfuelled_shuttle_kills_its_pilot() {
    let Some(g) = play_with(40, 40, "B{w12}", 1, &[], |r| {
        on_shuttle(r);
        r.set_u16(off::SHUTTLE_1, 0);
    }) else {
        return;
    };
    assert!(said(&g, "Thou hast lifted off!"));
    assert!(said(&g, "thou art dead"));
}

#[test]
fn shuttle_launch_flight_and_landing() {
    let Some(g) = play_with(40, 40, "B{w12}", 1, &[], on_shuttle) else { return };
    assert!(said(&g, "10..9..8..7..6..5..4..3..2..1"));
    assert!(said(&g, "Thou hast lifted off!"));
    // Thrust out of the station dock: a crunch, then a drift onto the
    // home planet lands the shuttle back where it left.
    let Some(g) = play_with(40, 40, "B{w12}{up}{w300}", 1, &[], on_shuttle) else { return };
    assert!(said(&g, "Crunch!"));
    assert!(said(&g, "Thou hast landed safely!"));
    assert_eq!(g.transport(), 6);
    assert!(g.rec.u16_at(off::SHUTTLE_1) < 1000);
    assert!(g.term.row_text(21).contains("Hits:"));
}

/// An idle top-down ship passes on its own after 1500 polls (DOSBox: a
/// Pass every ~165 s at two ticks a poll), counted from zero each prompt.
#[test]
fn idle_space_prompt_passes_after_1500_polls() {
    let passes = |w: usize| {
        let g = play_with(40, 40, &format!("B{{w12}}{{w{w}}}"), 1, &[], on_shuttle).unwrap();
        (0..25).filter(|&r| g.term.row_text(r).contains("Pass")).count()
    };
    if play(40, 40, "").is_none() {
        return;
    }
    assert_eq!(passes(1400), 0);
    assert_eq!(passes(1600), 1);
    assert_eq!(passes(3100), 2);
}

/// Standing on the time machine with the four gems.
fn at_time_machine(r: &mut PlayerRecord) {
    r.set_object(1, &MapObject { kind: 0x12, unknown_02: 1, x: 40, y: 40, hits: 0 });
    r.set_u16(off::LAST_OBJECT, 1);
    for k in 0..4 {
        r.set_u16(off::GEMS + 2 * k, 1);
    }
    r.set_u16(off::HITS, 900);
    r.set_u16(off::FOOD, 500);
}

const TO_GEM: &str = "B    {right}{right}{right}{right}{right}{right}{right}{right}{right}{right}";

#[test]
fn time_machine_intro_and_arena() {
    let Some(g) = play_with(40, 40, "B", 1, &[], at_time_machine) else { return };
    assert!(g.term.row_text(3).contains("Entering the craft"));
    assert!(g.stopped.is_none());
    let g = play_with(40, 40, &format!("{TO_GEM}{{w1}}"), 1, &[], at_time_machine).unwrap();
    assert!(said(&g, "You hear a strange chanting!"));
    // Without all four gems the machine does not go.
    let g = play_with(40, 40, "B ", 1, &[], |r| {
        at_time_machine(r);
        r.set_u16(off::GEMS, 0);
    })
    .unwrap();
    assert!(!g.term.row_text(3).contains("Entering the craft"));
}

#[test]
fn taking_the_gem_and_dying_returns_to_out() {
    let Some(g) = play_with(40, 40, &format!("{TO_GEM}G{{w1}}"), 1, &[], at_time_machine) else { return };
    assert!(said(&g, "The Gem is DESTROYED!"));
    assert_eq!(g.hits(), 225);
    // Mondain keeps away, so chase him until his spells finish the player.
    let mut bot = LairBot::new(&format!("{TO_GEM}G"), 600);
    play_io(40, 40, &mut bot, 1, &[], |r| {
        at_time_machine(r);
        r.set_u16(off::HITS, 20);
    })
    .unwrap();
    assert!(bot.saw("thou art dead"), "{:?}", bot.seen);
    assert!(bot.saw("Attempting resurrection!"));
}

/// Plays the lair by looking at the screen, as a player would: Mondain
/// backs away from the player, so he has to be hunted down. Cells are
/// 16x16 from (8,8); Mondain is told by his colours (the wizard has
/// magenta, the demon only light grey, the corpse only white), the player
/// by brown.
struct LairBot {
    queue: std::collections::VecDeque<u8>,
    left: u32,
    /// Every message line shown during the run.
    seen: Vec<String>,
    /// With `U1_LAIR_FRAMES` set: the last distinct screens, as RGB.
    frames: Option<std::collections::VecDeque<Vec<u8>>>,
}

impl LairBot {
    fn new(prefix: &str, polls: u32) -> Self {
        LairBot { queue: ScriptIo::from_text(prefix).keys, left: polls, seen: Vec::new(), frames: std::env::var_os("U1_LAIR_FRAMES").map(|_| Default::default()) }
    }

    fn keep(&mut self, t: &Term) {
        if let Some(f) = &mut self.frames {
            let rgb = t.to_rgb(&u1_assets::palette::EGA_PALETTE, 1);
            if f.back() != Some(&rgb) {
                f.push_back(rgb);
                if f.len() > 40 {
                    f.pop_front();
                }
            }
        }
    }

    fn saw(&self, s: &str) -> bool {
        self.seen.iter().any(|l| l.contains(s))
    }

    fn cell(t: &Term, x: i32, y: i32) -> Vec<u8> {
        let mut c = Vec::new();
        for py in 0..16 {
            for px in 0..16 {
                let v = t.at(8 + 16 * x as usize + px, 8 + 16 * y as usize + py);
                if v != 0 && !c.contains(&v) {
                    c.push(v);
                }
            }
        }
        c
    }

    fn decide(&mut self, t: &Term) -> u8 {
        let (mut pl, mut mo) = (None, None);
        for x in 0..19 {
            for y in 0..9 {
                let c = Self::cell(t, x, y);
                if c.contains(&6) && (x, y) != (2, 4) {
                    pl = Some((x, y));
                } else if c.contains(&13) || (!c.is_empty() && c.iter().all(|&v| v == 7 || v == 15)) {
                    mo = Some((x, y));
                }
            }
        }
        let (Some(pl), Some(mo)) = (pl, mo) else { return b' ' };
        // A shot only flies on while it is inside the border (SPEC §13).
        let can_hit = |p: (i32, i32)| {
            let (dx, dy) = (mo.0 - p.0, mo.1 - p.1);
            if !((dy == 0 && (1..=3).contains(&dx.abs())) || (dx == 0 && (1..=3).contains(&dy.abs()))) {
                return None;
            }
            let (sx, sy) = (dx.signum(), dy.signum());
            let mut q = p;
            for _ in 1..dx.abs() + dy.abs() {
                q = (q.0 + sx, q.1 + sy);
                if q.0 <= 0 || q.1 <= 0 {
                    return None;
                }
            }
            Some(match (sx, sy) {
                (-1, _) => keys::LEFT,
                (1, _) => keys::RIGHT,
                (_, -1) => keys::UP,
                _ => keys::DOWN,
            })
        };
        if let Some(k) = can_hit(pl) {
            self.queue.push_back(k);
            return b'A';
        }
        // Step towards the nearest empty cell to shoot from.
        let empty = |c: (i32, i32)| (0..19).contains(&c.0) && (0..9).contains(&c.1) && c != mo && Self::cell(t, c.0, c.1).is_empty();
        let mut prev = std::collections::HashMap::new();
        let mut q = std::collections::VecDeque::from([pl]);
        prev.insert(pl, pl);
        while let Some(c) = q.pop_front() {
            if c != pl && can_hit(c).is_some() {
                let mut s = c;
                while prev[&s] != pl {
                    s = prev[&s];
                }
                return match (s.0 - pl.0, s.1 - pl.1) {
                    (-1, _) => keys::LEFT,
                    (1, _) => keys::RIGHT,
                    (_, -1) => keys::UP,
                    _ => keys::DOWN,
                };
            }
            for d in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let n = (c.0 + d.0, c.1 + d.1);
                if !prev.contains_key(&n) && empty(n) {
                    prev.insert(n, c);
                    q.push_back(n);
                }
            }
        }
        b' '
    }
}

impl Io for LairBot {
    fn poll(&mut self, t: &Term) -> IoResult<Option<u8>> {
        if self.left == 0 {
            return Err(Halt);
        }
        self.left -= 1;
        self.keep(t);
        for l in t.messages() {
            if !l.trim().is_empty() && self.seen.last() != Some(&l) {
                self.seen.push(l);
            }
        }
        if let Some(k) = self.queue.pop_front() {
            return Ok(Some(k));
        }
        Ok(Some(self.decide(t)))
    }
    fn pause(&mut self, t: &Term, _ticks: u32) -> IoResult<()> {
        self.keep(t);
        Ok(())
    }
    fn wait_or_key(&mut self, t: &Term) -> IoResult<()> {
        self.keep(t);
        Ok(())
    }
}

#[test]
fn defeating_mondain_shows_the_ending() {
    let mut bot = LairBot::new(&format!("{TO_GEM}G"), 3000);
    let Some(g) = play_io(40, 40, &mut bot, 1, &[], |r| {
        at_time_machine(r);
        r.set_u16(off::HITS, 9999);
        r.set_u16(off::STATS, 99);
        r.set_u16(off::STATS + 2, 99);
        r.set_u16(off::STATS + 4, 99);
        r.set_u16(off::READY_WEAPON, 15);
    }) else {
        return;
    };
    assert!(bot.saw("Hit Mondain!"));
    assert!(g.term.row_text(24).contains("Press CONTROL-ALT-DEL to Restart"), "{:?}", g.term.messages());
    assert!(g.stopped.unwrap().contains("Mondain"));
    // For side-by-sides with DOSBox: the closing screens as PPM files.
    if let (Some(dir), Some(f)) = (std::env::var_os("U1_LAIR_FRAMES"), &bot.frames) {
        for (i, rgb) in f.iter().enumerate() {
            let mut out = b"P6 320 200 255\n".to_vec();
            out.extend_from_slice(rgb);
            std::fs::write(std::path::Path::new(&dir).join(format!("lair{i:02}.ppm")), out).unwrap();
        }
    }
}

#[test]
fn starving_shows_the_death_picture_then_resurrects() {
    let starving = |r: &mut PlayerRecord| r.set_u16(off::FOOD, 1);
    // Stopped at the pause after "thou art dead": the skull fills the view.
    let Some(g) = play_with(40, 40, "{right}{right}{p}", 1, &[], starving) else { return };
    assert!(said(&g, "thou art dead"));
    assert_eq!(g.term.at(160, 60), 15);
    assert_eq!(g.term.at(129, 31), 0);
    // Afterwards the restored values stick: no second death on the next moves.
    let g = play_with(40, 40, "{right}{right}{right}{right}{w1}", 1, &[], starving).unwrap();
    assert_eq!(g.hits(), 99);
    assert!(g.food() >= 95, "food {}", g.food());
    assert_eq!(g.rec.coin(), 0);
    assert!(!said(&g, "thou art dead"));
}

#[test]
fn leading_the_princess_out_pays_500_of_each() {
    use u1_assets::rules::Dir;
    // Inside Lord British's castle at the entrance (0,9).
    let Some(mut g) = play(40, 39, "{up}E{w1}") else { return };
    assert_eq!((g.px, g.py), (0, 9));
    let (hits, coin, exp) = (g.hits(), g.rec.coin(), g.exp());
    // Put the princess next to the player, as after opening her cell.
    let i = g.npcs.iter().position(|n| n.tile == 0x16).unwrap();
    g.npcs[i].x = 1;
    g.npcs[i].y = 9;
    // Stop at the reward's pause, while the box is still shown.
    let _ = g.town_move(&mut ScriptIo::from_text("{p}"), Dir::West);
    assert!(g.term.row_text(7).contains("Thou hast saved the princess"));
    assert!(g.term.row_text(9).contains("500 hit points, pence and"));
    assert_eq!(g.hits(), hits + 500);
    assert_eq!(g.rec.coin(), coin + 500);
    assert_eq!(g.exp(), exp + 500);
}

#[test]
fn intro_runs_to_the_credits_in_every_video_mode() {
    use u1_assets::gamedir::VideoMode;
    use u1_assets::title::{blank_term, run_intro, IntroEnd, TitleFiles};
    let Some(g) = game_dir() else { return };
    for v in [VideoMode::Ega, VideoMode::Cga, VideoMode::Tandy] {
        let f = TitleFiles::load(&g, v).unwrap();
        let mut t = blank_term(&f);
        let mut io = ScriptIo::from_text("{w81}{w5}x");
        assert!(matches!(run_intro(&f, &mut t, &mut io), Ok(IntroEnd::ToMenu)));
    }
}

#[test]
fn character_program_writes_the_original_record_and_selects_it() {
    use u1_assets::gamedir::VideoMode;
    use u1_assets::gen::{Gen, GenEnd, GenFiles};
    use u1_assets::term::Term;
    let Some(g) = game_dir() else { return };
    // Same keys as the DOSBox run the record was compared with (SPEC §17):
    // Str 25 and Agi 25, Elf, male fighter "Hero", kept; then play slot 1.
    let s = format!("a{{right}}{{right}}{{down}}{{right}}{{up}}{}{{down}}{} baaHero{{enter}}yb1", "{right}".repeat(13), "{right}".repeat(14));
    for v in [VideoMode::Ega, VideoMode::Cga, VideoMode::Tandy] {
        let dir = std::env::temp_dir().join(format!("u1-gen-test-{}-{v:?}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let f = GenFiles::load(&g, v).unwrap();
        let mut gen = Gen::new(&f, &dir, Box::new(|| [0, 0, 0, 0]));
        let mut t = Term::with_cga(v == VideoMode::Cga);
        let mut io = ScriptIo::from_text(&s);
        let end = gen.run(&mut t, &mut io).unwrap();
        let GenEnd::Play { slot, path } = end;
        assert_eq!(slot, 1);
        let b = std::fs::read(&path).unwrap();
        assert_eq!(b.len(), 820);
        let w = |o: usize| u16::from_le_bytes([b[o], b[o + 1]]);
        assert_eq!(&b[..5], b"Hero\0");
        assert_eq!((w(0x10), w(0x12), w(0x14)), (1, 0, 0), "elf male fighter");
        assert_eq!([w(0x16), w(0x18), w(0x1a), w(0x1c), w(0x1e), w(0x20), w(0x22)], [150, 35, 40, 10, 10, 10, 10]);
        assert_eq!([w(0x24), w(0x28), w(0x34), w(0x36), w(0xb8), w(0xba)], [100, 200, 49, 40, 49, 40]);
        // Seed: the clock value for 00:00:00.00 plus Wisdom and class.
        assert_eq!(w(0x32), (0x91eb & 0xfff) + 10);
        assert!(t.row_text(10).contains("thy game loads"));
        let r = PlayerRecord::parse(&b).unwrap();
        assert_eq!(r.u16_at(0x16), 150);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

/// A Dark knight (tile 0x21) placed at (x, y) as object 1.
fn knight_at(r: &mut PlayerRecord, x: u16, y: u16) {
    r.set_object(1, &u1_assets::save::MapObject { kind: 0x21, unknown_02: 0, x, y, hits: 40 });
    r.set_u16(off::LAST_OBJECT, 1);
}

#[test]
fn outdoor_magic_missile_flies_and_hits() {
    let Some(g) = play_with(40, 39, "C{right}", 2, &[], |r| {
        r.set_u16(off::READY_SPELL, 3);
        r.set_u16(0x86, 5);
        r.set_u16(0x20, 99);
        knight_at(r, 42, 39);
    }) else {
        return;
    };
    assert!(said(&g, "\"DELCIO-ERE-UI\""));
    assert!(said(&g, "Dark knight"));
    assert_eq!(g.rec.u16_at(0x86), 4);
}

#[test]
fn outdoor_prayer_can_remove_an_adjacent_monster() {
    let Some(g) = play_with(40, 39, "{w1}C", 3, &[], |r| {
        r.set_u16(off::READY_SPELL, 0);
        knight_at(r, 41, 39);
    }) else {
        return;
    };
    assert!(said(&g, "Monster removed!"));
    assert!(g.ow.entity_at(41, 39).is_none());
    assert_eq!(g.ow.uncounted, 1);
}

/// Keys from a script; at every timed pause the screen is checked for the
/// light red of a fighter's laser in the viewer (the sun on the
/// lift-off map is red too, hence the message check).
struct LaserWatch {
    s: ScriptIo,
    shots: Vec<Vec<u8>>,
}

impl Io for LaserWatch {
    fn poll(&mut self, t: &Term) -> IoResult<Option<u8>> {
        self.s.poll(t)
    }
    fn pause(&mut self, t: &Term, _ticks: u32) -> IoResult<()> {
        let red = (24..140).flat_map(|y| (24..296).map(move |x| (x, y))).filter(|&(x, y)| t.at(x, y) == 12).count();
        if red > 10 && t.messages().iter().any(|m| m.contains("Alien fires!")) {
            self.shots.push(t.to_rgb(&u1_assets::palette::EGA_PALETTE, 1));
        }
        Ok(())
    }
    fn wait_or_key(&mut self, t: &Term) -> IoResult<()> {
        self.s.wait_or_key(t)
    }
    fn timeout_polls(&self) -> u32 {
        1
    }
}

/// A fighter in a hyperjumped-to sector shoots now and then; a laser that
/// ends at the bottom of the viewer hits and drains 321 of the shield
/// (DOSBox with a 9999 shield: 9999, 9678, ... 8715).
#[test]
fn fighters_fire_back() {
    let mut io = LaserWatch { s: ScriptIo::from_text("B{w12}VH{w300}"), shots: Vec::new() };
    let Some(g) = play_io(40, 40, &mut io, 1, &[], |r| {
        on_shuttle(r);
        r.set_u16(off::SHUTTLE_0, 9999);
        r.set_u16(off::SHUTTLE_1, 9999);
    }) else {
        return;
    };
    assert!(!io.shots.is_empty(), "{:?}", g.term.messages());
    assert!(said(&g, "You've been hit!"));
    let shield = g.term.row_text(21);
    assert!(shield.contains("Shld:9678") || shield.contains("Shld:9357"), "{shield}");
    if let Some(dir) = std::env::var_os("U1_LAIR_FRAMES") {
        for (i, rgb) in io.shots.iter().enumerate() {
            let mut out = b"P6 320 200 255\n".to_vec();
            out.extend_from_slice(rgb);
            std::fs::write(std::path::Path::new(&dir).join(format!("laser{i:02}.ppm")), out).unwrap();
        }
    }
}

/// A straight corridor running south from (5, 1) with a secret door and a
/// door beside the second square, a chest on the third and a skeleton on
/// the fourth, seen from (5, 2) facing south.
fn corridor_view() -> Option<Term> {
    use u1_assets::dungeon::{Cell, DungeonLevel, Item, DUNGEON_SIZE};
    use u1_assets::dview::{self, ViewTables, SOUTH};
    let g = game_dir()?;
    let exe = Exe::new(g.read("OUT.EXE").ok()?).unwrap();
    let n = DUNGEON_SIZE;
    let mut lv = DungeonLevel {
        level: 1,
        cells: [[Cell::Wall; DUNGEON_SIZE]; DUNGEON_SIZE],
        items: [[Item::None; DUNGEON_SIZE]; DUNGEON_SIZE],
        monsters: [[-1; DUNGEON_SIZE]; DUNGEON_SIZE],
        hp: [[0; DUNGEON_SIZE]; DUNGEON_SIZE],
        used: [false; 5],
    };
    for b in 1..n - 2 {
        lv.cells[5][b] = Cell::Empty;
    }
    lv.cells[4][3] = Cell::SecretDoor;
    lv.cells[6][3] = Cell::Door;
    lv.items[5][4] = Item::Chest;
    lv.monsters[5][5] = 1;
    lv.hp[5][5] = 20;
    let mut t = Term::new();
    dview::draw(&mut t, &exe, &ViewTables::from_exe(&exe), &lv, 5, 2, SOUTH);
    Some(t)
}

/// A 4x4 picture of one colour.
fn solid(c: [u8; 4]) -> u1_assets::enhanced::Rgba {
    let mut r = u1_assets::enhanced::Rgba::new(4, 4);
    for p in r.px.chunks_mut(4) {
        p.copy_from_slice(&c);
    }
    r
}

/// A pack with the given `[section] = colour` pictures, 32-pixel tiles.
fn colour_pack(parts: &[(&str, [u8; 4])]) -> u1_assets::enhanced::Pack {
    let mut text = String::from("name = \"t\"\ntile_size = 32\n");
    for (sec, _) in parts {
        text += &format!("[{sec}]\nfile = \"{sec}.png\"\n");
    }
    let map: std::collections::HashMap<String, [u8; 4]> = parts.iter().map(|(s, c)| (format!("{s}.png"), *c)).collect();
    u1_assets::enhanced::Pack::parse(&text, |f| Ok(solid(map[f]))).unwrap()
}

#[test]
fn dungeon_view_notes_its_surfaces_and_objects() {
    use u1_assets::dview::DPiece;
    use u1_assets::term::{dun, ArtId};
    let Some(t) = corridor_view() else { return };
    let sc = t.dscene.as_ref().expect("scene kept");
    let has_face = |id: ArtId| sc.pieces.iter().any(|(_, p)| matches!(p, DPiece::Face(f) if f.id == id));
    let has_obj = |id: ArtId| sc.pieces.iter().any(|(_, p)| matches!(p, DPiece::Obj(o) if o.id == id));
    assert!(has_face(ArtId::Dun(dun::WALL)));
    assert!(has_face(ArtId::Dun(dun::SECRET)));
    assert!(has_face(ArtId::Dun(dun::DOOR)));
    assert!(has_obj(ArtId::Dun(dun::CHEST)));
    assert!(has_obj(ArtId::DMon(1)));
    // Every piece lies inside the view, and the farthest is drawn first.
    for (_, p) in &sc.pieces {
        let (x0, x1) = match p {
            DPiece::Face(f) => (f.x0, f.x1),
            DPiece::Obj(o) => (o.x0, o.x1),
        };
        assert!(8 <= x0 && x0 <= x1 && x1 <= 311, "{p:?}");
    }
    let keys: Vec<i32> = sc.pieces.iter().map(|p| p.0).collect();
    let order = sc.ordered();
    assert_eq!(order.len(), keys.len());
    assert!(t.art.iter().any(|op| op.id == ArtId::Screen("dungeon")));
    // The chest (one square nearer) is drawn after the skeleton.
    let pos = |id: ArtId| order.iter().position(|p| matches!(p, DPiece::Obj(o) if o.id == id)).unwrap();
    assert!(pos(ArtId::Dun(dun::CHEST)) > pos(ArtId::DMon(1)));
}

#[test]
fn enhanced_dungeon_textures_with_fallbacks() {
    use u1_assets::dview::DPiece;
    use u1_assets::enhanced::{compose, Pack};
    use u1_assets::term::ArtId;
    let Some(t) = corridor_view() else { return };
    let pal = &u1_assets::palette::EGA_PALETTE;
    let plain = t.to_rgb(pal, 2);
    // No dungeon art: the original, scaled.
    assert_eq!(compose(&t, pal, &Pack::empty(32), 0), plain);
    let red = [200, 0, 0, 255];
    let walls = colour_pack(&[("dungeon.wall", red)]);
    let out = compose(&t, pal, &walls, 0);
    assert_ne!(out, plain);
    let reddish = out.chunks(3).filter(|p| p[0] > 40 && p[1] == 0 && p[2] == 0).count();
    assert!(reddish > 20_000, "{reddish}");
    // A secret door without its own picture looks like a wall (it does in
    // the original too), so a pack giving it the wall's colour changes
    // nothing.
    let same = colour_pack(&[("dungeon.wall", red), ("dungeon.secret", red)]);
    assert_eq!(compose(&t, pal, &same, 0), out);
    let green = colour_pack(&[("dungeon.wall", red), ("dungeon.secret", [0, 200, 0, 255])]);
    assert_ne!(compose(&t, pal, &green, 0), out);
    // The skeleton has no picture: its original outline shows.
    let sc = t.dscene.as_ref().unwrap();
    let Some(DPiece::Obj(o)) = sc.pieces.iter().map(|p| &p.1).find(|p| matches!(p, DPiece::Obj(o) if o.id == ArtId::DMon(1))) else { panic!("no skeleton") };
    let lit: Vec<(usize, usize)> = (o.y0..=o.y1).flat_map(|y| (o.x0..=o.x1).map(move |x| (x as usize, y as usize))).filter(|&(x, y)| t.px[y * 320 + x] != 0).collect();
    assert!(!lit.is_empty());
    for (x, y) in lit {
        let i = ((y * 2) * 640 + x * 2) * 3;
        assert_eq!(out[i..i + 3], plain[i..i + 3]);
    }
    // With a sprite it is drawn instead.
    let sk = colour_pack(&[("dungeon.wall", red), ("dmon.1", [0, 0, 220, 255])]);
    let out2 = compose(&t, pal, &sk, 0);
    assert!(out2.chunks(3).any(|p| p[0] == 0 && p[1] == 0 && p[2] > 100));
    // A chest is a low, wide box in the original: a square picture fills
    // the box's width (rising above it) rather than shrinking to its height.
    let Some(DPiece::Obj(c)) = sc.pieces.iter().map(|p| &p.1).find(|p| matches!(p, DPiece::Obj(o) if o.id == ArtId::Dun(u1_assets::term::dun::CHEST))) else { panic!("no chest") };
    assert!(c.x1 - c.x0 > c.y1 - c.y0, "{c:?}");
    let ch = colour_pack(&[("dungeon.wall", red), ("dungeon.chest", [0, 230, 0, 255])]);
    let out3 = compose(&t, pal, &ch, 0);
    let y = (c.y1 as usize) * 2;
    let green: Vec<usize> = (0..640)
        .filter(|&x| {
            let p = &out3[(y * 640 + x) * 3..][..3];
            p[1] > 60 && p[0] == 0 && p[2] == 0
        })
        .collect();
    let span = green.last().unwrap() - green.first().unwrap() + 1;
    assert!(span as i32 >= 2 * (c.x1 - c.x0), "{span} vs {c:?}");
}

#[test]
fn enhanced_dungeon_holes_keep_the_original_outline() {
    use u1_assets::dungeon::{Cell, DungeonLevel, Item, DUNGEON_SIZE};
    use u1_assets::dview::{self, DPiece, ViewTables, SOUTH};
    use u1_assets::enhanced::compose;
    use u1_assets::term::{dun, ArtId};
    let Some(g) = game_dir() else { return };
    let exe = Exe::new(g.read("OUT.EXE").unwrap()).unwrap();
    let vt = ViewTables::from_exe(&exe);
    let pal = &u1_assets::palette::EGA_PALETTE;
    for (cell, id, sec) in [(Cell::LadderUp, dun::HOLE_UP, "dungeon.hole_up"), (Cell::LadderDown, dun::HOLE_DOWN, "dungeon.hole_down")] {
        let mut lv = DungeonLevel {
            level: 1,
            cells: [[Cell::Wall; DUNGEON_SIZE]; DUNGEON_SIZE],
            items: [[Item::None; DUNGEON_SIZE]; DUNGEON_SIZE],
            monsters: [[-1; DUNGEON_SIZE]; DUNGEON_SIZE],
            hp: [[0; DUNGEON_SIZE]; DUNGEON_SIZE],
            used: [false; 5],
        };
        for b in 1..DUNGEON_SIZE - 2 {
            lv.cells[5][b] = Cell::Empty;
        }
        lv.cells[5][3] = cell;
        let mut t = Term::new();
        dview::draw(&mut t, &exe, &vt, &lv, 5, 2, SOUTH);
        let sc = t.dscene.as_ref().unwrap();
        let Some(DPiece::Obj(h)) = sc.pieces.iter().map(|p| &p.1).find(|p| matches!(p, DPiece::Obj(o) if o.id == ArtId::Dun(id))) else { panic!("no hole") };
        // Four sides, level top and bottom; the far edge is the narrow one.
        let [tl, tr, br, bl] = h.quad.expect("hole outline");
        assert_eq!((tl.1, tr.1, br.1, bl.1), (h.y0, h.y0, h.y1, h.y1));
        let (top, bot) = (tr.0 - tl.0, br.0 - bl.0);
        if id == dun::HOLE_UP {
            assert!(top > bot, "{h:?}");
        } else {
            assert!(bot > top, "{h:?}");
        }
        let out = compose(&t, pal, &colour_pack(&[("dungeon.wall", [200, 0, 0, 255]), (sec, [0, 0, 230, 255])]), 0);
        let blue = |x: i32, y: i32| {
            let p = &out[((y as usize * 2) * 640 + x as usize * 2) * 3..][..3];
            p[2] > 60 && p[0] == 0 && p[1] == 0
        };
        // Inside the outline (left of the ladder) the picture shows; the
        // box's corners beside the narrow edge stay as they were.
        assert!(blue((tl.0 + bl.0) / 2 + 4, (h.y0 + h.y1) / 2), "{h:?}");
        let (cx, cy) = if id == dun::HOLE_UP { (h.x0, h.y1) } else { (h.x0, h.y0) };
        assert!(!blue(cx, cy), "{h:?}");
    }
}

#[test]
fn enhanced_town_tiles_and_people() {
    use u1_assets::enhanced::compose;
    let Some(g) = play(66, 41, "e") else { return };
    let pal = &u1_assets::palette::EGA_PALETTE;
    let plain = g.term.to_rgb(pal, 2);
    let floor = colour_pack(&[("town.1", [200, 0, 0, 255])]);
    let out = compose(&g.term, pal, &floor, 0);
    let count = |o: &[u8], c: [u8; 3]| o.chunks(3).filter(|p| *p == c).count();
    assert!(count(&out, [200, 0, 0]) > 10_000);
    assert_eq!(count(&plain, [200, 0, 0]), 0);
    // Guards (tile 0x11) and the player (0x12) get their own pictures.
    let people = colour_pack(&[("town.1", [200, 0, 0, 255]), ("town.17", [0, 0, 220, 255]), ("town.18", [0, 220, 0, 255])]);
    let out = compose(&g.term, pal, &people, 0);
    assert!(count(&out, [0, 0, 220]) >= 16 * 16);
    assert_eq!(count(&out, [0, 220, 0]), 16 * 16);
}

/// The space view is noted as pictures for the enhanced renderer: the home
/// sector's Earth, sun, station and ships (turned to their headings), the
/// front view's crosshair; none while the sector scan is up.
#[test]
fn space_scene_notes_the_pictures_drawn() {
    use u1_assets::enhanced::compose;
    use u1_assets::term::space_part as sp;
    let Some(g) = play_with(40, 40, "B{w12}", 1, &[], on_shuttle) else { return };
    let sc = g.term.sscene.as_ref().expect("scene kept");
    assert!(!sc.fp && sc.frame.intact(&g.term));
    let find = |p| sc.objs.iter().find(|o| o.part == p).unwrap();
    let e = find(sp::EARTH);
    assert_eq!((e.x, e.y, e.w, e.h), (60, 75, 21, 18));
    assert_eq!((find(sp::SUN).x, find(sp::SUN).y, find(sp::STATION).x, find(sp::STATION).y), (140, 70, 200, 100));
    // The shuttle points west, the docked ships east and north.
    assert_eq!((find(sp::SHIP0).rot, find(sp::SHIP0 + 1).rot, find(sp::SHIP0 + 2).rot), (3, 1, 0));
    assert!(sc.objs.iter().all(|o| o.shown(&g.term)));
    let pal = &u1_assets::palette::EGA_PALETTE;
    let out = compose(&g.term, pal, &colour_pack(&[("space.starfield", [200, 0, 0, 255]), ("space.earth", [0, 200, 0, 255])]), 0);
    let at = |x: usize, y: usize| out[(y * 2 * 640 + x * 2) * 3..][..3].to_vec();
    assert_eq!(at(30, 30), [200, 0, 0]);
    assert_eq!(at(70, 84), [0, 200, 0]);
    // The shuttle has no picture: its pixels stay; the frame too.
    let s0 = find(sp::SHIP0);
    let i = s0.lit.iter().find(|&&i| g.term.px[i as usize] == 15).unwrap();
    assert_eq!(at(*i as usize % 320, *i as usize / 320), pal[15].to_vec());
    assert_eq!(at(9, 9), pal[g.term.at(9, 9) as usize].to_vec());
    let g = play_with(40, 40, "B{w12}V{w2}", 1, &[], on_shuttle).unwrap();
    let sc = g.term.sscene.as_ref().unwrap();
    assert!(sc.fp);
    let r = sc.objs.iter().find(|o| o.part == sp::RETICLE).unwrap();
    assert_eq!((r.x, r.y, r.w, r.h), (157, 77, 5, 5));
    let g = play_with(40, 40, "B{w12}I", 1, &[], on_shuttle).unwrap();
    assert!(g.term.sscene.is_none());
}

/// Scripted keys, with the options menu's display switch asked for after
/// `at` polls (as F1 would).
struct SwitchIo {
    inner: ScriptIo,
    polls: usize,
    at: usize,
    to: Vec<u1_assets::gamedir::VideoMode>,
    shown: Vec<u1_assets::gamedir::VideoMode>,
}

impl Io for SwitchIo {
    fn poll(&mut self, t: &Term) -> IoResult<Option<u8>> {
        self.polls += 1;
        self.inner.poll(t)
    }
    fn pause(&mut self, t: &Term, ticks: u32) -> IoResult<()> {
        self.inner.pause(t, ticks)
    }
    fn wait_or_key(&mut self, t: &Term) -> IoResult<()> {
        self.inner.wait_or_key(t)
    }
    fn video_switch(&mut self) -> Option<u1_assets::game::VideoSet> {
        if self.polls < self.at || self.to.is_empty() {
            return None;
        }
        let mode = self.to.remove(0);
        let g = game_dir()?;
        Some(u1_assets::game::VideoSet {
            mode,
            world_tiles: g.load_world_tiles(mode).ok()?,
            town_tiles: g.load_town_tiles(mode).ok()?,
            space: SpaceFiles::load(&g, mode).ok(),
            mondain: u1_assets::mondain::MondainFiles::load(&g, mode).ok(),
        })
    }
    fn video_shown(&mut self, mode: u1_assets::gamedir::VideoMode) {
        self.shown.push(mode);
    }
}

#[test]
fn options_menu_switches_the_display_live() {
    use u1_assets::gamedir::VideoMode;
    let keys = "{right}{right}{left}";
    let Some(plain) = play(45, 38, keys) else { return };
    let mut io = SwitchIo { inner: ScriptIo::from_text(keys), polls: 0, at: 2, to: vec![VideoMode::Cga], shown: vec![] };
    let g = play_io(45, 38, &mut io, 2, &[], |_| {}).unwrap();
    assert_eq!(io.shown, vec![VideoMode::Cga]);
    assert_eq!(g.a.video, VideoMode::Cga);
    assert!(g.term.cga);
    assert!(g.term.px.iter().all(|&c| c < 4), "CGA colours only");
    assert_eq!((g.ow.x, g.ow.y), (plain.ow.x, plain.ow.y), "the game itself is unchanged");
    assert_eq!(g.term.messages(), plain.term.messages());
    // And back: the same screen as a game that never left EGA.
    let mut io = SwitchIo { inner: ScriptIo::from_text(keys), polls: 0, at: 2, to: vec![VideoMode::Cga, VideoMode::Ega], shown: vec![] };
    let g = play_io(45, 38, &mut io, 2, &[], |_| {}).unwrap();
    assert_eq!(io.shown, vec![VideoMode::Cga, VideoMode::Ega]);
    assert!(!g.term.cga);
    let diff: Vec<(usize, usize, u8, u8)> = (0..g.term.px.len()).filter(|&i| g.term.px[i] != plain.term.px[i]).map(|i| (i % 320, i / 320, g.term.px[i], plain.term.px[i])).collect();
    assert!(diff.is_empty(), "{} differ, e.g. {:?}", diff.len(), &diff[..diff.len().min(12)]);
}

#[test]
fn only_near_townspeople_steps_get_the_enhanced_sound() {
    use u1_assets::sound::Sfx;
    // In Moon, eight squares in; one wanderer is put beside the player.
    let Some(mut g) = play(66, 41, &format!("e{UP8}")) else { return };
    assert_eq!(g.mode, 1);
    let (px, py) = (g.px as i16, g.py as i16);
    g.npcs[0].x = px + 1;
    g.npcs[0].y = py + 1;
    let mut io = ScriptIo::from_text("");
    for _ in 0..60 {
        let _ = g.npc_turn(&mut io);
    }
    // Every townsperson's step still beeps as in the original (effect 4);
    // only those within range are given the enhanced footstep.
    let own = io.events.iter().filter(|&&e| e == Sfx::Step).count();
    let beeps = io.sounds.iter().filter(|&&n| n == 4).count() - own;
    let near = io.events.iter().filter(|&&e| e == Sfx::NpcStep).count();
    assert!(near > 0 && beeps > near, "{near} near of {beeps}");
}

#[test]
fn options_menu_switches_the_display_in_space() {
    use u1_assets::gamedir::VideoMode;
    let keys = "B{w12}{w5}";
    let Some(plain) = play_with(40, 40, keys, 1, &[], on_shuttle) else { return };
    let mut io = SwitchIo { inner: ScriptIo::from_text(keys), polls: 0, at: 14, to: vec![VideoMode::Cga], shown: vec![] };
    let g = play_io(40, 40, &mut io, 1, &[], on_shuttle).unwrap();
    assert_eq!(io.shown, vec![VideoMode::Cga], "taken in space, not at the return to OUT");
    assert!(g.term.sscene.is_some(), "still in space");
    assert!(g.term.cga && g.term.px.iter().all(|&c| c < 4), "CGA colours only");
    assert_eq!(g.term.messages(), plain.term.messages());
    // And back: the same screen as a flight that never left EGA.
    let mut io = SwitchIo { inner: ScriptIo::from_text(keys), polls: 0, at: 14, to: vec![VideoMode::Cga, VideoMode::Ega], shown: vec![] };
    let g = play_io(40, 40, &mut io, 1, &[], on_shuttle).unwrap();
    assert_eq!(io.shown, vec![VideoMode::Cga, VideoMode::Ega]);
    let diff = (0..g.term.px.len()).filter(|&i| g.term.px[i] != plain.term.px[i]).count();
    assert_eq!(diff, 0);
}

#[test]
fn options_menu_switches_the_display_in_mondains_lair() {
    use u1_assets::gamedir::VideoMode;
    let keys = format!("{TO_GEM}{{w3}}");
    let Some(plain) = play_with(40, 40, &keys, 1, &[], at_time_machine) else { return };
    // Mid-way along the corridor to the gem.
    let at = 12;
    let mut io = SwitchIo { inner: ScriptIo::from_text(&keys), polls: 0, at, to: vec![VideoMode::Cga, VideoMode::Ega], shown: vec![] };
    let g = play_io(40, 40, &mut io, 1, &[], at_time_machine).unwrap();
    assert_eq!(io.shown, vec![VideoMode::Cga, VideoMode::Ega]);
    assert!(said(&g, "You hear a strange chanting!"));
    let diff = (0..g.term.px.len()).filter(|&i| g.term.px[i] != plain.term.px[i]).count();
    assert_eq!(diff, 0, "back on EGA: the same arena");
    let mut io = SwitchIo { inner: ScriptIo::from_text(&keys), polls: 0, at, to: vec![VideoMode::Cga], shown: vec![] };
    let g = play_io(40, 40, &mut io, 1, &[], at_time_machine).unwrap();
    assert_eq!(io.shown, vec![VideoMode::Cga]);
    assert!(g.term.cga && g.term.px.iter().all(|&c| c < 4), "CGA colours only");
}

/// Scripted keys; after `at` waits the options menu's display change stops
/// the start-up program (once), as the front end does.
struct RestartIo {
    inner: ScriptIo,
    calls: usize,
    at: usize,
    to: Option<u1_assets::gamedir::VideoMode>,
    pending: Option<u1_assets::gamedir::VideoMode>,
    shown: Vec<u1_assets::gamedir::VideoMode>,
}

impl RestartIo {
    fn step(&mut self) -> IoResult<()> {
        self.calls += 1;
        if self.calls == self.at {
            if let Some(m) = self.to {
                self.pending = Some(m);
                return Err(Halt);
            }
        }
        Ok(())
    }
}

impl Io for RestartIo {
    fn poll(&mut self, t: &Term) -> IoResult<Option<u8>> {
        self.step()?;
        self.inner.poll(t)
    }
    fn pause(&mut self, t: &Term, ticks: u32) -> IoResult<()> {
        self.step()?;
        self.inner.pause(t, ticks)
    }
    fn wait_or_key(&mut self, t: &Term) -> IoResult<()> {
        self.step()?;
        self.inner.wait_or_key(t)
    }
    fn display_restart(&mut self) -> Option<u1_assets::gamedir::VideoMode> {
        let m = self.pending.take()?;
        self.shown.push(m);
        Some(m)
    }
    fn video_shown(&mut self, mode: u1_assets::gamedir::VideoMode) {
        self.shown.push(mode);
    }
}

#[test]
fn a_display_chosen_on_the_title_restarts_it_in_that_display() {
    use u1_assets::gamedir::VideoMode;
    let Some(g) = game_dir() else { return };
    let mut io = RestartIo { inner: ScriptIo::from_text("{w81}{w5}x"), calls: 0, at: 4, to: Some(VideoMode::Cga), pending: None, shown: vec![] };
    let mut mode = VideoMode::Ega;
    assert!(u1_assets::front::intro(&g, &mut mode, &mut io).unwrap().is_none(), "ran to the credits");
    assert_eq!((mode, io.shown.clone()), (VideoMode::Cga, vec![VideoMode::Cga]));
    // A real halt stays a halt.
    let mut io = RestartIo { inner: ScriptIo::from_text("{w3}"), calls: 0, at: 0, to: None, pending: None, shown: vec![] };
    let mut mode = VideoMode::Ega;
    assert!(u1_assets::front::intro(&g, &mut mode, &mut io).unwrap().is_some());
    assert_eq!(mode, VideoMode::Ega);
}

#[test]
fn a_display_chosen_in_the_character_program_restarts_it_in_that_display() {
    use u1_assets::gamedir::VideoMode;
    use u1_assets::gen::GenEnd;
    let Some(g) = game_dir() else { return };
    let s = format!("a{{right}}{{right}}{{down}}{{right}}{{up}}{}{{down}}{} baaHero{{enter}}yb1", "{right}".repeat(13), "{right}".repeat(14));
    let dir = std::env::temp_dir().join(format!("u1-gen-restart-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    // Stopped on its first screen, before a key was read.
    let mut io = RestartIo { inner: ScriptIo::from_text(&s), calls: 0, at: 1, to: Some(VideoMode::Tandy), pending: None, shown: vec![] };
    let mut mode = VideoMode::Ega;
    let mut clock = || -> Box<dyn FnMut() -> [u8; 4]> { Box::new(|| [0, 0, 0, 0]) };
    let (t, r) = u1_assets::front::character(&g, &dir, &mut mode, &mut clock, &mut io).unwrap();
    let Ok(GenEnd::Play { slot, path }) = r else { panic!("{r:?}") };
    assert_eq!((slot, mode, io.shown.clone()), (1, VideoMode::Tandy, vec![VideoMode::Tandy]));
    assert!(!t.cga);
    assert_eq!(&std::fs::read(&path).unwrap()[..5], b"Hero\0");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn save_slots_write_original_records_and_load_them() {
    use u1_assets::slots::{slot_path, Slot};
    let Some(_) = game_dir() else { return };
    let dir = std::env::temp_dir().join(format!("u1slots-game-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let base = dir.join("PLAYER2.U1");
    // Saved outdoors to slot 3, the quicksave and slot 1 (Quit's file).
    let mut g = play(66, 41, "").unwrap();
    g.save_to = Some(base.clone());
    let mut io = ScriptIo::from_text("{save3}{saveq}{save1}");
    let _ = g.run(&mut io);
    assert_eq!(io.done, ["Saved to slot 3", "Saved to the quicksave", "Saved to slot 1"]);
    assert!(g.stopped.is_none(), "saving does not stop the game");
    for s in [Slot::N(3), Slot::Quick, Slot::N(1)] {
        let b = std::fs::read(slot_path(&base, s)).unwrap();
        assert_eq!(b.len(), 820, "the original's record");
        let r = PlayerRecord::parse(&b).unwrap();
        assert_eq!((r.position(), r.name()), ((66, 41), g.rec.name()));
    }
    assert_eq!(std::fs::read(&base).unwrap(), std::fs::read(slot_path(&base, Slot::Quick)).unwrap());
    // Loading stops the run with the record for the front end.
    let mut g = play(66, 41, "").unwrap();
    g.save_to = Some(base.clone());
    let mut io = ScriptIo::from_text("{down}{load2}{load3}{down}");
    let _ = g.run(&mut io);
    assert_eq!(io.done, ["Nothing saved in slot 2", "Loaded slot 3"]);
    assert_eq!(g.stopped, Some("load"));
    assert_eq!(g.reload.as_ref().unwrap().as_bytes(), &std::fs::read(slot_path(&base, Slot::N(3))).unwrap()[..]);
    assert_eq!(io.keys.len(), 1, "the run ends at the load");
    // Not in a dungeon (as Quit).
    let mut g = play(39, 60, "e").unwrap();
    assert_eq!(g.mode, 3);
    g.save_to = Some(dir.join("D.U1"));
    let mut io = ScriptIo::from_text("{save2}");
    let _ = g.run(&mut io);
    assert_eq!(io.done, ["Saving works outdoors only (as Quit)"]);
    assert!(!slot_path(&dir.join("D.U1"), Slot::N(2)).exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn qol_off_leaves_the_screen_and_record_as_they_were() {
    // The message log and the slot tokens' idle polls change nothing a
    // plain run shows: the same keys with the log kept draw the same.
    let Some(plain) = play(66, 41, "e{down}{left}{left}") else { return };
    let mut io = ScriptIo::from_text("e{down}{left}{left}");
    io.log = true;
    let logged = play_io(66, 41, &mut io, 2, &[], |_| {}).unwrap();
    assert_eq!(plain.term.px, logged.term.px);
    assert_eq!(plain.rec.as_bytes(), logged.rec.as_bytes());
    assert!(plain.term.log.is_none() && logged.term.log.is_some());
}

// ------------------------------------------------------------ SPEC §23

fn rep(t: &str, n: usize) -> String {
    t.repeat(n)
}

fn second_group() -> u1_assets::qol::Qol {
    u1_assets::qol::Qol { automap: true, minimap: true, charsheet: true, help: true, ..Default::default() }
}

const DUNGEON_WALK: &str = "e{up}{up}{left}{up}{up}{right}{up}";
// Each screen waits for a key and drops it, so each is typed twice (open,
// close); with the screens off all of them are ignored.
const DUNGEON_WALK_VIEWS: &str = "e{map}{map}{up}{help}{help}{up}{left}{up}{up}{map}{map}{right}{up}";

#[test]
fn second_group_off_ignores_the_view_keys() {
    // With everything off the engine's keys do nothing at all: the same
    // run with them typed draws and records the same.
    let Some(plain) = play(59, 29, DUNGEON_WALK) else { return };
    let typed = play(59, 29, DUNGEON_WALK_VIEWS).unwrap();
    assert_eq!(plain.term.px, typed.term.px);
    assert_eq!(plain.rec.as_bytes(), typed.rec.as_bytes());
    assert!(typed.explore.is_empty(), "nothing is noted while the maps are off");
}

#[test]
fn second_group_on_changes_no_record_and_puts_the_screen_back() {
    let Some(plain) = play(59, 29, DUNGEON_WALK) else { return };
    let mut io = ScriptIo::from_text(DUNGEON_WALK_VIEWS);
    io.qol = second_group();
    let on = play_io(59, 29, &mut io, 2, &[], |_| {}).unwrap();
    assert_eq!(plain.rec.as_bytes(), on.rec.as_bytes());
    assert_eq!(plain.term.px, on.term.px, "every view closes back to the game");
    // The automap knows only part of the level, and only as seen.
    let d = on.dungeon.as_ref().expect("still in the dungeon");
    let key = (on.ow.x as u8, on.ow.y as u8, d.depth);
    let grid = on.explore.level(key).expect("level noted");
    let seen = grid.iter().flatten().filter(|c| **c != u1_assets::explore::Seen::Unseen).count();
    assert!(seen > 5 && seen < 121, "seen {seen}");
    for a in 0..grid.len() {
        for b in 0..grid.len() {
            if d.level.cell(a as i32, b as i32) == u1_assets::dungeon::Cell::SecretDoor {
                assert!(matches!(grid[a][b], u1_assets::explore::Seen::Wall | u1_assets::explore::Seen::Unseen));
            }
        }
    }
}

#[test]
fn automap_screen_shows_the_seen_count() {
    let mut io = ScriptIo::from_text(&format!("{DUNGEON_WALK}{{map}}"));
    io.qol = second_group();
    let Some(g) = play_io(59, 29, &mut io, 2, &[], |_| {}) else { return };
    let all: String = (0..25).map(|r| g.term.row_text(r)).collect::<Vec<_>>().join("\n");
    assert!(all.contains("Automap"), "{all}");
    assert!(all.contains("Seen ") && all.contains(" of 121"), "{all}");
}

#[test]
fn automap_facing_is_the_original_compass_word() {
    // The dungeon view's compass (the original's words) and the automap
    // agree, for each way round.
    for (turns, word) in [("", "South"), ("{left}", "East"), ("{left}{left}", "North"), ("{right}", "West")] {
        let keys = format!("e{turns}");
        let mut io = ScriptIo::from_text(&keys);
        io.qol = second_group();
        let Some(g) = play_io(59, 29, &mut io, 2, &[], |_| {}) else { return };
        let compass = g.term.row_text(19);
        assert!(compass.contains(word), "{turns}: compass {compass:?}");
        let mut io = ScriptIo::from_text(&format!("{keys}{{map}}"));
        io.qol = second_group();
        let g = play_io(59, 29, &mut io, 2, &[], |_| {}).unwrap();
        let all: String = (0..25).map(|r| g.term.row_text(r)).collect::<Vec<_>>().join("\n");
        assert!(all.contains(&format!("Facing {word}")), "{turns}: {all}");
    }
}

#[test]
fn minimap_opens_zoomed_and_z_switches_to_the_whole_world() {
    let walk = format!("{}{}", rep("{right}", 4), rep("{down}", 2));
    let Some(plain) = play(45, 38, &walk) else { return };
    let text = |g: &Game| (0..25).map(|r| g.term.row_text(r)).collect::<Vec<_>>().join("\n");
    let mut io = ScriptIo::from_text(&format!("{walk}{{map}}"));
    io.qol = second_group();
    let zoomed = play_io(45, 38, &mut io, 2, &[], |_| {}).unwrap();
    assert!(text(&zoomed).contains("Z whole map"), "{}", text(&zoomed));
    let mut io = ScriptIo::from_text(&format!("{walk}{{map}}z"));
    io.qol = second_group();
    let whole = play_io(45, 38, &mut io, 2, &[], |_| {}).unwrap();
    assert!(text(&whole).contains("Z zoom in"), "{}", text(&whole));
    assert_ne!(zoomed.term.px, whole.term.px);
    // Z, Z again, then any key: back to the game as it was (the text; the
    // water moves on with each key, so not every pixel).
    let mut io = ScriptIo::from_text(&format!("{walk}{{map}}zZ{{map}}"));
    io.qol = second_group();
    let back = play_io(45, 38, &mut io, 2, &[], |_| {}).unwrap();
    assert_eq!(text(&plain), text(&back));
    assert_eq!(plain.rec.as_bytes(), back.rec.as_bytes());
}

#[test]
fn sidecar_is_written_beside_the_save_and_the_record_is_the_original_s() {
    let dir = std::env::temp_dir().join(format!("u1-qol2-sidecar-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let walk = format!("{}{}q", rep("{right}", 6), rep("{down}", 3));
    let (a, b) = (dir.join("A.U1"), dir.join("B.U1"));
    SAVE_TO.with(|s| *s.borrow_mut() = Some(a.clone()));
    let Some(_) = play(45, 38, &walk) else {
        SAVE_TO.with(|s| s.borrow_mut().take());
        return;
    };
    SAVE_TO.with(|s| *s.borrow_mut() = Some(b.clone()));
    let mut io = ScriptIo::from_text(&walk);
    io.qol = second_group();
    play_io(45, 38, &mut io, 2, &[], |_| {}).unwrap();
    let (ra, rb) = (std::fs::read(&a).unwrap(), std::fs::read(&b).unwrap());
    assert_eq!(ra.len(), 820);
    assert_eq!(ra, rb, "the map sidecar never touches the record");
    assert!(!u1_assets::explore::sidecar_path(&a).exists(), "no sidecar with the maps off");
    let side = u1_assets::explore::Explore::load(&b);
    assert!(side.world_count() > 0);
    let _ = std::fs::remove_dir_all(&dir);
}

/// A script that notes, at each command prompt, what the map sidecar on
/// disk holds then (so writes during a run can be told from the one at
/// its end).
struct Watch<'a> {
    inner: ScriptIo,
    side: &'a std::path::Path,
    seen: Vec<Option<String>>,
}

impl Io for Watch<'_> {
    fn poll(&mut self, t: &Term) -> IoResult<Option<u8>> {
        self.inner.poll(t)
    }
    fn key_timeout(&mut self, t: &Term) -> IoResult<u8> {
        self.inner.key_timeout(t)
    }
    fn timeout_polls(&self) -> u32 {
        self.inner.timeout_polls()
    }
    fn pause(&mut self, t: &Term, n: u32) -> IoResult<()> {
        self.inner.pause(t, n)
    }
    fn wait_or_key(&mut self, t: &Term) -> IoResult<()> {
        self.inner.wait_or_key(t)
    }
    fn features(&self) -> u1_assets::qol::Qol {
        self.inner.features()
    }
    fn command_prompt(&mut self, on: bool) {
        if on {
            self.seen.push(std::fs::read_to_string(self.side).ok());
        }
    }
}

#[test]
fn sidecar_is_written_when_the_run_ends_and_on_entering_a_dungeon_never_the_record() {
    let dir = std::env::temp_dir().join(format!("u1-qol2-sidecar2-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::create_dir_all(&dir);
    let save = dir.join("P.U1");
    let side = u1_assets::explore::sidecar_path(&save);
    // Into Mt. Drash and a few steps, then the keys run out (as the
    // window closing: no Quit).
    let mut inner = ScriptIo::from_text(DUNGEON_WALK);
    inner.qol = second_group();
    let mut io = Watch { inner, side: &side, seen: vec![] };
    SAVE_TO.with(|s| *s.borrow_mut() = Some(save.clone()));
    let Some(g) = play_io(59, 29, &mut io, 2, &[], |_| {}) else {
        SAVE_TO.with(|s| s.borrow_mut().take());
        return;
    };
    assert_eq!(g.mode, u1_assets::dgame::DUNGEON);
    assert!(!save.exists(), "no Quit: the record is never written");
    assert_eq!(io.seen[0], None, "nothing written before the first move");
    let mid = io.seen.iter().flatten().find(|t| t.contains("\nlevel ")).expect("written on entering the dungeon, before the run ended");
    assert!(mid.contains("\nworld "));
    let end = std::fs::read_to_string(&side).expect("written when the run ended");
    let count = |t: &str| t.lines().filter_map(|l| l.strip_prefix("level ")).map(|l| l.rsplit(' ').next().unwrap().chars().filter(|c| !"?/".contains(*c)).count()).sum::<usize>();
    assert!(count(&end) > count(mid), "the end has the steps taken after: {} {}", count(mid), count(&end));
    assert!(!dir.join("P.U1.map.tmp").exists());
    // The maps off: nothing at all.
    let off = dir.join("Q.U1");
    SAVE_TO.with(|s| *s.borrow_mut() = Some(off.clone()));
    play(59, 29, DUNGEON_WALK).unwrap();
    assert!(!u1_assets::explore::sidecar_path(&off).exists() && !off.exists());
    let _ = std::fs::remove_dir_all(&dir);
}

fn weapon_shop(tail: &str) -> String {
    format!("{{left}}{{left}}e{}{}{{down}}tsb{tail}", rep("{up}", 6), rep("{left}", 4))
}

#[test]
fn dropping_asks_first_when_set_and_no_keeps_the_item() {
    // In Britain: Drop, Weapon, b (a dagger); then Drop, Pence, 5.
    let keys = |w: &str, p: &str| format!("{{left}}{{left}}edwb{w}dp5{{enter}}{p}");
    let Some(plain) = play(41, 39, &keys("", "")) else { return };
    let start = PlayerRecord::parse(&game_dir().unwrap().read("PLAYER2.U1").unwrap()).unwrap();
    let (daggers, coin) = (|g: &Game| g.rec.u16_at(0x62), |g: &Game| g.rec.u16_at(off::COIN));
    let run = |w: &str, p: &str| {
        let mut io = ScriptIo::from_text(&keys(w, p));
        io.qol.confirm = 1;
        play_io(41, 39, &mut io, 2, &[], |_| {}).unwrap()
    };
    let (no, yes) = (run("n", "n"), run("y", "y"));
    assert!(start.u16_at(0x62) > 0);
    assert_eq!(daggers(&plain), start.u16_at(0x62) - 1, "off: dropped at once, as the original");
    assert_eq!((daggers(&no), coin(&no)), (daggers(&plain) + 1, coin(&plain) + 5), "N keeps both");
    assert_eq!(yes.rec.as_bytes(), plain.rec.as_bytes(), "Y: the same as the original");
    // Above the value set: no question, the keys go on as in the original.
    let mut io = ScriptIo::from_text(&keys("", ""));
    io.qol.confirm = 30000;
    let high = play_io(41, 39, &mut io, 2, &[], |_| {}).unwrap();
    assert_eq!(high.rec.as_bytes(), plain.rec.as_bytes());
    // The box asks in words.
    let mut io = ScriptIo::from_text("{left}{left}edwb");
    io.qol.confirm = 1;
    let asking = play_io(41, 39, &mut io, 2, &[], |_| {}).unwrap();
    let all: String = (0..25).map(|r| asking.term.row_text(r)).collect::<Vec<_>>().join("\n");
    assert!(all.contains("Drop Dagger?") && all.contains("Sure? (Y/N)"), "{all}");
}

#[test]
fn selling_asks_first_when_set_and_no_keeps_the_item() {
    let Some(plain) = play(41, 39, &weapon_shop("")) else { return };
    let daggers = |g: &Game| g.rec.u16_at(0x62);
    let mut io = ScriptIo::from_text(&weapon_shop("n"));
    io.qol.confirm = 1;
    let no = play_io(41, 39, &mut io, 2, &[], |_| {}).unwrap();
    let mut io = ScriptIo::from_text(&weapon_shop("y"));
    io.qol.confirm = 1;
    let yes = play_io(41, 39, &mut io, 2, &[], |_| {}).unwrap();
    let start = PlayerRecord::parse(&game_dir().unwrap().read("PLAYER2.U1").unwrap()).unwrap();
    let before = start.u16_at(0x62);
    assert!(before > 0);
    assert_eq!(daggers(&plain), before - 1, "off: sold at once, as the original");
    assert_eq!(daggers(&yes), before - 1);
    assert_eq!(daggers(&no), before);
    assert_eq!(yes.rec.as_bytes(), plain.rec.as_bytes());
}

/// The area music follows the game: towns, dungeons, space and back, the
/// lair, and the death sting before the resurrection.
#[test]
fn area_music_follows_the_game() {
    use u1_assets::music::Track::*;
    let run = |x, y, keys: &str, adjust: &dyn Fn(&mut PlayerRecord)| {
        let mut io = ScriptIo::from_text(keys);
        play_io(x, y, &mut io, 1, &[], adjust)?;
        Some(io.music)
    };
    let Some(m) = run(66, 41, "e{up}", &|_| {}) else { return };
    assert_eq!(m, vec![Overworld, Town]);
    assert_eq!(run(32, 27, "e", &|_| {}).unwrap(), vec![Overworld, Castle]);
    assert_eq!(run(39, 60, "ek ", &|_| {}).unwrap(), vec![Overworld, Dungeon, Overworld]);
    let m = run(40, 40, "B{w12}{up}{w300} ", &on_shuttle).unwrap();
    assert_eq!(m, vec![Overworld, Space, Overworld]);
    let m = run(40, 40, "B{w12}    ", &|r| {
        on_shuttle(r);
        r.set_u16(off::SHUTTLE_0, 0);
        r.set_u16(off::SHUTTLE_1, 0);
    })
    .unwrap();
    assert!(m.windows(2).any(|w| w == [Space, Death]) || m.contains(&Death), "{m:?}");
    let m = run(40, 40, "B{w12}", &at_time_machine).unwrap();
    assert_eq!(&m[..2], &[Overworld, Lair]);
}

/// The king's "How much?" shows every digit typed (the blinking cursor
/// moves on past each one, as in the original), and pence buy hit points.
#[test]
fn king_pence_shows_all_digits_typed() {
    let keys = format!("e{}{{up}}{{up}}{{up}}tp123", rep("{right}", 25));
    let Some(g) = play_with(32, 27, &keys, 2, &[], |r| r.set_u16(off::COIN, 2000)) else { return };
    let all: String = (0..25).map(|r| g.term.row_text(r)).collect::<Vec<_>>().join("\n");
    assert!(all.contains("How much? 123"), "{all}");
}

// ------------------------------------------------------------ Ztats and the character screen

fn screen_text(g: &Game) -> String {
    (0..25).map(|r| g.term.row_text(r)).collect::<Vec<_>>().join("\n")
}

fn quests_given(r: &mut PlayerRecord, done: &[usize], active: &[usize]) {
    for &k in done {
        r.set_u16(off::QUESTS + 2 * k, 1);
    }
    for &k in active {
        r.set_u16(off::QUESTS + 2 * k, 0xFFFF);
    }
}

#[test]
fn ztats_with_the_character_option_off_is_the_original() {
    // Every other engine screen on, the character screen off: Z draws and
    // records exactly what it does with nothing on (the original Ztats).
    let Some(plain) = play(66, 41, "z") else { return };
    for keys in ["z", "z ", "z {up}"] {
        let plain = play(66, 41, keys).unwrap();
        let mut io = ScriptIo::from_text(keys);
        io.qol = u1_assets::qol::Qol { automap: true, minimap: true, help: true, hints: true, charsheet: false, ..Default::default() };
        let other = play_io(66, 41, &mut io, 2, &[], |_| {}).unwrap();
        assert_eq!(plain.term.px, other.term.px, "{keys}");
        assert_eq!(plain.rec.as_bytes(), other.rec.as_bytes(), "{keys}");
    }
    let all = screen_text(&plain);
    assert!(!all.contains("Ztats - character"), "{all}");
}

#[test]
fn ztats_with_the_character_option_on_shows_everything_and_the_quests() {
    let run = |keys: &str| {
        let mut io = ScriptIo::from_text(keys);
        io.qol = u1_assets::qol::Qol { charsheet: true, ..Default::default() };
        play_io(66, 41, &mut io, 2, &[], |r| quests_given(r, &[3], &[2]))
    };
    let Some(g) = run("z") else { return };
    let pages = g.character_pages();
    // Every page, joined.
    let all: String = (0..pages).map(|n| screen_text(&run(&format!("z{}", " ".repeat(n))).unwrap())).collect::<Vec<_>>().join("\n");
    for want in ["Ztats - character", "Hit points", "Coin", "Weapons:", "Armour:", "Spells:", "Transport:", "Gems:", "Quests:"] {
        assert!(all.contains(want), "{want}\n{all}");
    }
    // The stats Ztats lists are all here.
    for i in 1..=6 {
        let n = g.ptr(u1_assets::exe::m::STAT_NAMES, i);
        assert!(all.contains(n.trim()), "{n}\n{all}");
    }
    assert!(all.contains("The Lost King:") && all.contains("Slay a Gelatinous Cube") && all.contains("Not done yet."), "{all}");
    assert!(all.contains("King of Barataria:") && all.contains("Done! Return to the king."), "{all}");
    // The last key closes it; nothing in the record changed.
    let closed = run(&format!("z{}", " ".repeat(pages))).unwrap();
    let plain = play_with(66, 41, "", 2, &[], |r| quests_given(r, &[3], &[2])).unwrap();
    assert_eq!(plain.rec.as_bytes(), closed.rec.as_bytes());
    assert!(!screen_text(&closed).contains("Ztats - character"));
}

#[test]
fn every_kings_quest_reads_from_the_game_data() {
    let Some(g) = play(66, 41, "") else { return };
    let got: Vec<(String, String)> = (1..=8).map(|k| (g.quest_giver(k), g.quest_task(k))).collect();
    let want = [
        ("Lord British", "Find the Grave of the Lost Soul"),
        ("The Lost King", "Slay a Gelatinous Cube"),
        ("King of Barataria", "Find the Southern Sign Post"),
        ("King of Rondorin", "Slay a Carrion Creeper"),
        ("King of Olympus", "Find the Pillar of Ozymandias"),
        ("King of the Black Dragon", "Slay a Lich"),
        ("King of the White Dragon", "Find the Tower of Knowledge"),
        ("King of Shamino", "Slay a Balron"),
    ];
    for (k, (w, t)) in want.iter().enumerate() {
        assert_eq!((got[k].0.as_str(), got[k].1.as_str()), (*w, *t), "castle {}", k + 1);
    }
}

/// Playtest report: G on a dungeon chest picked nothing up. As in the
/// original (Get `5550` only acts in castles; elsewhere it prints "?"),
/// G does nothing in a dungeon; a chest is opened with U (Unlock, `7b63`),
/// standing on its square. A chest one square ahead is not "here".
#[test]
fn dungeon_chest_is_unlocked_not_got() {
    use u1_assets::dungeon::Item;
    let Some(mut g) = play(39, 60, "e") else { return };
    let (a, b) = {
        let d = g.dungeon.as_ref().unwrap();
        (d.a as usize, d.b as usize)
    };
    g.dungeon.as_mut().unwrap().level.items[a][b] = Item::Chest;
    let mut io = ScriptIo::from_text("");
    let _ = g.command(&mut io, b'G');
    assert!(said(&g, "Get?"), "{:?}", g.term.messages());
    assert_eq!(g.dungeon.as_ref().unwrap().level.items[a][b], Item::Chest, "Get leaves the chest");
    let _ = g.command(&mut io, b'U');
    let gone = g.dungeon.as_ref().unwrap().level.items[a][b] == Item::None;
    assert!(gone && said(&g, "Thou dost find") || said(&g, "trap") || said(&g, "Trap"), "{:?}", g.term.messages());
}
