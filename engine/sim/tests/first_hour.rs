//! The first hour, through the body: a new Survival mole that holds S (dig straight down) from its burrow
//! never dies of what it couldn't see under its feet. Before the hollow/hot floor rule, seed 9 dug into lava
//! about 37 m down within 12 s and burned (about 16 s at the stone dig tempo, so the run is 20 s). Drops that only hurt, and water, are left alone (scenarios
//! `dig_down_shallow`, `dig_down_water`); a thin floor over a deadly drop is `dig_hollow_floor`.
use molehill_sim::{world::{World, HOLLOW, HOT}, mat::S, survival::Mode};

fn survival(seed: u32) -> World { let mut w = World::new(256 * S, 256 * S, seed); w.set_mode(Mode::Survival); w.set_hunger(false); w.disasters.enabled = false; w }

/// Hold S (the game's dig-down: the cell two tiles under the feet) for `secs`; (alive and never died, the
/// floor rang hollow or hot).
fn dig_down(w: &mut World, secs: u32) -> (bool, bool) {
    for _ in 0..secs * 60 { let m = &w.moles[0]; w.input.tool = 1; w.input.mine = Some((m.x, m.y + 2 * S)); w.step(); if !w.moles[0].alive { break; } }
    (w.moles[0].alive && w.pl.deaths == 0, w.log.iter().any(|(_, l)| l == HOLLOW || l == HOT))
}

#[test]
fn digging_straight_down_never_kills() {
    let mut rang = vec![];
    // (35 and 56 dig into lava within the 20 s now that the regions' sideways edges wander; seed 9 did before that)
    for seed in (0..32).chain([35, 56]) { let mut w = survival(seed); let (ok, r) = dig_down(&mut w, 20); assert!(ok, "seed {seed}: died holding S from the burrow"); if r { rang.push(seed); } }
    assert!(rang.contains(&35) && rang.contains(&56), "seeds 35 and 56 dig into lava: the floor should have felt hot (rang on {rang:?})");
}
