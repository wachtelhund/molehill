//! One isolated charge is one blast, heard once. After it settles nothing keeps booming: no more blasts,
//! no further `Explode` or `Quake`, and a `Crumble` (the collapse rumble) only for a real collapse, never
//! for the odd cell trickling off a cracked face (that used to rumble every second or so for a while
//! after a blast, which played as repeated bangs). See also the `blastevents` example.
use molehill_sim::mat::CPM;
use molehill_sim::world::{Event, World, CRUMBLE_LOUD};

fn one_charge(seed: u32) {
    let mut w = World::new(1024 * molehill_sim::mat::S, 512 * molehill_sim::mat::S, seed); w.disasters.enabled = false;
    for _ in 0..120 { w.step(); }
    let (x, y) = (w.moles[0].x + 3 * CPM, w.moles[0].y);
    let b0 = w.blasts_run; w.blast_tnt(x, y); assert_eq!(w.blasts_run, b0 + 1, "seed {seed}: one charge, one blast");
    let (mut booms, mut quakes, mut late_rumbles) = (0, 0, 0);
    for t in 0..12 * 60 {
        let before = w.crumbled; w.step(); let crumbled = (w.crumbled - before) as usize;
        for e in &w.events {
            match e {
                Event::Explode => booms += 1,
                Event::Quake => quakes += 1,
                Event::Crumble => { assert!(crumbled >= CRUMBLE_LOUD, "seed {seed} tick {t}: a rumble for only {crumbled} cells"); if t > 2 * 60 { late_rumbles += 1; } }
                _ => {}
            }
        }
    }
    assert_eq!(w.blasts_run, b0 + 1, "seed {seed}: nothing else went off");
    assert_eq!((booms, quakes), (0, 0), "seed {seed}: the bang and its jolt are heard once, at the blast");
    assert_eq!(late_rumbles, 0, "seed {seed}: no collapse rumbles once the crater has settled");
}

#[test] fn one_charge_is_heard_once_seed_4242() { one_charge(4242); }
#[test] fn one_charge_is_heard_once_seed_7() { one_charge(7); }
