//! Wiring, devices, fuses and Thrumming on a 4×4-cell (1 m) block layer. Port of legacy/src/sand/logic.ts.
//! * Wires: union-find networks over 4-connected wire blocks. A net is ON if any adjacent source drives it.
//! * Sources: lever (toggle), pressure plate (mole or ≥3 powder cells), fluid sensor (≥4 liquid cells),
//!   Thrumming RX, and gate outputs (gates drive the wire block to their east).
//! * Gates read west (a) and north (b) inputs; outputs update one tick later (gate delay).
//! * Sinks: door / floodgate (cells become Door), pump (lifts liquid up through ≤16 blocks of rock),
//!   lamp (light source), TNT, Thrumming TX.
//! * TNT is inert when placed. Lighting it (right mouse, fire, lava) starts a short fuse, and water puts it out.
//!   Current on an adjacent copperline (lever, knot, listener) sets it off at once, as do a nearby blast and a
//!   fuse cord burning into it.
//! * Thrumming: link budget = P − distance(m) − Σ per-cell attenuation along the Bresenham ray (material
//!   `atten`), links ≥ 0 form a graph; TX reaches RX on the same channel directly or via repeaters.
//! * Lava (≥2 cells) or rubble (≥10 loose cells) in a block destroys its wire/device.
use crate::grid::Grid;
use crate::mat::{Kind, Mat, CPM, S};

pub const B: usize = CPM; // one block = 1 m
/// Cell counts below were set for a 4×4-cell block; A scales them to the block's area.
const A: usize = S * S;
/// Knock strength above the listener's threshold (dB).
pub const THRUM_POWER: f32 = 60.;
pub const FUSE_TICKS: i16 = 24;
/// A lit TNT charge goes off after this many ticks (1.5 s): time to run, short enough to feel snappy.
pub const TNT_FUSE: i16 = 90;

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Dev { None = 0, Lever, Plate, Sensor, Door, Floodgate, Pump, Lamp, Tnt, Fuse, And, Or, Not, Xor, Timer, Latch, Tx, Rx, Rep, Vent, Heat, Wheel }
pub const DEVS: [Dev; 21] = [Dev::Lever, Dev::Plate, Dev::Sensor, Dev::Door, Dev::Floodgate, Dev::Pump, Dev::Lamp, Dev::Tnt, Dev::Fuse, Dev::And, Dev::Or, Dev::Not, Dev::Xor, Dev::Timer, Dev::Latch, Dev::Tx, Dev::Rx, Dev::Rep, Dev::Vent, Dev::Heat, Dev::Wheel];
impl Dev {
    pub fn from_u8(v: u8) -> Dev { if v == 0 || v as usize > DEVS.len() { Dev::None } else { DEVS[v as usize - 1] } }
    pub fn name(self) -> &'static str { ["", "Lever", "Pressure plate", "Fluid sensor", "Door", "Floodgate", "Pump", "Lamp", "TNT", "Fuse", "Both-knot", "Either-knot", "Turn-knot", "Odd-knot", "Ticker", "Catch-knot", "Thrummer", "Listener", "Relay thrummer", "Vent", "Heatstone", "Water wheel"][self as usize] }
    /// One plain line for the hover tooltip. Right mouse sets or flips devices that have a setting.
    pub fn desc(self) -> &'static str {
        ["", "Flip it with right mouse to send current down the line.", "Sends current while something stands on it.",
         "Sends current when water or lava rises around it.", "Opens while its line has current.", "Holds back water; opens while its line has current.",
         "Pushes water along while its line has current.", "Lights while its line has current.", "Blasting charge. Goes off when current or a fuse reaches it.",
         "Burns along slowly, then sets off the next fuse or charge.", "Both-knot: current out only when both lines in carry it.",
         "Either-knot: current out when either line in carries it.", "Turn-knot: current out when its line in is quiet.",
         "Odd-knot: current out when exactly one line in carries it.", "Ticker: pulses on and off. Right mouse changes the beat.",
         "Catch-knot: one line sets it, the other lets it go.", "Thrummer: knocks its line's state into the rock. Right mouse picks the pattern.",
         "Listener: hears a thrummer knocking the same pattern.", "Relay thrummer: hears a pattern and knocks it on further.",
         "Draws stale air out of the tunnels around it.", "Heatstone: makes current while lava touches it.", "Water wheel: makes current while water flows past it."][self as usize]
    }
    pub fn is_gate(self) -> bool { (Dev::And as u8..=Dev::Latch as u8).contains(&(self as u8)) }
    pub fn is_thrum(self) -> bool { (Dev::Tx as u8..=Dev::Rep as u8).contains(&(self as u8)) }
    /// The pack item a device is built from (None: free).
    pub fn cost(self) -> Option<crate::craft::Item> { use crate::craft::Item; match self { Dev::Tnt => Some(Item::Tnt), Dev::Lamp => Some(Item::Lamp), Dev::Vent => Some(Item::Vent), Dev::Heat => Some(Item::IronIngot), Dev::Wheel => Some(Item::Wood), _ if self.is_thrum() => Some(Item::Thrummer), _ if self.is_gate() => Some(Item::Gate), _ => None } }
}

