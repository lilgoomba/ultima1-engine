//! End-to-end checks of towns, castles, shops and saving that need the
//! user's own Ultima I files. Set `U1_GAME_DIR` to the directory holding
//! OUT.EXE, TCD.BIN, MAP.BIN, the tile files and PLAYER2.U1; without it
//! every test here is skipped. Expected values were observed side by side
//! in DOSBox (see SPEC.md); no game data is stored in this repository.

use u1_assets::exe::Exe;
use u1_assets::game::{Assets, Game};
use u1_assets::gamedir::GameDir;
use u1_assets::io::ScriptIo;
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
    game.space = SpaceFiles::load(&g).ok();
    game.mondain = u1_assets::mondain::MondainFiles::load(&g).ok();
    game.fixed_times = times.iter().copied().collect();
    let _ = game.run(&mut ScriptIo::from_text(keys));
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
    let spaces = " ".repeat(4);
    let g = play_with(40, 40, &format!("{TO_GEM}G{{w1}}{spaces}"), 1, &[], |r| {
        at_time_machine(r);
        r.set_u16(off::HITS, 20);
    })
    .unwrap();
    assert!(said(&g, "thou art dead"));
    assert!(said(&g, "Attempting resurrection!"));
}

#[test]
fn defeating_mondain_shows_the_ending() {
    let swings = "A{left}A{right}A{up}A{down}".repeat(30);
    let Some(g) = play_with(40, 40, &format!("{TO_GEM}G{swings}{{w5}}"), 1, &[], |r| {
        at_time_machine(r);
        // 9999 hits: three quarters of that overflows the original's
        // 16-bit sum, so taking the gem *adds* hits, as in MONDAIN.EXE.
        r.set_u16(off::HITS, 9999);
        r.set_u16(off::STATS, 99);
        r.set_u16(off::STATS + 2, 99);
        r.set_u16(off::STATS + 4, 99);
        r.set_u16(off::READY_WEAPON, 15);
    }) else {
        return;
    };
    assert!(g.term.row_text(24).contains("Press CONTROL-ALT-DEL to Restart"));
    assert!(g.stopped.unwrap().contains("Mondain"));
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
