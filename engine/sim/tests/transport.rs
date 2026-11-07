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
    let t = tip(&w, mx - c - c / 2, my - 2 * c); assert!(t.starts_with("Rope ladder") && t.contains("W / S") && t.contains("E: take it down"), "ladder: {t}");
    assert_eq!(w.place_transport(Item::Lift, mx - 3 * c - c / 2, my - c), Ok(true));
    let t = tip(&w, mx - 3 * c - c / 2, my - S); assert!(t.starts_with("Lift") && t.contains("W / S") && t.contains("calls the cage") && t.contains("Right mouse: balance"), "lift: {t}");
    assert_eq!(w.place_transport(Item::TubeHopper, mx + 4 * c + 2, my), Ok(true));
    let t = tip(&w, mx + 4 * c + 2, my - 2); assert!(t.starts_with("Tube hopper") && t.contains("No current") && t.contains("only receives"), "hopper: {t}");
}

/// The production line (production.rs) can be built in Survival: chutes and bumpers from the workbench (tier 1
/// wood and stone), the ore hopper from the forge (copper), the shaker screen once there's iron.
#[test] fn production_is_reachable_in_survival() {
    let have = reachable();
    for it in [Item::Chute, Item::Bumper, Item::OreHopper, Item::Screen, Item::Riffle, Item::Shaft, Item::Gear, Item::WaterWheel, Item::Belt, Item::Counterweight, Item::Boiler, Item::Engine, Item::SteamPipe, Item::StampMill, Item::Washer, Item::Smelter, Item::Bellows, Item::Mould, Item::Rod, Item::TreadPlate, Item::Float, Item::SluiceGate, Item::Scale, Item::Clutch, Item::SteamBorer] { assert!(have[it as usize], "{it:?} can't be made in Survival"); }
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

/// A tube hopper and an ore hopper are two machines with one word in their names: neither can be set down
/// over the other (right mouse and `E` would reach only the first), and the refusal names which one is there.
#[test] fn hoppers_do_not_stack() {
    use molehill_sim::{grid::F_SEEN, mat::CPM};
    let mut w = World::new(256 * molehill_sim::mat::S, 128 * molehill_sim::mat::S, 3); w.disasters.enabled = false; w.set_mode(Mode::Creative);
    let (mx, my) = (w.moles[0].x, w.moles[0].y); let c = CPM;
    for y in my - 5 * c..=my + 2 * c { for x in mx - 6 * c..=mx + 6 * c { let i = w.g.idx(x, y);
        let rock = y > my || y <= my - 5 * c || x < mx - 5 * c || x > mx + 5 * c;
        w.g.set(i, if rock { Mat::Granite } else { Mat::Empty }); w.g.flags[i] = F_SEEN; } }
    for x in mx + c..mx + 4 * c { let i = w.g.idx(x, my + 1); w.g.set(i, Mat::Tube); }
    w.drops.clear(); for _ in 0..3 { w.step(); }
    let at = mx + 2 * c + c / 2;
    assert_eq!(w.place_transport(Item::TubeHopper, at, my), Ok(true));
    assert_eq!(w.place_transport(Item::OreHopper, at, my), Err("there's a tube hopper there"));
    assert_eq!(w.tr.bins.len(), 0);
    w.tr.hoppers.clear();
    assert_eq!(w.place_transport(Item::OreHopper, at, my), Ok(true));
    assert_eq!(w.place_transport(Item::TubeHopper, at, my), Err("there's an ore hopper there"));
    assert_eq!(w.tr.hoppers.len(), 0);
}

#[test] fn tube_hopper_messages_say_tube_hopper() {
    // the two hoppers are told apart in what they say: an ore hopper's messages say "ore hopper", a tube hopper's
    // tip and log lines "tube hopper", never a bare "hopper"
    use molehill_sim::{grid::F_SEEN, mat::CPM, tips::describe};
    let mut w = World::new(256 * molehill_sim::mat::S, 128 * molehill_sim::mat::S, 3); w.disasters.enabled = false; w.set_mode(Mode::Creative);
    let (mx, my) = (w.moles[0].x, w.moles[0].y); let c = CPM;
    for y in my - 5 * c..=my + 2 * c { for x in mx - 6 * c..=mx + 6 * c { let i = w.g.idx(x, y);
        let rock = y > my || y <= my - 5 * c || x < mx - 5 * c || x > mx + 5 * c;
        w.g.set(i, if rock { Mat::Granite } else { Mat::Empty }); w.g.flags[i] = F_SEEN; } }
    for x in mx + c..mx + 4 * c { let i = w.g.idx(x, my + 1); w.g.set(i, Mat::Tube); }
    w.drops.clear(); for _ in 0..3 { w.step(); }
    let hx = mx + 2 * c + c / 2;
    assert_eq!(w.place_transport(Item::TubeHopper, hx, my), Ok(true));
    let t = describe(&w, hx, my - c / 2).map(|t| format!("{}: {}", t.name, t.lines.join(" / "))).unwrap_or_default();
    assert!(t.starts_with("Tube hopper") && t.contains("No other tube hopper"), "tip: {t}");
    w.gear.held = None; assert!(w.use_transport(hx, my - c / 2));
    let said = &w.log.last().unwrap().1; assert!(said.contains("tube hopper"), "said: {said}");
}