pub struct Logic {
    pub bw: usize, pub bh: usize,
    pub wire: Vec<u8>, pub dev: Vec<u8>, pub dev_p: Vec<i16>, pub dev_s: Vec<u8>, pub burn: Vec<i16>, pub powered: Vec<u8>,
    net_id: Vec<i32>, pub net_on: Vec<u8>, nets: usize, dirty: bool,
    pub dev_list: Vec<usize>, pub links: Vec<(usize, usize, f32)>, rx_on: Vec<u8>, tx_on: Vec<u8>, thrum_dirty: bool,
    pub booms: Vec<usize>, pub msgs: Vec<String>,
}
fn find(p: &mut [u32], mut a: usize) -> usize { while p[a] as usize != a { p[a] = p[p[a] as usize]; a = p[a] as usize; } a }

impl Logic {
    pub fn new(w: usize, h: usize) -> Logic {
        let (bw, bh) = (w / B, h / B); let n = bw * bh;
        Logic { bw, bh, wire: vec![0; n], dev: vec![0; n], dev_p: vec![0; n], dev_s: vec![0; n], burn: vec![0; n], powered: vec![0; n],
            net_id: vec![-1; n], net_on: vec![0], nets: 0, dirty: true, dev_list: vec![], links: vec![], rx_on: vec![0; n], tx_on: vec![0; n], thrum_dirty: true, booms: vec![], msgs: vec![] }
    }
    pub fn block_of(&self, x: usize, y: usize) -> usize { (y / B).min(self.bh - 1) * self.bw + (x / B).min(self.bw - 1) }
    pub fn centre(&self, b: usize) -> (usize, usize) { ((b % self.bw) * B + 2, (b / self.bw) * B + 2) }
    fn count(&self, g: &Grid, b: usize, f: impl Fn(usize) -> bool) -> usize {
        let (x0, y0) = ((b % self.bw) * B, (b / self.bw) * B); let mut n = 0;
        for y in y0..y0 + B { for x in x0..x0 + B { if f(y * g.w + x) { n += 1; } } } n
    }
    /// Slide all block layers by `br` block rows (+ = window moved deeper, contents move up).
    pub fn shift_rows(&mut self, br: i64) {
        let (bw, bh) = (self.bw, self.bh); let s = br.unsigned_abs() as usize; if s == 0 { return; }
        fn sh<T: Copy + Default>(v: &mut Vec<T>, bw: usize, bh: usize, s: usize, down: bool) {
            if s >= bh { v.iter_mut().for_each(|x| *x = T::default()); return; }
            let d = s * bw; let n = bw * bh;
            if down { v.copy_within(d.., 0); v[n - d..].iter_mut().for_each(|x| *x = T::default()); } else { v.copy_within(..n - d, d); v[..d].iter_mut().for_each(|x| *x = T::default()); }
        }
        let down = br > 0;
        sh(&mut self.wire, bw, bh, s, down); sh(&mut self.dev, bw, bh, s, down); sh(&mut self.dev_p, bw, bh, s, down); sh(&mut self.dev_s, bw, bh, s, down);
        sh(&mut self.burn, bw, bh, s, down); sh(&mut self.powered, bw, bh, s, down); sh(&mut self.rx_on, bw, bh, s, down); sh(&mut self.tx_on, bw, bh, s, down);
        self.invalidate();
    }
    pub fn invalidate(&mut self) { self.dirty = true; self.thrum_dirty = true; }
    fn mark(&mut self) { self.dirty = true; self.thrum_dirty = true; }

