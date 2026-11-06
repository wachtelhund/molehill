//! The words beside and behind the vitals: what each bar is, what is left in the pack to refill it, and
//! the choice it asks for (when to eat, when to light the lamp, when to turn back). Pure functions of the
//! world, so they are tested headless. The numbers belong to the sim (`survival.rs`); this only reads them.
use molehill_sim::craft::{Goal, Item, QUESTS};
use molehill_sim::survival::{FUNGUS_PER_OIL, OIL_SECS};
use molehill_sim::world::World;

/// The vitals rows, top to bottom: health, food, breath, lamp oil.
pub const ROWS: usize = 4;
/// What a hungry mole eats from its pack, in the order it reaches for them (as in `World::step`).
pub const FOOD: [Item; 4] = [Item::Grub, Item::Mushroom, Item::Root, Item::Fungus];

/// Whether a row is shown: food only while hunger is on, lamp oil only where the lamp burns fuel.
pub fn shown(w: &World, row: usize) -> bool { match row { 1 => !w.rules.no_hunger, 3 => w.rules.fuel, _ => true } }
/// How full a row's bar is, 0..100.
pub fn fill(w: &World, row: usize) -> f32 {
    let m = &w.moles[0];
    match row { 0 => m.hp, 1 => m.hunger, 2 => m.breath, _ => w.lamp_oil / OIL_SECS * 100. }.clamp(0., 100.)
}
/// Meals in the pack (every food item counts once).
pub fn meals(w: &World) -> u32 { FOOD.iter().map(|&it| w.inv[it as usize]).sum() }

/// The short note after a bar: what is left to refill it. Empty for health and breath.
pub fn note(w: &World, row: usize) -> String {
    match row {
        1 => match meals(w) { 0 => "no food".into(), 1 => "1 meal".into(), n => format!("{n} meals") },
        3 => { let spare = w.oil_measures(); let s = match spare { 0 => "none spare".to_string(), 1 => "1 spare".into(), n => format!("{n} spare") };
            if w.lamp_lit() { s } else { format!("lamp out · {s}") } }
        _ => String::new(),
    }
}

/// Hover text for a row: a title with the reading, and what it asks of you.
pub fn tip(w: &World, row: usize) -> (String, String) {
    let m = &w.moles[0];
    match row {
        0 => (format!("Health {:.0}", m.hp), "Falls, rockfall, heat and bad air hurt. It mends slowly while you are fed, rested and breathing easy.".into()),
        1 => {
            let have: Vec<String> = FOOD.iter().filter(|&&it| w.inv[it as usize] > 0).map(|&it| format!("{} {}", w.inv[it as usize], it.name().to_lowercase())).collect();
            (format!("Food {:.0}", m.hunger), format!("Below 40 you eat from your pack on your own{}. Grubs and roots turn up in soil; fungus feeds you, or keep it to press into lamp oil.",
                if have.is_empty() { ", but it holds nothing to eat".to_string() } else { format!(" ({})", have.join(", ")) }))
        }
        2 => (format!("Breath {:.0}", m.breath), "Runs down under water and in stale air; it comes back in open air.".into()),
        _ => {
            let s = w.lamp_oil.ceil() as u32;
            (format!("Lamp oil: {}:{:02} in this measure, {} spare", s / 60, s % 60, w.oil_measures()),
             format!("G puts the headlamp out to save oil; torches burn without it. Fungus presses into oil ({FUNGUS_PER_OIL} make a measure). When the last measure runs low, think about the way back."))
        }
    }
}

/// How far along the current quest is, for the quest line: "(2/6)", "(3/5 m)", or nothing for a build.
pub fn quest_progress(w: &World) -> String {
    let Some(q) = QUESTS.get(w.quest) else { return String::new() };
    if w.quest_done(w.quest) { return "  (done)".into(); }
    let s2 = (molehill_sim::mat::S * molehill_sim::mat::S) as u32;
    match q.goal {
        Goal::Depth(m) => format!("  ({:.0}/{:.0} m)", w.max_depth.max(0.).floor().min(m), m),
        Goal::Have(it, n) => format!("  ({}/{n})", w.inv[it as usize].min(n)),
        Goal::Chests(n) => format!("  ({}/{n})", w.chests_opened.min(n)),
        Goal::Pillars(n) => format!("  ({}/{n})", (w.pillars / s2).min(n)),
        Goal::Built(_) => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use molehill_sim::survival::Mode;
    fn survival() -> World { let mut w = World::new(512 * molehill_sim::mat::S, 256 * molehill_sim::mat::S, 7); w.set_mode(Mode::Survival); w }

    #[test] fn a_survival_start_reads_its_food_and_oil() {
        let mut w = survival(); w.gear.lamp = true; // the game lights it from Settings
        assert!((0..ROWS).all(|r| shown(&w, r)));
        assert_eq!(note(&w, 1), "5 meals", "3 grubs + 2 roots");
        assert_eq!(note(&w, 3), "2 spare");
        assert!(fill(&w, 3) > 99.);
        let (t, d) = tip(&w, 1); assert!(t.starts_with("Food")); assert!(d.contains("3 grub") && d.contains("2 root"), "{d}");
        let (t, d) = tip(&w, 3); assert!(t.starts_with("Lamp oil: 3:00"), "{t}"); assert!(d.contains("G ") && d.contains("fungus") || d.contains("Fungus"));
        // the lamp out: the note says so, so saving oil is a visible choice
        w.gear.lamp = false; assert_eq!(note(&w, 3), "lamp out · 2 spare");
        // nothing left to eat
        for it in FOOD { w.inv[it as usize] = 0; } assert_eq!(note(&w, 1), "no food"); assert!(tip(&w, 1).1.contains("holds nothing"));
        // hunger off hides the food row; Creative burns no oil, so no oil row
        w.set_hunger(false); assert!(!shown(&w, 1));
        let mut c = survival(); c.set_mode(Mode::Creative); assert!(!shown(&c, 3));
    }

    #[test] fn the_quest_line_shows_how_far_along_it_is() {
        let mut w = survival(); w.quest = 1; // "Gather 6 wood"; the kit has 6
        assert_eq!(quest_progress(&w), "  (done)");
        w.inv[Item::Wood as usize] = 2; assert_eq!(quest_progress(&w), "  (2/6)");
        w.quest = 0; w.max_depth = 3.4; assert_eq!(quest_progress(&w), "  (3/5 m)");
        w.quest = 2; assert_eq!(quest_progress(&w), "", "a build has no count");
        // every tip is plain words: no modern tech, no magic
        for r in 0..ROWS { let (a, b) = tip(&w, r); let t = format!("{a} {b}").to_lowercase(); for bad in ["magic", "battery", "electric", "sky"] { assert!(!t.contains(bad)); } }
    }
}