/// The borer runs on coal alone and the water wagon is gone (save v13). A save from before (`data/v12_water_wagon.bin`,
/// written by the last build with the water wagon) still loads: water wagons in the pack, in a store wagon or a cart,
/// on the floor in the window or in a tile off it come back as store wagons; a placed water wagon is a store wagon
/// (its water gone); the coal wagon (numbered after the water wagon then) is still a coal wagon; the borer keeps its
/// fuel and steam and drops its tank; the queued "Water wagon" is dropped from the craft queue and the rest kept.
#[test] fn old_water_wagon_save_loads() {
    use molehill_sim::{band::BH, craft::RECIPES, save, wagon::WagonKind};
    let mut w = save::load(include_bytes!("data/v12_water_wagon.bin")).expect("a v12 save with water wagons loads");
    assert_eq!((w.inv[Item::StoreWagon as usize], w.inv[Item::CoalWagon as usize], w.inv[Item::Drill as usize]), (2, 3, 1), "pack");
    assert!(w.known[Item::StoreWagon as usize] && w.known[Item::CoalWagon as usize]);
    let kinds: Vec<(WagonKind, f32)> = w.tr.wagons.iter().map(|g| (g.kind, g.x)).collect();
    assert_eq!(kinds, vec![(WagonKind::Store, 100.), (WagonKind::Coal, 120.), (WagonKind::Store, 140.)], "placed wagons");
    assert_eq!(w.tr.wagons[1].cargo, vec![(Item::Coal, 5)]);
    assert_eq!(w.tr.wagons[2].cargo, vec![(Item::StoreWagon, 1), (Item::CoalWagon, 1), (Item::Stone, 4)], "a store wagon's load");
    assert_eq!(w.tr.carts[0].cargo, vec![(Item::StoreWagon, 1), (Item::CoalWagon, 2)], "a cart's load");
    let b = &w.tr.borers[0]; assert_eq!((b.fuel.clone(), b.burning, b.burn, b.steam), (vec![(Item::Coal, 2)], Some(Item::Coal), 100, 50.), "the borer");
    let names: Vec<&str> = w.craft_queue.iter().map(|&k| RECIPES[k].name).collect(); assert_eq!(names, ["Coal wagon", "Drill"], "craft queue");
    let at = |w: &World, x: usize, it: Item| w.drops.iter().any(|d| d.x == x && d.item == it);
    assert!(at(&w, 300, Item::StoreWagon) && at(&w, 310, Item::CoalWagon), "drops in the window");
    w.shift(-(BH as i64)); assert!(at(&w, 320, Item::StoreWagon), "a drop in a tile that was off the window");
    let l = save::load(&save::save(&w)).expect("saves again as v13");
    assert_eq!(l.tr.wagons.iter().map(|g| g.kind).collect::<Vec<_>>(), [WagonKind::Store, WagonKind::Coal, WagonKind::Store]);
    assert_eq!((l.inv[Item::StoreWagon as usize], l.inv[Item::CoalWagon as usize]), (2, 3));
}

/// The borer is a worm now (transport blob 14: its heading, path and train). A save from before
/// (`data/v13_borer_train.bin`, written by the last build with the cab on wheels: a borer heading east with a coal and a
/// store wagon coupled behind it on its wheel row, and a bunk wagon standing loose far off) still loads: the borer heads
/// east with its head at the old cab's front, the two coupled wagons are its segments in order with their loads, set on
/// its path behind it, the loose one stays loose where it was; it keeps its fuel and steam, and the train still follows
/// when it bores on, and saves again.
#[test] fn old_borer_train_loads_as_a_worm() {
    use molehill_sim::{save, wagon::WagonKind, borer::{seg_dist, CAB_D, BORER_H}};
    let mut w = save::load(include_bytes!("data/v13_borer_train.bin")).expect("a v13 save with a borer train loads");
    let fy = 60. * molehill_sim::mat::S as f32; let bx = 80. * molehill_sim::mat::S as f32;
    let b = &w.tr.borers[0];
    assert!((b.x - (bx + CAB_D)).abs() < 0.01 && (b.y - (fy - BORER_H / 2.)).abs() < 0.01 && b.ang.cos() > 0.99, "head at ({}, {}) heading {}", b.x, b.y, b.ang);
    assert_eq!((b.fuel.clone(), b.burning, b.steam), (vec![(Item::Coal, 3)], Some(Item::Coal), molehill_sim::borer::STEAM_MAX / 2.));
    let train = w.train(0); assert_eq!(train.len(), 2, "the coupled wagons are its segments");
    assert_eq!(train.iter().map(|&j| w.tr.wagons[j].kind).collect::<Vec<_>>(), [WagonKind::Coal, WagonKind::Store]);
    assert_eq!(w.tr.wagons[train[0]].cargo, vec![(Item::Coal, 7)]);
    for (n, &j) in train.iter().enumerate() { let g = &w.tr.wagons[j]; assert!((g.x - (b.x - seg_dist(n))).abs() < 0.01, "wagon {n} at {} on the path", g.x); }
    let loose = w.tr.wagons.iter().find(|g| g.kind == WagonKind::Bunk).unwrap(); assert!(!loose.coupled && (loose.x - 20. * molehill_sim::mat::S as f32).abs() < 0.01);
    // it bores on east, the train after it
    w.tr.borer_ride = Some(0); let x0 = w.tr.borers[0].x; let wx0 = w.tr.wagons[train[1]].x;
    for _ in 0..120 { w.input.right = true; w.step(); }
    assert!(w.tr.borers[0].x > x0 + 4. && w.tr.wagons[train[1]].x > wx0 + 4., "the train moved on: head {} wagon {}", w.tr.borers[0].x, w.tr.wagons[train[1]].x);
    let l = save::load(&save::save(&w)).expect("saves again");
    assert_eq!(l.train(0).len(), 2); assert!((l.tr.borers[0].x - w.tr.borers[0].x).abs() < 0.01 && l.tr.borers[0].path.len() == w.tr.borers[0].path.len());
}

