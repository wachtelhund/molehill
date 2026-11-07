//! The window sliding sideways (`World::shift_x`): the column twin of every `shift_*` the row slide runs.
//! Each thing with an x moves with the rock; one that would leave the window is dropped, as a slide
//! down drops what leaves the top (a vehicle, a rope or a lift is not kept in a tile). Cell-index lists
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
        // carts
        let riding = self.tr.riding; let (mut keep, mut nr) = (vec![], None);
        for (k, mut c) in std::mem::take(&mut self.tr.carts).into_iter().enumerate() { c.x -= dx; c.px -= dx; if !inx(c.x, 2.) { continue; } if riding == Some(k) { nr = Some(keep.len()); } keep.push(c); }
        self.tr.carts = keep; self.tr.riding = nr;
        // the borer and its wagons move together in this one pass (a train is found by spacing, so it stays coupled)
        let riding = self.tr.borer_ride; let (mut keep, mut nr) = (vec![], None);
        for (k, mut b) in std::mem::take(&mut self.tr.borers).into_iter().enumerate() { b.x -= dx; b.px -= dx; if !inx(b.x, 2.) { continue; } if riding == Some(k) { nr = Some(keep.len()); } keep.push(b); }
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
        // ropes (a hook still in the air goes back into the pack, not lost) and ladders and lifts
        self.tr.on_rope = None; if self.tr.hook.take().is_some() { self.inv[crate::craft::Item::HookRope as usize] += 1; }
        self.tr.ropes.retain_mut(|r| { let Some(ax) = col(r.ax, 1) else { return false }; if r.pts.iter().any(|p| !inx(p.0 - dx, 1.)) { return false; }
            r.ax = ax; for p in r.pts.iter_mut().chain(r.prev.iter_mut()) { p.0 -= dx; } true });
        self.tr.ladders.retain_mut(|l| { let (Some(x), Some(ax)) = (col(l.x, 1), col(l.ax, 0)) else { return false }; l.x = x; l.ax = ax; true });
        self.tr.lifts.retain_mut(|l| { let Some(x) = col(l.x, 4) else { return false }; l.x = x; true });
        // mechanical power: weights hang from drum cells; the trains are found again
        self.tr.power.weights.retain_mut(|wt| { let (Some(x), Some(d)) = (col(wt.x, 1), reindex(wt.drum, w, cols)) else { return false }; wt.x = x; wt.drum = d; true });
        self.tr.power.train.clear(); self.tr.power.cells.clear(); self.tr.power.scanned = 0;
        // linkage: gates are cell lists, scales stand on a column
        self.tr.links.gates.retain_mut(|g| { for i in g.cells.iter_mut() { let Some(j) = reindex(*i, w, cols) else { return false }; *i = j; } true });
        self.tr.links.scales.retain_mut(|s| { let Some(x) = col(s.x, 2) else { return false }; s.x = x; true });
        self.tr.links.pulled.clear();
    }
}
