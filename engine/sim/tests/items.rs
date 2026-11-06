//! The item registry (`craft::ITEM_DEFS`) is hand-written rows; these check the rules a row must keep
//! that the old per-item matches used to derive, so a new item can't quietly miss one.
use molehill_sim::craft::*;

#[test]
fn rows_are_in_item_order_and_named_once() {
    for (i, &it) in ITEMS.iter().enumerate() {
        assert_eq!(it as usize, i);
        assert_eq!(it.def().item, it);
        assert!(!it.name().is_empty() && !it.desc().is_empty(), "{it:?} needs a name and a tooltip line");
        assert!(ITEMS.iter().filter(|o| o.name().eq_ignore_ascii_case(it.name())).count() == 1, "two items are called {}", it.name());
    }
}

#[test]
fn blocks_pigments_furniture_and_walls_can_be_set_down() {
    for &it in ITEMS.iter() {
        if it.block().is_some() || it.pigment().is_some() || it.furn().is_some() || it.is_wall() {
            assert!(it.uses().place, "{it:?} is a block, pigment, piece of furniture or wall but can't be set down");
        }
        if it.furn().is_some() { assert!(it.uses().pick_up, "{it:?}: furniture is picked back up with E"); }
        if let Some(p) = it.pigment() {
            assert!((1..PIGMENTS.len() as u8).contains(&p), "{it:?}: pigment index {p} out of range");
            assert_eq!(it.color(), PIGMENTS[p as usize], "{it:?}: a pigment's swatch is its paint colour");
        }
    }
}

#[test]
fn every_item_has_a_sane_density_and_a_controls_line_when_usable() {
    for &it in ITEMS.iter() {
        let d = it.density();
        assert!(d > 0. && d < 20000., "{it:?}: density {d} kg/m³");
        let u = it.uses();
        if u.place || u.throw { assert!(!it.controls().is_empty(), "{it:?} can be used but its controls line is empty"); }
    }
}
