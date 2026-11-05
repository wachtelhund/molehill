//! Hotbar clarity in Survival: what you can use now comes first, what you have none of steps back.
//! A new or loaded Survival holt lays the hotbar out with the things in your pack first (the pickaxe,
//! torches, wood...), then the things that cost nothing to use (stations to plan, free devices), then the
//! rest. Slots holding something you have none of are drawn faint, without a "0". When you gain something
//! you had none of and it sits in the pack, it moves into the first faint hotbar slot (never displacing a
//! thing you have). Creative has everything, so it keeps its fixed layout.
use crate::{Game, Thing, thing_count, thing_usable};

/// Tint (multiplies the icon's colours) for a slot holding something you have none of: a cool grey at
/// about half strength, so the shape stays readable but the colour drops out and it sits back.
pub const FAINT_ICON: [f32; 4] = [0.62, 0.62, 0.68, 0.45];
/// The brush ring over rock your tools can't dig: a cold slate blue, darker and bluer than the warm
/// white dig ring (1, 0.97, 0.9) so it never reads as a white outline, and solid enough to see on rock.
pub const REFUSE_RING: [f32; 4] = [0.42, 0.52, 0.72, 0.6];

/// How usable a thing is right now. Ordered: owned first, free next, none last.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Have { Owned, Free, None }

/// `count`: how many the pack holds (None: the thing has no count, e.g. a station or a free device).
/// `usable`: for a dig tool, whether you own it (without the pickaxe, slot 1 is your claws).
pub fn have_of(t: Thing, count: Option<u32>, usable: bool) -> Have {
    match t {
        Thing::Pick(1) if !usable => Have::Free, // claws
        Thing::Pick(_) => if usable { Have::Owned } else { Have::None },
        Thing::Station(_) => Have::Free,
        _ => match count { Some(0) => Have::None, Some(_) => Have::Owned, None => Have::Free },
    }
}
pub fn have(g: &Game, t: Thing) -> Have { have_of(t, thing_count(g, t), thing_usable(g, t)) }
/// The same read straight from a world (for tests; matches `thing_count` / `thing_usable`).
#[cfg(test)]
pub fn have_w(w: &molehill_sim::world::World, t: Thing) -> Have {
    use molehill_sim::{craft::Item, logic::DEVS};
    let count = match t { Thing::Item(it) => Some(w.inv[it as usize]), Thing::Dev(d) => DEVS[d].cost().map(|it| w.inv[it as usize]), _ => None };
    let usable = match t { Thing::Pick(n) => !w.rules.tools || w.inv[[Item::Pickaxe, Item::Drill, Item::Bore][(n as usize).clamp(1, 3) - 1] as usize] > 0, _ => true };
    have_of(t, count, usable)
}
/// Survival's rules (tools must be owned): the only mode where the hotbar adapts.
pub fn adaptive(g: &Game) -> bool { g.w.rules.tools }

/// Stable sort: owned, then free, then none, then empty slots; within each, the given order.
pub fn arrange(slots: &mut [Option<Thing>], f: impl Fn(Thing) -> Have) {
    let rank = |s: &Option<Thing>| s.map_or(3, |t| f(t) as u8);
    slots.sort_by_key(rank);
}

/// Move each newly owned thing found in the pack (slot >= `hot_n`) into the first hotbar slot that is
/// empty or holds a thing you have none of (swapping them). Returns how many moved.
pub fn promote(slots: &mut [Option<Thing>], hot_n: usize, newly: &[Thing], f: impl Fn(Thing) -> Have) -> usize {
    let mut moved = 0;
    for &t in newly {
        let Some(k) = slots.iter().position(|&s| s == Some(t)) else { continue };
        if k < hot_n { continue; }
        let Some(j) = (0..hot_n).find(|&j| slots[j].map_or(true, |u| f(u) == Have::None)) else { break };
        slots.swap(j, k); moved += 1;
    }
    moved
}

/// Each frame: lay out a fresh holt's hotbar, and promote newly owned things (not while the pack is open
/// or a thing is in the cursor, so a player's own arranging is never fought).
pub fn update(mut g: bevy::prelude::ResMut<Game>, mut prev: bevy::prelude::Local<Vec<(Thing, Have)>>) {
    if !adaptive(&g) { prev.clear(); g.relayout = false; return; }
    if g.relayout {
        g.relayout = false; prev.clear();
        let mut s = crate::default_slots(); arrange(&mut s, |t| have(&g, t)); g.slots = s; g.hot = 0;
    }
    let now: Vec<(Thing, Have)> = g.slots.iter().flatten().map(|&t| (t, have(&g, t))).collect();
    let newly: Vec<Thing> = now.iter().filter(|(t, h)| *h == Have::Owned && prev.iter().any(|(u, p)| u == t && *p == Have::None)).map(|(t, _)| *t).collect();
    if !newly.is_empty() && !g.inv_open && g.held.is_none() {
        let mut s = std::mem::take(&mut g.slots); promote(&mut s, 10, &newly, |t| have(&g, t)); g.slots = s;
    }
    *prev = now;
}

#[cfg(test)]
mod tests {
    use super::*;
    use molehill_sim::craft::Item;
    fn f(owned: &'static [Thing]) -> impl Fn(Thing) -> Have {
        move |t| match t { Thing::Station(_) => Have::Free, _ if owned.contains(&t) => Have::Owned, _ => Have::None }
    }