    // ---------- editing ----------
    /// Lay copperline on a block. False if there is already some, or a knot sits there (knots take no wire).
    pub fn place_wire(&mut self, b: usize) -> bool { if self.wire[b] != 0 || Dev::from_u8(self.dev[b]).is_gate() { return false; } self.wire[b] = 1; self.mark(); true }
    /// Clear a block's per-device state so a new device never inherits a burning fuse, a latch or a channel.
    fn reset(&mut self, b: usize) { self.dev_s[b] = 0; self.dev_p[b] = 0; self.burn[b] = 0; self.powered[b] = 0; self.rx_on[b] = 0; self.tx_on[b] = 0; }
    /// Returns false if the block is occupied or not open enough.
    pub fn place(&mut self, g: &mut Grid, d: Dev, b: usize) -> bool {
        if self.dev[b] != 0 || self.count(g, b, |i| g.is_solidish(i)) > 8 * A { return false; }
        self.reset(b); self.dev[b] = d as u8; self.dev_p[b] = if d == Dev::Timer { 40 } else if d.is_thrum() { 1 } else { 0 };
        if d.is_gate() { self.wire[b] = 0; }
        if d == Dev::Door { self.set_door(g, b, false); }
        self.mark(); true
    }
    pub fn remove(&mut self, g: &mut Grid, b: usize) -> Option<Dev> {
        let d = Dev::from_u8(self.dev[b]);
        if matches!(d, Dev::Door | Dev::Floodgate) { self.set_door(g, b, true); }
        if self.wire[b] != 0 || self.dev[b] != 0 { self.wire[b] = 0; self.dev[b] = 0; self.reset(b); self.mark(); }
        if d == Dev::None { None } else { Some(d) }
    }
    pub fn interact(&mut self, b: usize) {
        let d = Dev::from_u8(self.dev[b]);
        match d { Dev::Lever => self.dev_s[b] ^= 1, Dev::Timer => self.dev_p[b] = match self.dev_p[b] { 20 => 40, 40 => 80, _ => 20 },
            _ if d.is_thrum() => self.dev_p[b] = (self.dev_p[b] + 1) % 8, Dev::Fuse | Dev::Tnt => self.ignite(b), _ => {} }
        self.thrum_dirty = true;
    }
    /// Light a fuse cord or a TNT charge's fuse (no effect if it is already burning).
    pub fn ignite(&mut self, b: usize) { match Dev::from_u8(self.dev[b]) { Dev::Fuse if self.burn[b] == 0 => self.burn[b] = FUSE_TICKS, Dev::Tnt if self.burn[b] == 0 => self.burn[b] = TNT_FUSE, _ => {} } }
    /// Set a TNT charge off now (current, a blast nearby, a fuse cord burning into it).
    pub fn detonate(&mut self, b: usize) { if self.dev[b] == Dev::Tnt as u8 && !self.booms.contains(&b) { self.booms.push(b); } }
    pub fn lit(&self, b: usize) -> bool { self.burn[b] > 0 }
    pub fn on_explosion(&mut self, g: &mut Grid, cx: usize, cy: usize, r: f32) {
        for b in 0..self.wire.len() {
            if self.wire[b] == 0 && self.dev[b] == 0 { continue; }
            let (x, y) = self.centre(b); let d = ((x as f32 - cx as f32).powi(2) + (y as f32 - cy as f32).powi(2)).sqrt();
            let dv = Dev::from_u8(self.dev[b]);
            if dv == Dev::Tnt && d <= r + 6. { self.detonate(b); continue; }
            if dv == Dev::Fuse && d <= r + 4. { self.ignite(b); continue; }
            if d <= r * 0.8 { self.remove(g, b); }
        }
    }
    /// A mole's body (feet cell (x, y), 3 tall, about a cell either side) overlaps block b.
    pub fn occupied(&self, b: usize, moles: &[(usize, usize)]) -> bool {
        moles.iter().any(|&(x, y)| (x.saturating_sub(S)..=x + S).any(|xx| (0..3 * S).any(|k| y >= k && self.block_of(xx, y - k) == b)))
    }
    fn set_door(&self, g: &mut Grid, b: usize, open: bool) {
        let (x0, y0) = ((b % self.bw) * B, (b / self.bw) * B);
        for y in y0..y0 + B { for x in x0..x0 + B {
            let i = y * g.w + x; let m = g.m(i);
            if open { if m == Mat::Door { g.set(i, Mat::Empty); } }
            else if m.kind() != Kind::Solid || g.loose(i) {
                if m != Mat::Empty { for yy in (y0.saturating_sub(12 * S)..y0).rev() { let j = yy * g.w + x; if g.mat[j] == 0 { g.set(j, m); break; } } }
                g.set(i, Mat::Door);
            }
        } }
    }

