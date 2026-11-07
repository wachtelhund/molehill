//! Vertical travel: rope ladders and lifts (see transport.rs for carts, and PROGRESSION.md).
//!
//! **Rope ladders** are cheap and need no power. One hangs from solid rock (a ledge or a shaft's lip) and
//! unrolls straight down until it meets the floor, at most `LADDER_M`. The mole climbs it with `W`/`S`
//! a little faster than it climbs a bare wall, and it can stop and hang without sliding. It grabs on at
//! once (`W` over the rope, `S` standing at its head), is pulled onto the rope, and climbs off the top of
//! a ledge's ladder onto the ledge by itself. Jumping, or pressing `A`/`D` alone, lets go. Climbing down
//! never counts as a fall. If the rock it hangs from goes, the ladder falls as a loose item.
//!
//! **Lifts** are a cage on a rope over a head frame, balanced by a counterweight. The head hangs under a
//! solid ceiling; the cage runs down the shaft below it, from just under the head to the shaft's floor.
//! * Placing snaps to the shaft: the cage is built to the shaft's width (1.5 to 3 m) and centred in it, so
//!   it meets the walls and every side opening is flush (`lift_plan`, which the game's ghost preview draws).
//! * **Stops**: the top, every side opening with a floor level with the cage (a passage off the shaft), and
//!   the shaft's floor. The head's brake holds the cage still between rides.
//! * **Riding**: on the cage `W` / `S` haul the hand rope to the next stop up or down; held, the cage runs
//!   on from stop to stop. **Calling**: standing at a stop beside the shaft (or on its floor), `W` / `S`
//!   haul the cage to you.
//! * **Balance**: the mole's arms haul up to `HAUL_KG` out of balance either way, which covers the empty cage
//!   and a rider. Heavier loads need ballast: right mouse on the lift balances the counterweight against
//!   what's on the cage now, with stone from the pack (`BALLAST_KG` each), or hands spare stone back.
//! * Driven by power: copperline at the head with current in it turns the winch, which hauls any load at
//!   `LIFT_POWER_V` on the same keys and calls.
//! * The cage is a platform: the mole stands on it and is carried with it. Loose items lie on it too.
//! * Something in the shaft (a cave-in, a block, a mole under it) stops the cage there.
use crate::craft::Item;
use crate::mat::{CPM, S, SF};
use crate::world::{Event, World, BODY_H, BODY_HW};

