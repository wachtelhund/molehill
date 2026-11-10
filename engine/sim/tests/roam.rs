//! The window slides sideways as it does down (`World::shift_x`): the Deepholt has no side walls, what
//! leaves the window is kept in its tiles and comes back, fresh ground is the same whichever way it is
//! reached, saves keep the window's column, and saves from before tiles (v11 and older) still load.
use molehill_sim::{band::{BH, TW}, craft::{Item, Station}, logic::Dev, mat::{Mat, CPM, S}, save, stream::band_cells, world::*};

/// The game-sized window (it slides sideways), with things built near its west side.
fn built() -> World {
    let mut w = World::new(1024 * S, 512 * S, 5); w.disasters.enabled = false;
    let (x, y) = (100, 300);
    for yy in y - 8..y + 8 { for xx in x - 4..x + 200 { let i = w.g.idx(xx, yy); w.g.set(i, Mat::Empty); } }
    let b = w.logic.block_of(x, y); let gg = &mut w.g; assert!(w.logic.place(gg, Dev::Lever, b));
    let c = w.logic.block_of(x + 20, y); w.logic.wire[c] = 1;
    w.torches.push((x + 60, y)); w.drops.push(Drop::at((x + 70) as f32 + 0.5, (y + 1) as f32 + 0.5, Item::Coal, 3));
    w.buildings.push(Building { kind: Station::Forge, x: x + 120, y: y + 6, built: true, claimed: false });
    w.farms.push(Farm { x: x + 170, y: y + 7, growth: 0.5, why: 0 });
    let i = w.g.idx(x + 30, y + 9); w.g.set(i, Mat::IronBeam);
    w
}
/// What stands in the west 64 m of the window, in world columns.
#[derive(Debug)]
struct Snap { cells: Vec<u8>, devs: Vec<(i64, usize, u8, u8)>, torches: Vec<(i64, usize)>, drops: Vec<(i64, usize, u32)>, blds: Vec<(i64, usize)>, farms: Vec<(i64, usize)> }
fn snap(w: &World, wx0: i64) -> Snap {
    let x0 = (wx0 - w.ox) as usize; let n = 2 * TW; let gw = w.g.w; let ax = |x: usize| w.ox + x as i64;
    let inx = |x: usize| x >= x0 && x < x0 + n;
    let mut t: Vec<(i64, usize)> = w.torches.iter().filter(|t| inx(t.0)).map(|&(x, y)| (ax(x), y)).collect(); t.sort();
    let bw = w.logic.bw;
    Snap { cells: (0..w.g.h).flat_map(|y| w.g.mat[y * gw + x0..y * gw + x0 + n].iter().copied()).collect(),
        devs: (0..w.logic.dev.len()).filter(|&b| (w.logic.dev[b] != 0 || w.logic.wire[b] != 0) && inx((b % bw) * CPM)).map(|b| (ax((b % bw) * CPM), b / bw, w.logic.dev[b], w.logic.wire[b])).collect(),
        torches: t, drops: sorted(w.drops.iter().filter(|d| inx(d.x)).map(|d| (ax(d.x), d.y, d.n)).collect()),
        blds: sorted(w.buildings.iter().filter(|b| inx(b.x)).map(|b| (ax(b.x), b.y)).collect()), farms: sorted(w.farms.iter().filter(|f| inx(f.x)).map(|f| (ax(f.x), f.y)).collect()) }
}
fn sorted<T: Ord>(mut v: Vec<T>) -> Vec<T> { v.sort(); v
}

