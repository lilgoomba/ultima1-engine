//! Print an ASCII overview of the overworld (every 2nd column/row).
//! `cargo run -p u1-assets --example ascii_map -- [MAP.BIN | --demo [seed]]`
use u1_assets::map::{Terrain, WorldMap};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let data = match args.first().map(String::as_str) {
        Some("--demo") | None => {
            let seed = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
            u1_assets::demo::demo_map_bin(seed)
        }
        Some(path) => std::fs::read(path).expect("read map"),
    };
    let m = WorldMap::parse_lenient(&data).expect("parse map");
    for y in (0..m.height).step_by(2) {
        let line: String = (0..m.width)
            .step_by(2)
            .map(|x| match m.terrain(x, y) {
                Terrain::Water => '~',
                Terrain::Grass => '.',
                Terrain::Forest => 'f',
                Terrain::Mountain => '^',
                Terrain::Castle => 'C',
                Terrain::Signpost => 's',
                Terrain::Town => 'T',
                Terrain::DungeonEntrance => 'D',
                Terrain::Unknown(_) => '?',
            })
            .collect();
        println!("{line}");
    }
}