/// The longest rope ladder (cells): 8 m.
pub const LADDER_M: usize = 8 * CPM;
/// Ladder climbing speed (cells/s): 3 m/s up, 3.5 m/s down (a wall climb is 2.6 m/s up, and slides).
pub const LADDER_UP: f32 = 3. * CPM as f32; pub const LADDER_DOWN: f32 = 3.5 * CPM as f32;
/// Lift cage: 1.5 m wide at the least, 3 m at most (built to the shaft), a 1-cell floor (cells). Its shaft
/// runs at most 32 m below the head.
pub const CAGE_W: f32 = 1.5 * CPM as f32; pub const CAGE_MAX: f32 = 3. * CPM as f32; pub const LIFT_M: usize = 32 * CPM;
/// What the mole's arms haul on the hand rope (kg out of balance, either way): the empty cage and a rider
/// against the bare counterweight (50 kg) and a couple of things besides. More needs ballast, or current.
pub const HAUL_KG: f32 = 60.;
/// The cage speeds up and slows down at 4 m/s² (cells/s²), so it stops level at a stop.
pub const LIFT_ACC: f32 = 4. * CPM as f32;
/// Stops closer than 1 m count as one (cells); a side opening counts as a stop if its floor is within 0.25 m
/// of the cage's side.
const STOP_GAP: usize = CPM; const STOP_REACH: usize = 2 * S / 2;
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
    /// the cage's width (cells): the shaft's, 1.5 to 3 m
    pub w: f32,
    /// the cage-floor rows it stops at, top first (the top, side openings, the shaft floor); not saved
    pub stops: Vec<usize>,
    /// the stop it is hauled to (None: the brake holds it)
    pub target: Option<usize>,
}
impl Lift {
    pub fn new(x: usize, head: usize, y: f32, bottom: usize, ballast: u32, w: f32) -> Lift { Lift { x, head, y, py: y, v: 0., bottom, ballast, w, stops: vec![], target: None } }
    /// The cage floor (x0, y0, x1, y1) in cells.
    pub fn cage(&self) -> (f32, f32, f32, f32) { let x = self.x as f32 + 0.5; (x - self.w / 2., self.y, x + self.w / 2., self.y + 1.) }
    /// The highest the cage floor goes: a mole's height and a cell under the head frame.
    pub fn top(&self) -> f32 { lift_top(self.head) as f32 }
    pub fn counter_kg(&self) -> f32 { CW_KG + self.ballast as f32 * BALLAST_KG }
    /// The stop the cage is standing at, if any (index into `stops`).
    pub fn at_stop(&self) -> Option<usize> { self.stops.iter().position(|&r| (r as f32 - self.y).abs() < 0.5) }
    /// The next stop beyond the cage going up (`dir` -1) or down (+1).
    pub fn next_stop(&self, dir: i32) -> Option<usize> {
        if dir < 0 { self.stops.iter().rev().copied().find(|&r| (r as f32) < self.y - 0.5) } else { self.stops.iter().copied().find(|&r| r as f32 > self.y + 0.5) }
    }
}
/// The top stop's cage-floor row under a head at row `head`: room for a mole standing on the cage.
pub fn lift_top(head: usize) -> usize { head + crate::world::MOLE_H + 1 }
/// Where a lift would go for a cursor at (x, y) (`World::lift_plan`): the shaft's middle column, the cage's
/// width, the head row, the shaft's floor and the stops (cage-floor rows, top first).
#[derive(Clone, Debug)]
pub struct LiftPlan { pub x: usize, pub w: f32, pub head: usize, pub bottom: usize, pub stops: Vec<usize> }
/// The balance of a lift as it is now: what's on the cage (kg), the counterweight (kg), and whether the
/// mole's arms can haul it (or the winch has current).
#[derive(Clone, Copy, Debug)]
pub struct Balance { pub cage: f32, pub weight: f32, pub rider: bool, pub powered: bool }
impl Balance {
    /// kg the cage side is heavier (negative: the counterweight is).
    pub fn over(&self) -> f32 { self.cage - self.weight }
    pub fn haulable(&self) -> bool { self.powered || self.over().abs() <= HAUL_KG }
}