/// The two snapshots agree, field by field (a cell mismatch reports where).
fn same(a: &Snap, b: &Snap, what: &str) {
    let n = 2 * TW; let bad: Vec<usize> = (0..a.cells.len()).filter(|&i| a.cells[i] != b.cells[i]).collect();
    assert!(bad.is_empty(), "{what}: {} cells differ, first at column {} row {} ({} vs {})", bad.len(), bad[0] % n, bad[0] / n, a.cells[bad[0]], b.cells[bad[0]]);
    assert_eq!(a.devs, b.devs, "{what}: devices"); assert_eq!(a.torches, b.torches, "{what}: torches"); assert_eq!(a.drops, b.drops, "{what}: drops");
    assert_eq!(a.blds, b.blds, "{what}: stations"); assert_eq!(a.farms, b.farms, "{what}: plots");
}

#[test]
fn no_side_walls_and_the_window_slides_sideways() {
    let mut w = World::new(1024 * S, 512 * S, 8); w.disasters.enabled = false;
    assert!(w.roams_x(), "the game-sized window slides sideways");
    assert!(!w.g.mat.iter().any(|&m| m == Mat::Bedrock as u8), "no bedrock walls down the sides");
    // the colony walks east: the window follows a tile at a time and the mole stays on the same rock
    let my = w.moles[0].y; let gw = w.g.w;
    for k in 0..6 { let x = gw - SIDE_MARGIN + 4 + k; let m = &mut w.moles[0]; m.x = x; m.y = my; w.step(); }
    for _ in 0..40 { w.step(); }
    assert!(w.ox >= TW as i64, "slid east: ox {}", w.ox);
    assert_eq!(w.ox % TW as i64, 0);
    assert!(w.moles[0].alive && w.moles[0].x < gw - SIDE_MARGIN, "the mole is back inside the margin: {}", w.moles[0].x);
    assert!(!w.g.mat.iter().any(|&m| m == Mat::Bedrock as u8), "nor in the columns that came in");
}

/// Devices, copperline, torches, drops, stations, plots and rock go with their tiles when the window
/// slides sideways, and come back when it slides back.
#[test]
fn things_survive_leaving_the_window_sideways() {
    let mut w = built(); let before = snap(&w, 0);
    w.shift_x(2 * TW as i64); assert_eq!(w.ox, 2 * TW as i64);
    assert_eq!(w.bands.len(), 2 * w.g.h / BH, "two tile columns stored");
    assert!(w.torches.iter().all(|t| w.ox + t.0 as i64 != 160), "the torch left with its tile");
    let west: Vec<(i64, i64)> = w.bands.keys().copied().collect(); assert!(west.iter().all(|k| k.0 < 2 * TW as i64));
    w.shift_x(-2 * (TW as i64)); assert_eq!(w.ox, 0);
    assert!(west.iter().all(|k| !w.bands.contains_key(k)), "back in the window, those tiles are no longer stored");
    assert!(w.bands.keys().all(|k| k.0 >= w.g.w as i64), "the east columns that left are stored now");
    same(&snap(&w, 0), &before, "back");
    // and further: west of the home columns, then back
    w.shift_x(-(TW as i64)); assert_eq!(w.ox, -(TW as i64)); w.shift_x(2 * TW as i64); w.shift_x(-(TW as i64));
    same(&snap(&w, 0), &before, "west and back");
}

/// A fresh tile is the generator's, shored on its own, whichever way the window comes onto it.
#[test]
fn fresh_ground_is_the_same_from_any_side() {
    let mut a = World::new(1024 * S, 512 * S, 12); a.disasters.enabled = false;
    let gw = a.g.w;
    a.shift_x(TW as i64);
    let key = (a.ox + (gw - TW) as i64, a.oy); let t = band_cells(&a.gen, TW, key);
    for y in 0..BH { let r = y * gw + gw - TW; assert_eq!(&a.g.mat[r..r + TW], &t.mat[y * TW..(y + 1) * TW], "row {y} of the tile that slid in"); }
    // the same tile reached from above (down, then east, then back up): its cells as built from the seed
    let mut b = World::new(1024 * S, 512 * S, 12); b.disasters.enabled = false;
    b.shift(SLIDE as i64); b.shift_x(TW as i64); b.shift(-(SLIDE as i64));
    let k = (key.0 - b.ox) as usize; let y0 = (key.1 - b.oy) as usize;
    for y in 0..BH { let r = (y0 + y) * gw + k; assert_eq!(&b.g.mat[r..r + TW], &t.mat[y * TW..(y + 1) * TW], "row {y} reached from above"); }
}

