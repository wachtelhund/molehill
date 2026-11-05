//! Loose items as physics bodies: things dug out, dropped and thrown fly, bounce, slide and roll on the
//! material they hit (`bounce`, `friction` in the material table), float or sink in water and lava by
//! density, burn up in heat, and fall asleep once they lie still. A thrown TNT charge is lit: it flies,
//! bounces and rolls, and goes off where its fuse runs out (water douses it; lava or fire lights one).
use crate::craft::Item;
use crate::logic::TNT_FUSE;
use crate::mat::{Kind, Mat, SF};
use crate::world::{Drop, Event, World, BODY_H, TUNE};

/// Half the side of an item's box (cells): a loose item is a 0.25 m lump.
pub const ITEM_HW: f32 = 0.5 * SF;
/// Ticks an item must lie still before it sleeps (no physics until something moves under it).
pub const REST_TICKS: u8 = 6;
/// Throw speed (cells/s) at no charge and at full charge; a full throw carries about 6 m on the level.
pub const THROW_MIN: f32 = 20. * SF;
pub const THROW_MAX: f32 = 60. * SF;
/// Ticks after a throw before the thrower can pick the item back up.
pub const PICK_HOLD: u16 = 30;

impl Drop {
    /// A loose item at rest with its centre at (px, py) (cells).
    pub fn at(px: f32, py: f32, item: Item, n: u32) -> Drop {
        Drop { x: px.max(0.) as usize, y: py.max(0.) as usize, item, n, claimed: false, px, py, ppx: px, ppy: py, vx: 0., vy: 0., rest: 0, hold: 0, fuse: 0 }
    }
    pub fn asleep(&self) -> bool { self.rest >= REST_TICKS }
}

impl World {
    /// Does an item box centred at (px, py) overlap anything solid?
    fn item_hits(&self, px: f32, py: f32) -> bool {
        let (x0, x1) = ((px - ITEM_HW).floor() as i32, (px + ITEM_HW - 1e-3).floor() as i32);
        let (y0, y1) = ((py - ITEM_HW).floor() as i32, (py + ITEM_HW - 1e-3).floor() as i32);
        for y in y0..=y1 { for x in x0..=x1 { if !self.g.in_bounds(x, y) || !self.passable(x as usize, y as usize) { return true; } } }
        false
    }
    fn mat_at(&self, px: f32, py: f32) -> Mat { let (x, y) = (px.floor() as i32, py.floor() as i32); if self.g.in_bounds(x, y) { self.g.get(x as usize, y as usize) } else { Mat::Bedrock } }
    /// How much of the box is in liquid (0..1, from three samples down its middle) and the densest liquid there.
    fn soak(&self, px: f32, py: f32) -> (f32, Mat) {
        let mut n = 0; let mut liq = Mat::Empty;
        for dy in [-ITEM_HW * 0.8, 0., ITEM_HW * 0.8] { let m = self.mat_at(px, py + dy);
            if m.kind() == Kind::Liquid { n += 1; if m.p().density > liq.p().density || liq == Mat::Empty { liq = m; } } }
        (n as f32 / 3., liq)
    }

