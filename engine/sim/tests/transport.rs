//! Transport: rails and carts, rope ladders and lifts (and the rest as it lands) are reachable in Survival from the
//! kit, and free in Creative. Behaviour is covered by the cart_*, rail_*, powered_rail_*, rope_ladder_*, lift_* and tube_* scenarios.
use molehill_sim::craft::{drop_of, tool_tier, Item, Station, NITEM, RECIPES};
use molehill_sim::mat::{Mat, NMAT};
use molehill_sim::survival::{Mode, KIT};
use molehill_sim::world::World;

/// Everything Survival can make, by the same rules as `survival_ladder_is_reachable`: dig what the tools
/// allow, build the stations, craft what the inputs allow, until nothing new turns up.
fn reachable() -> [bool; NITEM] {
    let mut have = [false; NITEM]; for &(it, _) in KIT { have[it as usize] = true; }
    for it in [Item::Grub, Item::Root, Item::Fungus, Item::Spore, Item::Mushroom] { have[it as usize] = true; }
    for _ in 0..30 {
        let tier = tool_tier(&std::array::from_fn(|k| have[k] as u32));
        for k in 0..NMAT { let m = Mat::from_u8(k as u8); if m == Mat::Bedrock { continue; }
            if m.p().tier <= tier.max(1) { if let Some(it) = drop_of(m) { have[it as usize] = true; } } }
        let built = |s: Station, have: &[bool; NITEM]| s.cost().iter().all(|&(it, _)| have[it as usize]);
        let ready = |s: Station, have: &[bool; NITEM]| match s { Station::Forge => have[Item::Coal as usize] || have[Item::Charcoal as usize], Station::Workshop => have[Item::CopperWire as usize], _ => true };
        for r in RECIPES.iter() { if built(r.station, &have) && ready(r.station, &have) && r.inputs.iter().all(|&(it, _)| have[it as usize]) { have[r.out.0 as usize] = true; } }
    }
    have
}

#[test] fn transport_is_reachable_in_survival() {
    let have = reachable();
    for it in [Item::Rail, Item::Cart, Item::RopeLadder, Item::Lift, Item::Tube, Item::TubeHopper] { assert!(have[it as usize], "{it:?} can't be made in Survival"); }
    // rails and carts are tier 2 (copper): they need no iron, so they come before the drill
    for name in ["Rail x8", "Mine cart"] { let r = RECIPES.iter().find(|r| r.name == name).unwrap();
        assert!(r.inputs.iter().all(|&(it, _)| !matches!(it, Item::IronIngot | Item::IronOre)), "{name} needs iron"); }
    // a rope ladder is tier 1 (wood and fungus at the workbench); a lift is tier 3 (iron at the workshop)
    let at = |n: &str| RECIPES.iter().find(|r| r.name == n).unwrap();
    assert_eq!(at("Rope ladder").station, Station::Workbench);
    assert!(at("Lift").inputs.iter().any(|&(it, _)| it == Item::IronIngot) && at("Lift").station == Station::Workshop);
}

#[test] fn creative_has_transport_freely() {
    let mut w = World::new(256 * molehill_sim::mat::S, 128 * molehill_sim::mat::S, 3); w.set_mode(Mode::Creative);
    for it in [Item::Rail, Item::Cart, Item::RopeLadder, Item::Lift, Item::Tube, Item::TubeHopper] { assert!(w.inv[it as usize] >= 999, "{it:?}"); }
}