    // ---------- networks ----------
    fn rebuild(&mut self) {
        self.dirty = false; let n = self.wire.len(); let bw = self.bw;
        let mut p: Vec<u32> = (0..n as u32).collect();
        for b in 0..n { if self.wire[b] == 0 { continue; }
            for c in [if b % bw < bw - 1 { b + 1 } else { usize::MAX }, b + bw] { if c < n && self.wire[c] != 0 { let (ra, rb) = (find(&mut p, b), find(&mut p, c)); if ra != rb { p[rb] = ra as u32; } } } }
        let mut map = std::collections::HashMap::new(); self.net_id.iter_mut().for_each(|v| *v = -1);
        for b in 0..n { if self.wire[b] != 0 { let r = find(&mut p, b); let l = map.len(); let id = *map.entry(r).or_insert(l); self.net_id[b] = id as i32; } }
        self.nets = map.len(); self.net_on = vec![0; self.nets.max(1)];
        self.dev_list = (0..n).filter(|&b| self.dev[b] != 0).collect();
    }
    fn nb(&self, b: usize) -> [usize; 5] { let (bw, n) = (self.bw, self.wire.len()); let x = b % bw;
        [b, if b >= bw { b - bw } else { usize::MAX }, if b + bw < n { b + bw } else { usize::MAX }, if x > 0 { b - 1 } else { usize::MAX }, if x < bw - 1 { b + 1 } else { usize::MAX }] }
    fn wire_on(&self, b: usize) -> bool { b < self.wire.len() && self.wire[b] != 0 && self.net_on[self.net_id[b] as usize] != 0 }
    fn is_powered(&self, b: usize) -> bool { self.nb(b).iter().any(|&c| self.wire_on(c)) }
    pub fn net_lit(&self, b: usize) -> bool { self.wire_on(b) }

