//! The cave editor never ships: players who download the game (native or the web build in site/) don't get
//! it. These read the build files so a change that would pull it in fails here.
use std::path::Path;
fn read(p: &str) -> String { std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(p)).unwrap() }

#[test]
fn the_editor_is_not_a_default_member() {
    let ws = read("../../Cargo.toml");
    let line = ws.lines().find(|l| l.trim_start().starts_with("default-members")).expect("default-members keeps plain cargo builds to the game");
    assert!(!line.contains("cave-editor"), "{line}");
}
#[test]
fn the_game_does_not_depend_on_the_editor() {
    assert!(!read("../../game/Cargo.toml").contains("cave-editor"));
    assert!(!read("../../sim/Cargo.toml").contains("cave-editor"));
}
#[test]
fn the_web_build_builds_only_the_game() {
    let sh = read("../../build-web.sh");
    let builds: Vec<&str> = sh.lines().filter(|l| l.contains("cargo build")).collect();
    assert!(!builds.is_empty() && builds.iter().all(|l| l.contains("-p molehill ") && !l.contains("cave")), "{builds:?}");
    let site = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../site");
    if let Ok(dir) = std::fs::read_dir(site) {
        for e in dir.flatten() { assert!(!e.file_name().to_string_lossy().contains("cave"), "site/ holds {:?}", e.file_name()); }
    }
}
