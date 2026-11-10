//! The window sliding sideways (`World::shift_x`): the column twin of every `shift_*` the row slide runs.
//! Each thing with an x moves with the rock; one that would leave the window is dropped, as a slide
//! down drops what leaves the top (a vehicle is not kept in a tile). Ropes, rope ladders and lifts wait in
//! `tr.away` instead and come back with the window, and so do carts (`away.rs`). Cell-index lists
//! (tube paths, gate and cast cells, drum cells) are re-indexed.
use crate::world::World;

/// Cell index `i` of a `w`-wide window after a slide of `cols` (+ = east: contents move west), if it stays in.
fn reindex(i: usize, w: usize, cols: i64) -> Option<usize> {
    let (x, y) = ((i % w) as i64 - cols, i / w); (x >= 0 && x < w as i64).then(|| y * w + x as usize)
}

impl World {
    pub(crate) fn shift_x_transport(&mut self, cols: i64, w: usize) {
        let (dx, ww) = (cols as f32, w as f32); let inx = |x: f32, m: f32| x >= m && x < ww - m;
        let col = |x: usize, m: usize| -> Option<usize> { let v = x as i64 - cols; (v >= m as i64 && v + (m as i64) < w as i64).then_some(v as usize) };
        // carts: one that leaves waits in `tr.away` with its cargo
        self.slide_carts(cols, 0);
        // the borer (its head, cab and path) and its wagons move together in this one pass
        let riding = self.tr.borer_ride; let (mut keep, mut nr) = (vec![], None);
        for (k, mut b) in std::mem::take(&mut self.tr.borers).into_iter().enumerate() { b.shift(dx, 0.); if !inx(b.x, 2.) { continue; } if riding == Some(k) { nr = Some(keep.len()); } keep.push(b); }
        self.tr.borers = keep; self.tr.borer_ride = nr;
        let riding = self.tr.wagon_ride; let (mut keep, mut nr) = (vec![], None);
        for (j, mut wg) in std::mem::take(&mut self.tr.wagons).into_iter().enumerate() { wg.x -= dx; wg.px -= dx; if !inx(wg.x, 2.) { continue; } if riding == Some(j) { nr = Some(keep.len()); } keep.push(wg); }
        self.tr.wagons = keep; self.tr.wagon_ride = nr; if nr.is_none() { self.tr.bench = None; }
        // tubes: hoppers and capsules (their paths are cell indices)
        self.tr.hoppers.retain_mut(|hp| { let (Some(x), Some(t)) = (col(hp.x, 1), reindex(hp.tube, w, cols)) else { return false }; if x + crate::tubes::HOPPER + 1 >= w { return false; } hp.x = x; hp.tube = t; true });
        let ride = self.tr.in_tube; let (mut keep, mut nr) = (vec![], None);
        for (k, mut c) in std::mem::take(&mut self.tr.capsules).into_iter().enumerate() {
            let path: Option<Vec<u32>> = c.path.iter().map(|&p| reindex(p as usize, w, cols).map(|q| q as u32)).collect();
            let Some(path) = path else { continue }; if c.to >= self.tr.hoppers.len() { continue; }
            c.path = path; if ride == Some(k) { nr = Some(keep.len()); } keep.push(c);
        }
        self.tr.capsules = keep; self.tr.in_tube = nr;
        // ore hoppers, boilers, works and their casts
        self.tr.bins.retain_mut(|b| { let Some(x) = col(b.x, 2) else { return false }; b.x = x; true });
        self.tr.boilers.retain_mut(|b| { let Some(x) = col(b.x, 4) else { return false }; b.x = x; b.engines.clear(); true });
        self.tr.works.retain_mut(|wk| { let (hw, _) = wk.kind.size(); let Some(x) = col(wk.x, hw / 2 + 2) else { return false }; wk.x = x; true });
        self.tr.casts.retain_mut(|c| { for i in c.cells.iter_mut() { let Some(j) = reindex(*i, w, cols) else { return false }; *i = j; } true });
        // ropes (a hook still in the air goes back into the pack, not lost) and ladders and lifts: the mole
        // keeps hold of a rope that stays in the window, as on a slide up or down; one that leaves waits in
        // `tr.away` and comes back with the window (`away.rs`)
        self.slide_ropes(cols, 0);
        let (ox, oy) = self.origin_before(cols, 0); let h = self.g.h;
        for l in std::mem::take(&mut self.tr.ladders) { match l.slid(cols, 0, w, h) { Some(l) => self.tr.ladders.push(l), None => self.tr.away.ladders.push(crate::away::Away { ox, oy, it: l }) } }
        for l in std::mem::take(&mut self.tr.lifts) { match l.slid(cols, 0, w, h) { Some(l) => self.tr.lifts.push(l), None => self.tr.away.lifts.push(crate::away::Away { ox, oy, it: l }) } }
        // mechanical power: weights hang from drum cells; the trains are found again
        self.tr.power.weights.retain_mut(|wt| { let (Some(x), Some(d)) = (col(wt.x, 1), reindex(wt.drum, w, cols)) else { return false }; wt.x = x; wt.drum = d; true });
        self.tr.power.train.clear(); self.tr.power.cells.clear(); self.tr.power.scanned = 0;
        // linkage: gates are cell lists, scales stand on a column
        self.tr.links.gates.retain_mut(|g| { for i in g.cells.iter_mut() { let Some(j) = reindex(*i, w, cols) else { return false }; *i = j; } true });
        self.tr.links.scales.retain_mut(|s| { let Some(x) = col(s.x, 2) else { return false }; s.x = x; true });
        self.tr.links.pulled.clear();
    }
}
