//! Mechanical power (power.rs): drive trains, belts, and that every block item can really be built.
//! Behaviour is covered by the belt_counterweight, belt_backs_up and water_wheel scenarios.
use molehill_sim::craft::{Item, ITEMS};
use molehill_sim::mat::{Mat, CPM, S};
use molehill_sim::survival::Mode;
use molehill_sim::world::World;

/// A granite room 10 m by 5 m round the mole, open inside.
fn room() -> (World, usize, usize) {
    let mut w = World::new(256 * S, 128 * S, 3); w.disasters.enabled = false; w.set_mode(Mode::Creative); w.drops.clear();
    let (mx, my) = (w.moles[0].x, w.moles[0].y); let c = CPM;
    for y in my - 5 * c..=my + 2 * c { for x in mx - 6 * c..=mx + 6 * c { let i = w.g.idx(x, y);
        let rock = y > my || y <= my - 5 * c || x < mx - 5 * c || x > mx + 5 * c; w.g.set(i, if rock { Mat::Granite } else { Mat::Empty }); } }
    (w, mx, my)
}

#[test] fn a_straight_shaft_needs_a_gear_to_turn_a_corner() {
    let (mut w, mx, my) = room();
    // an L of shaft along the floor and up the wall: one cell thick so the corner is a single cell
    let (x0, y0) = (mx + CPM, my);
    for k in 0..3 * CPM { let i = w.g.idx(x0 + k, y0); w.g.set(i, Mat::Shaft); }
    for k in 1..3 * CPM { let i = w.g.idx(x0, y0 - k); w.g.set(i, Mat::Shaft); }
    for _ in 0..13 { w.step(); }
    let t = |w: &World, x: usize, y: usize| w.tr.power.train.get(&w.g.idx(x, y)).copied();
    assert_ne!(t(&w, x0 + 2 * CPM, y0), t(&w, x0, y0 - 2 * CPM), "a plain shaft carried turning round a corner");
    let i = w.g.idx(x0, y0); w.g.set(i, Mat::Gear);
    for _ in 0..13 { w.step(); }
    assert_eq!(t(&w, x0 + 2 * CPM, y0), t(&w, x0, y0 - 2 * CPM), "a gear at the corner joins the two lines");
}

#[test] fn right_mouse_turns_a_belt_run_round() {
    let (mut w, mx, my) = room();
    for k in 0..2 * CPM { let i = w.g.idx(mx + CPM + k, my); w.g.set(i, Mat::Belt); }
    assert!(w.use_device(mx + CPM + 3, my), "right mouse on a belt does something");
    assert!((0..2 * CPM).all(|k| w.g.get(mx + CPM + k, my) == Mat::BeltL), "the whole run turned round");
}

#[test] fn every_block_item_can_be_built() {
    // (chutes, screens, bumpers and riffle bars once had no build code, so the brush refused them in the game)
    for &it in ITEMS.iter().filter(|it| it.block().is_some()) {
        let (mut w, mx, my) = room();
        assert!(w.place_block(mx + CPM, my, it).is_ok(), "{it:?} can't be built");
        let m = w.g.get(mx + CPM, my);
        assert!(m == it.block().unwrap() || it == Item::Belt && m == Mat::BeltL, "{it:?} built {m:?}");
    }
}

#[test] fn every_material_has_its_own_template_char() {
    // (a site template's palette must be one-to-one: the boiler shell once shared crystal's 'K')
    use molehill_sim::mat::NMAT; use molehill_sim::templates::{default_char, HOST};
    let mut seen = std::collections::HashMap::new();
    for k in 0..NMAT as u8 { let m = Mat::from_u8(k); let c = default_char(m);
        assert!(c != HOST && !c.is_ascii_digit(), "{m:?}: '{c}' is reserved"); if let Some(o) = seen.insert(c, m) { panic!("{m:?} and {o:?} both use '{c}'"); } }
}
