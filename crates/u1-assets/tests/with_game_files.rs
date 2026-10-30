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
use u1_assets::save::{off, PlayerRecord};
use u1_assets::tcd::TownCastleMaps;

fn game_dir() -> Option<GameDir> {
    let d = std::env::var_os("U1_GAME_DIR")?;
    GameDir::new(d).ok()
}

/// PLAYER2.U1 moved to (x, y), run with seed 2 and the typed keys.
fn play(x: u16, y: u16, keys: &str) -> Option<Game> {
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
    let mut game = Game::new(assets, rec, 2);
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