/// The steam borer is the deep's machine: in Survival its recipe (iron, bronze, bricks, and crystal and quartz for its
/// cutters, at the workshop) stays off the list, even with every input known, until the mole has been down to
/// `BORER_DEEP_M`; reaching it says so once. Creative lists it at once.
#[test] fn steam_borer_opens_in_the_deep() {
    use molehill_sim::{craft::{recipe_by_name, BORER_DEEP_M, RECIPES}, survival::Mode, world::World};
    let r = recipe_by_name("Steam borer").unwrap();
    assert!(RECIPES[r].inputs.iter().any(|&(it, n)| it == Item::Crystal && n >= 6) && RECIPES[r].inputs.iter().any(|&(it, n)| it == Item::IronIngot && n >= 20), "a costly recipe");
    let mut w = World::new(256 * molehill_sim::mat::S, 128 * molehill_sim::mat::S, 3); w.set_mode(Mode::Survival);
    for &(it, _) in RECIPES[r].inputs { w.known[it as usize] = true; }
    w.max_depth = BORER_DEEP_M - 1.; assert!(!w.recipe_known(r), "not before the deep");
    w.max_depth = BORER_DEEP_M; assert!(w.recipe_known(r), "open in the deep");
    let mut c = World::new(256 * molehill_sim::mat::S, 128 * molehill_sim::mat::S, 3); c.set_mode(Mode::Creative); assert!(c.recipe_known(r));
}

/// A borer out of steam goes only as fast as its firebox raises it: boring granite flat out takes more than the fire's
/// 4.2 kW, so a lit borer with an empty steam store bores slower than one with steam in hand. It slows smoothly (it
/// moves every tick), it isn't stuck, and it never bores for free (its steam never goes below what it had).
#[test] fn starved_borer_bores_at_what_its_fire_raises() {
    use molehill_sim::{borer::{Borer, STEAM_MAX}, mat::CPM};
    let run = |steam: f32| -> (f32, u32) {
        let mut w = World::new(256 * molehill_sim::mat::S, 128 * molehill_sim::mat::S, 3);
        for i in 0..w.g.mat.len() { w.g.set(i, Mat::Granite); }
        let (x0, y0) = (10 * CPM, 16 * CPM);
        for y in y0 - 2 * CPM..y0 { for x in x0 - 4 * CPM..x0 + CPM { let i = w.g.idx(x, y); w.g.set(i, Mat::Empty); } }
        let mut b = Borer::new(x0 as f32 - 6., y0 as f32 - 6., 0.); b.fuel = vec![(Item::Coal, 5)]; b.burning = Some(Item::Coal); b.burn = 4000; b.steam = steam;
        w.tr.borers.push(b); w.tr.borer_ride = Some(0);
        let (sx, mut moved) = (w.tr.borers[0].x, 0);
        for t in 0..600 { w.input.right = true; let x = w.tr.borers[0].x; w.step(); if t > 60 && w.tr.borers[0].x > x { moved += 1; } assert!(w.tr.borers[0].steam >= 0.); }
        (w.tr.borers[0].x - sx, moved)
    };
    let (full, _) = run(STEAM_MAX); let (starved, moving) = run(0.);
    assert!(starved < 0.92 * full, "starved {starved} vs with steam {full}: it should be held to what its fire raises");
    assert!(starved > 0.6 * full, "starved {starved} vs {full}: it should still bore on");
    assert!(moving >= 530, "it bores on smoothly, every tick ({moving} of 539)");
}