    /// Throw one `it` from the pack toward `aim` (a direction in cells, from the mole), with `power` 0..1 of
    /// the full throw. A TNT charge goes out lit.
    pub fn throw_item(&mut self, it: Item, aim: (f32, f32), power: f32) -> Result<(), &'static str> {
        if !self.moles.first().map_or(false, |m| m.alive) { return Err("no mole"); }
        if self.inv[it as usize] == 0 { return Err("none left"); }
        if !it.uses().throw { return Err("that isn't something to throw"); }
        let len = (aim.0 * aim.0 + aim.1 * aim.1).sqrt(); if len < 1e-3 { return Err("no aim"); }
        self.inv[it as usize] -= 1;
        let sp = THROW_MIN + (THROW_MAX - THROW_MIN) * power.clamp(0., 1.);
        let mut d = Drop::at(self.pl.px, self.pl.py - BODY_H * 0.7, it, 1);
        d.vx = aim.0 / len * sp + self.pl.vx * 0.5; d.vy = aim.1 / len * sp + self.pl.vy.min(0.) * 0.5; d.hold = PICK_HOLD;
        if it == Item::Tnt { d.fuse = TNT_FUSE; }
        self.moles[0].facing = if aim.0 >= 0. { 1 } else { -1 };
        self.drops.push(d); self.events.push(Event::Click); Ok(())
    }

    /// A blast at (cx, cy) with radius r throws loose items outward.
    pub(crate) fn fling_items(&mut self, cx: f32, cy: f32, r: f32) {
        for d in self.drops.iter_mut() {
            let (dx, dy) = (d.px - cx, d.py - cy); let dist = (dx * dx + dy * dy).sqrt(); if dist > r * 1.5 { continue; }
            let k = 60. * SF * (1. - dist / (r * 1.5)); let (ux, uy) = if dist > 1e-3 { (dx / dist, dy / dist) } else { (0., -1.) };
            d.vx += ux * k; d.vy += uy * k - 10. * SF; d.rest = 0;
        }
    }

    /// One tick of every loose item.
    pub(crate) fn step_items(&mut self) {
        let dt = 1. / 60.; let t = TUNE; let mut gone = vec![]; let mut booms = vec![];
        for k in 0..self.drops.len() {
            let mut d = std::mem::replace(&mut self.drops[k], Drop::at(0., 0., Item::Dirt, 0));
            d.ppx = d.px; d.ppy = d.py; d.hold = d.hold.saturating_sub(1);
            let here = self.mat_at(d.px, d.py); let heat = here.p().heat;
            let (wet, liq) = self.soak(d.px, d.py);
            // heat burns things up and lights a charge; water puts a lit one out
            if heat > 0 && d.item.burns(heat) { gone.push(k); self.drops[k] = d; continue; }
            if d.item == Item::Tnt {
                if heat > 0 && d.fuse == 0 { d.fuse = TNT_FUSE; }
                if d.fuse > 0 && liq == Mat::Water && wet > 0. { d.fuse = 0; self.say("TNT fuse doused".into()); }
                if d.fuse > 0 { d.fuse -= 1; if d.fuse == 0 { booms.push(k); self.drops[k] = d; continue; } }
            }
            let supported = self.item_hits(d.px, d.py + 0.05);
            if d.asleep() { if supported && wet == 0. { self.drops[k] = d; continue; } d.rest = 0; } // something gave way: wake
            // gravity less buoyancy (by the share of the box under liquid), and drag in a liquid (thicker in lava)
            let rho = d.item.density();
            let lift = if wet > 0. { wet * liq.p().density / rho } else { 0. };
            d.vy = (d.vy + t.gravity * (1. - lift) * dt).clamp(-t.terminal, t.terminal);
            if wet > 0. { let drag = 1. - wet * 0.12 * 10. / liq.p().flow_in_10.max(1) as f32; d.vx *= drag; d.vy *= drag; }
            // swept, axis-separated movement; bounce and skid on what it hits
            let (mx, my) = (d.vx * dt, d.vy * dt);
            let n = ((mx.abs().max(my.abs()) / (0.5 * ITEM_HW)).ceil() as i32).max(1);
            // (an axis stops for the rest of the tick once it hits; an impact also costs speed along the
            // surface in proportion to the knock, so a hard landing doesn't skid on for metres)
            let (mut hx, mut hy) = (mx == 0., my == 0.);
            for _ in 0..n {
                if !hx { let nx = d.px + mx / n as f32; if !self.item_hits(nx, d.py) { d.px = nx; } else {
                    hx = true; let s = self.mat_at(nx + mx.signum() * ITEM_HW, d.py).p(); let v0 = d.vx; d.vx = -v0 * s.bounce;
                    let fx = if mx > 0. { (nx + ITEM_HW).floor() - ITEM_HW - 1e-3 } else { (nx - ITEM_HW).ceil() + ITEM_HW }; if !self.item_hits(fx, d.py) { d.px = fx; } // flush against it
                    let f = s.friction * (v0 - d.vx).abs(); d.vy = if d.vy.abs() <= f { 0. } else { d.vy - d.vy.signum() * f }; } }
                if !hy { let ny = d.py + my / n as f32; if !self.item_hits(d.px, ny) { d.py = ny; } else {
                    hy = true; let s = self.mat_at(d.px, ny + my.signum() * ITEM_HW).p(); let v0 = d.vy;
                    let fy = if my > 0. { (ny + ITEM_HW).floor() - ITEM_HW - 1e-3 } else { (ny - ITEM_HW).ceil() + ITEM_HW }; if !self.item_hits(d.px, fy) { d.py = fy; } // flush against it
                    let b = -v0 * s.bounce; d.vy = if b.abs() < 6. * SF { 0. } else { b };
                    let f = s.friction * (v0 - d.vy).abs(); d.vx = if d.vx.abs() <= f { 0. } else { d.vx - d.vx.signum() * f }; } }
            }
            // sliding friction on the ground (soft earth grabs, stone and iron let it roll on)
            let floor = self.mat_at(d.px, d.py + ITEM_HW + 0.05);
            if self.item_hits(d.px, d.py + 0.05) && d.vy >= 0. {
                let f = floor.p().friction * t.gravity * dt; d.vx = if d.vx.abs() <= f { 0. } else { d.vx - d.vx.signum() * f };
                if d.vx.abs() < 0.3 * SF && d.vy.abs() < 0.3 * SF && wet == 0. { d.rest = d.rest.saturating_add(1); if d.asleep() { d.vx = 0.; d.vy = 0.; } } else { d.rest = 0; }
            } else { d.rest = 0; }
            d.px = d.px.clamp(1., self.g.w as f32 - 2.); d.py = d.py.clamp(1., self.g.h as f32 - 2.);
            d.x = d.px as usize; d.y = d.py as usize;
            self.drops[k] = d;
        }
        for &k in &booms { let (x, y) = (self.drops[k].x, self.drops[k].y); self.drops[k].n = 0; self.blast_tnt(x, y); }
        for k in gone { self.drops[k].n = 0; }
        self.drops.retain(|d| d.n > 0);
    }
}