/// A save made far east keeps the window's column and its stored tiles, and the home columns come back.
#[test]
fn a_save_away_from_home_loads_back() {
    let mut w = built(); let before = snap(&w, 0);
    for _ in 0..3 { w.shift_x(TW as i64); } w.shift(SLIDE as i64);
    let mut l = save::load(&save::save(&w)).expect("load");
    assert_eq!((l.ox, l.oy), (w.ox, w.oy)); assert_eq!(l.bands.len(), w.bands.len());
    assert_eq!(l.pl.spawn, w.pl.spawn, "the burrow is remembered in world columns");
    l.shift(-(SLIDE as i64)); for _ in 0..3 { l.shift_x(-(TW as i64)); }
    same(&snap(&l, 0), &before, "loaded and back");
}

/// A save from before tiles (v11: full-width bands, no window column) loads: its bands are cut into
/// tiles, its window sits at the home columns, and the window then slides sideways like any other.
#[test]
fn v11_save_still_loads() {
    let mut w = save::load(include_bytes!("data/v11_cpm8.bin")).expect("v11 loads");
    assert_eq!(w.ox, 0); let tw = molehill_sim::band::tile_w(w.g.w);
    assert_eq!(w.bands.len(), 128 * S / BH * (w.g.w / tw), "its far bands became tiles");
    assert_eq!(w.g.get(20, 300), Mat::IronBeam, "the window is as saved");
    let far: Vec<(i64, i64)> = w.bands.keys().copied().collect();
    w.shift(-(128 * S as i64));
    assert!(far.iter().all(|k| !w.bands.contains_key(k)), "the far tiles are back in the window");
    assert!(w.torches.contains(&(160, 40)) && w.torches.contains(&(400, 44)), "torches from both halves of the far band: {:?}", w.torches);
    assert_eq!((w.g.get(300, 20), w.g.get(450, 30)), (Mat::IronBeam, Mat::IronBeam), "rock from both halves");
    assert!(w.buildings.iter().any(|b| b.x == 220 && b.kind == Station::Forge) && w.farms.iter().any(|f| f.x == 380));
    let b = w.logic.block_of(100, 40); assert_eq!(w.logic.dev[b], Dev::Lever as u8, "the lever came back");
    assert!(w.drops.iter().any(|d| d.x == 170 && d.n == 3));
    let l = save::load(&save::save(&w)).expect("and saves again as v12"); assert_eq!((l.ox, l.oy), (w.ox, w.oy));
}

/// A slide down in the middle of a fall: the body keeps falling at its speed (no stop in mid-air), and the
/// height already fallen still counts toward the landing.
#[test]
fn a_fall_carries_through_a_slide_down() {
    let mut w = World::new(1024 * S, 512 * S, 5); w.disasters.enabled = false;
    let (x, y0) = (w.moles[0].x, w.moles[0].y); let (gw, h) = (w.g.w, w.g.h); let start = w.oy + y0 as i64;
    for y in y0 - 3 * CPM..h { for xx in x - CPM..x + CPM { let i = y * gw + xx; w.g.set(i, Mat::Empty); } }
    let (mut vy, mut slid) = (0., false);
    for _ in 0..3000 { let oy = w.oy; vy = w.pl.vy; w.step(); if w.oy != oy { slid = true; break; } }
    assert!(slid, "the window slid during the fall");
    assert!(vy > 40. * CPM as f32 / 10., "falling fast before the slide: {vy}");
    w.step();
    assert!(w.pl.vy > 0.8 * vy, "still falling after the slide: {} (was {vy})", w.pl.vy);
    assert_eq!(w.oy + w.pl.fall_from as i64, start, "the fall is measured from where it began");
}

