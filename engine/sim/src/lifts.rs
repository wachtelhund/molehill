//! Vertical travel: rope ladders and lifts (see transport.rs for carts, and PROGRESSION.md).
//!
//! **Rope ladders** are cheap and need no power. One hangs from solid rock (a ledge or a shaft's lip) and
//! unrolls straight down until it meets the floor, at most `LADDER_M`. The mole climbs it with `W`/`S`
//! slower than it climbs a wall, but it can stop and hang without sliding. Jumping lets go. If the rock it
//! hangs from goes, the ladder falls as a loose item.
//!
//! **Lifts** are a cage on a rope over a head frame, balanced by a counterweight. The head hangs under a
//! solid ceiling; the cage runs down the shaft below it, from just under the head to the shaft's floor.
//! * Balance by hand: right mouse on the head with stone in the paw adds a stone of ballast (`BALLAST_KG`)
//!   to the counterweight; right mouse with anything else takes one back. The cage rises while the
//!   counterweight outweighs it (cage, plus a mole and anything standing on it) and sinks while it doesn't:
//!   a = g·(Mcw − Mcage)/(Mcw + Mcage), with a brake on the head that holds it to `LIFT_MAX`.
//! * Driven by power: copperline at the head with current in it turns the winch, which winds the cage up
//!   at `LIFT_POWER_V` whatever it carries. Cut the current and the balance takes over again.
//! * The cage is a platform: the mole stands on it and is carried with it. Loose items lie on it too.
//! * Something in the shaft (a cave-in, a block) stops the cage there.
use crate::craft::Item;
use crate::mat::{CPM, S, SF};
use crate::world::{Event, World, BODY_H, BODY_HW};

/// The longest rope ladder (cells): 8 m.
pub const LADDER_M: usize = 8 * CPM;
/// Ladder climbing speed (cells/s): 2 m/s up, 2.5 m/s down (a wall climb is 2.6 m/s up, but slides).
pub const LADDER_UP: f32 = 2. * CPM as f32; pub const LADDER_DOWN: f32 = 2.5 * CPM as f32;
/// Lift cage: 1.5 m wide, a 1-cell floor (cells). Its shaft runs at most 32 m below the head.
pub const CAGE_W: f32 = 1.5 * CPM as f32; pub const LIFT_M: usize = 32 * CPM;
/// Masses (kg): the empty cage, the counterweight's frame, one stone of ballast, and a riding mole (and
/// an item lying on the cage).
pub const CAGE_KG: f32 = 60.; pub const CW_KG: f32 = 40.; pub const BALLAST_KG: f32 = 10.; pub const MOLE_KG: f32 = 30.; pub const ITEM_KG: f32 = 5.;
/// The head's brake holds the cage to 2 m/s; the winch winds it up at 1.5 m/s (cells/s).
pub const LIFT_MAX: f32 = 2. * CPM as f32; pub const LIFT_POWER_V: f32 = 1.5 * CPM as f32;

#[derive(Clone, Debug)]
pub struct Ladder { pub x: usize, pub top: usize, pub len: usize, /** the rock cell it hangs from */ pub ax: usize, pub ay: usize }
#[derive(Clone, Debug)]
pub struct Lift {
    /// the head: column of the shaft's middle, and the open row under the ceiling it hangs from
    pub x: usize, pub head: usize,
    /// the cage floor's top row (cells, fractional), a tick ago, and its speed (cells/s, + = down)
    pub y: f32, pub py: f32, pub v: f32,
    /// the shaft's floor row (the lowest the cage floor can sit on)
    pub bottom: usize,
    pub ballast: u32,
}
impl Lift {
    /// The cage floor (x0, y0, x1, y1) in cells.
    pub fn cage(&self) -> (f32, f32, f32, f32) { let x = self.x as f32 + 0.5; (x - CAGE_W / 2., self.y, x + CAGE_W / 2., self.y + 1.) }
    pub fn top(&self) -> f32 { (self.head + 2 * S) as f32 }
    pub fn counter_kg(&self) -> f32 { CW_KG + self.ballast as f32 * BALLAST_KG }
}