impl World {
    /// Hang a rope ladder from the rock above (x, y): it unrolls down to the floor.
    pub(crate) fn place_ladder(&mut self, x: usize, y: usize) -> Result<(), &'static str> {
        let l = self.ladder_plan(x, y)?;
        if !self.in_reach(x, y) { return Err("out of reach"); }
        self.tr.ladders.push(l); self.events.push(Event::Place); Ok(())
    }
    /// Where a rope ladder would hang for a cursor at (x, y), snapped: tied on the lip of a ledge within 1 m
    /// beside the cursor (hanging down the ledge's face, against it), else from the rock right above. It
    /// unrolls to the floor, at most `LADDER_M`. The game draws this as the placing ghost.
    pub fn ladder_plan(&self, x: usize, y: usize) -> Result<Ladder, &'static str> {
        if x >= self.g.w || y >= self.g.h { return Err("needs open space"); }
        if !self.passable(x, y) { return Err("needs open space: aim below the ledge or ceiling"); }
        // a ledge's lip near the cursor: rock with open air over it, the face under it open on the cursor's side
        let lip = (y.saturating_sub(4 * CPM)..=y).rev().find_map(|r| (1..=CPM).flat_map(|d| [x.wrapping_sub(d), x + d]).find(|&c| c < self.g.w && r > 0
            && !self.passable(c, r) && self.passable(c, r - 1) && { let side = if c < x { c + 1 } else { c - 1 }; (x.min(side)..=x.max(side)).all(|cc| self.passable(cc, r)) }).map(|c| (c, r)));
        let above = |x: usize| (y.saturating_sub(4 * CPM)..y).rev().find(|&r| !self.passable(x, r));
        let (lx, ax, ay, top) = match (above(x), lip) {
            (a, Some((c, r))) if a.map_or(true, |a| r > a) => { let lx = if c < x { c + 1 } else { c - 1 }; (lx, c, r, r.saturating_sub(2 * S).max(above(lx).map_or(0, |a| a + 1))) }
            (Some(a), _) => (x, x, a, a + 1),
            _ => return Err("needs rock above it or a ledge beside it to hang from") };
        if self.tr.ladders.iter().any(|l| l.x.abs_diff(lx) < S && l.top.abs_diff(top) < S) { return Err("there's a ladder there"); }
        let len = self.ladder_len(lx, top); if len < 2 * S { return Err("no room under it to hang"); }
        Ok(Ladder { x: lx, top, len, ax, ay })
    }
    fn ladder_len(&self, x: usize, top: usize) -> usize { (top..(top + LADDER_M).min(self.g.h - 1)).take_while(|&r| self.passable(x, r)).count() }
    /// Hang a lift's head under the ceiling above (x, y); the cage starts at the bottom of its shaft.
    pub(crate) fn place_lift(&mut self, x: usize, y: usize) -> Result<(), &'static str> {
        if !self.in_reach(x, y) { return Err("out of reach"); }
        let p = self.lift_plan(x, y)?;
        let y = (p.bottom - 1) as f32;
        let mut l = Lift::new(p.x, p.head, y, p.bottom, 0, p.w); l.stops = p.stops;
        self.tr.lifts.push(l); self.events.push(Event::Place);
        self.say("Lift hung. Step on and W / S to ride it; at a stop, W / S calls it.".into()); Ok(())
    }
    /// Where a lift would go for a cursor at (x, y), snapped to the shaft: the open run of the cursor's row
    /// gives the cage's width (1.5 to 3 m; a wider cave gets a 3 m cage under the cursor) and its middle
    /// column; the head hangs under the ceiling above that, and the cage runs down to the shaft's floor.
    pub fn lift_plan(&self, x: usize, y: usize) -> Result<LiftPlan, &'static str> {
        if x >= self.g.w || y >= self.g.h || !self.passable(x, y) { return Err("aim into the open shaft"); }
        // the shaft's walls: the narrowest open run through the cursor's column within 2 m above or below (so a
        // cursor level with a side passage still finds the shaft, not the passage)
        let reach = CAGE_MAX as usize + CPM;
        let run = |r: usize| { let a = (x.saturating_sub(reach)..=x).rev().take_while(|&c| self.passable(c, r)).last().unwrap_or(x);
            let b = (x..(x + reach + 1).min(self.g.w)).take_while(|&c| self.passable(c, r)).last().unwrap_or(x); (a, b) };
        let (a, b) = (y.saturating_sub(2 * CPM)..=(y + 2 * CPM).min(self.g.h - 1)).filter(|&r| self.passable(x, r)).map(run)
            .filter(|&(a, b)| (b + 1 - a) as f32 >= CAGE_W).min_by_key(|&(a, b)| b - a).unwrap_or_else(|| run(y));
        let span = b + 1 - a;
        if (span as f32) < CAGE_W { return Err("the shaft is too narrow for the cage (1.5 m)"); }
        // an odd number of cells wide, so the cage sits square on its middle column
        let mut wc = span.min(CAGE_MAX as usize); if wc % 2 == 0 { wc -= 1; } let hw = (wc - 1) / 2;
        // the middle column: the run's, or under the cursor in a cave wider than a cage
        let xc = if span <= CAGE_MAX as usize { (a + b) / 2 } else { x.clamp(a + hw, b - hw) };
        let w = wc as f32;
        let cols = ((xc - hw) as i32, (xc + hw + 1) as i32);
        let open = |r: usize| (cols.0..cols.1).all(|c| self.g.in_bounds(c, r as i32) && self.passable(c as usize, r));
        if !open(y) { return Err("the shaft is too narrow for the cage (1.5 m)"); }
        // the head hangs under the shaft's ceiling, as far up as the rope reaches
        let anchor = (y.saturating_sub(LIFT_M)..=y).rev().find(|&r| !open(r) || !self.passable(xc, r)).ok_or("needs a ceiling to hang from")?;
        if self.passable(xc, anchor) { return Err("the shaft narrows above: needs a flat ceiling over the cage"); }
        let head = anchor + 1;
        if self.tr.lifts.iter().any(|l| (l.x as f32 - xc as f32).abs() < (l.w + w) / 2. && l.head.abs_diff(head) < LIFT_M) { return Err("there's a lift there"); }
        let first = lift_top(head) + 1;
        if !(head..first).all(open) { return Err("the shaft is too narrow for the cage (1.5 m)"); }
        let bottom = (first..(head + LIFT_M).min(self.g.h - 2)).find(|&r| !open(r)).ok_or("the shaft is too deep for its rope (32 m)")?;
        if bottom < first + 3 * S { return Err("the shaft is too short (2 m at least)"); }
        Ok(LiftPlan { x: xc, w, head, bottom, stops: self.lift_stops(xc, w, head, bottom) })
    }
    /// The cage-floor rows a lift stops at, top first: the top, each side opening whose floor meets the cage's
    /// side (within 0.25 m) with room for a mole over it, and the shaft's floor.
    pub(crate) fn lift_stops(&self, x: usize, w: f32, head: usize, bottom: usize) -> Vec<usize> {
        let x0 = (x as f32 + 0.5 - w / 2.).floor() as usize; let x1 = (x as f32 + 0.5 + w / 2.).ceil() as usize; // cage columns x0..x1
        let top = lift_top(head); let mut stops = vec![top];
        let room = |c: usize, f: usize| f >= crate::world::MOLE_H && (f - crate::world::MOLE_H..f).all(|r| self.passable(c, r));
        for f in top + STOP_GAP..bottom.saturating_sub(STOP_GAP) {
            let side = |c: usize| c < self.g.w && !self.passable(c, f) && room(c, f);
            let left = (1..=STOP_REACH + 1).map(|d| x0.wrapping_sub(d)).take_while(|&c| c < self.g.w && room(c, f) ).any(|c| side(c));
            let right = (0..=STOP_REACH).map(|d| x1 + d).take_while(|&c| c < self.g.w && room(c, f)).any(|c| side(c));
            if (left || right) && stops.last().map_or(true, |&l| f >= l + STOP_GAP) { stops.push(f); }
        }
        let floor = bottom - 1; if stops.last().is_some_and(|&l| floor < l + STOP_GAP) { stops.pop(); } stops.push(floor);
        stops
    }
    pub(crate) fn ladder_at(&self, x: usize, y: usize) -> Option<usize> { self.tr.ladders.iter().position(|l| l.x.abs_diff(x) <= S + 1 && y + 1 >= l.top && y <= l.top + l.len) }
    pub(crate) fn lift_head_at(&self, x: usize, y: usize) -> Option<usize> { self.tr.lifts.iter().position(|l| l.x.abs_diff(x) <= l.w as usize / 2 && y + 2 * S >= l.head && y <= l.head + 2 * S) }
    /// The lift whose head or cage (the cage and the 2 m over it) is at (x, y).
    pub fn lift_at(&self, x: usize, y: usize) -> Option<usize> {
        self.lift_head_at(x, y).or_else(|| self.tr.lifts.iter().position(|l| { let (x0, y0, x1, _) = l.cage(); (x as f32) >= x0 && (x as f32) < x1 && (y as f32) <= y0 + 1. && (y as f32) >= y0 - 2. * CPM as f32 }))
    }

    /// The balance of lift `k` now: the cage with whatever is on it against the counterweight.
    pub fn lift_balance(&self, k: usize) -> Balance {
        let l = &self.tr.lifts[k]; let (x0, y0, x1, _) = l.cage();
        let rider = self.moles.first().is_some_and(|m| m.alive) && self.on_cage(l);
        let items = self.drops.iter().filter(|d| d.px >= x0 && d.px < x1 && (d.py - (y0 - crate::items::ITEM_HW)).abs() < SF).map(|d| d.n).sum::<u32>();
        Balance { cage: CAGE_KG + if rider { MOLE_KG } else { 0. } + items as f32 * ITEM_KG, weight: l.counter_kg(), rider, powered: self.lift_powered(l) }
    }
    /// Live copperline at the head frame (in any block it spans: the cage's width, 0.5 m tall) turns the winch.
    pub fn lift_powered(&self, l: &Lift) -> bool {
        let (x0, _, x1, _) = l.cage(); let b = crate::logic::B;
        (x0.max(0.) as usize / b..=(x1 as usize).saturating_sub(1) / b).any(|bx| [l.head, l.head + 2 * S].iter().any(|&r| self.logic.net_lit(self.logic.block_of(bx * b, r))))
    }
    /// Right mouse on a lift's head or cage balances it: stone from the pack onto the counterweight (or spare
    /// stone back) until it matches what's on the cage now, a mole on it counted. Says how it stands.
    pub(crate) fn use_lift(&mut self, x: usize, y: usize) -> bool {
        let Some(k) = self.lift_at(x, y) else { return false };
        let b = self.lift_balance(k);
        // the ballast that balances the empty cage and a rider at the least (so W / S stay light after)
        let want = (((b.cage.max(CAGE_KG + MOLE_KG) - CW_KG) / BALLAST_KG).round().max(0.)) as u32;
        let have = self.tr.lifts[k].ballast;
        if want > have { let n = (want - have).min(self.inv[Item::Stone as usize]); self.inv[Item::Stone as usize] -= n; self.tr.lifts[k].ballast += n; }
        else { self.inv[Item::Stone as usize] += have - want; self.tr.lifts[k].ballast = want; }
        let b = self.lift_balance(k); let short = want.saturating_sub(self.tr.lifts[k].ballast);
        self.say(if short > 0 { format!("Counterweight {:.0} kg, cage {:.0} kg: {short} more stone would balance it.", b.weight, b.cage) }
            else { format!("Balanced: counterweight {:.0} kg against the cage's {:.0} kg. Light to haul.", b.weight, b.cage) });
        self.events.push(Event::Click); true
    }
    /// The lift the mole stands on, if any.
    pub fn lift_ridden(&self) -> Option<usize> { if !self.moles.first().is_some_and(|m| m.alive) { return None; } self.tr.lifts.iter().position(|l| self.on_cage(l)) }
    /// The lift the mole could call (standing at one of its stops beside the shaft or on its floor, the cage
    /// elsewhere), and that stop.
    pub fn lift_callable(&self) -> Option<(usize, usize)> {
        if !self.moles.first().is_some_and(|m| m.alive) || !self.pl.grounded || self.lift_ridden().is_some() { return None; }
        let (px, py) = (self.pl.px, self.pl.py);
        self.tr.lifts.iter().enumerate().find_map(|(k, l)| {
            let (x0, _, x1, _) = l.cage(); if px < x0 - CPM as f32 || px > x1 + CPM as f32 { return None; }
            let s = l.stops.iter().copied().find(|&r| (r as f32 - py).abs() <= 1.5 || r + 1 == l.bottom && (l.bottom as f32 - py).abs() <= 1.5)?;
            if (s as f32 - l.y).abs() < 0.5 || l.target == Some(s) { return None; }
            Some((k, s))
        })
    }
    /// A cage standing still just over the head of a mole on the floor under it (it came down onto the mole and
    /// stopped), with room on it for the mole: `W` climbs up through its floor hatch.
    pub fn lift_over_head(&self) -> Option<usize> {
        if !self.moles.first().is_some_and(|m| m.alive) || !self.pl.grounded || self.lift_ridden().is_some() { return None; }
        let (px, py) = (self.pl.px, self.pl.py); let head = py - BODY_H;
        self.tr.lifts.iter().position(|l| { let (x0, _, x1, _) = l.cage();
            l.v == 0. && px + BODY_HW > x0 && px - BODY_HW < x1 && l.y + 1. <= head + 0.01 && l.y + 1. >= head - S as f32 && !self.box_hits(px, l.y) })
    }
    /// `W` here hauls a lift (so it doesn't jump): on a cage with a stop above, calling one, or under one to climb on.
    pub fn lift_takes_up(&self) -> bool { self.lift_ridden().is_some_and(|k| self.tr.lifts[k].next_stop(-1).is_some()) || self.lift_callable().is_some() || self.lift_over_head().is_some() }
    /// `S` here hauls a lift (so it doesn't dig): on a cage with a stop below, or calling one.
    pub fn lift_takes_down(&self) -> bool { self.lift_ridden().is_some_and(|k| self.tr.lifts[k].next_stop(1).is_some()) || self.lift_callable().is_some() }
    /// `W` / `S` on the lifts this tick: a rider hauls to the next stop that way (held: on to the next);
    /// a fresh press at a stop beside the shaft calls the cage there.
    fn lift_keys(&mut self) {
        let (up, down) = (self.input.up, self.input.down); let (uw, dw) = self.tr.lift_keys_was; self.tr.lift_keys_was = (up, down);
        let dir = if up { -1 } else if down { 1 } else { 0 };
        if let Some(k) = self.lift_ridden() {
            let l = &self.tr.lifts[k];
            let going = l.target.map(|t| if (t as f32) < l.y { -1 } else { 1 });
            if dir != 0 && going != Some(dir) { if let Some(t) = l.next_stop(dir) { self.lift_haul(k, t); } }
            return;
        }
        // under the cage that came down over its head: W climbs up through the floor hatch onto it
        if up && !uw { if let Some(k) = self.lift_over_head() { let y = self.tr.lifts[k].y; self.pl.py = y; self.pl.vy = 0.;
            let (x, yy) = (self.pl.px as usize, (y - 0.001) as usize); self.moles[0].x = x; self.moles[0].y = yy; self.pl.sync = Some((x, yy)); self.pl.fall_from = yy;
            self.say("You climb up through the cage's floor hatch.".into()); self.events.push(Event::Click); return; } }
        if (up && !uw) || (down && !dw) { if let Some((k, s)) = self.lift_callable() { self.lift_haul(k, s); if self.tr.lifts[k].target.is_some() { self.say("You haul on the hand rope: the cage is coming.".into()); } } }
    }
    /// Start lift `k` toward stop row `t`, if it can be hauled (else say why, once).
    fn lift_haul(&mut self, k: usize, t: usize) {
        let b = self.lift_balance(k);
        if !b.haulable() { let s = if b.over() > 0. { format!("Too heavy to haul: the cage is {:.0} kg over its counterweight. Right mouse on the lift balances it with stone.", b.over()) }
                else { format!("The counterweight is {:.0} kg heavier than the cage: right mouse on the lift takes stone off.", -b.over()) };
            if self.pl.place_said.as_ref().is_some_and(|(t, k)| *t == s && self.tick < k + crate::world::DENY_REPEAT) { return; }
            self.pl.place_said = Some((s.clone(), self.tick)); self.say(s); return; }
        self.tr.lifts[k].target = Some(t); self.events.push(Event::Click);
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
    /// `dir` is the A/D input: pressed alone (no `W`/`S`) it lets go at once, so the mole walks or drops off
    /// sideways at its own speed. At the head of a ladder tied on a ledge, `W` doesn't grab (the rope above
    /// the lip is the knot, and the mole has already stepped off onto the ledge), and `S` there takes the mole
    /// onto the rope.
    pub(crate) fn ladder_climb(&mut self, ground: bool, jump_edge: bool, dir: i32) -> bool {
        let (up, down) = (self.input.up, self.input.down);
        let let_go = jump_edge || (dir != 0 && !up && !down);
        let k = self.ladder_on(self.pl.px, self.pl.py).or_else(|| if down && ground && !jump_edge { self.ladder_head_by(self.pl.px, self.pl.py) } else { None });
        let Some(k) = k else { self.tr.climbing = false; return false };
        let at_head = self.at_ladder_head(k, self.pl.py);
        let grab = (up && !at_head) || down;
        let was = self.tr.climbing;
        self.tr.climbing = !let_go && (grab || was && !ground);
        // stepping onto a ledge's ladder from its head: over the rope, clear of the rock it's tied to
        if self.tr.climbing && !was && at_head && down { let hx = self.ladder_hang_x(k); if !self.box_hits(hx, self.pl.py) { self.pl.px = hx; } }
        self.tr.climbing
    }
    /// The ladder the mole's body (feet at `py`) is on: the rope runs past its feet or its head.
    pub(crate) fn ladder_on(&self, px: f32, py: f32) -> Option<usize> {
        self.ladder_at(px as usize, (py - 1.) as usize).or_else(|| self.ladder_at(px as usize, (py - BODY_H) as usize))
    }
    /// A ladder tied on a ledge whose head is right beside a mole standing on that ledge.
    pub(crate) fn ladder_head_by(&self, px: f32, py: f32) -> Option<usize> {
        self.tr.ladders.iter().enumerate().position(|(k, l)| l.ax != l.x && (py - l.ay as f32).abs() <= 1. && (px - self.ladder_hang_x(k)).abs() <= 2. * BODY_HW + S as f32)
    }
    /// The mole (feet at `py`) is at the head of ladder `k`: on top of the ledge it's tied to, or level with it.
    fn at_ladder_head(&self, k: usize, py: f32) -> bool { let l = &self.tr.ladders[k]; l.ax != l.x && py <= l.ay as f32 + 0.5 }
    /// Where the mole's middle hangs on ladder `k`: on the rope, its body clear of the rock face the rope is
    /// tied over (a ceiling's ladder: on the rope itself).
    pub(crate) fn ladder_hang_x(&self, k: usize) -> f32 {
        let l = &self.tr.ladders[k];
        let c = l.x as f32 + 0.5;
        if l.ax < l.x { c.max((l.ax + 1) as f32 + BODY_HW + 0.01) } else if l.ax > l.x { c.min(l.ax as f32 - BODY_HW - 0.01) } else { c }
    }
    /// The rope's top beside the ledge it's tied to, when the mole climbing ladder `k` has reached it with `W`:
    /// the row to stand on and the side the ledge is on.
    pub(crate) fn ladder_top_out(&self, k: usize) -> Option<(f32, i8)> {
        let l = &self.tr.ladders[k]; if l.ax == l.x { return None; }
        Some((l.ay as f32, if l.ax < l.x { -1 } else { 1 }))
    }
    /// `W` here grabs a rope ladder (so it climbs instead of jumping): the mole is on a ladder, below its head.
    pub fn ladder_takes_up(&self) -> bool { self.ladder_on(self.pl.px, self.pl.py).map_or(false, |k| !self.at_ladder_head(k, self.pl.py)) }
    /// `S` here climbs down a rope ladder (so it doesn't dig the floor): the rope runs on below the mole's
    /// feet, or the mole stands at the head of a ledge's ladder.
    pub fn ladder_takes_down(&self) -> bool {
        let py = self.pl.py;
        self.ladder_on(self.pl.px, py).map_or(false, |k| { let l = &self.tr.ladders[k]; (l.top + l.len) as f32 > py + 1. && !self.box_hits(self.ladder_hang_x(k), py + 1.) })
            || self.ladder_head_by(self.pl.px, py).is_some()
    }

    /// One tick of every ladder and lift.
    pub(crate) fn step_lifts(&mut self) {
        // ladders: a ladder whose rock has gone falls; a floor that has risen under it (sand, a block) shortens it
        if self.tick % 15 == 0 {
            let mut k = 0;
            while k < self.tr.ladders.len() {
                let l = self.tr.ladders[k].clone();
                // (it falls from where a mole would hang on it, clear of the face it hung against)
                if self.passable(l.ax, l.ay) { let hx = self.ladder_hang_x(k); self.tr.ladders.remove(k); self.drops.push(crate::world::Drop::at(hx, (l.top + l.len / 2) as f32 + 0.5, Item::RopeLadder, 1)); self.say("A rope ladder fell.".into()); continue; }
                self.tr.ladders[k].len = self.ladder_len(l.x, l.top).max(1); k += 1;
            }
        }
        // stops follow the rock (a passage dug off the shaft becomes one)
        if self.tick % 30 == 0 { for k in 0..self.tr.lifts.len() { let l = &self.tr.lifts[k]; let st = self.lift_stops(l.x, l.w, l.head, l.bottom); self.tr.lifts[k].stops = st; } }
        self.lift_keys();
        let dt = 1. / 60.;
        for k in 0..self.tr.lifts.len() {
            let mut l = self.tr.lifts[k].clone(); l.py = l.y;
            // a head whose ceiling has gone falls, cage and all
            if l.head == 0 || self.passable(l.x, l.head - 1) { self.tr.lifts.remove(k); self.inv_drop(l.x, l.head, Item::Lift, l.ballast); self.say("A lift's head frame came down.".into()); return; }
            let (x0, _, x1, _) = l.cage();
            let b = self.lift_balance(k); let rider = b.rider;
            // hauled: speed up toward the stop and slow to stand level at it; otherwise the brake holds it
            let mut arrive = false;
            if let Some(t) = l.target {
                if !b.haulable() { l.target = None; l.v = 0.; }
                else {
                    let vmax = if b.powered { LIFT_POWER_V } else { LIFT_MAX };
                    // a key held the way it goes: no slowing for this stop, it runs on to the next
                    let dir = if self.input.up { -1. } else if self.input.down { 1. } else { 0. };
                    let end = if rider && dir != 0. && dir * (t as f32 - l.y) > 0. { if dir < 0. { l.stops.first() } else { l.stops.last() }.copied().unwrap_or(t) } else { t };
                    let d = t as f32 - l.y; let de = end as f32 - l.y; let want = d.signum() * vmax.min((2. * LIFT_ACC * de.abs()).sqrt());
                    l.v += (want - l.v).clamp(-LIFT_ACC * dt, LIFT_ACC * dt);
                    if d.abs() < 0.05 || (l.v * dt).abs() >= d.abs() && l.v * d > 0. { l.v = d / dt; arrive = true; }
                }
            } else { l.v = 0.; }
            // move, cell by cell; stop at the top, the floor, or anything in the shaft (a mole under the cage too)
            let dy = l.v * dt; let n = (dy.abs().ceil() as i32).max(1); let mut moved = 0.;
            let (mx, my) = (self.pl.px, self.pl.py); let under = !rider && self.moles.first().is_some_and(|m| m.alive) && mx + BODY_HW > x0 && mx - BODY_HW < x1;
            for _ in 0..n {
                let ny = l.y + dy / n as f32;
                let row = if dy > 0. { (ny + 1. - 1e-3).floor() as i32 } else { ny.floor() as i32 };
                let clear = (x0.floor() as i32..x1.ceil() as i32).all(|c| self.g.in_bounds(c, row) && self.passable(c as usize, row as usize));
                let mole = dy > 0. && under && ny + 1. > my - BODY_H && l.y + 1. <= my - BODY_H + 0.01;
                if ny < l.top() - 1e-3 || ny > (l.bottom - 1) as f32 + 1e-3 || !clear || mole {
                    if mole { self.say("The cage stops over your head: W climbs up through its hatch.".into()); }
                    // (it rests on a whole row, so whoever stands on it stands square)
                    l.v = 0.; l.target = None; arrive = false; l.y = if dy > 0. { l.y.floor() } else { l.y.ceil() }.clamp(l.top(), (l.bottom - 1) as f32); break; }
                l.y = ny;
            }
            moved = l.y - l.py;
            if arrive { if let Some(t) = l.target.take() { l.y = t as f32; moved = l.y - l.py;
                // a key still held runs on to the next stop that way, without stopping
                let dir = if self.input.up { -1 } else if self.input.down { 1 } else { 0 };
                let on = if rider && dir != 0 && (dir < 0) == ((t as f32) < l.py) { l.next_stop(dir) } else { None };
                if let Some(nt) = on { l.target = Some(nt); } else { l.v = 0.; self.events.push(Event::Click); } } }
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