/// A found place stamped by a script (a scenario, the game's `--scene`) isn't in the generator, so a
/// slide's refresh used to forget it: gone from `sites` with its chest, and `--dump-art` on the pump and
/// halt scenes cropped some other site and panicked. It now stays while it touches the window, sideways
/// and down, leaves when it doesn't, and comes back with its chest.
#[test]
fn a_stamped_site_survives_a_slide() {
    use molehill_sim::sites::Kind;
    let mut w = World::new(1024 * S, 512 * S, 4242); w.disasters.enabled = false;
    let (sw, _) = Kind::Pumpworks.size(); let (x0, y0) = (w.g.w - sw - 4 * CPM, 200);
    w.stamp_site(Kind::Pumpworks, x0, y0);
    let st = *w.sites.last().unwrap(); let (ax, ay) = (st.x0, st.y0);
    let here = |w: &World| w.sites.iter().any(|s| s.kind == Kind::Pumpworks && s.x0 == ax && s.y0 == ay);
    let chest = |w: &World| st.chest.is_some_and(|(cx, cy)| w.chests.iter().any(|c| w.ox + c.x as i64 == cx && w.oy + c.y as i64 == cy));
    assert!(here(&w), "stamped");
    w.shift_x(TW as i64); assert!(here(&w), "after a slide east");
    if st.chest.is_some() { assert!(chest(&w), "its chest after a slide east"); }
    w.shift(BH as i64); assert!(here(&w), "after a slide down");
    w.shift(-(BH as i64)); w.shift_x(-(TW as i64)); assert!(here(&w), "back where it was");
    w.shift_x(-4 * (TW as i64)); assert!(!here(&w), "out of the window, not listed");
    w.shift_x(4 * TW as i64); assert!(here(&w), "back in the window");
    if st.chest.is_some() { assert!(chest(&w), "its chest back too"); }
}

/// A found place is furnished once. What the mole takes from it (its torches, benches, crops, the mine's carts with
/// their stone, its rope ladders) is not set out again by a reload, or by the window sliding with the place still in
/// it: each load used to hand the lot back, and a site ladder that had fallen was hung again to fall after every load,
/// dropping one more rope ladder each time. A place the window leaves keeps what it has too: its carts wait with
/// their stone (`tr.away`) and are back when the window is, and one the mole took is not set out again.
#[test] fn a_found_place_is_furnished_once() {
    use molehill_sim::sites::Kind;
    let mut w = World::new(1024 * S, 512 * S, 1); w.disasters.enabled = false;
    let mine = w.sites.iter().find(|s| s.kind == Kind::Mine).expect("seed 1 has a mine in the first window").clone();
    assert!(w.sites.iter().any(|s| s.kind == Kind::Holt), "and a holt");
    let count = |w: &World| (w.torches.len(), w.furniture.len(), w.farms.len(), w.tr.carts.len(), w.tr.ladders.len());
    let full = count(&w);
    assert!(full.0 >= 9 && full.1 >= 8 && full.2 >= 2 && full.3 == 2 && full.4 == 2, "the places are furnished (torches, furniture, farms, carts, ladders): {full:?}");
    assert!(mine.x0 - w.ox < TW as i64, "the mine is by the west side");
    // left alone, a place that leaves the window and comes back is as it was: nothing lost, nothing twice
    let mut u = save::load(&save::save(&w)).unwrap(); assert_eq!(count(&u), full, "a reload changes nothing");
    let loads = |w: &World| { let mut v: Vec<(i64, i64, u32)> = w.tr.carts.iter().map(|c| (w.ox + c.x as i64, w.oy + c.y as i64, c.load())).collect(); v.sort(); v };
    let stone = loads(&u); assert!(stone.iter().all(|c| c.2 > 0), "the mine's carts hold stone");
    u.shift_x(2 * TW as i64); assert_eq!((u.tr.carts.len(), u.tr.away.carts.len()), (0, 2), "the window left the mine's carts: they wait");
    u.shift_x(-2 * (TW as i64)); assert_eq!(count(&u), full, "back in the window the mine is as it was (torches, furniture, farms, carts, ladders)");
    assert_eq!(loads(&u), stone, "each cart where it stood, with its stone"); assert!(u.tr.away.is_empty());
    // the mole takes the lot
    w.torches.clear(); w.furniture.clear(); w.farms.clear(); w.tr.carts.clear(); w.tr.ladders.clear();
    let mut l = save::load(&save::save(&w)).unwrap(); assert_eq!(count(&l), (0, 0, 0, 0, 0), "a reload set things out again");
    for _ in 0..40 { l.step(); } assert!(!l.log.iter().any(|(_, s)| s.contains("rope ladder fell")), "no site ladder is hung again to fall");
    let l2 = save::load(&save::save(&l)).unwrap(); assert_eq!(count(&l2), (0, 0, 0, 0, 0), "a second reload set things out again");
    // a slide that keeps both places in the window
    w.shift(SLIDE as i64 / 2); w.shift(-(SLIDE as i64) / 2); assert_eq!(count(&w), (0, 0, 0, 0, 0), "a short slide set things out again");
    // nor does the mine leaving by the west side and coming back, nor a reload after that
    w.shift_x(2 * TW as i64); w.shift_x(-2 * (TW as i64));
    assert_eq!(count(&w), (0, 0, 0, 0, 0), "back in the window the mine was furnished again");
    let l = save::load(&save::save(&w)).unwrap(); assert_eq!(count(&l), (0, 0, 0, 0, 0));
}

