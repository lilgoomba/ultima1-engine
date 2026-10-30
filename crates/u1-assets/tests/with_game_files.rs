//! End-to-end checks of towns, castles, shops and saving that need the
//! user's own Ultima I files. Set `U1_GAME_DIR` to the directory holding
//! OUT.EXE, TCD.BIN, MAP.BIN, the tile files and PLAYER2.U1; without it
//! every test here is skipped. Expected values were observed side by side
//! in DOSBox (see SPEC.md); no game data is stored in this repository.

use u1_assets::exe::Exe;
use u1_assets::game::{Assets, Game};
use u1_assets::gamedir::GameDir;
use u1_assets::io::{keys, Halt, Io, IoResult, ScriptIo};
use u1_assets::term::Term;
use u1_assets::locations::LocationTable;
use u1_assets::rules::ExeTables;
use u1_assets::save::{off, MapObject, PlayerRecord};
use u1_assets::space::SpaceFiles;
use u1_assets::tcd::TownCastleMaps;

fn game_dir() -> Option<GameDir> {
    let d = std::env::var_os("U1_GAME_DIR")?;
    GameDir::new(d).ok()
}

/// PLAYER2.U1 moved to (x, y), run with seed 2 and the typed keys.
fn play(x: u16, y: u16, keys: &str) -> Option<Game> {
    play_with(x, y, keys, 2, &[], |_| {})
}

/// As `play`, with a chosen seed, recorded clock values for the time seeds
/// and a hook to adjust the save before starting.
fn play_with(
    x: u16,
    y: u16,
    keys: &str,
    seed: u16,
    times: &[u16],
    adjust: impl FnOnce(&mut PlayerRecord),
) -> Option<Game> {
    play_io(x, y, &mut ScriptIo::from_text(keys), seed, times, adjust)
}

/// As `play_with`, driven by any `Io`.
fn play_io(
    x: u16,
    y: u16,
    io: &mut dyn Io,
    seed: u16,
    times: &[u16],
    adjust: impl FnOnce(&mut PlayerRecord),
) -> Option<Game> {
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
    let _ = game.run(io);
    Some(game)
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
        LairBot {
            queue: ScriptIo::from_text(prefix).keys,
            left: polls,
            seen: Vec::new(),
            frames: std::env::var_os("U1_LAIR_FRAMES").map(|_| Default::default()),
        }
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
    let s = format!(
        "a{{right}}{{right}}{{down}}{{right}}{{up}}{}{{down}}{} baaHero{{enter}}yb1",
        "{right}".repeat(13),
        "{right}".repeat(14)
    );
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
    let Some(DPiece::Obj(o)) = sc.pieces.iter().map(|p| &p.1).find(|p| matches!(p, DPiece::Obj(o) if o.id == ArtId::DMon(1))) else {
        panic!("no skeleton")
    };
    let lit: Vec<(usize, usize)> = (o.y0..=o.y1)
        .flat_map(|y| (o.x0..=o.x1).map(move |x| (x as usize, y as usize)))
        .filter(|&(x, y)| t.px[y * 320 + x] != 0)
        .collect();
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
    let Some(DPiece::Obj(c)) = sc.pieces.iter().map(|p| &p.1).find(|p| matches!(p, DPiece::Obj(o) if o.id == ArtId::Dun(u1_assets::term::dun::CHEST))) else {
        panic!("no chest")
    };
    assert!(c.x1 - c.x0 > c.y1 - c.y0, "{c:?}");
    let ch = colour_pack(&[("dungeon.wall", red), ("dungeon.chest", [0, 230, 0, 255])]);
    let out3 = compose(&t, pal, &ch, 0);
    let y = (c.y1 as usize) * 2;
    let green: Vec<usize> = (0..640).filter(|&x| {
        let p = &out3[(y * 640 + x) * 3..][..3];
        p[1] > 60 && p[0] == 0 && p[2] == 0
    }).collect();
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
        let Some(DPiece::Obj(h)) = sc.pieces.iter().map(|p| &p.1).find(|p| matches!(p, DPiece::Obj(o) if o.id == ArtId::Dun(id))) else {
            panic!("no hole")
        };
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
