//! Game modes (see PROGRESSION.md). **Survival** starts with a small kit: you can only dig with tools you
//! own, food drains and must be found (grubs, roots, fungus, mushrooms), the headlamp and torches burn
//! fungus oil, and dying leaves your whole pack where you fell. **Creative** has everything in the pack
//! and nothing hurts. **Plain** is the bare simulation with none of these rules (tests and scenarios).
use crate::craft::{Item, ITEMS, NITEM};
use crate::items::ITEM_HW;
use crate::mat::SF;
use crate::world::{Drop, Event, World, BODY_H};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Mode { #[default] Plain, Survival, Creative }
impl Mode {
    pub fn name(self) -> &'static str { match self { Mode::Plain => "Plain", Mode::Survival => "Survival", Mode::Creative => "Creative" } }
    pub fn from_u8(v: u8) -> Mode { match v { 1 => Mode::Survival, 2 => Mode::Creative, _ => Mode::Plain } }
    /// The rules this mode switches on.
    pub fn rules(self) -> Rules {
        match self {
            Mode::Plain => Rules::default(),
            Mode::Survival => Rules { tools: true, fuel: true, drop_pack: true, crafting: true, ..Rules::default() },
            Mode::Creative => Rules { harmless: true, ..Rules::default() },
        }
    }
}
/// The rules a mode switches on.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Rules {
    /// you can only dig with tools in the pack (the hotbar's drill acts as your best tool)
    pub tools: bool,
    /// the headlamp burns fungus oil, and a torch needs a measure of oil to be lit
    pub fuel: bool,
    /// dying leaves the whole pack as loose items where you fell
    pub drop_pack: bool,
    /// nothing hurts: vitals stay full and nothing kills
    pub harmless: bool,
    /// hunger off (a Survival option, saved with the world): food never drains
    pub no_hunger: bool,
    /// crafting is earned: recipes appear once an input has been picked up, and you craft at a working
    /// station (a hot forge, a powered workshop). Off: every recipe, anywhere (Creative, Plain)
    pub crafting: bool,
}

/// Seconds of headlamp light in one measure of fungus oil.
pub const OIL_SECS: f32 = 180.;
/// Fungus pressed into one measure of oil when the pack has no oil left.
pub const FUNGUS_PER_OIL: u32 = 2;
/// The Survival start kit: a pickaxe, a few torches, some food and oil.
/// Wood is rare near most burrows (often none within 40 m: `examples/survey.rs`), so the kit carries enough
/// for the workbench (4) and a little over.
pub const KIT: &[(Item, u32)] = &[(Item::Pickaxe, 1), (Item::Wood, 6), (Item::Torch, 4), (Item::Grub, 3), (Item::Root, 2), (Item::FungusOil, 2)];

impl World {
    /// Start this world in `mode`: its rules, and its pack (Survival's kit; Creative's 999 of everything).
    pub fn set_mode(&mut self, mode: Mode) {
        self.mode = mode; self.rules = mode.rules();
        match mode {
            Mode::Survival => { self.inv = [0; NITEM]; for &(it, n) in KIT { self.inv[it as usize] = n; } self.lamp_oil = OIL_SECS; self.known = [false; NITEM]; }
            Mode::Creative => { for v in self.inv.iter_mut() { *v = (*v).max(999); } self.lamp_oil = OIL_SECS; }
            Mode::Plain => {}
        }
        self.lamp_dry = false;
    }
    /// Turn hunger on or off for this world (a Survival option; Creative never hungers anyway).
    pub fn set_hunger(&mut self, on: bool) { self.rules.no_hunger = !on; if !on { for m in self.moles.iter_mut() { m.hunger = 100.; } } }
    /// Is the headlamp shining? It has to be on, and (in Survival) have oil in it.
    pub fn lamp_lit(&self) -> bool { self.gear.lamp && (!self.rules.fuel || self.lamp_oil > 0.) }
    /// Take one measure of fungus oil from the pack, pressing fungus into oil if there's none. False if
    /// there's neither.
    pub fn take_oil(&mut self) -> bool {
        if self.inv[Item::FungusOil as usize] > 0 { self.inv[Item::FungusOil as usize] -= 1; return true; }
        if self.inv[Item::Fungus as usize] >= FUNGUS_PER_OIL { self.inv[Item::Fungus as usize] -= FUNGUS_PER_OIL; self.say("Pressed fungus into oil".into()); self.events.push(Event::Craft); return true; }
        false
    }
    /// Once a tick, after the moles: the lamp burns oil; Creative keeps vitals full.
    pub(crate) fn step_survival(&mut self) {
        let dt = 1. / 60.;
        let alive = self.moles.first().map_or(false, |m| m.alive);
        if self.rules.fuel && self.gear.lamp && alive {
            let burn = if self.inv[Item::GroundLens as usize] > 0 { 0.5 } else { 1. }; // the Ancients' lens: half the oil
            self.lamp_oil = (self.lamp_oil - dt * burn).max(0.);
            if self.lamp_oil <= 0. {
                if self.take_oil() { self.lamp_oil += OIL_SECS; self.lamp_dry = false; }
                else if !self.lamp_dry { self.lamp_dry = true; self.say("The headlamp is out of oil. Press fungus to refill it.".into()); self.events.push(Event::Warning); }
            }
        }
        if self.rules.harmless { for m in self.moles.iter_mut() { m.hp = 100.; m.breath = 100.; m.hunger = 100.; } }
    }
    /// The player died: everything in the pack spills where they fell, as loose items, to go back for.
    pub(crate) fn drop_pack(&mut self) {
        let (cx, cy) = (self.pl.px, self.pl.py - BODY_H * 0.5); let mut k = 0;
        for (i, &it) in ITEMS.iter().enumerate() {
            let n = self.inv[i]; if n == 0 { continue; }
            // a little spread so the pile reads as a pile; seeded, so runs stay deterministic
            let mut d = Drop::at(cx, cy.max(ITEM_HW + 1.), it, n); d.vx = (((k * 7) % 11) as f32 - 5.) * 2. * SF; d.vy = -6. * SF; k += 1;
            self.drops.push(d); self.inv[i] = 0;
        }
        if k > 0 { self.say(format!("Your pack lies where you fell, {:.0} m down. Go back for it.", self.depth_m(self.pl.px as usize, self.pl.py as usize).max(0.))); }
    }
}