/// A cart the window leaves waits with its cargo (`tr.away`) and is back, standing still, when the window covers
/// it again: down and back, sideways and back, and through a save made while it is away. It used to be lost with
/// the rows it stood in, cargo and all, on any trip 70 m down from it.
#[test] fn a_cart_waits_with_its_cargo() {
    use molehill_sim::{craft::Item, mat::Mat, transport::Cart};
    for sideways in [false, true] {
        let mut w = World::new(512 * S, 512 * S, 4); w.disasters.enabled = false;
        // a granite floor with headroom in the middle of the window, a cart on it rolling east
        let (cx, fy) = (w.g.w / 2, w.g.h / 2);
        for y in fy - 40..fy + 8 { for x in cx - 40..cx + 40 { let i = w.g.idx(x, y); w.g.set(i, if y >= fy { Mat::Granite } else { Mat::Empty }); w.g.flags[i] = 0; } }
        w.tr.carts.clear();
        w.tr.carts.push(Cart { x: cx as f32, y: fy as f32, px: cx as f32, py: fy as f32, v: 3., vy: 0., dir: 1, on_rail: false, cargo: vec![(Item::Stone, 7), (Item::Coal, 3)], tip: 0 });
        let at = |w: &World| { let c = &w.tr.carts[0]; (w.ox + c.x as i64, w.oy + c.y as i64) };
        let home = at(&w);
        let step = if sideways { TW as i64 } else { SLIDE as i64 }; let n = if sideways { w.g.w } else { w.g.h } as i64 / step;
        let go = |w: &mut World, d: i64| if sideways { w.shift_x(d) } else { w.shift(d) };
        for _ in 0..n { go(&mut w, step); }
        assert_eq!((w.tr.carts.len(), w.tr.away.carts.len()), (0, 1), "it waits (sideways: {sideways})");
        // saved and loaded while it is away
        let mut l = save::load(&save::save(&w)).expect("loads"); assert_eq!((l.tr.carts.len(), l.tr.away.carts.len()), (0, 1), "it waits in the save");
        for w in [&mut w, &mut l] {
            for _ in 0..n { go(w, -step); }
            assert_eq!((w.tr.carts.len(), w.tr.away.carts.len()), (1, 0), "it is back (sideways: {sideways})");
            let c = &w.tr.carts[0];
            assert_eq!(at(w), home, "where it stood"); assert_eq!(c.cargo, vec![(Item::Stone, 7), (Item::Coal, 3)], "with its cargo"); assert_eq!(c.v, 0., "standing still");
            for _ in 0..60 { w.step(); }
            assert_eq!(at(w), home, "and it stays there, on its floor"); assert_eq!(w.tr.carts[0].load(), 10);
        }
    }
}