    /// Back-wall items sit in the pack so Creative (and a Survival craft) can panel without hunting.
    #[test] fn the_pack_holds_every_back_wall() {
        let s = crate::default_slots();
        for it in [Item::RockWall, Item::TimberWall, Item::MasonryWall, Item::BeamWall, Item::PipeWall, Item::CarvedWall] {
            assert!(s.contains(&Some(Thing::Item(it))), "{it:?} missing from the pack");
        }
    }

    /// A real Survival start: the hotbar holds the pickaxe, the torches and the rest of the kit, and none
    /// of its ten slots is a faint thing you have none of. Creative has everything.
    #[test] fn the_survival_kit_fills_the_hotbar() {
        use molehill_sim::{world::World, survival::{Mode, KIT}};
        let mut w = World::new(512 * molehill_sim::mat::S, 256 * molehill_sim::mat::S, 7); w.set_mode(Mode::Survival);
        let mut s = crate::default_slots(); arrange(&mut s, |t| have_w(&w, t));
        let hot: Vec<Thing> = s[..10].iter().flatten().copied().collect();
        assert_eq!(hot[0], Thing::Pick(1));
        for &(it, _) in KIT { if it != Item::Pickaxe && s.contains(&Some(Thing::Item(it))) { assert!(hot.contains(&Thing::Item(it)), "{it:?} not on the hotbar: {hot:?}"); } }
        assert!(hot.iter().all(|&t| have_w(&w, t) != Have::None), "{hot:?}");
        let d = crate::default_slots(); assert!(d[..10].iter().flatten().any(|&t| have_w(&w, t) == Have::None), "the fixed layout had faint slots");
        w.set_mode(Mode::Creative); assert!(d.iter().flatten().all(|&t| have_w(&w, t) != Have::None));
    }

    /// The two "you can't" looks stay readable but quiet: the refusal ring is cold (blue above red), much
    /// darker than the warm dig ring and solid enough to see; a faint icon keeps its shape at about half
    /// strength with its colour mostly drained.
    #[test] fn refusal_ring_and_faint_icons_read_but_stay_quiet() {
        let lum = |c: [f32; 4]| 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
        let warm = [1., 0.97, 0.9, 1.];
        let r = REFUSE_RING;
        assert!(r[2] - r[0] >= 0.2, "cold: blue well above red");
        assert!(lum(r) < 0.6 * lum(warm), "not a white outline: {} vs {}", lum(r), lum(warm));
        assert!((0.45..=0.8).contains(&r[3]), "visible on rock, not shouting");
        let f = FAINT_ICON;
        assert!((0.35..=0.6).contains(&f[3]), "readable, not cluttering");
        assert!(f[0..3].iter().all(|&v| (0.5..=0.75).contains(&v)) && f[2] >= f[0], "a cool grey tint");
    }

    #[test] fn have_reads_counts_tools_and_free_things() {
        assert_eq!(have_of(Thing::Pick(1), None, false), Have::Free, "claws still dig");
        assert_eq!(have_of(Thing::Pick(1), None, true), Have::Owned);
        assert_eq!(have_of(Thing::Pick(2), None, false), Have::None);
        assert_eq!(have_of(Thing::Item(Item::Torch), Some(4), true), Have::Owned);
        assert_eq!(have_of(Thing::Item(Item::Tnt), Some(0), true), Have::None);
        assert_eq!(have_of(Thing::Dev(0), None, true), Have::Free);
        assert_eq!(have_of(Thing::Station(0), None, true), Have::Free);
    }

    #[test] fn a_survival_start_puts_what_you_hold_first() {
        let (pick, drill, brick, torch, tnt, wood) = (Thing::Pick(1), Thing::Pick(2), Thing::Item(Item::Brick), Thing::Item(Item::Torch), Thing::Item(Item::Tnt), Thing::Item(Item::Wood));
        let mut s = vec![Some(pick), Some(drill), Some(brick), Some(torch), Some(tnt), None, Some(Thing::Station(0)), Some(wood)];
        arrange(&mut s, f(&[Thing::Pick(1), Thing::Item(Item::Torch), Thing::Item(Item::Wood)]));
        assert_eq!(s, vec![Some(pick), Some(torch), Some(wood), Some(Thing::Station(0)), Some(drill), Some(brick), Some(tnt), None]);
    }

    #[test] fn a_new_thing_takes_a_faint_slot_and_never_bumps_an_owned_one() {
        let (pick, torch, brick, tnt) = (Thing::Pick(1), Thing::Item(Item::Torch), Thing::Item(Item::Brick), Thing::Item(Item::Tnt));
        // hotbar of 3: pick, torch, brick (none); pack: tnt (just found)
        let mut s = vec![Some(pick), Some(torch), Some(brick), Some(tnt)];
        assert_eq!(promote(&mut s, 3, &[tnt], f(&[Thing::Pick(1), Thing::Item(Item::Torch), Thing::Item(Item::Tnt)])), 1);
        assert_eq!(s, vec![Some(pick), Some(torch), Some(tnt), Some(brick)]);
        // a full hotbar of owned things: the new thing stays in the pack
        let mut s = vec![Some(pick), Some(torch), Some(tnt), Some(brick)];
        assert_eq!(promote(&mut s, 3, &[brick], f(&[Thing::Pick(1), Thing::Item(Item::Torch), Thing::Item(Item::Tnt), Thing::Item(Item::Brick)])), 0);
        assert_eq!(s[3], Some(brick));
        // already on the hotbar: nothing moves
        let mut s = vec![Some(pick), Some(brick), None, Some(tnt)];
        assert_eq!(promote(&mut s, 3, &[brick], f(&[Thing::Item(Item::Brick)])), 0);
        // an empty hotbar slot is taken too
        let mut s = vec![Some(pick), Some(torch), None, Some(tnt)];
        assert_eq!(promote(&mut s, 3, &[tnt], f(&[Thing::Pick(1), Thing::Item(Item::Torch), Thing::Item(Item::Tnt)])), 1); assert_eq!(s[2], Some(tnt));
    }
}
