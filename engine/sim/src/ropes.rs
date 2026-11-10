//! The rope gun: a crank-wound spring throws a stone hook trailing a fungus-fibre rope (see PROGRESSION.md).
//!
//! * **Firing** (left mouse with the rope gun in hand, aimed at the cursor): one hook rope (`Item::HookRope`)
//!   leaves the pack. The hook flies at `HOOK_V`, falls under gravity and pays the rope out behind it.
//! * **Biting**: the hook bites into solid, settled rock or built block (anything `Kind::Solid` that isn't
//!   loose) and holds there for good. It glances off sand, gravel and loose rubble (and is spent: it bites
//!   nothing after), and sinks in water or lava: then it fails, and the hook rope falls where it stopped, to be picked up again. Past `ROPE_M` from
//!   the gun the rope runs out and the hook drops short (a failed shot too).
//! * **The rope** is a chain of `NSEG` segments (`SEG` long) moved by verlet: it hangs, goes slack, swings and
//!   lies on the rock it touches. It stays in the world (it is saved) until the rock the hook bit goes, when
//!   it falls as a hook rope item, or until the mole takes it back with `E` (the hook rope returns to the pack).
//! * **Climbing**: `W` with the body on the rope grabs it. On the rope `W`/`S` climb (`ROPE_UP`, `ROPE_DOWN`)
//!   and `A`/`D` swing: the mole is a pendulum on the rope's length below the hook. `Space` lets go (with a
//!   little hop and the swing's speed); climbing down onto the floor lets go too. A rope never counts as a fall.
use crate::craft::Item;
use crate::mat::{Kind, CPM};
use crate::world::{Event, World, BODY_H, BODY_HW};

/// Rope length (m: a big hall's ceiling from its floor) and the cells per segment (0.5 m); `NSEG` segments make one rope.
pub const ROPE_M: f32 = 16.; pub const SEG: f32 = 0.5 * CPM as f32; pub const NSEG: usize = 32;
/// The spring throws the hook at 20 m/s (cells/s), and it falls at 9.81 m/s².
pub const HOOK_V: f32 = 20. * CPM as f32; pub const HOOK_G: f32 = 9.81 * CPM as f32;
/// Climbing a rope (cells/s): 2 m/s up, 3 m/s down (a rope ladder is 3 up, 3.5 down: it has rungs).
pub const ROPE_UP: f32 = 2. * CPM as f32; pub const ROPE_DOWN: f32 = 3. * CPM as f32;
/// Pumping a swing (cells/s², pushed while it swings the key's way) with A/D, and the highest a mole hangs under the hook (m of rope).
pub const SWING_A: f32 = 16. * CPM as f32; pub const ROPE_MIN: f32 = 0.5 * CPM as f32;
/// A hook in the air longer than this (ticks) has failed.
const HOOK_TICKS: u32 = 120;

/// A rope fixed in the rock: `pts[0]` is the hook at the anchor, `pts[NSEG]` the free end.
#[derive(Clone, Debug)]
pub struct Rope { pub pts: Vec<(f32, f32)>, pub prev: Vec<(f32, f32)>, /** the rock cell the hook bit */ pub ax: usize, pub ay: usize }
/// The hook in flight: position, velocity (cells/s), where it was fired from, and its rope paying out.
#[derive(Clone, Debug)]
pub struct Hook { pub x: f32, pub y: f32, pub vx: f32, pub vy: f32, pub ox: f32, pub oy: f32, pub t: u32, pub pts: Vec<(f32, f32)>, pub prev: Vec<(f32, f32)> }

/// The mole's hand (where it holds the rope or the gun): near the top of its body.
pub fn hand(px: f32, py: f32) -> (f32, f32) { (px, py - BODY_H * 0.85) }