    // ---------- per tick ----------
    pub fn step(&mut self, g: &mut Grid, moles: &[(usize, usize)], tick: u64) {
        if tick % 5 == 0 { self.check_severing(g); }
        if self.dirty { self.rebuild(); }
        if self.dev_list.is_empty() && self.nets == 0 { return; }
        self.step_fuses(g, tick);
        let mut on = vec![0u8; self.nets.max(1)];
        let list = self.dev_list.clone();
        for &b in &list {
            let d = Dev::from_u8(self.dev[b]);
            let out = match d {
                Dev::Lever => self.dev_s[b] == 1,
                Dev::Plate => { let o = self.occupied(b, moles) || self.count(g, b, |i| g.m(i).kind() == Kind::Powder || (g.loose(i) && g.m(i).kind() == Kind::Solid)) >= 3 * A; self.dev_s[b] = o as u8; o }
                Dev::Sensor => { let o = self.count(g, b, |i| g.m(i).kind() == Kind::Liquid) >= 4 * A; self.dev_s[b] = o as u8; o }
                Dev::Rx => { let o = self.rx_on[b] != 0; self.dev_s[b] = o as u8; o }
                // heatstone: thermoelement, live while lava touches its block or the blocks around it
                Dev::Heat => { let o = self.nb(b).iter().filter(|&&c| c != usize::MAX && c != b).map(|&c| self.count(g, c, |i| g.m(i) == Mat::Lava)).sum::<usize>() >= 2 * A; self.dev_s[b] = o as u8; o }
                // water wheel: turns while water moves through its block
                Dev::Wheel => { let o = self.count(g, b, |i| g.m(i) == Mat::Water && g.moved[i] == ((g.tick % 255) + 1) as u8) >= 2 * A; self.dev_s[b] = o as u8; o }
                _ => false,
            };
            if out { for c in self.nb(b) { if c < self.wire.len() && self.wire[c] != 0 { on[self.net_id[c] as usize] = 1; } } }
            if d.is_gate() && self.dev_s[b] != 0 && b % self.bw < self.bw - 1 && self.wire[b + 1] != 0 { on[self.net_id[b + 1] as usize] = 1; }
        }
        self.net_on = on;
        let mut next = vec![];
        for &b in &list {
            let d = Dev::from_u8(self.dev[b]); if !d.is_gate() { continue; }
            let a = b % self.bw > 0 && self.wire_on(b - 1); let c = b >= self.bw && self.wire_on(b - self.bw);
            let mut s = self.dev_s[b] != 0;
            match d { Dev::And => s = a && c, Dev::Or => s = a || c, Dev::Not => s = !a, Dev::Xor => s = a != c,
                Dev::Timer => { let p = if self.dev_p[b] > 0 { self.dev_p[b] as u64 } else { 40 }; let gated = b % self.bw == 0 || self.wire[b - 1] == 0 || a; s = (tick % p) < p / 2 && gated; }
                Dev::Latch => { if a { s = true } else if c { s = false } } _ => {} }
            next.push((b, s as u8));
        }
        for (b, s) in next { self.dev_s[b] = s; }
        if self.thrum_dirty || tick % 15 == 0 { self.rebuild_links(g); }
        self.propagate_thrum();
        for &b in &list {
            let d = Dev::from_u8(self.dev[b]); if d == Dev::None { continue; }
            let p = self.is_powered(b); self.powered[b] = p as u8;
            match d {
                // a door never shuts on a mole: it waits until the doorway is clear
                Dev::Door => if (self.dev_s[b] == 1) != p && (p || !self.occupied(b, moles)) { self.dev_s[b] = p as u8; self.set_door(g, b, p); },
                Dev::Floodgate => if (self.dev_s[b] == 1) != p && (!p || !self.occupied(b, moles)) { self.dev_s[b] = p as u8; self.set_door(g, b, !p); },
                Dev::Pump => if p { self.pump(g, b) },
                Dev::Lamp => self.dev_s[b] = p as u8,
                Dev::Tnt => if p { self.detonate(b) },
                Dev::Tx => self.tx_on[b] = p as u8,
                _ => {}
            }
        }
    }
    fn check_severing(&mut self, g: &mut Grid) {
        for b in 0..self.wire.len() {
            if self.wire[b] == 0 && self.dev[b] == 0 { continue; }
            let d = Dev::from_u8(self.dev[b]); if matches!(d, Dev::Door | Dev::Floodgate | Dev::Plate) { continue; }
            let lava = self.count(g, b, |i| g.mat[i] == Mat::Lava as u8);
            let buried = self.count(g, b, |i| g.m(i).kind() == Kind::Powder || (g.loose(i) && g.mat[i] != 0)) >= 10 * A;
            if matches!(d, Dev::Tnt | Dev::Fuse) { if lava > 0 { self.ignite(b); } continue; } // a buried charge stays put
            if lava >= 2 * A || buried {
                self.wire[b] = 0; self.dev[b] = 0; self.mark();
                self.msgs.push(format!("{} destroyed by {}", if d == Dev::None { "Wire" } else { d.name() }, if lava > 0 { "lava" } else { "cave-in" }));
            }
        }
    }
    fn step_fuses(&mut self, g: &Grid, _tick: u64) {
        let mut ign = vec![];
        for &b in &self.dev_list.clone() {
            let tnt = self.dev[b] == Dev::Tnt as u8;
            if self.dev[b] != Dev::Fuse as u8 && !tnt { continue; }
            if self.burn[b] == 0 && self.count(g, b, |i| g.mat[i] == Mat::Lava as u8 || g.mat[i] == Mat::Fire as u8) > 0 { self.ignite(b); }
            if self.burn[b] <= 0 { continue; }
            if self.count(g, b, |i| g.mat[i] == Mat::Water as u8) >= 4 * A { self.burn[b] = 0; self.msgs.push(format!("{} doused", if tnt { "TNT fuse" } else { "Fuse" })); continue; }
            self.burn[b] -= 1;
            if tnt { if self.burn[b] == 0 { self.detonate(b); } continue; }
            if self.burn[b] == 0 { self.dev[b] = 0; self.dirty = true;
                for n in self.nb(b) { if n >= self.dev.len() || n == b { continue; } if self.dev[n] == Dev::Fuse as u8 && self.burn[n] == 0 { ign.push(n); } if self.dev[n] == Dev::Tnt as u8 { if !self.booms.contains(&n) { self.booms.push(n); } } } }
        }
        for n in ign { self.burn[n] = FUSE_TICKS; }
    }
    fn pump(&self, g: &mut Grid, b: usize) {
        let bx = b % self.bw; let mut by = (b / self.bw) as i64 - 1; let mut crossed = false; let mut out = None;
        for _ in 0..16 { if by < 0 { break; } let ob = by as usize * self.bw + bx;
            if self.count(g, ob, |i| g.is_solidish(i)) >= 12 * A { crossed = true; } else if crossed && self.count(g, ob, |i| g.mat[i] == 0) > 0 { out = Some(ob); break; }
            by -= 1; }
        let Some(out) = out else { return };
        let (x0, y0, ox, oy) = (bx * B, (b / self.bw) * B, (out % self.bw) * B, (out / self.bw) * B); let mut moved = 0;
        for y in (y0..y0 + B).rev() { for x in x0..x0 + B { if moved >= 3 * A { return; }
            let i = y * g.w + x; let m = g.m(i); if m.kind() != Kind::Liquid { continue; }
            'o: for yy in (oy..oy + B).rev() { for xx in ox..ox + B { let j = yy * g.w + xx; if g.mat[j] == 0 { g.set(j, m); g.set(i, Mat::Empty); moved += 1; break 'o; } } }
        } }
    }
    pub fn link_budget(&self, g: &Grid, a: usize, b: usize) -> f32 {
        let (x0, y0) = self.centre(a); let (x1, y1) = self.centre(b);
        let (x0, y0, x1, y1) = (x0 as i64, y0 as i64, x1 as i64, y1 as i64);
        // 2D (cylindrical) spreading from 1 m, then absorption + boundary losses along the ray.
        // The thrummer and listener are braced against their own block, so those cells are skipped.
        let r_m = ((((x1 - x0).pow(2) + (y1 - y0).pow(2)) as f32).sqrt() * crate::mat::CELL_M).max(1.);
        let mut budget = THRUM_POWER - 10. * r_m.log10();
        let (dx, dy) = ((x1 - x0).abs(), -(y1 - y0).abs()); let (sx, sy) = (if x0 < x1 { 1 } else { -1 }, if y0 < y1 { 1 } else { -1 });
        let (mut x, mut y, mut err) = (x0, y0, dx + dy);
        let mut prev: Option<f32> = None;
        while !(x == x1 && y == y1) {
            let (ux, uy) = (x as usize, y as usize); let blk = self.block_of(ux, uy);
            if blk != a && blk != b {
                let i = uy * g.w + ux; let (z, att) = crate::mat::wave(g.m(i), g.loose(i));
                budget -= att; if let Some(pz) = prev { budget -= crate::mat::interface_db(pz, z); } prev = Some(z);
                if budget < 0. { return budget; }
            }
            let e2 = 2 * err; if e2 >= dy { err += dy; x += sx; } if e2 <= dx { err += dx; y += sy; }
        }
        budget
    }
    fn rebuild_links(&mut self, g: &Grid) {
        self.thrum_dirty = false; self.links.clear();
        let nodes: Vec<usize> = self.dev_list.iter().copied().filter(|&b| Dev::from_u8(self.dev[b]).is_thrum()).collect();
        for i in 0..nodes.len() { for j in i + 1..nodes.len() { let bud = self.link_budget(g, nodes[i], nodes[j]); if bud >= 0. { self.links.push((nodes[i], nodes[j], bud)); } } }
    }
    fn propagate_thrum(&mut self) {
        self.rx_on.iter_mut().for_each(|v| *v = 0);
        if self.links.is_empty() { return; }
        for &tx in &self.dev_list {
            if self.dev[tx] != Dev::Tx as u8 || self.tx_on[tx] == 0 { continue; }
            let ch = self.dev_p[tx]; let mut seen = vec![tx]; let mut q = vec![tx];
            while let Some(n) = q.pop() {
                for &(a, b, _) in &self.links { let m = if a == n { b } else if b == n { a } else { continue };
                    if seen.contains(&m) { continue; } seen.push(m);
                    let d = Dev::from_u8(self.dev[m]);
                    if d == Dev::Rx && self.dev_p[m] == ch { self.rx_on[m] = 1; }
                    if d == Dev::Rep { q.push(m); } }
            }
        }
    }
}
