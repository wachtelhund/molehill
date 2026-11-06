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

/// Hover tips for transport: resting the cursor on each machine says what it is and the action that
/// matters (get in / push, climb, ballast, send / receive), and track and pipe say what makes them go.
#[test] fn transport_hover_tips_say_what_to_do() {
    use molehill_sim::{grid::F_SEEN, mat::{CPM, S}, tips::describe};
    let mut w = World::new(256 * S, 128 * S, 3); w.disasters.enabled = false; w.set_mode(Mode::Creative);
    let (mx, my) = (w.moles[0].x, w.moles[0].y); let c = CPM;
    // a room 10 m wide and 5 m tall round the mole, rock all round; seen
    for y in my - 5 * c..=my + 2 * c { for x in mx - 6 * c..=mx + 6 * c { let i = w.g.idx(x, y);
        let rock = y > my || y <= my - 5 * c || x < mx - 5 * c || x > mx + 5 * c;
        w.g.set(i, if rock { Mat::Granite } else { Mat::Empty }); w.g.flags[i] = F_SEEN; } }
    for x in mx + c..mx + 3 * c { let i = w.g.idx(x, my + 1); w.g.set(i, Mat::Rail); }       // track in the floor's top row
    for x in mx + 4 * c..mx + 5 * c { let i = w.g.idx(x, my + 1); w.g.set(i, Mat::Tube); }   // and pipe
    w.drops.clear(); for _ in 0..3 { w.step(); }
    let tip = |w: &World, x: usize, y: usize| describe(w, x, y).map(|t| format!("{}: {}", t.name, t.lines.join(" / "))).unwrap_or_default();
    assert!(tip(&w, mx + 2 * c, my + 1).contains("Set a mine cart on it"), "rail: {}", tip(&w, mx + 2 * c, my + 1));
    assert!(tip(&w, mx + 4 * c + 2, my + 1).contains("tube hoppers"), "tube: {}", tip(&w, mx + 4 * c + 2, my + 1));
    assert_eq!(w.place_transport(Item::Cart, mx + 2 * c, my), Ok(true));
    let t = tip(&w, mx + 2 * c, my - 1); assert!(t.starts_with("Mine cart") && t.contains("Right mouse: get in") && t.contains("push") && t.contains("throw things in"), "cart: {t}");
    w.tr.riding = Some(0); let t = tip(&w, mx + 2 * c, my - 1); assert!(t.contains("Space: jump out"), "cart, riding: {t}"); w.tr.riding = None;
    assert_eq!(w.place_transport(Item::RopeLadder, mx - c - c / 2, my - 2 * c), Ok(true));
    let t = tip(&w, mx - c - c / 2, my - 2 * c); assert!(t.starts_with("Rope ladder") && t.contains("W / S: climb"), "ladder: {t}");
    assert_eq!(w.place_transport(Item::Lift, mx - 3 * c - c / 2, my - c), Ok(true));
    let t = tip(&w, mx - 3 * c - c / 2, my - S); assert!(t.starts_with("Lift") && t.contains("Counterweight 40 kg") && t.contains("sinks") && t.contains("stone"), "lift: {t}");
    assert_eq!(w.place_transport(Item::TubeHopper, mx + 4 * c + 2, my), Ok(true));
    let t = tip(&w, mx + 4 * c + 2, my - 2); assert!(t.starts_with("Tube hopper") && t.contains("No current") && t.contains("only receives"), "hopper: {t}");
}

/// The production line (production.rs) can be built in Survival: chutes and bumpers from the workbench (tier 1
/// wood and stone), the ore hopper from the forge (copper), the shaker screen once there's iron.
#[test] fn production_is_reachable_in_survival() {
    let have = reachable();
    for it in [Item::Chute, Item::Bumper, Item::OreHopper, Item::Screen, Item::Riffle, Item::Shaft, Item::Gear, Item::WaterWheel, Item::Belt, Item::Counterweight, Item::Boiler, Item::Engine, Item::SteamPipe, Item::StampMill, Item::Washer, Item::Smelter, Item::Bellows, Item::Mould, Item::Rod, Item::TreadPlate, Item::Float, Item::SluiceGate, Item::Scale, Item::Clutch] { assert!(have[it as usize], "{it:?} can't be made in Survival"); }
    let at = |n: &str| RECIPES.iter().find(|r| r.name == n).unwrap();
    for n in ["Chute x8", "Bumper x2", "Riffle bar x8"] { assert_eq!(at(n).station, Station::Workbench, "{n}"); }
    assert!(at("Ore hopper").inputs.iter().all(|&(it, _)| !matches!(it, Item::IronIngot | Item::IronOre)), "the ore hopper needs no iron");
}

/// Hover tips for the production line: an ore hopper set over track says what it holds and how its gate
/// opens; chute, screen and bumper cells say what moves things along them.
#[test] fn production_hover_tips_say_what_to_do() {
    use molehill_sim::{grid::F_SEEN, mat::CPM, tips::describe};
    let mut w = World::new(256 * molehill_sim::mat::S, 128 * molehill_sim::mat::S, 3); w.disasters.enabled = false; w.set_mode(Mode::Creative);
    let (mx, my) = (w.moles[0].x, w.moles[0].y); let c = CPM;
    for y in my - 5 * c..=my + 2 * c { for x in mx - 6 * c..=mx + 6 * c { let i = w.g.idx(x, y);
        let rock = y > my || y <= my - 5 * c || x < mx - 5 * c || x > mx + 5 * c;
        w.g.set(i, if rock { Mat::Granite } else { Mat::Empty }); w.g.flags[i] = F_SEEN; } }
    for x in mx + c..mx + 4 * c { let i = w.g.idx(x, my + 1); w.g.set(i, Mat::Rail); }
    for (x, m) in [(mx - 4 * c, Mat::Chute), (mx - 3 * c, Mat::Screen), (mx - 2 * c, Mat::Bumper)] { let i = w.g.idx(x, my + 1); w.g.set(i, m); }
    w.drops.clear(); for _ in 0..3 { w.step(); }
    let tip = |w: &World, x: usize, y: usize| describe(w, x, y).map(|t| format!("{}: {}", t.name, t.lines.join(" / "))).unwrap_or_default();
    assert_eq!(w.place_transport(Item::OreHopper, mx + 2 * c + c / 2, my), Ok(true));
    let t = tip(&w, mx + 2 * c + c / 2, my - c); assert!(t.starts_with("Ore hopper") && t.contains("Holding 0 of 60") && t.contains("cart under its spout"), "hopper: {t}");
    assert!(tip(&w, mx - 4 * c, my + 1).contains("slide down it"), "chute: {}", tip(&w, mx - 4 * c, my + 1));
    assert!(tip(&w, mx - 3 * c, my + 1).contains("shakes it"), "screen: {}", tip(&w, mx - 3 * c, my + 1));
    assert!(tip(&w, mx - 2 * c, my + 1).contains("tips its load"), "bumper: {}", tip(&w, mx - 2 * c, my + 1));
}
