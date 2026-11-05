//! The first ~25 m of a new Survival holt: what a first-time player is told and what the ground offers.
use molehill_sim::{world::World, mat::{Mat, CPM, S}, craft::{drop_of, Item, Station, RECIPES, CELLS_PER_ITEM}, survival::{Mode, KIT}, tips::describe, gen::START_SAFE};

/// A survival world with one material set at a seen cell beside the mole; returns the hover tip's lines.
fn tip_on(w: &mut World, m: Mat) -> (String, String) {
    let (x, y) = (w.moles[0].x + 3 * S, w.moles[0].y);
    let i = w.g.idx(x, y); w.g.set(i, m); w.g.flags[i] |= molehill_sim::grid::F_SEEN;
    let t = describe(w, x, y).expect("a seen solid has a tip"); (t.name, t.lines.join("\n"))
}

#[test]
fn hover_tips_speak_to_the_tools_you_own() {
    let mut w = World::new(128 * S, 96 * S, 14085); w.set_mode(Mode::Survival);
    let (_, l) = tip_on(&mut w, Mat::Granite);
    assert!(l.contains("too hard for your pickaxe: it needs a drill"), "{l}");
    assert!(!l.contains("Needs a drill."), "the table's general note doubles the line: {l}");
    let (_, l) = tip_on(&mut w, Mat::CopperOre); assert!(l.contains("Digging it gives copper ore."), "{l}");
    let (_, l) = tip_on(&mut w, Mat::Roots); assert!(l.contains("Digging it gives wood."), "{l}");
    let (_, l) = tip_on(&mut w, Mat::Rock); assert!(!l.to_lowercase().contains("pickaxe"), "no tool talk on ground the kit digs: {l}");
    // with a drill the granite tip stops talking about drills and says what it gives
    w.inv[Item::Drill as usize] = 1;
    let (_, l) = tip_on(&mut w, Mat::Granite);
    assert!(!l.to_lowercase().contains("drill") && l.contains("Digging it gives stone."), "{l}");
    let (_, l) = tip_on(&mut w, Mat::Basalt); assert!(l.contains("too hard for your drill: it needs a bore"), "{l}");
    // bedrock keeps its one line, said once
    let (_, l) = tip_on(&mut w, Mat::Bedrock); assert_eq!(l.matches("Nothing digs it").count(), 1, "{l}");
}

#[test]
fn stations_say_what_they_need_and_why_they_idle() {
    let mut w = World::new(128 * S, 96 * S, 14085); w.set_mode(Mode::Survival);
    let (x, y) = (w.moles[0].x + 4 * S, w.moles[0].y);
    w.buildings.push(molehill_sim::world::Building { kind: Station::Forge, x, y, built: false, claimed: false });
    for yy in y + 1 - Station::H..=y { for xx in x..x + Station::W { let i = w.g.idx(xx, yy); w.g.flags[i] |= molehill_sim::grid::F_SEEN; } }
    let l = describe(&w, x + 1, y).unwrap().lines.join("\n");
    assert!(l.contains("10 stone (have 0)") && l.contains("4 clay (have 0)"), "{l}");
    w.buildings[0].built = true; w.inv[Item::Coal as usize] = 0; w.inv[Item::Charcoal as usize] = 0;
    let l = describe(&w, x + 1, y).unwrap().lines.join("\n");
    assert!(l.contains("The forge is cold") && !l.contains("open the pack"), "a cold forge says why: {l}");
    w.inv[Item::Coal as usize] = 1;
    let l = describe(&w, x + 1, y).unwrap().lines.join("\n");
    assert!(l.contains("open the pack (I)"), "{l}");
}

/// The tool ladder's first rungs need nothing from further than the start zone: over many seeds, the ground
/// within 25 m of the burrow holds the stone and clay for a workbench and forge, coal to fire it, and copper
/// and tin ore for the first ingots, the bronze and the drill. Wood is scarce near most burrows, so the kit
/// carries what the workbench, the workshop and the first wood pillars take.
#[test]
fn early_tool_ladder_is_within_the_start_zone() {
    let cost = |s: Station, it: Item| s.cost().iter().find(|c| c.0 == it).map_or(0, |c| c.1);
    let recipe = |name: &str, it: Item| RECIPES.iter().find(|r| r.name == name).and_then(|r| r.inputs.iter().find(|c| c.0 == it)).map_or(0, |c| c.1);
    let kit = |it: Item| KIT.iter().find(|k| k.0 == it).map_or(0, |k| k.1);
    assert!(kit(Item::Wood) >= cost(Station::Workbench, Item::Wood) + cost(Station::Workshop, Item::Wood), "kit wood");
    let need = [(Item::Stone, cost(Station::Workbench, Item::Stone) + cost(Station::Forge, Item::Stone) + cost(Station::Workshop, Item::Stone)),
        (Item::Clay, cost(Station::Forge, Item::Clay)), (Item::Coal, 4), (Item::CopperOre, 8), (Item::TinOre, 2 * recipe("Drill", Item::TinIngot).max(1))];
    for seed in (0..24u32).chain([14085]) {
        let w = World::new(256 * S, 256 * S, seed);
        let (mx, my) = (w.moles[0].x as f32, w.moles[0].y as f32); let r = START_SAFE * CPM as f32;
        let mut n = [0u32; 5];
        for y in 0..w.g.h { for x in 0..w.g.w {
            let (dx, dy) = (x as f32 - mx, y as f32 - my); if dx * dx + dy * dy >= r * r { continue; }
            let m = w.g.get(x, y); if !w.can_dig(m) && m.p().tier > 1 { continue; }
            if let Some(it) = drop_of(m) { if let Some(k) = need.iter().position(|c| c.0 == it) { n[k] += 1; } }
        } }
        for (k, &(it, want)) in need.iter().enumerate() {
            let have = n[k] / CELLS_PER_ITEM as u32;
            assert!(have >= want, "seed {seed}: {have} {it:?} within {START_SAFE} m of the burrow, the early ladder wants {want}");
        }
    }
}