impl World {
    /// Hang a rope ladder from the rock above (x, y): it unrolls down to the floor.
    pub(crate) fn place_ladder(&mut self, x: usize, y: usize) -> Result<(), &'static str> {
        if !self.in_reach(x, y) { return Err("out of reach"); }
        if !self.passable(x, y) { return Err("needs open space"); }
        // it hangs from the nearest rock above: a ceiling over it, or the lip of a ledge beside it (the rope tied
        // on the ledge's top, hanging down its face)
        let above = (y.saturating_sub(4 * CPM)..y).rev().find(|&r| !self.passable(x, r));
        let lip = (y.saturating_sub(4 * CPM)..=y).rev().find_map(|r| [x.wrapping_sub(1), x + 1, x.wrapping_sub(2), x + 2].into_iter().find(|&c| c < self.g.w && r > 0 && !self.passable(c, r) && self.passable(c, r - 1)).map(|c| (c, r)));
        let (ax, ay, top) = match (above, lip) { (Some(a), Some((c, r))) if r > a => (c, r, r.saturating_sub(2 * S).max(a + 1)), (Some(a), _) => (x, a, a + 1), (None, Some((c, r))) => (c, r, r.saturating_sub(2 * S)), _ => return Err("needs rock above or a ledge beside to hang from") };
        if self.tr.ladders.iter().any(|l| l.x.abs_diff(x) < S && l.top == top) { return Err("there's a ladder there"); }
        let len = self.ladder_len(x, top); if len < 2 * S { return Err("no room to hang it"); }
        self.tr.ladders.push(Ladder { x, top, len, ax, ay }); self.events.push(Event::Place); Ok(())
    }
    fn ladder_len(&self, x: usize, top: usize) -> usize { (top..(top + LADDER_M).min(self.g.h - 1)).take_while(|&r| self.passable(x, r)).count() }
    /// Hang a lift's head under the ceiling above (x, y); the cage starts at the bottom of its shaft.
    pub(crate) fn place_lift(&mut self, x: usize, y: usize) -> Result<(), &'static str> {
        if !self.in_reach(x, y) { return Err("out of reach"); }
        // the head hangs under the shaft's ceiling, as far up as the rope reaches
        let anchor = (y.saturating_sub(LIFT_M)..=y).rev().find(|&r| !self.passable(x, r)).ok_or("needs a ceiling to hang from")?;
        let head = anchor + 1;
        if self.tr.lifts.iter().any(|l| l.x.abs_diff(x) < CAGE_W as usize) { return Err("there's a lift there"); }
        let open = |r: usize| { let x0 = (x as f32 + 0.5 - CAGE_W / 2.).floor() as i32; (x0..x0 + CAGE_W as i32).all(|c| self.g.in_bounds(c, r as i32) && self.passable(c as usize, r)) };
        let first = head + 2 * S + 1;
        if !(head..first).all(open) { return Err("the shaft is too narrow for the cage (1.5 m)"); }
        let bottom = (first..(head + LIFT_M).min(self.g.h - 2)).find(|&r| !open(r)).ok_or("the shaft is too deep for its rope (32 m)")?;
        if bottom < first + 3 * S { return Err("the shaft is too short"); }
        let y = (bottom - 1) as f32;
        self.tr.lifts.push(Lift { x, head, y, py: y, v: 0., bottom, ballast: 0 }); self.events.push(Event::Place); Ok(())
    }
    pub(crate) fn ladder_at(&self, x: usize, y: usize) -> Option<usize> { self.tr.ladders.iter().position(|l| l.x.abs_diff(x) <= S + 1 && y + 1 >= l.top && y <= l.top + l.len) }
    pub(crate) fn lift_head_at(&self, x: usize, y: usize) -> Option<usize> { self.tr.lifts.iter().position(|l| l.x.abs_diff(x) <= CAGE_W as usize / 2 && y + 2 * S >= l.head && y <= l.head + 2 * S) }
    /// The lift whose head or cage (the cage and the 2 m over it) is at (x, y).
    pub(crate) fn lift_at(&self, x: usize, y: usize) -> Option<usize> {
        self.lift_head_at(x, y).or_else(|| self.tr.lifts.iter().position(|l| { let (x0, y0, x1, _) = l.cage(); (x as f32) >= x0 && (x as f32) < x1 && (y as f32) <= y0 + 1. && (y as f32) >= y0 - 2. * CPM as f32 }))
    }

    /// Right mouse on a lift's head or its cage: with stone in the paw, a stone of ballast onto the counterweight;
    /// otherwise one back off it into the pack.
    pub(crate) fn use_lift(&mut self, x: usize, y: usize) -> bool {
        let Some(k) = self.lift_at(x, y) else { return false };
        if self.gear.held == Some(Item::Stone) && self.inv[Item::Stone as usize] > 0 { self.inv[Item::Stone as usize] -= 1; self.tr.lifts[k].ballast += 1; }
        else if self.tr.lifts[k].ballast > 0 { self.tr.lifts[k].ballast -= 1; self.inv[Item::Stone as usize] += 1; }
        let l = &self.tr.lifts[k]; self.say(format!("Counterweight: {} kg (the cage is {} kg empty).", l.counter_kg() as u32, CAGE_KG as u32));
        self.events.push(Event::Click); true
    }
    /// `E` on a ladder or a lift's head: back into the pack (a lift's ballast too).
    pub(crate) fn pick_up_lift(&mut self, x: usize, y: usize) -> Option<Item> {
        if let Some(k) = self.lift_head_at(x, y) { let l = self.tr.lifts.remove(k); self.inv[Item::Stone as usize] += l.ballast; self.inv[Item::Lift as usize] += 1; return Some(Item::Lift); }
        if let Some(k) = self.ladder_at(x, y) { self.tr.ladders.remove(k); self.tr.climbing = false; self.inv[Item::RopeLadder as usize] += 1; return Some(Item::RopeLadder); }
        None
    }

    /// Does the player's box at (px, py) overlap a lift cage? (The cage floor is solid to stand on.)
    pub fn platform_hits(&self, px: f32, py: f32) -> bool {
        self.tr.lifts.iter().any(|l| { let (x0, y0, x1, y1) = l.cage(); px + BODY_HW > x0 && px - BODY_HW < x1 && py > y0 && py - BODY_H < y1 })
    }
    /// Is the box (an item's) at (px, py) half-size hw overlapping a cage floor?
    pub fn platform_hits_box(&self, px: f32, py: f32, hw: f32) -> bool {
        self.tr.lifts.iter().any(|l| { let (x0, y0, x1, y1) = l.cage(); px + hw > x0 && px - hw < x1 && py + hw > y0 && py - hw < y1 })
    }

    /// The mole on a rope ladder: decides whether it is climbing this tick (it grabs on with `W`/`S` while
    /// over one, holds on until it jumps or stands on the ground, and lets go off the rope's ends).
    pub(crate) fn ladder_climb(&mut self, ground: bool, jump_edge: bool) -> bool {
        let (x, y) = (self.pl.px as usize, (self.pl.py - 1.) as usize);
        let on = self.ladder_at(x, y).is_some() || self.ladder_at(x, (self.pl.py - BODY_H) as usize).is_some();
        let grab = self.input.up || self.input.down;
        self.tr.climbing = on && !jump_edge && (grab || self.tr.climbing && !ground);
        self.tr.climbing
    }

    /// One tick of every ladder and lift.
    pub(crate) fn step_lifts(&mut self) {
        // ladders: a ladder whose rock has gone falls; a floor that has risen under it (sand, a block) shortens it
        if self.tick % 15 == 0 {
            let mut k = 0;
            while k < self.tr.ladders.len() {
                let l = self.tr.ladders[k].clone();
                if self.passable(l.ax, l.ay) { self.tr.ladders.remove(k); self.drop_loose(l.x, l.top + l.len / 2, Item::RopeLadder); self.say("A rope ladder fell.".into()); continue; }
                self.tr.ladders[k].len = self.ladder_len(l.x, l.top).max(1); k += 1;
            }
        }
        let dt = 1. / 60.; let g = 9.81 * CPM as f32;
        for k in 0..self.tr.lifts.len() {
            let mut l = self.tr.lifts[k].clone(); l.py = l.y;
            // a head whose ceiling has gone falls, cage and all
            if l.head == 0 || self.passable(l.x, l.head - 1) { self.tr.lifts.remove(k); self.inv_drop(l.x, l.head, Item::Lift, l.ballast); self.say("A lift's head frame came down.".into()); return; }
            let (x0, y0, x1, _) = l.cage();
            let rider = self.moles.first().is_some_and(|m| m.alive) && self.on_cage(&l);
            let items = self.drops.iter().filter(|d| d.px >= x0 && d.px < x1 && (d.py - (y0 - crate::items::ITEM_HW)).abs() < SF).map(|d| d.n).sum::<u32>();
            let cage = CAGE_KG + if rider { MOLE_KG } else { 0. } + items as f32 * ITEM_KG; let cw = l.counter_kg();
            let powered = self.logic.net_lit(self.logic.block_of(l.x, l.head));
            if powered { l.v += (-LIFT_POWER_V - l.v).clamp(-4. * CPM as f32 * dt, 4. * CPM as f32 * dt); }
            else { l.v += g * (cage - cw) / (cage + cw) * dt; }
            l.v = l.v.clamp(-LIFT_MAX, LIFT_MAX);
            // move, cell by cell; stop at the top, the floor, or anything in the shaft
            let dy = l.v * dt; let n = (dy.abs().ceil() as i32).max(1); let mut moved = 0.;
            for _ in 0..n {
                let ny = l.y + dy / n as f32;
                let row = if dy > 0. { (ny + 1. - 1e-3).floor() as i32 } else { ny.floor() as i32 };
                let clear = (x0.floor() as i32..x1.ceil() as i32).all(|c| self.g.in_bounds(c, row) && self.passable(c as usize, row as usize));
                if ny < l.top() || ny > (l.bottom - 1) as f32 || !clear { l.v = 0.; l.y = l.y.clamp(l.top(), (l.bottom - 1) as f32); break; }
                moved += ny - l.y; l.y = ny;
            }
            // whatever stands on it goes with it
            if moved != 0. {
                if rider { self.pl.py += moved; if moved < 0. { self.pl.vy = self.pl.vy.min(0.); } let (x, y) = (self.pl.px as usize, (self.pl.py - 0.001) as usize); self.moles[0].x = x; self.moles[0].y = y; self.pl.sync = Some((x, y)); self.pl.fall_from = y; }
                for d in self.drops.iter_mut() { if d.px >= x0 && d.px < x1 && (d.py - (l.py - crate::items::ITEM_HW)).abs() < SF { d.py += moved; d.y = d.py.max(0.) as usize; d.rest = 0; } }
            }
            self.tr.lifts[k] = l;
        }
    }
    /// The mole stands on this cage (feet on its floor, over it).
    pub(crate) fn on_cage(&self, l: &Lift) -> bool { let (x0, y0, x1, _) = l.cage(); self.pl.px + BODY_HW > x0 && self.pl.px - BODY_HW < x1 && (self.pl.py - y0).abs() < 0.6 }
    fn drop_loose(&mut self, x: usize, y: usize, it: Item) { self.drops.push(crate::world::Drop::at(x as f32 + 0.5, y as f32 + 0.5, it, 1)); }
    fn inv_drop(&mut self, x: usize, y: usize, it: Item, ballast: u32) {
        self.drop_loose(x, y, it); if ballast > 0 { self.drops.push(crate::world::Drop::at(x as f32 + 0.5, y as f32 + 0.5, Item::Stone, ballast)); }
    }
}
