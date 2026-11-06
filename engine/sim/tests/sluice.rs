//! Sluicing (sluice.rs): running water carries what is light for its size and leaves the heavy lumps.
//! Behaviour is covered by the water_through_screen, sluice_sorts and sluice_washes_rubble scenarios.
use molehill_sim::craft::Item;
use molehill_sim::mat::{Mat, CPM};
use molehill_sim::sluice::{current, mobility, WASH_K};
use molehill_sim::world::World;

#[test] fn fines_wash_and_lumps_stay_by_mobility() {
    let mob = |it: Item| mobility(it.density(), it.size_m());
    // a modest sluice flow: 0.25 m deep on a 1-in-8 surface
    let tau = WASH_K * 0.25 * 0.125;
    for it in [Item::Sand, Item::Dirt, Item::Clay, Item::Gravel, Item::Coal] { assert!(mob(it) < tau, "{it:?} ({}) should wash in a sluice", mob(it)); }
    for it in [Item::CopperOre, Item::TinOre, Item::IronOre, Item::Stone] { assert!(mob(it) > tau, "{it:?} ({}) should stay in a sluice", mob(it)); }
    assert!(mob(Item::IronOre) > mob(Item::CopperOre), "the denser ore is the harder to shift");
}

#[test] fn still_water_has_no_current() {
    let mut w = World::new(256 * molehill_sim::mat::S, 128 * molehill_sim::mat::S, 3); w.disasters.enabled = false;
    let (mx, my) = (w.moles[0].x, w.moles[0].y); let c = CPM;
    // a 4 m by 1 m pool in a granite box, level on top
    for y in my - 3 * c..=my + 2 * c { for x in mx..=mx + 6 * c { let i = w.g.idx(x, y);
        let water = y > my - c && y <= my && x > mx && x < mx + 5 * c;
        w.g.set(i, if water { Mat::Water } else if y <= my - c && x > mx && x < mx + 5 * c { Mat::Empty } else { Mat::Granite }); } }
    for k in 1..5 { assert!(current(&w, (mx + k * c) as f32 + 0.5, (my - c / 2) as f32).is_none(), "a level pool runs at {k} m"); }
    assert!(current(&w, (mx + 2 * c) as f32 + 0.5, (my - 2 * c) as f32).is_none(), "no current in the air over it");
}

/// Water capped by rock has no free surface: a flooded passage under a low roof beside an open, deeper hole
/// isn't a downhill surface (it once read as a current and dragged a dead mole's pack under the roof).
#[test] fn water_under_a_roof_is_not_a_current() {
    let mut w = World::new(256 * molehill_sim::mat::S, 128 * molehill_sim::mat::S, 3); w.disasters.enabled = false;
    let (mx, my) = (w.moles[0].x, w.moles[0].y); let c = CPM;
    // an open hole 1 m wide, full to its brim, opening into a passage whose roof sits a tile under the brim
    for y in my - 2 * c..=my + 3 * c { for x in mx..=mx + 6 * c { let i = w.g.idx(x, y);
        let m = if y > my + 2 * c || x == mx || x == mx + 6 * c { Mat::Bedrock }
            else if x <= mx + c { if y > my { Mat::Water } else { Mat::Empty } }
            else if y <= my + molehill_sim::mat::S { Mat::Bedrock } else { Mat::Water };
        w.g.set(i, m); } }
    for x in [mx + c / 2, mx + c, mx + 2 * c, mx + 3 * c] { assert!(current(&w, x as f32 + 0.5, (my + 2 * c) as f32).is_none(), "a current at {}", x - mx); }
}