/// A transport blob from before carts waited (v15) still loads: it ends where the waiting lifts do.
#[test] fn a_v15_transport_blob_still_loads() {
    let mut w = World::new(512 * S, 512 * S, 4); w.disasters.enabled = false;
    let carts = w.tr.carts.len(); assert!(w.tr.away.is_empty());
    let mut b = w.encode_transport(); assert_eq!(b[0], 16); assert_eq!(b[b.len() - 4..], [0; 4], "no cart waits: the blob ends with their count");
    b.truncate(b.len() - 4); b[0] = 15;
    let mut l = World::blank(w.g.w, w.g.h, 4); l.decode_transport(&b).expect("the v15 blob loads");
    assert_eq!(l.tr.carts.len(), carts); assert!(l.tr.away.is_empty());
}

/// A save from before sites were marked as furnished (v13 and older) loads with the places in its window as they
/// were left: each load of such a save had furnished them already, so nothing the mole took is set out again.
#[test] fn an_old_saves_places_stay_as_they_were_left() {
    let mut w = World::new(1024 * S, 512 * S, 1); w.disasters.enabled = false;
    let whole = w.sites_stamped.len(); assert!(whole >= 2 && !w.torches.is_empty() && !w.tr.carts.is_empty(), "seed 1 has a mine and a holt in the first window");
    w.torches.clear(); w.furniture.clear(); w.farms.clear(); w.tr.carts.clear(); w.tr.ladders.clear();
    // today's save made a v13 one: the list of furnished sites cut out (marked so it can be found), the version put back
    w.sites_stamped = [0xA5A5_0001, 0xA5A5_0002].into();
    let mut b = save::save(&w);
    let list: Vec<u8> = [2u32, 0xA5A5_0001, 0xA5A5_0002].iter().flat_map(|v| v.to_le_bytes()).collect();
    let at: Vec<usize> = (0..b.len() - 12).filter(|&k| b[k..k + 12] == list[..]).collect(); assert_eq!(at.len(), 1, "the list is in the save once");
    assert_eq!(b[8..12], save::VERSION.to_le_bytes());
    // (a v14 one had a second list after it, the sites whose carts were set: it loads with the first kept)
    let mut b14 = b.clone(); b14.splice(at[0] + 12..at[0] + 12, [1u32, 0xA5A5_0003].iter().flat_map(|v| v.to_le_bytes())); b14[8..12].copy_from_slice(&14u32.to_le_bytes());
    let l14 = save::load(&b14).expect("the v14 save loads"); assert!(l14.sites_stamped.is_superset(&w.sites_stamped) && !l14.sites_stamped.contains(&0xA5A5_0003), "a v14 save's furnished sites");
    b.drain(at[0]..at[0] + 12); b[8..12].copy_from_slice(&13u32.to_le_bytes());
    let mut l = save::load(&b).expect("the v13 save loads");
    let count = |w: &World| (w.torches.len(), w.furniture.len(), w.farms.len(), w.tr.carts.len(), w.tr.ladders.len());
    assert_eq!(count(&l), (0, 0, 0, 0, 0), "the old save's places were furnished again (torches, furniture, farms, carts, ladders)");
    assert_eq!(l.sites_stamped.len(), whole, "the places in the window count as furnished");
    l.shift(SLIDE as i64 / 2); assert_eq!(count(&l), (0, 0, 0, 0, 0), "and a slide leaves them so");
}