impl World {
    /// Is there rock for the rope to lie on at (x, y)? (Grains in flight don't hold it up.)
    fn rope_blocked(&self, x: f32, y: f32) -> bool {
        let (cx, cy) = (x.floor() as i32, y.floor() as i32);
        !self.g.in_bounds(cx, cy) || !self.passable(cx as usize, cy as usize) && !self.in_flight(cx as usize, cy as usize)
    }
    /// Fire the rope gun from the mole's hand toward `aim` (cells). Uses one hook rope from the pack.
    pub fn fire_rope_gun(&mut self, aim: (f32, f32)) -> Result<(), &'static str> {
        if self.moles.first().map_or(true, |m| !m.alive) { return Err("not now"); }
        if self.inv[Item::RopeGun as usize] == 0 { return Err("no rope gun"); }
        if self.inv[Item::HookRope as usize] == 0 { return Err("no hook rope left (workbench: 3 fungus, 1 stone)"); }
        if self.tr.hook.is_some() { return Err("the hook is still flying"); }
        let (ox, oy) = hand(self.pl.px, self.pl.py);
        let (dx, dy) = (aim.0 - ox, aim.1 - oy); let d = (dx * dx + dy * dy).sqrt(); if d < 0.5 { return Err("aim away from the mole"); }
        self.inv[Item::HookRope as usize] -= 1;
        let pts = vec![(ox, oy); NSEG + 1];
        self.tr.hook = Some(Hook { x: ox, y: oy, vx: dx / d * HOOK_V, vy: dy / d * HOOK_V, ox, oy, t: 0, prev: pts.clone(), pts });
        self.events.push(Event::Place); Ok(())
    }
    /// The rope `k`'s nearest point to (x, y) and how far it is (cells).
    fn rope_near(&self, k: usize, x: f32, y: f32) -> (usize, f32) {
        self.tr.ropes[k].pts.iter().enumerate().map(|(j, p)| (j, ((p.0 - x).powi(2) + (p.1 - y).powi(2)).sqrt())).min_by(|a, b| a.1.total_cmp(&b.1)).unwrap_or((0, f32::MAX))
    }
    /// A rope the mole's body (feet at `py`) can grab: the rope runs within 0.5 m of its body.
    pub(crate) fn rope_by(&self, px: f32, py: f32) -> Option<usize> {
        (0..self.tr.ropes.len()).find(|&k| self.tr.ropes[k].pts.iter().any(|p| (p.0 - px).abs() <= BODY_HW + 0.5 * CPM as f32 && p.1 >= py - BODY_H - 0.5 * CPM as f32 && p.1 <= py))
    }
    /// `W` here grabs a rope (so it climbs instead of jumping).
    pub fn rope_takes_up(&self) -> bool { self.tr.on_rope.is_some() || self.rope_by(self.pl.px, self.pl.py).is_some() }
    /// The rope (at `x`, `y` in reach) under the cursor, for `E`: back into the pack as a hook rope.
    pub(crate) fn pick_up_rope(&mut self, x: usize, y: usize) -> Option<Item> {
        let (xf, yf) = (x as f32 + 0.5, y as f32 + 0.5);
        let k = (0..self.tr.ropes.len()).find(|&k| self.rope_near(k, xf, yf).1 <= 0.3 * CPM as f32)?;
        self.tr.ropes.remove(k); self.let_go_rope_removed(k);
        self.inv[Item::HookRope as usize] += 1; Some(Item::HookRope)
    }
    fn let_go_rope_removed(&mut self, k: usize) {
        self.tr.on_rope = match self.tr.on_rope { Some((r, _)) if r == k => None, Some((r, s)) if r > k => Some((r - 1, s)), o => o };
    }
    /// The mole on a rope this tick: grabs with `W` while touching one, lets go with a jump, or when it
    /// climbs down onto the floor. Returns the rope and the length of rope above the hands.
    pub(crate) fn rope_hold(&mut self, ground: bool, jump_edge: bool) -> Option<(usize, f32)> {
        let (up, down) = (self.input.up, self.input.down);
        let (hx, hy) = hand(self.pl.px, self.pl.py);
        if let Some((k, s)) = self.tr.on_rope {
            if jump_edge || (ground && !up) { self.tr.on_rope = None; if jump_edge { self.pl.vy = self.pl.vy.min(0.) - 0.5 * crate::world::TUNE.jump_v; } return None; }
            let _ = down; return Some((k, s));
        }
        let fresh = up && !self.pl.up_was;
        // W grabs: held, in the air; a fresh press from the floor (so W held through climbing off the top onto a
        // ledge doesn't grab the rope again); never while rolling over a ledge
        if !up || self.pl.mantle > 0. || ground && !fresh { return None; }
        let k = self.rope_by(self.pl.px, self.pl.py)?;
        let (ax, ay) = self.tr.ropes[k].pts[0];
        let s = (((hx - ax).powi(2) + (hy - ay).powi(2)).sqrt()).clamp(ROPE_MIN, NSEG as f32 * SEG);
        self.tr.on_rope = Some((k, s)); Some((k, s))
    }
    /// The mole's velocity on rope `k`, `s` cells of it above the hands: gravity, A/D pumping the swing, W/S
    /// climbing, and the rope pulling the hands back onto a circle of radius `s` round the hook.
    pub(crate) fn rope_swing(&mut self, k: usize, s: f32, dir: i32, dt: f32) {
        let t = crate::world::TUNE;
        let s = (s + if self.input.up { -ROPE_UP * dt } else if self.input.down { ROPE_DOWN * dt } else { 0. }).clamp(ROPE_MIN, NSEG as f32 * SEG);
        self.tr.on_rope = Some((k, s));
        let (ax, ay) = self.tr.ropes[k].pts[0];
        let (hx, hy) = hand(self.pl.px, self.pl.py);
        let (mut vx, mut vy) = (self.pl.vx, self.pl.vy + t.gravity * dt);
        // A/D pump the swing along the rope's tangent; with no key the swing dies away slowly
        let (rx, ry) = (hx - ax, hy - ay); let r = (rx * rx + ry * ry).sqrt().max(1e-3);
        // (a push only while the swing already goes that way, below the hook: held, each swing goes higher)
        if dir != 0 { let (tx, ty) = (-ry / r, rx / r); let sg = if tx * dir as f32 >= 0. { 1. } else { -1. };
            let along = (vx * tx + vy * ty) * sg; if (along > 0. || vx.abs() < 0.5 * CPM as f32) && ry > 0. { vx += tx * sg * SWING_A * dt; vy += ty * sg * SWING_A * dt; } }
        else { vx *= 1. - 0.6 * dt; }
        let (nx, ny) = (hx + vx * dt, hy + vy * dt); let (qx, qy) = (nx - ax, ny - ay); let d = (qx * qx + qy * qy).sqrt();
        if d > s { let (cx, cy) = (ax + qx * s / d, ay + qy * s / d); vx = (cx - hx) / dt; vy = (cy - hy) / dt; }
        let cap = 30. * CPM as f32; self.pl.vx = vx.clamp(-cap, cap); self.pl.vy = vy.clamp(-cap, cap);
    }

    /// One tick of the hook in flight and every rope (from `step_transport`).
    pub(crate) fn step_ropes(&mut self) {
        let dt = 1. / 60.;
        if let Some(mut h) = self.tr.hook.take() {
            h.t += 1; h.vy += HOOK_G * dt;
            let n = ((h.vx.abs().max(h.vy.abs()) * dt).ceil() as i32).max(1);
            let mut done: Option<bool> = None; // Some(true) bit the rock, Some(false) failed
            for _ in 0..n {
                let (nx, ny) = (h.x + h.vx * dt / n as f32, h.y + h.vy * dt / n as f32);
                let (cx, cy) = (nx.floor() as i32, ny.floor() as i32);
                if !self.g.in_bounds(cx, cy) { done = Some(false); break; }
                let i = self.g.idx(cx as usize, cy as usize); let m = self.g.m(i);
                if m.kind() == Kind::Liquid { done = Some(false); break; } // sinks: no bite
                if self.passable(cx as usize, cy as usize) || self.in_flight(cx as usize, cy as usize) { h.x = nx; h.y = ny; continue; }
                if m.kind() == Kind::Solid && !self.g.loose(i) {
                    // bit the rock: the hook sits on the face of the cell it struck
                    let r = self.anchor_rope(&h, cx as usize, cy as usize); done = Some(r); break;
                }
                // sand, gravel, rubble: it glances off, spent (the hook rope bounces away and falls)
                let (bx, by) = (self.rope_blocked(nx, h.y), self.rope_blocked(h.x, ny));
                if bx || !by { h.vx = -h.vx * 0.3; } if by || !bx { h.vy = -h.vy * 0.3; }
                done = Some(false); break;
            }
            // the rope pays out behind the hook; past its length it runs out and the hook drops short
            let paid = ((h.x - h.ox).powi(2) + (h.y - h.oy).powi(2)).sqrt();
            if done.is_none() && (paid > ROPE_M * CPM as f32 || h.t > HOOK_TICKS) { done = Some(false); }
            match done {
                Some(true) => { self.events.push(Event::Click); }
                Some(false) => { let mut d = crate::world::Drop::at(h.x, h.y.clamp(2., self.g.h as f32 - 2.), Item::HookRope, 1); d.vx = h.vx; d.vy = h.vy; self.drops.push(d); self.say("The hook didn't bite. It glances off loose ground and sinks in water: aim at solid rock.".into()); }
                None => {
                    let (gx, gy) = hand(self.pl.px, self.pl.py);
                    let out = ((paid / SEG).ceil() as usize + 1).min(NSEG);
                    let (mut pts, mut prev) = (std::mem::take(&mut h.pts), std::mem::take(&mut h.prev));
                    pts[0] = (h.x, h.y); prev[0] = pts[0];
                    for j in out..=NSEG { pts[j] = (gx, gy); prev[j] = (gx, gy); } // still on the spool, at the gun
                    self.verlet(&mut pts, &mut prev, &[(0, (h.x, h.y)), (NSEG, (gx, gy))]);
                    h.pts = pts; h.prev = prev; self.tr.hook = Some(h);
                }
            }
        }
        // ropes: a hook whose rock has gone lets go, and the rope falls as a hook rope
        if self.tick % 15 == 0 {
            let mut k = 0;
            while k < self.tr.ropes.len() {
                let r = &self.tr.ropes[k]; let i = self.g.idx(r.ax, r.ay);
                if self.g.m(i).kind() != Kind::Solid || self.g.loose(i) {
                    let p = r.pts[NSEG / 2]; self.tr.ropes.remove(k); self.let_go_rope_removed(k);
                    self.drops.push(crate::world::Drop::at(p.0, p.1, Item::HookRope, 1)); self.say("A rope came down: the rock its hook bit has gone.".into()); continue;
                }
                k += 1;
            }
        }
        let held = self.tr.on_rope.map(|(k, s)| (k, ((s / SEG).round() as usize).clamp(1, NSEG), hand(self.pl.px, self.pl.py)));
        for k in 0..self.tr.ropes.len() {
            let (mut pts, mut prev) = (std::mem::take(&mut self.tr.ropes[k].pts), std::mem::take(&mut self.tr.ropes[k].prev));
            let a = pts[0];
            let pins: Vec<(usize, (f32, f32))> = match held { Some((hk, j, p)) if hk == k => vec![(0, a), (j, p)], _ => vec![(0, a)] };
            self.verlet(&mut pts, &mut prev, &pins);
            self.tr.ropes[k].pts = pts; self.tr.ropes[k].prev = prev;
        }
    }
    /// The hook bit rock cell (cx, cy): fix its rope there (the hook on the face it struck).
    fn anchor_rope(&mut self, h: &Hook, cx: usize, cy: usize) -> bool {
        let (hx, hy) = (h.x.clamp(cx as f32 - 0.5, cx as f32 + 1.5), h.y.clamp(cy as f32 - 0.5, cy as f32 + 1.5));
        let mut pts = h.pts.clone(); pts[0] = (hx, hy); let prev = pts.clone();
        self.tr.ropes.push(Rope { pts, prev, ax: cx, ay: cy }); true
    }
    /// Verlet for a rope's points: gravity, a little drag, segments no longer than `SEG` (a rope goes slack but
    /// doesn't stretch), the pinned points held, and every point kept out of the rock.
    fn verlet(&self, pts: &mut [(f32, f32)], prev: &mut [(f32, f32)], pins: &[(usize, (f32, f32))]) {
        let dt = 1. / 60.; let g = HOOK_G * dt * dt;
        let pinned = |j: usize| pins.iter().find(|p| p.0 == j).map(|p| p.1);
        for j in 0..pts.len() {
            if let Some(p) = pinned(j) { pts[j] = p; prev[j] = p; continue; }
            let (x, y) = pts[j]; let (vx, vy) = ((x - prev[j].0) * 0.98, (y - prev[j].1) * 0.98);
            prev[j] = (x, y); let (nx, ny) = (x + vx, y + vy + g);
            pts[j] = self.rope_move((x, y), (nx, ny));
            if pts[j] != (nx, ny) { prev[j] = (prev[j].0 + (pts[j].0 - prev[j].0) * 0.3, pts[j].1); } // friction where it lies
        }
        for _ in 0..12 {
            for j in 0..pts.len() - 1 {
                let (a, b) = (pts[j], pts[j + 1]); let (dx, dy) = (b.0 - a.0, b.1 - a.1); let d = (dx * dx + dy * dy).sqrt();
                if d <= SEG || d < 1e-6 { continue; }
                let (pa, pb) = (pinned(j).is_some(), pinned(j + 1).is_some()); if pa && pb { continue; }
                let e = (d - SEG) / d; let (wa, wb) = if pa { (0., 1.) } else if pb { (1., 0.) } else { (0.5, 0.5) };
                let na = (a.0 + dx * e * wa, a.1 + dy * e * wa); let nb = (b.0 - dx * e * wb, b.1 - dy * e * wb);
                if !pa { pts[j] = self.rope_move(a, na); } if !pb { pts[j + 1] = self.rope_move(b, nb); }
            }
        }
    }
    /// Move a rope point from `a` toward `b`, stopping short of rock (sliding along it on the free axis).
    fn rope_move(&self, a: (f32, f32), b: (f32, f32)) -> (f32, f32) {
        if !self.rope_blocked(b.0, b.1) { return b; }
        if self.rope_blocked(a.0, a.1) { return a; } // buried (sand fell on it): stays where it is
        if !self.rope_blocked(b.0, a.1) { return (b.0, a.1); }
        if !self.rope_blocked(a.0, b.1) { return (a.0, b.1); }
        a
    }
    /// Slide the ropes by `rows` (the window moved deeper); a rope that leaves the window waits in `tr.away`
    /// (`away.rs`). The mole keeps hold of a rope that stays (climbing past the window's margin slides it mid-climb).
    pub(crate) fn shift_ropes(&mut self, rows: i64) { self.slide_ropes(0, rows); }
    /// The same for a slide of `cols` east and `rows` down.
    pub(crate) fn slide_ropes(&mut self, cols: i64, rows: i64) {
        let (w, h) = (self.g.w, self.g.h); let (ox, oy) = self.origin_before(cols, rows);
        // a hook still in the air when the window slides: back into the pack, not lost
        if self.tr.hook.take().is_some() { self.inv[Item::HookRope as usize] += 1; }
        let held = self.tr.on_rope.take(); let mut keep = vec![];
        for (k, r) in std::mem::take(&mut self.tr.ropes).into_iter().enumerate() {
            let Some(r) = r.slid(cols, rows, w, h) else { self.tr.away.ropes.push(crate::away::Away { ox, oy, it: r }); continue };
            if let Some((hk, s)) = held { if hk == k { self.tr.on_rope = Some((keep.len(), s)); } }
            keep.push(r);
        }
        self.tr.ropes = keep;
    }
    pub(crate) fn encode_ropes(&self, o: &mut crate::band::Wr) {
        o.u32(self.tr.ropes.len() as u32); for r in &self.tr.ropes { write_rope(o, r); }
    }
    pub(crate) fn decode_ropes(&mut self, r: &mut crate::band::Rd) -> Result<(), String> {
        let (w, h) = (self.g.w, self.g.h);
        for _ in 0..r.len(4096)? { if let Some(rp) = read_rope(r, w, h)? { self.tr.ropes.push(rp); } }
        Ok(())
    }
}
pub(crate) fn write_rope(o: &mut crate::band::Wr, r: &Rope) { o.u32(r.ax as u32); o.u32(r.ay as u32); o.u32(r.pts.len() as u32); for p in &r.pts { o.f32(p.0); o.f32(p.1); } }
/// One saved rope (None: not one a `w` × `h` window can hold).
pub(crate) fn read_rope(r: &mut crate::band::Rd, w: usize, h: usize) -> Result<Option<Rope>, String> {
    let (ax, ay, n) = (r.u32()? as usize, r.u32()? as usize, r.len(1024)?);
    let mut pts = Vec::with_capacity(n); for _ in 0..n { pts.push((r.f32()?, r.f32()?)); }
    Ok((ax < w && ay < h && n == NSEG + 1 && pts.iter().all(|p| p.0.is_finite() && p.1.is_finite())).then(|| Rope { prev: pts.clone(), pts, ax, ay }))
}
